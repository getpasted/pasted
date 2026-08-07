use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

const MAX_QUEUE_ITEMS: usize = 1_000;
const MAX_QUEUE_BYTES: usize = 256 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialStatus {
    pub is_active: bool,
    pub queue: Vec<String>,
    pub item_ids: Vec<u64>,
    pub current_index: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone)]
struct SequentialQueueItem {
    id: u64,
    text: String,
}

pub struct SequentialQueueState {
    pub is_active: Mutex<bool>,
    queue: Mutex<Vec<SequentialQueueItem>>,
    next_item_id: AtomicU64,
    internal_clipboard_write: Mutex<Option<(String, Instant)>>,
}

impl SequentialQueueState {
    pub fn new() -> Self {
        Self {
            is_active: Mutex::new(false),
            queue: Mutex::new(Vec::new()),
            next_item_id: AtomicU64::new(1),
            internal_clipboard_write: Mutex::new(None),
        }
    }

    pub fn start_queue(&self) {
        let mut active = self.is_active.lock();
        *active = true;
    }

    /// Add an item explicitly from the UI. Explicit additions are useful even
    /// when automatic clipboard recording is stopped.
    pub fn push_item(&self, item: String) -> Result<(), String> {
        let mut queue = self.queue.lock();
        if queue.len() >= MAX_QUEUE_ITEMS {
            return Err("The Copy Queue is full".to_string());
        }
        let queued_bytes = queue
            .iter()
            .try_fold(0usize, |total, queued| total.checked_add(queued.text.len()));
        if queued_bytes
            .and_then(|total| total.checked_add(item.len()))
            .is_none_or(|total| total > MAX_QUEUE_BYTES)
        {
            return Err("The Copy Queue has reached its memory safety limit".to_string());
        }
        queue.push(SequentialQueueItem {
            id: self.next_item_id.fetch_add(1, Ordering::Relaxed),
            text: item,
        });
        Ok(())
    }

    /// Record an external clipboard change while recording mode is active.
    /// Returns true only when the item was actually appended.
    pub fn capture_item(&self, item: String) -> bool {
        let mut internal_write = self.internal_clipboard_write.lock();
        if let Some((expected, written_at)) = internal_write.as_ref() {
            if written_at.elapsed() <= Duration::from_secs(2) && expected == &item {
                *internal_write = None;
                return false;
            }
            if written_at.elapsed() > Duration::from_secs(2) {
                *internal_write = None;
            }
        }
        drop(internal_write);

        if !*self.is_active.lock() {
            return false;
        }
        self.push_item(item).is_ok()
    }

    pub fn mark_internal_clipboard_write(&self, item: &str) {
        *self.internal_clipboard_write.lock() = Some((item.to_string(), Instant::now()));
    }

    pub fn clear_internal_clipboard_write(&self) {
        *self.internal_clipboard_write.lock() = None;
    }

    pub fn peek_item(&self, index: usize) -> Option<(u64, String)> {
        self.queue
            .lock()
            .get(index)
            .map(|item| (item.id, item.text.clone()))
    }

    pub fn consume_item(&self, expected_id: u64) -> Result<String, String> {
        let mut queue = self.queue.lock();
        let Some(index) = queue.iter().position(|item| item.id == expected_id) else {
            return Err("The Copy Queue changed before the paste completed".to_string());
        };
        Ok(queue.remove(index).text)
    }

    pub fn consume_prefix(&self, expected_ids: &[u64]) -> Result<(), String> {
        let mut queue = self.queue.lock();
        if queue.len() < expected_ids.len()
            || queue
                .iter()
                .zip(expected_ids)
                .any(|(item, expected_id)| item.id != *expected_id)
        {
            return Err("The Copy Queue changed before the paste completed".to_string());
        }
        queue.drain(..expected_ids.len());
        Ok(())
    }

    pub fn remove_item_by_index(&self, index: usize) -> Option<String> {
        let mut q = self.queue.lock();
        if index < q.len() {
            Some(q.remove(index).text)
        } else {
            None
        }
    }

    pub fn reorder_items(&self, item_ids: &[u64]) -> Result<(), String> {
        let mut queue = self.queue.lock();
        if item_ids.len() != queue.len() {
            return Err("Queue order does not include every item".to_string());
        }
        let mut items_by_id: HashMap<u64, SequentialQueueItem> =
            queue.iter().cloned().map(|item| (item.id, item)).collect();
        let mut reordered = Vec::with_capacity(item_ids.len());
        for item_id in item_ids {
            let Some(item) = items_by_id.remove(item_id) else {
                return Err("Queue order contains an unknown or duplicate item".to_string());
            };
            reordered.push(item);
        }
        if !items_by_id.is_empty() {
            return Err("Queue order does not include every item".to_string());
        }
        *queue = reordered;
        Ok(())
    }

