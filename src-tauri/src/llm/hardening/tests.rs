use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures::stream::{self, BoxStream};
use tokio::sync::Mutex as AsyncMutex;

use super::*;
use crate::llm::{LLMMessage, TokenUsage};

enum MockOutcome {
    Success,
    Failure(LLMError),
}

struct MockProvider {
    outcomes: AsyncMutex<VecDeque<MockOutcome>>,
    calls: AtomicUsize,
    delay: Duration,
}

impl MockProvider {
    fn new(outcomes: Vec<MockOutcome>) -> Self {
        Self {
            outcomes: AsyncMutex::new(outcomes.into()),
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        }
    }

    fn with_delay(delay: Duration) -> Self {
        Self {
            outcomes: AsyncMutex::new(VecDeque::from([MockOutcome::Success])),
            calls: AtomicUsize::new(0),
            delay,
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl LLMProvider for MockProvider {
    async fn complete(&self, _request: LLMRequest) -> Result<LLMResponse, LLMError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }

        match self.outcomes.lock().await.pop_front() {
            Some(MockOutcome::Success) | None => Ok(LLMResponse {
                content: "ok".to_string(),
                usage: TokenUsage::default(),
                model: "mock".to_string(),
                finish_reason: Some("stop".to_string()),
            }),
            Some(MockOutcome::Failure(error)) => Err(error),
        }
    }

    async fn complete_stream(
        &self,
        _request: LLMRequest,
    ) -> Result<BoxStream<'static, StreamEvent>, LLMError> {
        Ok(Box::pin(stream::empty()))
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        Ok(())
    }

    fn name(&self) -> &str {
        "mock"
    }

    async fn list_models(&self) -> Result<Vec<String>, LLMError> {
        Ok(vec!["mock".to_string()])
    }
}

fn request() -> LLMRequest {
    LLMRequest {
        messages: vec![LLMMessage {
            role: "user".to_string(),
            content: "hello".to_string(),
        }],
        ..LLMRequest::default()
    }
}

fn config() -> LLMHardeningConfig {
    LLMHardeningConfig {
        rate_limit: RateLimiterConfig {
            requests_per_minute: 600,
            burst_capacity: 10,
        },
        retry: RetryPolicy {
            max_retries: 2,
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(5),
            jitter: Duration::ZERO,
        },
        circuit_breaker: CircuitBreakerConfig {
            failure_threshold: 2,
            cooldown: Duration::from_millis(20),
        },
        request_timeout: Duration::from_millis(50),
        connect_timeout: Duration::from_millis(10),
    }
}

#[tokio::test]
async fn token_bucket_allows_burst_then_refills() {
    let limiter = RateLimiter::new(RateLimiterConfig {
        requests_per_minute: 1200,
        burst_capacity: 2,
    });

    assert!(limiter.try_acquire().await);
    assert!(limiter.try_acquire().await);
    assert!(!limiter.try_acquire().await);

    tokio::time::sleep(Duration::from_millis(60)).await;
    assert!(limiter.try_acquire().await);
}

#[tokio::test]
async fn retry_logic_retries_transient_failure() {
    let raw = Arc::new(MockProvider::new(vec![
        MockOutcome::Failure(LLMError::RateLimitExceeded),
        MockOutcome::Success,
    ]));
    let provider = HardenedLLMProvider::new(raw.clone(), "mock".to_string(), config());

    let response = provider
        .complete(request())
        .await
        .expect("retry should recover");
    let diagnostics = provider.diagnostics().await;

    assert_eq!(response.content, "ok");
    assert_eq!(raw.calls(), 2);
    assert_eq!(diagnostics.retries, 1);
}

#[tokio::test]
async fn retry_policy_uses_exponential_backoff() {
    let policy = RetryPolicy {
        max_retries: 3,
        base_delay: Duration::from_millis(10),
        max_delay: Duration::from_millis(100),
        jitter: Duration::ZERO,
    };

    assert_eq!(policy.backoff_delay(1), Duration::from_millis(10));
    assert_eq!(policy.backoff_delay(2), Duration::from_millis(20));
    assert_eq!(policy.backoff_delay(3), Duration::from_millis(40));
}

