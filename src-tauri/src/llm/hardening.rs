//! Production hardening for LLM providers.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use futures::stream::BoxStream;
use serde::Serialize;
use tokio::sync::Mutex;

use crate::llm::{LLMError, LLMProvider, LLMRequest, LLMResponse, StreamEvent};

#[derive(Debug, Clone)]
pub struct LLMHardeningConfig {
    pub rate_limit: RateLimiterConfig,
    pub retry: RetryPolicy,
    pub circuit_breaker: CircuitBreakerConfig,
    pub request_timeout: Duration,
    pub connect_timeout: Duration,
}

impl Default for LLMHardeningConfig {
    fn default() -> Self {
        Self {
            rate_limit: RateLimiterConfig {
                requests_per_minute: 60,
                burst_capacity: 10,
            },
            retry: RetryPolicy {
                max_retries: 3,
                base_delay: Duration::from_millis(250),
                max_delay: Duration::from_secs(4),
                jitter: Duration::from_millis(100),
            },
            circuit_breaker: CircuitBreakerConfig {
                failure_threshold: 5,
                cooldown: Duration::from_secs(30),
            },
            request_timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    pub requests_per_minute: u32,
    pub burst_capacity: u32,
}

struct BucketState {
    tokens: f64,
    last_refill: Instant,
}

pub struct RateLimiter {
    capacity: f64,
    refill_per_second: f64,
    state: Mutex<BucketState>,
}

impl RateLimiter {
    pub fn new(config: RateLimiterConfig) -> Self {
        let capacity = config.burst_capacity.max(1) as f64;
        let refill_per_second = config.requests_per_minute.max(1) as f64 / 60.0;
        Self {
            capacity,
            refill_per_second,
            state: Mutex::new(BucketState {
                tokens: capacity,
                last_refill: Instant::now(),
            }),
        }
    }

    pub async fn try_acquire(&self) -> bool {
        let mut state = self.state.lock().await;
        let elapsed = state.last_refill.elapsed().as_secs_f64();
        state.tokens = (state.tokens + elapsed * self.refill_per_second).min(self.capacity);
        state.last_refill = Instant::now();

        if state.tokens >= 1.0 {
            state.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub jitter: Duration,
}

impl RetryPolicy {
    pub fn should_retry(&self, error: &LLMError) -> bool {
        matches!(
            error,
            LLMError::RateLimitExceeded | LLMError::NetworkError(_) | LLMError::Timeout
        ) || matches!(error, LLMError::ApiError(message) if message.starts_with("HTTP 5"))
    }

    pub fn backoff_delay(&self, retry_number: u32) -> Duration {
        let exponent = retry_number.saturating_sub(1).min(10);
        let base_ms = self.base_delay.as_millis() as u64;
        let delay_ms = base_ms.saturating_mul(2u64.saturating_pow(exponent));
        let capped = Duration::from_millis(delay_ms).min(self.max_delay);
        capped + self.jitter_delay(retry_number)
    }

    fn jitter_delay(&self, retry_number: u32) -> Duration {
        let jitter_ms = self.jitter.as_millis() as u64;
        if jitter_ms == 0 {
            return Duration::ZERO;
        }
        Duration::from_millis((retry_number as u64 * 37) % (jitter_ms + 1))
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub cooldown: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}

struct CircuitState {
    state: CircuitBreakerState,
    failures: u32,
    opened_at: Option<Instant>,
}

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    state: Mutex<CircuitState>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Mutex::new(CircuitState {
                state: CircuitBreakerState::Closed,
                failures: 0,
                opened_at: None,
            }),
        }
    }

    pub async fn before_request(&self) -> Result<(), LLMError> {
        let mut state = self.state.lock().await;
        match state.state {
            CircuitBreakerState::Closed | CircuitBreakerState::HalfOpen => Ok(()),
            CircuitBreakerState::Open => {
                let ready = state
                    .opened_at
                    .is_some_and(|opened_at| opened_at.elapsed() >= self.config.cooldown);
                if ready {
                    state.state = CircuitBreakerState::HalfOpen;
                    Ok(())
                } else {
                    Err(LLMError::ApiError("Circuit breaker open".to_string()))
                }
            }
        }
    }

    pub async fn record_success(&self) {
        let mut state = self.state.lock().await;
        state.state = CircuitBreakerState::Closed;
        state.failures = 0;
        state.opened_at = None;
    }

    pub async fn record_failure(&self) {
        let mut state = self.state.lock().await;
        match state.state {
            CircuitBreakerState::Closed => {
                state.failures = state.failures.saturating_add(1);
                if state.failures >= self.config.failure_threshold {
                    state.state = CircuitBreakerState::Open;
                    state.opened_at = Some(Instant::now());
                }
            }
            CircuitBreakerState::HalfOpen => {
                state.state = CircuitBreakerState::Open;
                state.opened_at = Some(Instant::now());
            }
            CircuitBreakerState::Open => {}
        }
    }

    pub async fn state(&self) -> CircuitBreakerState {
        self.state.lock().await.state
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LLMProviderDiagnostics {
    pub provider: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub retries: u64,
    pub retry_rate: f64,
    pub rate_limited_requests: u64,
    pub circuit_breaker_state: CircuitBreakerState,
    pub average_latency_ms: f64,
    pub p95_latency_ms: u64,
    pub p99_latency_ms: u64,
    pub provider_uptime_seconds: u64,
}

pub struct ProviderMetrics {
    provider: String,
    started_at: Instant,
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    failed_requests: AtomicU64,
    retries: AtomicU64,
    rate_limited_requests: AtomicU64,
    latencies_ms: Mutex<Vec<u64>>,
}

impl ProviderMetrics {
    pub fn new(provider: String) -> Self {
        Self {
            provider,
            started_at: Instant::now(),
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            failed_requests: AtomicU64::new(0),
            retries: AtomicU64::new(0),
            rate_limited_requests: AtomicU64::new(0),
            latencies_ms: Mutex::new(Vec::with_capacity(256)),
        }
    }

    pub fn record_total(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_success(&self) {
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.failed_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_rate_limited(&self) {
        self.rate_limited_requests.fetch_add(1, Ordering::Relaxed);
    }

    pub async fn record_latency(&self, latency: Duration) {
        let mut latencies = self.latencies_ms.lock().await;
        if latencies.len() >= 1024 {
            latencies.remove(0);
        }
        latencies.push(latency.as_millis() as u64);
    }

    pub async fn snapshot(&self, circuit_state: CircuitBreakerState) -> LLMProviderDiagnostics {
        let mut latencies = self.latencies_ms.lock().await.clone();
        latencies.sort_unstable();
        let average = if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
        };
        let total = self.total_requests.load(Ordering::Relaxed);
        let retries = self.retries.load(Ordering::Relaxed);

        LLMProviderDiagnostics {
            provider: self.provider.clone(),
            total_requests: total,
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            failed_requests: self.failed_requests.load(Ordering::Relaxed),
            retries,
            retry_rate: if total == 0 {
                0.0
            } else {
                retries as f64 / total as f64
            },
            rate_limited_requests: self.rate_limited_requests.load(Ordering::Relaxed),
            circuit_breaker_state: circuit_state,
            average_latency_ms: average,
            p95_latency_ms: percentile(&latencies, 95),
            p99_latency_ms: percentile(&latencies, 99),
            provider_uptime_seconds: self.started_at.elapsed().as_secs(),
        }
    }
}

fn percentile(values: &[u64], percentile: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() * percentile).div_ceil(100)).saturating_sub(1);
    values[index.min(values.len() - 1)]
}

pub struct HardenedLLMProvider {
    inner: Arc<dyn LLMProvider>,
    rate_limiter: RateLimiter,
    retry_policy: RetryPolicy,
    circuit_breaker: CircuitBreaker,
    metrics: ProviderMetrics,
    request_timeout: Duration,
}

impl HardenedLLMProvider {
    pub fn new(inner: Arc<dyn LLMProvider>, provider: String, config: LLMHardeningConfig) -> Self {
        let _connect_timeout = config.connect_timeout;
        Self {
            inner,
            rate_limiter: RateLimiter::new(config.rate_limit),
            retry_policy: config.retry,
            circuit_breaker: CircuitBreaker::new(config.circuit_breaker),
            metrics: ProviderMetrics::new(provider),
            request_timeout: config.request_timeout,
        }
    }

    pub async fn diagnostics(&self) -> LLMProviderDiagnostics {
        self.metrics
            .snapshot(self.circuit_breaker.state().await)
            .await
    }

    async fn before_request(&self) -> Result<(), LLMError> {
        self.metrics.record_total();
        if !self.rate_limiter.try_acquire().await {
            self.metrics.record_rate_limited();
            self.metrics.record_failure();
            return Err(LLMError::RateLimitExceeded);
        }
        self.circuit_breaker
            .before_request()
            .await
            .inspect_err(|_| {
                self.metrics.record_failure();
            })
    }

    async fn record_result(&self, start: Instant, result: &Result<(), LLMError>) {
        self.metrics.record_latency(start.elapsed()).await;
        match result {
            Ok(()) => {
                self.metrics.record_success();
                self.circuit_breaker.record_success().await;
            }
            Err(error) => {
                self.metrics.record_failure();
                if self.retry_policy.should_retry(error) {
                    self.circuit_breaker.record_failure().await;
                }
            }
        }
    }
}

#[async_trait]
impl LLMProvider for HardenedLLMProvider {
    async fn complete(&self, request: LLMRequest) -> Result<LLMResponse, LLMError> {
        self.before_request().await?;
        let start = Instant::now();
        let mut retries = 0;

        loop {
            let attempt_result =
                tokio::time::timeout(self.request_timeout, self.inner.complete(request.clone()))
                    .await
                    .map_err(|_| LLMError::Timeout)
                    .and_then(|result| result);

            match attempt_result {
                Ok(response) => {
                    self.record_result(start, &Ok(())).await;
                    return Ok(response);
                }
                Err(error)
                    if self.retry_policy.should_retry(&error)
                        && retries < self.retry_policy.max_retries =>
                {
                    retries += 1;
                    self.metrics.record_retry();
                    tracing::warn!(
                        provider = self.name(),
                        retry = retries,
                        error = %error,
                        "retrying transient LLM provider failure"
                    );
                    tokio::time::sleep(self.retry_policy.backoff_delay(retries)).await;
                }
                Err(error) => {
                    self.record_result(start, &Err(error.clone_for_recording()))
                        .await;
                    return Err(error);
                }
            }
        }
    }

    async fn complete_stream(
        &self,
        request: LLMRequest,
    ) -> Result<BoxStream<'static, StreamEvent>, LLMError> {
        self.before_request().await?;
        let start = Instant::now();
        let result =
            tokio::time::timeout(self.request_timeout, self.inner.complete_stream(request))
                .await
                .map_err(|_| LLMError::Timeout)
                .and_then(|result| result);
        self.record_result(start, &result.as_ref().map(|_| ()).map_err(clone_error))
            .await;
        result
    }

    async fn test_connection(&self) -> Result<(), LLMError> {
        self.before_request().await?;
        let start = Instant::now();
        let result = tokio::time::timeout(self.request_timeout, self.inner.test_connection())
            .await
            .map_err(|_| LLMError::Timeout)
            .and_then(|result| result);
        self.record_result(start, &result.as_ref().map(|_| ()).map_err(clone_error))
            .await;
        result
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn list_models(&self) -> Result<Vec<String>, LLMError> {
        self.before_request().await?;
        let start = Instant::now();
        let result = tokio::time::timeout(self.request_timeout, self.inner.list_models())
            .await
            .map_err(|_| LLMError::Timeout)
            .and_then(|result| result);
        self.record_result(start, &result.as_ref().map(|_| ()).map_err(clone_error))
            .await;
        result
    }
}

trait CloneForRecording {
    fn clone_for_recording(&self) -> LLMError;
}

impl CloneForRecording for LLMError {
    fn clone_for_recording(&self) -> LLMError {
        clone_error(self)
    }
}

fn clone_error(error: &LLMError) -> LLMError {
    match error {
        LLMError::NotConfigured => LLMError::NotConfigured,
        LLMError::InvalidApiKey => LLMError::InvalidApiKey,
        LLMError::RateLimitExceeded => LLMError::RateLimitExceeded,
        LLMError::ContextLengthExceeded => LLMError::ContextLengthExceeded,
        LLMError::NetworkError(message) => LLMError::NetworkError(message.clone()),
        LLMError::ApiError(message) => LLMError::ApiError(message.clone()),
        LLMError::Timeout => LLMError::Timeout,
        LLMError::InvalidRequest(message) => LLMError::InvalidRequest(message.clone()),
        LLMError::SerializationError(message) => LLMError::SerializationError(message.clone()),
    }
}

#[cfg(test)]
mod tests;