    pub fn stop_queue(&self) {
        let mut active = self.is_active.lock();
        *active = false;
    }

    #[allow(dead_code)]
    pub fn clear_queue(&self) {
        let mut active = self.is_active.lock();
        let mut q = self.queue.lock();
        *active = false;
        q.clear();
    }

    pub fn get_status(&self) -> SequentialStatus {
        let active = *self.is_active.lock();
        let q = self.queue.lock();
        SequentialStatus {
            is_active: active,
            queue: q.iter().map(|item| item.text.clone()).collect(),
            item_ids: q.iter().map(|item| item.id).collect(),
            current_index: 0,
            total_count: q.len(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sequential_queue_flow() {
        let seq = SequentialQueueState::new();
        assert!(!seq.get_status().is_active);

        // Start queue
        seq.start_queue();
        assert!(seq.get_status().is_active);

        // Push items
        seq.push_item("First".to_string()).unwrap();
        seq.push_item("Second".to_string()).unwrap();
        assert_eq!(seq.get_status().total_count, 2);

        // Pop items FIFO
        assert_eq!(seq.remove_item_by_index(0).as_deref(), Some("First"));
        assert_eq!(seq.remove_item_by_index(0).as_deref(), Some("Second"));

        // Recording remains active until the user explicitly stops it.
        assert!(seq.get_status().is_active);
        assert_eq!(seq.remove_item_by_index(0), None);
    }

    #[test]
    fn explicit_items_can_be_added_while_recording_is_stopped() {
        let seq = SequentialQueueState::new();
        seq.push_item("Queued manually".to_string()).unwrap();

        assert!(!seq.get_status().is_active);
        assert_eq!(seq.get_status().queue, vec!["Queued manually"]);
    }

    #[test]
    fn internal_pastes_are_not_recorded_back_into_the_queue() {
        let seq = SequentialQueueState::new();
        seq.start_queue();
        seq.mark_internal_clipboard_write("First");

        assert!(!seq.capture_item("First".to_string()));
        assert!(seq.capture_item("Second".to_string()));
        assert_eq!(seq.get_status().queue, vec!["Second"]);
    }

    #[test]
    fn queue_rejects_items_beyond_its_bounded_capacity() {
        let seq = SequentialQueueState::new();
        for index in 0..MAX_QUEUE_ITEMS {
            seq.push_item(format!("Item {index}"))
                .expect("items within the limit should be queued");
        }

        assert_eq!(seq.get_status().total_count, MAX_QUEUE_ITEMS);
        assert_eq!(
            seq.push_item("One too many".to_string()).unwrap_err(),
            "The Copy Queue is full"
        );
    }

    #[test]
    fn duplicate_text_items_reorder_by_identity() {
        let seq = SequentialQueueState::new();
        seq.push_item("Same".to_string()).unwrap();
        seq.push_item("Different".to_string()).unwrap();
        seq.push_item("Same".to_string()).unwrap();
        let original = seq.get_status();

        seq.reorder_items(&[
            original.item_ids[2],
            original.item_ids[0],
            original.item_ids[1],
        ])
        .unwrap();

        let reordered = seq.get_status();
        assert_eq!(reordered.queue, vec!["Same", "Same", "Different"]);
        assert_eq!(
            reordered.item_ids,
            vec![
                original.item_ids[2],
                original.item_ids[0],
                original.item_ids[1]
            ]
        );
    }

    #[test]
    fn invalid_reorder_does_not_mutate_the_queue() {
        let seq = SequentialQueueState::new();
        seq.push_item("First".to_string()).unwrap();
        seq.push_item("Second".to_string()).unwrap();
        let original = seq.get_status();

        assert!(seq.reorder_items(&[original.item_ids[0], 999_999]).is_err());
        assert_eq!(seq.get_status().queue, original.queue);
        assert_eq!(seq.get_status().item_ids, original.item_ids);
    }

    #[test]
    fn peek_does_not_consume_until_paste_is_committed() {
        let seq = SequentialQueueState::new();
        seq.push_item("First".to_string()).unwrap();
        let (item_id, text) = seq.peek_item(0).unwrap();

        assert_eq!(text, "First");
        assert_eq!(seq.get_status().total_count, 1);
        assert_eq!(seq.consume_item(item_id).unwrap(), "First");
        assert_eq!(seq.get_status().total_count, 0);
    }

    #[test]
    fn test_stop_queue() {
        let seq = SequentialQueueState::new();
        seq.start_queue();
        seq.push_item("Item 1".to_string()).unwrap();
        seq.stop_queue();

        assert!(!seq.get_status().is_active);
        assert_eq!(seq.get_status().total_count, 1);

        seq.clear_queue();
        assert_eq!(seq.get_status().total_count, 0);
    }
}