#[tokio::test]
async fn retry_logic_does_not_retry_invalid_api_key() {
    let raw = Arc::new(MockProvider::new(vec![
        MockOutcome::Failure(LLMError::InvalidApiKey),
        MockOutcome::Success,
    ]));
    let provider = HardenedLLMProvider::new(raw.clone(), "mock".to_string(), config());

    let error = provider
        .complete(request())
        .await
        .expect_err("invalid api key should fail immediately");

    assert!(matches!(error, LLMError::InvalidApiKey));
    assert_eq!(raw.calls(), 1);
}

#[tokio::test]
async fn circuit_breaker_opens_and_recovers_half_open() {
    let breaker = CircuitBreaker::new(CircuitBreakerConfig {
        failure_threshold: 2,
        cooldown: Duration::from_millis(15),
    });

    breaker.record_failure().await;
    assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
    breaker.record_failure().await;
    assert_eq!(breaker.state().await, CircuitBreakerState::Open);
    assert!(breaker.before_request().await.is_err());

    tokio::time::sleep(Duration::from_millis(20)).await;
    assert!(breaker.before_request().await.is_ok());
    assert_eq!(breaker.state().await, CircuitBreakerState::HalfOpen);
    breaker.record_success().await;
    assert_eq!(breaker.state().await, CircuitBreakerState::Closed);
}

#[tokio::test]
async fn timeout_handling_fails_gracefully() {
    let raw = Arc::new(MockProvider::with_delay(Duration::from_millis(100)));
    let provider = HardenedLLMProvider::new(raw, "mock".to_string(), config());

    let error = provider
        .complete(request())
        .await
        .expect_err("slow request should time out");

    assert!(matches!(error, LLMError::Timeout));
}

#[tokio::test]
async fn concurrent_requests_are_rate_limited_safely() {
    let raw = Arc::new(MockProvider::new(vec![
        MockOutcome::Success,
        MockOutcome::Success,
    ]));
    let mut config = config();
    config.rate_limit = RateLimiterConfig {
        requests_per_minute: 1,
        burst_capacity: 2,
    };
    let provider = Arc::new(HardenedLLMProvider::new(raw, "mock".to_string(), config));
    let mut handles = Vec::new();

    for _ in 0..5 {
        let provider = provider.clone();
        handles.push(tokio::spawn(
            async move { provider.complete(request()).await },
        ));
    }

    let mut successes = 0;
    let mut rate_limited = 0;
    for handle in handles {
        match handle.await.expect("task should join") {
            Ok(_) => successes += 1,
            Err(LLMError::RateLimitExceeded) => rate_limited += 1,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }

    assert_eq!(successes, 2);
    assert_eq!(rate_limited, 3);
}

#[tokio::test]
async fn provider_failures_open_circuit_breaker() {
    let raw = Arc::new(MockProvider::new(vec![
        MockOutcome::Failure(LLMError::NetworkError("down".to_string())),
        MockOutcome::Failure(LLMError::NetworkError("down".to_string())),
        MockOutcome::Success,
    ]));
    let mut config = config();
    config.retry.max_retries = 0;
    let provider = HardenedLLMProvider::new(raw, "mock".to_string(), config);

    assert!(provider.complete(request()).await.is_err());
    assert!(provider.complete(request()).await.is_err());
    let error = provider
        .complete(request())
        .await
        .expect_err("open circuit should block request");

    assert!(matches!(error, LLMError::ApiError(message) if message == "Circuit breaker open"));
    assert_eq!(
        provider.diagnostics().await.circuit_breaker_state,
        CircuitBreakerState::Open
    );
}

#[tokio::test]
async fn quota_exhaustion_records_rate_limited_metric() {
    let raw = Arc::new(MockProvider::new(vec![MockOutcome::Success]));
    let mut config = config();
    config.rate_limit = RateLimiterConfig {
        requests_per_minute: 1,
        burst_capacity: 1,
    };
    let provider = HardenedLLMProvider::new(raw, "mock".to_string(), config);

    assert!(provider.complete(request()).await.is_ok());
    assert!(matches!(
        provider.complete(request()).await,
        Err(LLMError::RateLimitExceeded)
    ));

    assert_eq!(provider.diagnostics().await.rate_limited_requests, 1);
}
