use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const RECENT_EVENT_LIMIT: usize = 80;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulerJobSnapshot {
    pub id: String,
    pub client_request_id: Option<String>,
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
    pub client_request_id: Option<String>,
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
    client_request_id: Option<String>,
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
static NEXT_DEMO_ID: AtomicU64 = AtomicU64::new(1);

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

struct SchedulerEventSource<'a> {
    job_id: &'a str,
    client_request_id: Option<&'a str>,
    connection_name: &'a str,
    label: &'a str,
}

fn push_event(
    scheduler: &Scheduler,
    state: &mut SchedulerState,
    source: SchedulerEventSource<'_>,
    status: &str,
    detail: Option<String>,
) {
    state.revision = state.revision.saturating_add(1);
    state.recent_events.push_front(SchedulerEventSnapshot {
        sequence: scheduler
            .next_event_sequence
            .fetch_add(1, Ordering::Relaxed),
        job_id: source.job_id.to_string(),
        client_request_id: source.client_request_id.map(str::to_string),
        connection_name: source.connection_name.to_string(),
        label: source.label.to_string(),
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
        SchedulerEventSource {
            job_id: &job.id,
            client_request_id: job.client_request_id.as_deref(),
            connection_name: &job.connection_name,
            label: &job.label,
        },
        completion.as_str(),
        detail,
    );
    scheduler.changed.notify_all();
}

pub fn acquire(
    connection_id: &str,
    connection_name: &str,
    label: &str,
    client_request_id: Option<&str>,
    cancellation: Option<&AtomicBool>,
) -> Result<SchedulerPermit, ()> {
    let scheduler = scheduler();
    let id = format!(
        "job-{}",
        scheduler.next_job_id.fetch_add(1, Ordering::Relaxed)
    );
    let job = ScheduledJob {
        id: id.clone(),
        client_request_id: client_request_id.map(str::to_string),
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
        SchedulerEventSource {
            job_id: &id,
            client_request_id,
            connection_name,
            label,
        },
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
                    SchedulerEventSource {
                        job_id: &job.id,
                        client_request_id: job.client_request_id.as_deref(),
                        connection_name: &job.connection_name,
                        label: &job.label,
                    },
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
            let (job_id, job_client_request_id, job_connection_name, job_label) = {
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
                    job.client_request_id.clone(),
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
                SchedulerEventSource {
                    job_id: &job_id,
                    client_request_id: job_client_request_id.as_deref(),
                    connection_name: &job_connection_name,
                    label: &job_label,
                },
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
            client_request_id: job.client_request_id.clone(),
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

pub fn run_demo(
    scenario: String,
    on_fallback: impl FnOnce() + Send + 'static,
) -> Result<(), String> {
    if !matches!(
        scenario.as_str(),
        "fifo" | "parallel" | "cancel" | "fallback"
    ) {
        return Err(format!("Unknown scheduler simulation: {scenario}"));
    }
    let demo_id = NEXT_DEMO_ID.fetch_add(1, Ordering::Relaxed);
    let request_prefix = format!("scheduler-demo-{demo_id}");
    std::thread::spawn(move || match scenario.as_str() {
        "fifo" => run_fifo_demo(&request_prefix),
        "parallel" => run_parallel_demo(&request_prefix),
        "cancel" => run_cancel_demo(&request_prefix),
        "fallback" => run_fallback_demo(&request_prefix, on_fallback),
        _ => unreachable!("scenario was validated before spawning"),
    });
    Ok(())
}

fn finish_demo_after(
    mut permit: SchedulerPermit,
    duration: Duration,
    completion: SchedulerCompletion,
    detail: &'static str,
) {
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        permit.finish(completion, Some(detail.to_string()));
    });
}

fn run_fifo_demo(request_prefix: &str) {
    let first_id = format!("{request_prefix}-1");
    let Ok(first) = acquire(
        "demo-alpha",
        "Demo Alpha",
        "FIFO job 1",
        Some(&first_id),
        None,
    ) else {
        return;
    };
    finish_demo_after(
        first,
        Duration::from_millis(2_000),
        SchedulerCompletion::Succeeded,
        "Simulation completed",
    );
    for position in 2..=3 {
        let request_id = format!("{request_prefix}-{position}");
        std::thread::spawn(move || {
            let Ok(permit) = acquire(
                "demo-alpha",
                "Demo Alpha",
                &format!("FIFO job {position}"),
                Some(&request_id),
                None,
            ) else {
                return;
            };
            finish_demo_after(
                permit,
                Duration::from_millis(2_000),
                SchedulerCompletion::Succeeded,
                "Simulation completed",
            );
        });
        std::thread::sleep(Duration::from_millis(40));
    }
}

fn run_parallel_demo(request_prefix: &str) {
    for (connection_id, connection_name, suffix) in [
        ("demo-alpha", "Demo Alpha", "alpha"),
        ("demo-bravo", "Demo Bravo", "bravo"),
    ] {
        let request_id = format!("{request_prefix}-{suffix}");
        std::thread::spawn(move || {
            let Ok(permit) = acquire(
                connection_id,
                connection_name,
                "Parallel job",
                Some(&request_id),
                None,
            ) else {
                return;
            };
            finish_demo_after(
                permit,
                Duration::from_millis(3_000),
                SchedulerCompletion::Succeeded,
                "Simulation completed",
            );
        });
    }
}

fn run_cancel_demo(request_prefix: &str) {
    let active_id = format!("{request_prefix}-active");
    let Ok(active) = acquire(
        "demo-cancel",
        "Demo Cancel",
        "Blocking job",
        Some(&active_id),
        None,
    ) else {
        return;
    };
    finish_demo_after(
        active,
        Duration::from_millis(3_000),
        SchedulerCompletion::Succeeded,
        "Simulation completed",
    );

    let cancellation = Arc::new(AtomicBool::new(false));
    let waiter_flag = Arc::clone(&cancellation);
    let queued_id = format!("{request_prefix}-queued");
    std::thread::spawn(move || {
        let _ = acquire(
            "demo-cancel",
            "Demo Cancel",
            "Cancelled queued job",
            Some(&queued_id),
            Some(waiter_flag.as_ref()),
        );
    });
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(900));
        cancellation.store(true, Ordering::Release);
    });
}

