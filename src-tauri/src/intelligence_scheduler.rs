use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const RECENT_EVENT_LIMIT: usize = 80;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerJobSnapshot {
    pub id: String,
    pub connection_id: String,
    pub connection_name: String,
    pub label: String,
    pub status: String,
    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub wait_ms: u64,
    pub run_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerEventSnapshot {
    pub sequence: u64,
    pub job_id: String,
    pub connection_name: String,
    pub label: String,
    pub status: String,
    pub timestamp_ms: u64,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerSnapshot {
    pub revision: u64,
    pub active_count: usize,
    pub queued_count: usize,
    pub jobs: Vec<SchedulerJobSnapshot>,
    pub recent_events: Vec<SchedulerEventSnapshot>,
}

#[derive(Debug)]
struct ScheduledJob {
    id: String,
    connection_id: String,
    connection_name: String,
    label: String,
    status: &'static str,
    queued_at: Instant,
    queued_at_ms: u64,
    started_at: Option<Instant>,
    started_at_ms: Option<u64>,
}

#[derive(Default)]
struct SchedulerState {
    revision: u64,
    jobs: VecDeque<ScheduledJob>,
    active_by_connection: HashMap<String, String>,
    recent_events: VecDeque<SchedulerEventSnapshot>,
}

struct Scheduler {
    state: Mutex<SchedulerState>,
    changed: Condvar,
    next_job_id: AtomicU64,
    next_event_sequence: AtomicU64,
}

static SCHEDULER: OnceLock<Scheduler> = OnceLock::new();

fn scheduler() -> &'static Scheduler {
    SCHEDULER.get_or_init(|| Scheduler {
        state: Mutex::new(SchedulerState::default()),
        changed: Condvar::new(),
        next_job_id: AtomicU64::new(1),
        next_event_sequence: AtomicU64::new(1),
    })
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

fn push_event(
    scheduler: &Scheduler,
    state: &mut SchedulerState,
    job_id: &str,
    connection_name: &str,
    label: &str,
    status: &str,
    detail: Option<String>,
) {
    state.revision = state.revision.saturating_add(1);
    state.recent_events.push_front(SchedulerEventSnapshot {
        sequence: scheduler
            .next_event_sequence
            .fetch_add(1, Ordering::Relaxed),
        job_id: job_id.to_string(),
        connection_name: connection_name.to_string(),
        label: label.to_string(),
        status: status.to_string(),
        timestamp_ms: unix_time_ms(),
        detail,
    });
    state.recent_events.truncate(RECENT_EVENT_LIMIT);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerCompletion {
    Succeeded,
    Failed,
    Cancelled,
}

impl SchedulerCompletion {
    fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

pub struct SchedulerPermit {
    job_id: String,
    finished: bool,
}

impl SchedulerPermit {
    pub fn finish(&mut self, completion: SchedulerCompletion, detail: Option<String>) {
        if self.finished {
            return;
        }
        self.finished = true;
        finish_job(&self.job_id, completion, detail);
    }
}

impl Drop for SchedulerPermit {
    fn drop(&mut self) {
        if !self.finished {
            finish_job(
                &self.job_id,
                SchedulerCompletion::Failed,
                Some("Execution ended unexpectedly".to_string()),
            );
        }
    }
}

fn finish_job(job_id: &str, completion: SchedulerCompletion, detail: Option<String>) {
    let scheduler = scheduler();
    let mut state = scheduler
        .state
        .lock()
        .expect("intelligence scheduler poisoned");
    let Some(position) = state.jobs.iter().position(|job| job.id == job_id) else {
        return;
    };
    let job = state
        .jobs
        .remove(position)
        .expect("scheduled job disappeared");
    state.active_by_connection.remove(&job.connection_id);
    push_event(
        scheduler,
        &mut state,
        &job.id,
        &job.connection_name,
        &job.label,
        completion.as_str(),
        detail,
    );
    scheduler.changed.notify_all();
}

pub fn acquire(
    connection_id: &str,
    connection_name: &str,
    label: &str,
    cancellation: Option<&AtomicBool>,
) -> Result<SchedulerPermit, ()> {
    let scheduler = scheduler();
    let id = format!(
        "job-{}",
        scheduler.next_job_id.fetch_add(1, Ordering::Relaxed)
    );
    let job = ScheduledJob {
        id: id.clone(),
        connection_id: connection_id.to_string(),
        connection_name: connection_name.to_string(),
        label: label.to_string(),
        status: "queued",
        queued_at: Instant::now(),
        queued_at_ms: unix_time_ms(),
        started_at: None,
        started_at_ms: None,
    };
    let mut state = scheduler
        .state
        .lock()
        .expect("intelligence scheduler poisoned");
    state.jobs.push_back(job);
    push_event(
        scheduler,
        &mut state,
        &id,
        connection_name,
        label,
        "queued",
        None,
    );
    scheduler.changed.notify_all();

    loop {
        if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            if let Some(position) = state.jobs.iter().position(|job| job.id == id) {
                let job = state.jobs.remove(position).expect("queued job disappeared");
                push_event(
                    scheduler,
                    &mut state,
                    &job.id,
                    &job.connection_name,
                    &job.label,
                    "cancelled",
                    Some("Cancelled while queued".to_string()),
                );
            }
            scheduler.changed.notify_all();
            return Err(());
        }

        let is_connection_idle = !state.active_by_connection.contains_key(connection_id);
        let is_first_for_connection = state
            .jobs
            .iter()
            .find(|job| job.connection_id == connection_id && job.status == "queued")
            .is_some_and(|job| job.id == id);
        if is_connection_idle && is_first_for_connection {
            let (job_id, job_connection_name, job_label) = {
                let job = state
                    .jobs
                    .iter_mut()
                    .find(|job| job.id == id)
                    .expect("queued job disappeared");
                job.status = "running";
                job.started_at = Some(Instant::now());
                job.started_at_ms = Some(unix_time_ms());
                (
                    job.id.clone(),
                    job.connection_name.clone(),
                    job.label.clone(),
                )
            };
            state
                .active_by_connection
                .insert(connection_id.to_string(), id.clone());
            push_event(
                scheduler,
                &mut state,
                &job_id,
                &job_connection_name,
                &job_label,
                "running",
                None,
            );
            return Ok(SchedulerPermit {
                job_id: id,
                finished: false,
            });
        }

        let (next_state, _) = scheduler
            .changed
            .wait_timeout(state, WAIT_POLL_INTERVAL)
            .expect("intelligence scheduler poisoned while waiting");
        state = next_state;
    }
}

pub fn snapshot() -> SchedulerSnapshot {
    let scheduler = scheduler();
    let state = scheduler
        .state
        .lock()
        .expect("intelligence scheduler poisoned");
    let now = Instant::now();
    let jobs = state
        .jobs
        .iter()
        .map(|job| SchedulerJobSnapshot {
            id: job.id.clone(),
            connection_id: job.connection_id.clone(),
            connection_name: job.connection_name.clone(),
            label: job.label.clone(),
            status: job.status.to_string(),
            queued_at_ms: job.queued_at_ms,
            started_at_ms: job.started_at_ms,
            wait_ms: job
                .started_at
                .unwrap_or(now)
                .saturating_duration_since(job.queued_at)
                .as_millis()
                .min(u64::MAX as u128) as u64,
            run_ms: job
                .started_at
                .map(|started| now.saturating_duration_since(started).as_millis())
                .unwrap_or(0)
                .min(u64::MAX as u128) as u64,
        })
        .collect::<Vec<_>>();
    SchedulerSnapshot {
        revision: state.revision,
        active_count: state.active_by_connection.len(),
        queued_count: jobs.iter().filter(|job| job.status == "queued").count(),
        jobs,
        recent_events: state.recent_events.iter().cloned().collect(),
    }
}

#[cfg(test)]
pub fn reset_for_tests() {
    let scheduler = scheduler();
    let mut state = scheduler
        .state
        .lock()
        .expect("intelligence scheduler poisoned");
    *state = SchedulerState::default();
    scheduler.next_job_id.store(1, Ordering::Relaxed);
    scheduler.next_event_sequence.store(1, Ordering::Relaxed);
    scheduler.changed.notify_all();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn same_connection_is_fifo_while_other_connections_run_independently() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_for_tests();
        let first = acquire("a", "A", "first", None).unwrap();
        let other = acquire("b", "B", "other", None).unwrap();
        assert_eq!(snapshot().active_count, 2);

        let acquired = Arc::new(AtomicBool::new(false));
        let acquired_in_thread = Arc::clone(&acquired);
        let waiter = thread::spawn(move || {
            let _second = acquire("a", "A", "second", None).unwrap();
            acquired_in_thread.store(true, Ordering::Release);
        });
        thread::sleep(Duration::from_millis(40));
        assert!(!acquired.load(Ordering::Acquire));
        drop(first);
        waiter.join().unwrap();
        assert!(acquired.load(Ordering::Acquire));
        drop(other);
    }

    #[test]
    fn queued_job_can_be_cancelled_without_waiting_for_the_active_job() {
        let _guard = TEST_LOCK.lock().unwrap();
        reset_for_tests();
        let _first = acquire("a", "A", "first", None).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let waiter = thread::spawn(move || acquire("a", "A", "second", Some(flag.as_ref())));
        thread::sleep(Duration::from_millis(40));
        cancelled.store(true, Ordering::Release);
        assert!(waiter.join().unwrap().is_err());
        let snapshot = snapshot();
        assert_eq!(snapshot.queued_count, 0);
        assert_eq!(snapshot.recent_events[0].status, "cancelled");
    }
}
