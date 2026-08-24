use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock,
};

use super::ExecutionError;

static EXECUTION_CANCELLATIONS: OnceLock<Mutex<HashMap<String, Arc<AtomicBool>>>> = OnceLock::new();

fn cancellation_registry() -> &'static Mutex<HashMap<String, Arc<AtomicBool>>> {
    EXECUTION_CANCELLATIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub struct CancellationRegistration {
    request_id: String,
    flag: Arc<AtomicBool>,
}

impl CancellationRegistration {
    pub fn register(request_id: String) -> Self {
        let flag = Arc::new(AtomicBool::new(false));
        cancellation_registry()
            .lock()
            .expect("transformation cancellation registry poisoned")
            .insert(request_id.clone(), Arc::clone(&flag));
        Self { request_id, flag }
    }

    pub fn flag(&self) -> &AtomicBool {
        self.flag.as_ref()
    }
}

impl Drop for CancellationRegistration {
    fn drop(&mut self) {
        let mut registry = cancellation_registry()
            .lock()
            .expect("transformation cancellation registry poisoned");
        if registry
            .get(&self.request_id)
            .is_some_and(|flag| Arc::ptr_eq(flag, &self.flag))
        {
            registry.remove(&self.request_id);
        }
    }
}

pub fn cancel_execution(client_request_id: &str) -> bool {
    let flag = cancellation_registry()
        .lock()
        .expect("transformation cancellation registry poisoned")
        .get(client_request_id)
        .cloned();
    if let Some(flag) = flag {
        flag.store(true, Ordering::Release);
        true
    } else {
        false
    }
}

pub(crate) fn ensure_not_cancelled(
    cancellation: Option<&AtomicBool>,
) -> Result<(), ExecutionError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        Err(ExecutionError::new(
            "execution_cancelled",
            "Transform was cancelled",
        ))
    } else {
        Ok(())
    }
}
