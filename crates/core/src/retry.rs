use boxy_error::Result;
use std::future::Future;
use tokio::time::{sleep, Duration};

pub const DEFAULT_MAX_ATTEMPTS: u32 = 3;
pub const DEFAULT_RETRY_BASE_DELAY: Duration = Duration::from_secs(1);

pub async fn retry_with_backoff<F, Fut, T>(
    max_attempts: u32,
    base_delay: Duration,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let mut attempt = 1;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(err) => {
                if attempt >= max_attempts {
                    return Err(err);
                }
                // 限制最大 factor 为 32 (2^5)，防止溢出
                let shift = (attempt - 1).min(5);
                let factor = 1u32 << shift;
                let delay = base_delay.checked_mul(factor).unwrap_or(base_delay);
                sleep(delay).await;
                attempt += 1;
            }
        }
    }
}
