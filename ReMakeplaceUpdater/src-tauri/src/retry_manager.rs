use anyhow::{Error, Result};
use rand::Rng;
use std::time::Duration;
use tokio::time::sleep;

/// Defines the types of errors that can be retried
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorType {
    NetworkTimeout,
    ConnectionReset,
    ChunkReadFailed,
    TemporaryFailure,
}

/// Defines different backoff strategies for retry delays
#[derive(Debug, Clone)]
pub enum BackoffStrategy {
    /// Exponential backoff with base delay and multiplier
    Exponential { base: u64, multiplier: f64 },
    /// Linear backoff with fixed increment
    Linear { increment: u64 },
    /// Fixed delay for all retry attempts
    Fixed { delay: u64 },
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        BackoffStrategy::Exponential {
            base: 1000, // 1 second base
            multiplier: 2.0,
        }
    }
}

/// Configuration for retry behavior
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Types of errors that should trigger a retry
    pub retry_on: Vec<ErrorType>,
    /// Strategy for calculating retry delays
    pub backoff_strategy: BackoffStrategy,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            retry_on: vec![
                ErrorType::NetworkTimeout,
                ErrorType::ConnectionReset,
                ErrorType::ChunkReadFailed,
                ErrorType::TemporaryFailure,
            ],
            backoff_strategy: BackoffStrategy::default(),
        }
    }
}

/// Manages retry logic with configurable backoff strategies
#[derive(Debug, Clone)]
pub struct RetryManager {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Base delay for backoff calculations (in milliseconds)
    pub base_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Timeout for individual operations
    pub timeout: Duration,
    /// Policy defining what errors to retry and how
    pub retry_policy: RetryPolicy,
}

impl RetryManager {
    /// Creates a new RetryManager with default configuration
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_millis(1000), // 1 second
            max_delay: Duration::from_secs(60),      // 1 minute max
            timeout: Duration::from_secs(30),        // 30 seconds timeout
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Creates a new RetryManager with custom configuration
    pub fn with_config(
        max_retries: u32,
        base_delay: Duration,
        max_delay: Duration,
        timeout: Duration,
    ) -> Self {
        Self {
            max_retries,
            base_delay,
            max_delay,
            timeout,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// Creates a RetryManager optimized for network operations
    pub fn for_network_operations() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            timeout: Duration::from_secs(30),
            retry_policy: RetryPolicy {
                retry_on: vec![
                    ErrorType::NetworkTimeout,
                    ErrorType::ConnectionReset,
                    ErrorType::ChunkReadFailed,
                    ErrorType::TemporaryFailure,
                ],
                backoff_strategy: BackoffStrategy::Exponential {
                    base: 1000,      // 1 second
                    multiplier: 2.0, // 1s, 2s, 4s, 8s, 16s
                },
            },
        }
    }

    /// Sets a custom retry policy
    pub fn with_policy(mut self, policy: RetryPolicy) -> Self {
        self.retry_policy = policy;
        self
    }

