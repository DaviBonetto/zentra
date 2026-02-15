use crate::stt::STTError;
use std::time::Duration;
use tokio::time::sleep;

pub struct RetryPolicy {
    max_retries: u8,
    base_delay: Duration,
}

impl RetryPolicy {
    pub fn new(max_retries: u8) -> Self {
        Self {
            max_retries,
            base_delay: Duration::from_secs(2),
        }
    }

    pub fn should_retry(&self, attempt: u8, error: &STTError) -> bool {
        if attempt >= self.max_retries {
            return false;
        }

        error.is_retryable()
    }

    pub async fn wait_before_retry(&self, attempt: u8) {
        let multiplier = 2u64.saturating_pow(attempt as u32);
        let delay_secs = self.base_delay.as_secs().saturating_mul(multiplier);
        let delay = Duration::from_secs(delay_secs.max(1));

        tracing::info!(
            "Retrying in {}s (attempt {})",
            delay.as_secs(),
            attempt + 2
        );
        sleep(delay).await;
    }
}
