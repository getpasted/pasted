use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequentialStatus {
    pub is_active: bool,
    pub queue: Vec<String>,
    pub current_index: usize,
    pub total_count: usize,
}

pub struct SequentialQueueState {
    pub is_active: Mutex<bool>,
    pub queue: Mutex<Vec<String>>,
}

impl SequentialQueueState {
    pub fn new() -> Self {
        Self {
            is_active: Mutex::new(false),
            queue: Mutex::new(Vec::new()),
        }
    }

    pub fn start_queue(&self) {
        let mut active = self.is_active.lock();
        let mut q = self.queue.lock();
        *active = true;
        q.clear();
    }

    pub fn push_item(&self, item: String) {
        let active = *self.is_active.lock();
        if active {
            let mut q = self.queue.lock();
            q.push(item);
        }
    }

    pub fn pop_next(&self) -> Option<String> {
        let mut active_lock = self.is_active.lock();
        let mut q = self.queue.lock();
        if q.is_empty() {
            return None;
        }
        let next_item = q.remove(0);
        if q.is_empty() {
            *active_lock = false;
        }
        Some(next_item)
    }

    pub fn remove_item_by_index(&self, index: usize) -> Option<String> {
        let mut active_lock = self.is_active.lock();
        let mut q = self.queue.lock();
        if index < q.len() {
            let removed = q.remove(index);
            if q.is_empty() {
                *active_lock = false;
            }
            Some(removed)
        } else {
            None
        }
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
            queue: q.clone(),
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
        seq.push_item("First".to_string());
        seq.push_item("Second".to_string());
        assert_eq!(seq.get_status().total_count, 2);

        // Pop items FIFO
        assert_eq!(seq.pop_next().as_deref(), Some("First"));
        assert_eq!(seq.pop_next().as_deref(), Some("Second"));

        // Queue automatically deactivates when empty
        assert!(!seq.get_status().is_active);
        assert_eq!(seq.pop_next(), None);
    }

    #[test]
    fn test_stop_queue() {
        let seq = SequentialQueueState::new();
        seq.start_queue();
        seq.push_item("Item 1".to_string());
        seq.stop_queue();

        assert!(!seq.get_status().is_active);
        assert_eq!(seq.get_status().total_count, 1);

        seq.clear_queue();
        assert_eq!(seq.get_status().total_count, 0);
    }
}
