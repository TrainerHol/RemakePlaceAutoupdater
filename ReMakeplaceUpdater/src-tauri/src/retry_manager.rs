use anyhow::Error;
use rand::Rng;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    NetworkTimeout,
    ConnectionReset,
    ChunkReadFailed,
    TemporaryFailure,
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub retry_on: Vec<ErrorType>,
    pub base_delay_ms: u64,
    pub multiplier: f64,
}

impl RetryPolicy {
    fn download_defaults() -> Self {
        Self {
            retry_on: vec![
                ErrorType::NetworkTimeout,
                ErrorType::ConnectionReset,
                ErrorType::ChunkReadFailed,
                ErrorType::TemporaryFailure,
            ],
            base_delay_ms: 1000,
            multiplier: 2.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryManager {
    pub max_retries: u32,
    max_delay: Duration,
    retry_policy: RetryPolicy,
}

impl RetryManager {
    pub fn for_network_operations() -> Self {
        Self {
            max_retries: 5,
            max_delay: Duration::from_secs(30),
            retry_policy: RetryPolicy::download_defaults(),
        }
    }

    pub fn should_retry(&self, error: &Error) -> bool {
        for cause in error.chain() {
            let msg = cause.to_string().to_lowercase();
            for error_type in &self.retry_policy.retry_on {
                match error_type {
                    ErrorType::NetworkTimeout => {
                        if msg.contains("timeout") || msg.contains("timed out") {
                            return true;
                        }
                    }
                    ErrorType::ConnectionReset => {
                        if msg.contains("connection reset")
                            || msg.contains("connection refused")
                            || msg.contains("broken pipe")
                            || msg.contains("econnreset")
                        {
                            return true;
                        }
                    }
                    ErrorType::ChunkReadFailed => {
                        if msg.contains("chunk")
                            || msg.contains("incomplete read")
                            || msg.contains("unexpected eof")
                            || msg.contains("failed to read download chunk")
                        {
                            return true;
                        }
                    }
                    ErrorType::TemporaryFailure => {
                        if msg.contains("temporary")
                            || msg.contains("service unavailable")
                            || msg.contains("too many requests")
                            || msg.contains("429")
                            || msg.contains("502")
                            || msg.contains("503")
                            || msg.contains("504")
                        {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = (self.retry_policy.base_delay_ms as f64)
            * self.retry_policy.multiplier.powi(attempt as i32);
        let delay = Duration::from_millis(delay_ms as u64);

        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }

    pub fn calculate_delay_with_jitter(&self, attempt: u32) -> Duration {
        let base = self.calculate_delay(attempt);
        let millis = base.as_millis() as u64;
        if millis == 0 {
            return base;
        }

        let jitter_range = (millis / 4).max(1);
        let mut rng = rand::thread_rng();
        let offset: i64 = rng.gen_range(-(jitter_range as i64)..=(jitter_range as i64));
        let adjusted = if offset.is_negative() {
            millis.saturating_sub(offset.wrapping_abs() as u64)
        } else {
            millis.saturating_add(offset as u64)
        };

        let delay = Duration::from_millis(adjusted);
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_profile_retries_download_failures_but_not_permanent_errors() {
        let retry_manager = RetryManager::for_network_operations();

        assert_eq!(retry_manager.max_retries, 5);
        for message in [
            "request timed out",
            "ECONNRESET while downloading",
            "failed to read download chunk",
            "HTTP 429 Too Many Requests",
            "HTTP 503 Service Unavailable",
        ] {
            assert!(
                retry_manager.should_retry(&anyhow::anyhow!(message)),
                "expected retry for: {message}"
            );
        }

        for message in ["HTTP 404 Not Found", "permission denied", "invalid archive"] {
            assert!(
                !retry_manager.should_retry(&anyhow::anyhow!(message)),
                "expected no retry for: {message}"
            );
        }
    }

    #[test]
    fn error_chain_is_checked_for_retryable_causes() {
        let retry_manager = RetryManager::for_network_operations();
        let error = anyhow::anyhow!("operation timed out").context("download failed");

        assert!(retry_manager.should_retry(&error));
    }

    #[test]
    fn exponential_delay_is_calculated_and_capped() {
        let retry_manager = RetryManager::for_network_operations();

        assert_eq!(
            retry_manager.calculate_delay(0),
            Duration::from_millis(1000)
        );
        assert_eq!(
            retry_manager.calculate_delay(4),
            Duration::from_millis(16000)
        );
        assert_eq!(retry_manager.calculate_delay(10), Duration::from_secs(30));
    }

    #[test]
    fn jittered_delay_stays_within_bounds() {
        let retry_manager = RetryManager::for_network_operations();

        for _ in 0..50 {
            let delay = retry_manager.calculate_delay_with_jitter(0);
            assert!(delay >= Duration::from_millis(750));
            assert!(delay <= Duration::from_millis(1250));
        }
    }
}
