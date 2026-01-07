use crate::retry::{retry_with_backoff, DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY};
use boxy_error::Result;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;

pub struct ManagerExecutor {
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    max_attempts: u32,
    base_delay: Duration,
}

impl Default for ManagerExecutor {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ATTEMPTS, DEFAULT_RETRY_BASE_DELAY)
    }
}

impl ManagerExecutor {
    pub fn new(max_attempts: u32, base_delay: Duration) -> Self {
        Self {
            locks: Mutex::new(HashMap::new()),
            max_attempts,
            base_delay,
        }
    }

    pub async fn execute<F, Fut, T>(&self, manager: &str, f: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(manager.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        let _guard = lock.lock().await;
        let mut task = f;
        retry_with_backoff(self.max_attempts, self.base_delay, || task()).await
    }
}