    /// Determines if an error should trigger a retry based on the retry policy
    pub fn should_retry(&self, error: &Error) -> bool {
        // Inspect the entire error chain for robust matching
        let mut messages: Vec<String> = Vec::new();
        for cause in error.chain() {
            messages.push(cause.to_string().to_lowercase());
        }

        for msg in &messages {
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

    /// Calculates the delay for a retry attempt based on the backoff strategy
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        let delay_ms = match &self.retry_policy.backoff_strategy {
            BackoffStrategy::Exponential { base, multiplier } => {
                let delay = (*base as f64) * multiplier.powi(attempt as i32);
                delay as u64
            }
            BackoffStrategy::Linear { increment } => {
                self.base_delay.as_millis() as u64 + (increment * attempt as u64)
            }
            BackoffStrategy::Fixed { delay } => *delay,
        };

        let delay = Duration::from_millis(delay_ms);

        // Cap the delay at max_delay
        if delay > self.max_delay {
            self.max_delay
        } else {
            delay
        }
    }

    /// Calculates a jittered delay to avoid thundering herd
    pub fn calculate_delay_with_jitter(&self, attempt: u32) -> Duration {
        let base = self.calculate_delay(attempt);
        let millis = base.as_millis() as u64;
        if millis == 0 {
            return base;
        }
        // ±25% jitter
        let jitter_range = (millis / 4).max(1);
        let mut rng = rand::thread_rng();
        let offset: i64 = rng.gen_range(-(jitter_range as i64)..=(jitter_range as i64));
        let adj = if offset.is_negative() {
            millis.saturating_sub(offset.wrapping_abs() as u64)
        } else {
            millis.saturating_add(offset as u64)
        };
        let dur = Duration::from_millis(adj);
        if dur > self.max_delay {
            self.max_delay
        } else {
            dur
        }
    }

    /// Executes an async operation with retry logic
    pub async fn execute_with_retry<F, Fut, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            // Execute the operation with timeout
            let result = tokio::time::timeout(self.timeout, operation()).await;

            match result {
                Ok(Ok(success)) => return Ok(success),
                Ok(Err(error)) => {
                    last_error = Some(error);

                    // Check if we should retry this error
                    if let Some(ref err) = last_error {
                        if attempt < self.max_retries && self.should_retry(err) {
                            let delay = self.calculate_delay(attempt);
                            println!(
                                "Operation failed (attempt {}/{}): {}. Retrying in {:?}...",
                                attempt + 1,
                                self.max_retries + 1,
                                err,
                                delay
                            );
                            sleep(delay).await;
                            continue;
                        }
                    }
                    break;
                }
                Err(_timeout_error) => {
                    let timeout_error =
                        anyhow::anyhow!("Operation timed out after {:?}", self.timeout);
                    last_error = Some(timeout_error);

                    if attempt < self.max_retries {
                        let delay = self.calculate_delay(attempt);
                        println!(
                            "Operation timed out (attempt {}/{}). Retrying in {:?}...",
                            attempt + 1,
                            self.max_retries + 1,
                            delay
                        );
                        sleep(delay).await;
                        continue;
                    }
                    break;
                }
            }
        }

        // All retries exhausted, return the last error
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Operation failed after {} retries", self.max_retries)
        }))
    }

    /// Executes a synchronous operation with retry logic (for non-async operations)
    pub async fn execute_sync_with_retry<F, T>(&self, mut operation: F) -> Result<T>
    where
        F: FnMut() -> Result<T>,
    {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            match operation() {
                Ok(success) => return Ok(success),
                Err(error) => {
                    last_error = Some(error);

                    // Check if we should retry this error
                    if let Some(ref err) = last_error {
                        if attempt < self.max_retries && self.should_retry(err) {
                            let delay = self.calculate_delay(attempt);
                            println!(
                                "Operation failed (attempt {}/{}): {}. Retrying in {:?}...",
                                attempt + 1,
                                self.max_retries + 1,
                                err,
                                delay
                            );
                            sleep(delay).await;
                            continue;
                        }
                    }
                    break;
                }
            }
        }

        // All retries exhausted, return the last error
        Err(last_error.unwrap_or_else(|| {
            anyhow::anyhow!("Operation failed after {} retries", self.max_retries)
        }))
    }
}