fn run_fallback_demo(request_prefix: &str, on_fallback: impl FnOnce()) {
    let request_id = format!("{request_prefix}-fallback");
    let Ok(mut first) = acquire(
        "demo-primary",
        "Demo Primary",
        "Fallback job",
        Some(&request_id),
        None,
    ) else {
        return;
    };
    std::thread::sleep(Duration::from_millis(1_200));
    first.finish(
        SchedulerCompletion::Failed,
        Some("Simulated provider failure".to_string()),
    );
    on_fallback();
    let Ok(fallback) = acquire(
        "demo-fallback",
        "Demo Fallback",
        "Fallback job",
        Some(&request_id),
        None,
    ) else {
        return;
    };
    finish_demo_after(
        fallback,
        Duration::from_millis(2_000),
        SchedulerCompletion::Succeeded,
        "Simulation completed after fallback",
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, PoisonError};
    use std::thread;

    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn wait_for_job_status(label: &str, status: &str) -> SchedulerJobSnapshot {
        let deadline = Instant::now() + Duration::from_secs(2);
        loop {
            if let Some(job) = snapshot()
                .jobs
                .into_iter()
                .find(|job| job.label == label && job.status == status)
            {
                return job;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {label} to become {status}"
            );
            thread::yield_now();
        }
    }

    #[test]
    fn same_connection_is_fifo_while_other_connections_run_independently() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let first = acquire("test-fifo-a", "Test FIFO A", "test-fifo-first", None, None).unwrap();
        let other = acquire("test-fifo-b", "Test FIFO B", "test-fifo-other", None, None).unwrap();
        let running_test_jobs = snapshot()
            .jobs
            .into_iter()
            .filter(|job| job.connection_id.starts_with("test-fifo-"))
            .filter(|job| job.status == "running")
            .count();
        assert_eq!(running_test_jobs, 2);

        let acquired = Arc::new(AtomicBool::new(false));
        let acquired_in_thread = Arc::clone(&acquired);
        let waiter = thread::spawn(move || {
            let _second = acquire(
                "test-fifo-a",
                "Test FIFO A",
                "test-fifo-second",
                Some("test-fifo-request-2"),
                None,
            )
            .unwrap();
            acquired_in_thread.store(true, Ordering::Release);
        });
        let queued = wait_for_job_status("test-fifo-second", "queued");
        assert!(!acquired.load(Ordering::Acquire));
        assert_eq!(
            queued.client_request_id.as_deref(),
            Some("test-fifo-request-2")
        );
        drop(first);
        waiter.join().unwrap();
        assert!(acquired.load(Ordering::Acquire));
        drop(other);
    }

    #[test]
    fn queued_job_can_be_cancelled_without_waiting_for_the_active_job() {
        let _guard = TEST_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let _first = acquire(
            "test-cancel-a",
            "Test Cancel A",
            "test-cancel-first",
            None,
            None,
        )
        .unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let waiter = thread::spawn(move || {
            acquire(
                "test-cancel-a",
                "Test Cancel A",
                "test-cancel-second",
                None,
                Some(flag.as_ref()),
            )
        });
        wait_for_job_status("test-cancel-second", "queued");
        cancelled.store(true, Ordering::Release);
        assert!(waiter.join().unwrap().is_err());
        let snapshot = snapshot();
        assert!(!snapshot
            .jobs
            .iter()
            .any(|job| job.label == "test-cancel-second"));
        assert!(snapshot
            .recent_events
            .iter()
            .any(|event| { event.label == "test-cancel-second" && event.status == "cancelled" }));
    }
}