impl Default for RetryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    // Constructor tests
    #[test]
    fn test_retry_manager_new_default_configuration() {
        let retry_manager = RetryManager::new();

        assert_eq!(retry_manager.max_retries, 3);
        assert_eq!(retry_manager.base_delay, Duration::from_millis(1000));
        assert_eq!(retry_manager.max_delay, Duration::from_secs(60));
        assert_eq!(retry_manager.timeout, Duration::from_secs(30));
        assert_eq!(retry_manager.retry_policy.retry_on.len(), 4);

        // Verify default backoff strategy
        match retry_manager.retry_policy.backoff_strategy {
            BackoffStrategy::Exponential { base, multiplier } => {
                assert_eq!(base, 1000);
                assert_eq!(multiplier, 2.0);
            }
            _ => panic!("Expected exponential backoff strategy"),
        }
    }

    #[test]
    fn test_retry_manager_with_config() {
        let retry_manager = RetryManager::with_config(
            5,
            Duration::from_millis(500),
            Duration::from_secs(30),
            Duration::from_secs(15),
        );

        assert_eq!(retry_manager.max_retries, 5);
        assert_eq!(retry_manager.base_delay, Duration::from_millis(500));
        assert_eq!(retry_manager.max_delay, Duration::from_secs(30));
        assert_eq!(retry_manager.timeout, Duration::from_secs(15));
    }

    #[test]
    fn test_retry_manager_for_network_operations() {
        let retry_manager = RetryManager::for_network_operations();

        assert_eq!(retry_manager.max_retries, 3);
        assert_eq!(retry_manager.base_delay, Duration::from_millis(1000));
        assert_eq!(retry_manager.max_delay, Duration::from_secs(30));
        assert_eq!(retry_manager.timeout, Duration::from_secs(30));
        assert_eq!(retry_manager.retry_policy.retry_on.len(), 3);

        // Verify it includes specific network error types
        assert!(retry_manager
            .retry_policy
            .retry_on
            .contains(&ErrorType::NetworkTimeout));
        assert!(retry_manager
            .retry_policy
            .retry_on
            .contains(&ErrorType::ConnectionReset));
        assert!(retry_manager
            .retry_policy
            .retry_on
            .contains(&ErrorType::ChunkReadFailed));
        assert!(!retry_manager
            .retry_policy
            .retry_on
            .contains(&ErrorType::TemporaryFailure));
    }

    #[test]
    fn test_retry_manager_with_policy() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::NetworkTimeout],
            backoff_strategy: BackoffStrategy::Fixed { delay: 1500 },
        };

        let retry_manager = RetryManager::new().with_policy(custom_policy);

        assert_eq!(retry_manager.retry_policy.retry_on.len(), 1);
        match retry_manager.retry_policy.backoff_strategy {
            BackoffStrategy::Fixed { delay } => assert_eq!(delay, 1500),
            _ => panic!("Expected fixed backoff strategy"),
        }
    }

    #[test]
    fn test_retry_manager_default_trait() {
        let retry_manager = RetryManager::default();
        assert_eq!(retry_manager.max_retries, 3);
    }

    // should_retry method tests
    #[test]
    fn test_should_retry_network_timeout_errors() {
        let retry_manager = RetryManager::new();

        // Various timeout error messages
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Connection timeout")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Operation timed out")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("REQUEST TIMEOUT")));
        assert!(
            retry_manager.should_retry(&anyhow::anyhow!("Network operation timed out after 30s"))
        );
    }

    #[test]
    fn test_should_retry_connection_reset_errors() {
        let retry_manager = RetryManager::new();

        // Connection reset variations
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Connection reset by peer")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("CONNECTION REFUSED")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Broken pipe error")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("TCP connection reset")));
    }

    #[test]
    fn test_should_retry_chunk_read_errors() {
        let retry_manager = RetryManager::new();

        // Chunk read failures
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Chunk read failed")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Incomplete read from stream")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Unexpected EOF while reading chunk")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("CHUNK parsing error")));
    }

    #[test]
    fn test_should_retry_temporary_failure_errors() {
        let retry_manager = RetryManager::new();

        // Temporary failures
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Temporary failure, please try again")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Service unavailable")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("HTTP 502 Bad Gateway")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("HTTP 503 Service Unavailable")));
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Error 504: Gateway Timeout")));
    }

    #[test]
    fn test_should_not_retry_non_retryable_errors() {
        let retry_manager = RetryManager::new();

        // Permanent failures that should not be retried
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Permission denied")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("File not found")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Invalid file format")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Authentication failed")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("HTTP 401 Unauthorized")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("HTTP 404 Not Found")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Disk full")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Out of memory")));
    }

    #[test]
    fn test_should_retry_with_custom_policy() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::NetworkTimeout],
            backoff_strategy: BackoffStrategy::default(),
        };
        let retry_manager = RetryManager::new().with_policy(custom_policy);

        // Should retry only network timeouts
        assert!(retry_manager.should_retry(&anyhow::anyhow!("Connection timeout")));

        // Should not retry other errors that were retryable in default policy
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Connection reset by peer")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Chunk read failed")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Service temporarily unavailable")));
    }

    #[test]
    fn test_should_retry_empty_policy() {
        let empty_policy = RetryPolicy {
            retry_on: vec![],
            backoff_strategy: BackoffStrategy::default(),
        };
        let retry_manager = RetryManager::new().with_policy(empty_policy);

        // Should not retry any errors
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Connection timeout")));
        assert!(!retry_manager.should_retry(&anyhow::anyhow!("Any error message")));
    }

    // calculate_delay method tests
    #[test]
    fn test_calculate_delay_exponential_backoff() {
        let retry_manager = RetryManager::new();

        let delay0 = retry_manager.calculate_delay(0);
        let delay1 = retry_manager.calculate_delay(1);
        let delay2 = retry_manager.calculate_delay(2);
        let delay3 = retry_manager.calculate_delay(3);

        assert_eq!(delay0, Duration::from_millis(1000)); // 1s * 2^0 = 1s
        assert_eq!(delay1, Duration::from_millis(2000)); // 1s * 2^1 = 2s
        assert_eq!(delay2, Duration::from_millis(4000)); // 1s * 2^2 = 4s
        assert_eq!(delay3, Duration::from_millis(8000)); // 1s * 2^3 = 8s
    }

    #[test]
    fn test_calculate_delay_exponential_with_custom_base_and_multiplier() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::NetworkTimeout],
            backoff_strategy: BackoffStrategy::Exponential {
                base: 500,
                multiplier: 3.0,
            },
        };
        let retry_manager = RetryManager::new().with_policy(custom_policy);

        let delay0 = retry_manager.calculate_delay(0);
        let delay1 = retry_manager.calculate_delay(1);
        let delay2 = retry_manager.calculate_delay(2);

        assert_eq!(delay0, Duration::from_millis(500)); // 500ms * 3^0 = 500ms
        assert_eq!(delay1, Duration::from_millis(1500)); // 500ms * 3^1 = 1500ms
        assert_eq!(delay2, Duration::from_millis(4500)); // 500ms * 3^2 = 4500ms
    }

    #[test]
    fn test_calculate_delay_linear_backoff() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::NetworkTimeout],
            backoff_strategy: BackoffStrategy::Linear { increment: 500 },
        };
        let retry_manager = RetryManager::new().with_policy(custom_policy);

        let delay0 = retry_manager.calculate_delay(0);
        let delay1 = retry_manager.calculate_delay(1);
        let delay2 = retry_manager.calculate_delay(2);
        let delay3 = retry_manager.calculate_delay(3);

        assert_eq!(delay0, Duration::from_millis(1000)); // base (1000ms)
        assert_eq!(delay1, Duration::from_millis(1500)); // base + 1*increment
        assert_eq!(delay2, Duration::from_millis(2000)); // base + 2*increment
        assert_eq!(delay3, Duration::from_millis(2500)); // base + 3*increment
    }

    #[test]
    fn test_calculate_delay_fixed_backoff() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::NetworkTimeout],
            backoff_strategy: BackoffStrategy::Fixed { delay: 2000 },
        };
        let retry_manager = RetryManager::new().with_policy(custom_policy);

        let delay0 = retry_manager.calculate_delay(0);
        let delay1 = retry_manager.calculate_delay(1);
        let delay2 = retry_manager.calculate_delay(2);
        let delay10 = retry_manager.calculate_delay(10);

        assert_eq!(delay0, Duration::from_millis(2000));
        assert_eq!(delay1, Duration::from_millis(2000));
        assert_eq!(delay2, Duration::from_millis(2000));
        assert_eq!(delay10, Duration::from_millis(2000));
    }

    #[test]
    fn test_calculate_delay_max_delay_capping() {
        let retry_manager = RetryManager::with_config(
            10,
            Duration::from_millis(1000),
            Duration::from_millis(5000), // Max delay of 5 seconds
            Duration::from_secs(30),
        );

        // With exponential backoff, attempt 3 would be 8000ms, but should be capped
        let delay3 = retry_manager.calculate_delay(3);
        let delay10 = retry_manager.calculate_delay(10); // Very large attempt

        assert_eq!(delay3, Duration::from_millis(5000)); // Capped at max_delay
        assert_eq!(delay10, Duration::from_millis(5000)); // Also capped
    }

    #[test]
    fn test_calculate_delay_edge_cases() {
        let retry_manager = RetryManager::new();

        // Test with very large attempt number
        let delay_large = retry_manager.calculate_delay(100);
        assert_eq!(delay_large, retry_manager.max_delay); // Should be capped

        // Test with zero attempt
        let delay_zero = retry_manager.calculate_delay(0);
        assert_eq!(delay_zero, Duration::from_millis(1000));
    }

    // execute_with_retry method tests
    #[tokio::test]
    async fn test_execute_with_retry_success_on_first_attempt() {
        let retry_manager = RetryManager::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Ok::<i32, Error>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once
    }

    #[tokio::test]
    async fn test_execute_with_retry_retries_on_retryable_error() {
        let retry_manager = RetryManager::with_config(
            2,
            Duration::from_millis(10), // Short delay for testing
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(anyhow::anyhow!("Network timeout occurred"))
                    } else {
                        Ok::<i32, Error>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
    }

    #[tokio::test]
    async fn test_execute_with_retry_stops_on_non_retryable_error() {
        let retry_manager = RetryManager::with_config(
            3,
            Duration::from_millis(10),
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, Error>(anyhow::anyhow!("Permission denied"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Called only once (no retries)
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Permission denied"));
    }

    #[tokio::test]
    async fn test_execute_with_retry_exhausts_max_retries() {
        let retry_manager = RetryManager::with_config(
            2, // Only 2 retries
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, Error>(anyhow::anyhow!("Connection timeout"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Initial + 2 retries
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Connection timeout"));
    }

    #[tokio::test]
    async fn test_execute_with_retry_timeout_handling() {
        let retry_manager = RetryManager::with_config(
            2,
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_millis(50), // Very short timeout
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    // Sleep longer than timeout
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    Ok::<i32, Error>(42)
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Should retry on timeout
        assert!(result.unwrap_err().to_string().contains("timed out"));
    }

    #[tokio::test]
    async fn test_execute_with_retry_success_after_retries() {
        let retry_manager = RetryManager::with_config(
            3,
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(anyhow::anyhow!("Temporary failure"))
                    } else {
                        Ok::<String, Error>("Success".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Success");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    // execute_sync_with_retry method tests
    #[tokio::test]
    async fn test_execute_sync_with_retry_success_on_first_attempt() {
        let retry_manager = RetryManager::new();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_sync_with_retry(|| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Ok::<i32, Error>(42)
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_execute_sync_with_retry_retries_on_retryable_error() {
        let retry_manager = RetryManager::with_config(
            2,
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_sync_with_retry(|| {
                let count = counter_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(anyhow::anyhow!("Connection reset by peer"))
                } else {
                    Ok::<i32, Error>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_execute_sync_with_retry_stops_on_non_retryable_error() {
        let retry_manager = RetryManager::with_config(
            3,
            Duration::from_millis(1),
            Duration::from_millis(100),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_sync_with_retry(|| {
                counter_clone.fetch_add(1, Ordering::SeqCst);
                Err::<i32, Error>(anyhow::anyhow!("File not found"))
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // No retries
        assert!(result.unwrap_err().to_string().contains("File not found"));
    }

    // Configuration and edge case tests
    #[test]
    fn test_backoff_strategy_default() {
        let strategy = BackoffStrategy::default();
        match strategy {
            BackoffStrategy::Exponential { base, multiplier } => {
                assert_eq!(base, 1000);
                assert_eq!(multiplier, 2.0);
            }
            _ => panic!("Expected exponential backoff strategy"),
        }
    }

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();

        assert_eq!(policy.retry_on.len(), 4);
        assert!(policy.retry_on.contains(&ErrorType::NetworkTimeout));
        assert!(policy.retry_on.contains(&ErrorType::ConnectionReset));
        assert!(policy.retry_on.contains(&ErrorType::ChunkReadFailed));
        assert!(policy.retry_on.contains(&ErrorType::TemporaryFailure));
    }

    #[test]
    fn test_error_type_equality() {
        assert_eq!(ErrorType::NetworkTimeout, ErrorType::NetworkTimeout);
        assert_ne!(ErrorType::NetworkTimeout, ErrorType::ConnectionReset);
    }

    #[test]
    fn test_error_type_debug() {
        let error_type = ErrorType::NetworkTimeout;
        let debug_string = format!("{:?}", error_type);
        assert_eq!(debug_string, "NetworkTimeout");
    }

    #[test]
    fn test_error_type_clone() {
        let original = ErrorType::ConnectionReset;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[tokio::test]
    async fn test_execute_with_retry_zero_max_retries() {
        let retry_manager = RetryManager::with_config(
            0, // No retries allowed
            Duration::from_millis(1000),
            Duration::from_millis(5000),
            Duration::from_secs(5),
        );
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, Error>(anyhow::anyhow!("Network timeout"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only initial attempt
    }

    #[test]
    fn test_calculate_delay_with_zero_base_delay() {
        let retry_manager = RetryManager::with_config(
            3,
            Duration::from_millis(0), // Zero base delay
            Duration::from_millis(5000),
            Duration::from_secs(5),
        );

        let delay0 = retry_manager.calculate_delay(0);
        let delay1 = retry_manager.calculate_delay(1);

        // With exponential backoff and base 1000ms (from strategy)
        assert_eq!(delay0, Duration::from_millis(1000));
        assert_eq!(delay1, Duration::from_millis(2000));
    }

    // Integration tests combining multiple components
    #[tokio::test]
    async fn test_integration_network_operations_config() {
        let retry_manager = RetryManager::for_network_operations();
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(anyhow::anyhow!("Connection reset by peer"))
                    } else {
                        Ok::<String, Error>("Network operation successful".to_string())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Network operation successful");
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_integration_custom_policy_and_delays() {
        let custom_policy = RetryPolicy {
            retry_on: vec![ErrorType::ChunkReadFailed],
            backoff_strategy: BackoffStrategy::Linear { increment: 100 },
        };

        let retry_manager = RetryManager::with_config(
            2,
            Duration::from_millis(200),
            Duration::from_millis(1000),
            Duration::from_secs(5),
        )
        .with_policy(custom_policy);

        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();

        let start_time = std::time::Instant::now();

        let result = retry_manager
            .execute_with_retry(|| {
                let counter = counter_clone.clone();
                async move {
                    let count = counter.fetch_add(1, Ordering::SeqCst);
                    if count < 2 {
                        Err(anyhow::anyhow!("Chunk read failed"))
                    } else {
                        Ok::<i32, Error>(100)
                    }
                }
            })
            .await;

        let elapsed = start_time.elapsed();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 100);
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        // Verify delays were applied (should be at least 200ms + 300ms = 500ms)
        assert!(elapsed >= Duration::from_millis(500));
    }
}
