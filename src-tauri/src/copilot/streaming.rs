//! Streaming session lifecycle and diagnostics for Copilot responses.

use std::collections::HashMap;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

use serde::Serialize;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::app_events::{emit, AppEventEmitter};

pub const EVENT_STREAM_STARTED: &str = "stream_started";
pub const EVENT_STREAM_CHUNK: &str = "stream_chunk";
pub const EVENT_STREAM_FINISHED: &str = "stream_finished";
pub const EVENT_STREAM_CANCELLED: &str = "stream_cancelled";
pub const EVENT_STREAM_ERROR: &str = "stream_error";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamStatus {
    Started,
    Streaming,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamEventPayload {
    pub stream_id: Uuid,
    pub conversation_id: Uuid,
    pub content: Option<String>,
    pub message_id: Option<Uuid>,
    pub status: StreamStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StreamingDiagnostics {
    pub active_streams: u64,
    pub started_streams: u64,
    pub finished_streams: u64,
    pub cancelled_streams: u64,
    pub stream_errors: u64,
    pub streamed_tokens: u64,
    pub average_tokens_per_second: f64,
    pub average_first_token_latency_ms: f64,
    pub average_stream_duration_ms: f64,
    pub provider_streaming_health: f64,
}

#[derive(Default)]
pub struct StreamingMetricsCollector {
    started_streams: AtomicU64,
    finished_streams: AtomicU64,
    cancelled_streams: AtomicU64,
    stream_errors: AtomicU64,
    streamed_tokens: AtomicU64,
    total_first_token_latency_ms: AtomicU64,
    first_token_samples: AtomicU64,
    total_stream_duration_ms: AtomicU64,
    stream_duration_samples: AtomicU64,
}

impl StreamingMetricsCollector {
    pub fn record_started(&self) {
        self.started_streams.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_finished(&self) {
        self.finished_streams.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_cancelled(&self) {
        self.cancelled_streams.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_error(&self) {
        self.stream_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_token(&self) {
        self.streamed_tokens.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_first_token_latency(&self, latency_ms: u64) {
        self.total_first_token_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.first_token_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_stream_duration(&self, duration_ms: u64) {
        self.total_stream_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.stream_duration_samples.fetch_add(1, Ordering::Relaxed);
    }

    pub fn diagnostics(&self, active_streams: u64) -> StreamingDiagnostics {
        let started = self.started_streams.load(Ordering::Relaxed);
        let finished = self.finished_streams.load(Ordering::Relaxed);
        let duration_samples = self.stream_duration_samples.load(Ordering::Relaxed);
        let total_duration_ms = self.total_stream_duration_ms.load(Ordering::Relaxed);
        let streamed_tokens = self.streamed_tokens.load(Ordering::Relaxed);

        StreamingDiagnostics {
            active_streams,
            started_streams: started,
            finished_streams: finished,
            cancelled_streams: self.cancelled_streams.load(Ordering::Relaxed),
            stream_errors: self.stream_errors.load(Ordering::Relaxed),
            streamed_tokens,
            average_tokens_per_second: if total_duration_ms == 0 {
                0.0
            } else {
                streamed_tokens as f64 / (total_duration_ms as f64 / 1000.0)
            },
            average_first_token_latency_ms: average(
                self.total_first_token_latency_ms.load(Ordering::Relaxed),
                self.first_token_samples.load(Ordering::Relaxed),
            ),
            average_stream_duration_ms: average(total_duration_ms, duration_samples),
            provider_streaming_health: if started == 0 {
                1.0
            } else {
                finished as f64 / started as f64
            },
        }
    }
}

struct ActiveStream {
    conversation_id: Uuid,
    token: CancellationToken,
    started_at: Instant,
    handle: Option<JoinHandle<()>>,
}

pub struct StreamingSessionManager {
    emitter: Arc<dyn AppEventEmitter>,
    active: RwLock<HashMap<Uuid, ActiveStream>>,
    metrics: Arc<StreamingMetricsCollector>,
}

impl StreamingSessionManager {
    pub fn new(emitter: Arc<dyn AppEventEmitter>) -> Self {
        Self {
            emitter,
            active: RwLock::new(HashMap::new()),
            metrics: Arc::new(StreamingMetricsCollector::default()),
        }
    }

    pub async fn start_stream(&self, conversation_id: Uuid) -> (Uuid, CancellationToken) {
        let stream_id = Uuid::new_v4();
        let token = CancellationToken::new();
        self.active.write().await.insert(
            stream_id,
            ActiveStream {
                conversation_id,
                token: token.clone(),
                started_at: Instant::now(),
                handle: None,
            },
        );
        self.metrics.record_started();
        self.emit(
            EVENT_STREAM_STARTED,
            StreamEventPayload {
                stream_id,
                conversation_id,
                content: None,
                message_id: None,
                status: StreamStatus::Started,
                error: None,
            },
        );
        (stream_id, token)
    }

    pub async fn register_task<F>(&self, stream_id: Uuid, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        let mut active = self.active.write().await;
        if let Some(stream) = active.get_mut(&stream_id) {
            stream.handle = Some(handle);
        } else {
            handle.abort();
        }
    }

    pub async fn cancel_stream(&self, stream_id: Uuid) -> bool {
        let active = self.active.read().await;
        if let Some(stream) = active.get(&stream_id) {
            stream.token.cancel();
            true
        } else {
            false
        }
    }

    pub async fn cancel_all(&self) {
        let active = self.active.read().await;
        for stream in active.values() {
            stream.token.cancel();
        }
    }

    pub fn emit_chunk(&self, stream_id: Uuid, conversation_id: Uuid, content: String) {
        self.metrics.record_token();
        self.emit(
            EVENT_STREAM_CHUNK,
            StreamEventPayload {
                stream_id,
                conversation_id,
                content: Some(content),
                message_id: None,
                status: StreamStatus::Streaming,
                error: None,
            },
        );
    }

    pub async fn record_first_token(&self, stream_id: Uuid) {
        if let Some(stream) = self.active.read().await.get(&stream_id) {
            self.metrics
                .record_first_token_latency(stream.started_at.elapsed().as_millis() as u64);
        }
    }

    pub async fn finish_stream(&self, stream_id: Uuid, message_id: Uuid) {
        if let Some(stream) = self.cleanup(stream_id).await {
            self.metrics.record_finished();
            self.emit(
                EVENT_STREAM_FINISHED,
                StreamEventPayload {
                    stream_id,
                    conversation_id: stream.conversation_id,
                    content: None,
                    message_id: Some(message_id),
                    status: StreamStatus::Completed,
                    error: None,
                },
            );
        }
    }

    pub async fn cancel_finished_stream(&self, stream_id: Uuid) {
        if let Some(stream) = self.cleanup(stream_id).await {
            self.metrics.record_cancelled();
            self.emit(
                EVENT_STREAM_CANCELLED,
                StreamEventPayload {
                    stream_id,
                    conversation_id: stream.conversation_id,
                    content: None,
                    message_id: None,
                    status: StreamStatus::Cancelled,
                    error: None,
                },
            );
        }
    }

    pub async fn error_stream(&self, stream_id: Uuid, error: String) {
        if let Some(stream) = self.cleanup(stream_id).await {
            self.metrics.record_error();
            self.emit(
                EVENT_STREAM_ERROR,
                StreamEventPayload {
                    stream_id,
                    conversation_id: stream.conversation_id,
                    content: None,
                    message_id: None,
                    status: StreamStatus::Failed,
                    error: Some(error),
                },
            );
        }
    }

    pub async fn diagnostics(&self) -> StreamingDiagnostics {
        self.metrics
            .diagnostics(self.active.read().await.len() as u64)
    }

    async fn cleanup(&self, stream_id: Uuid) -> Option<ActiveStream> {
        let stream = self.active.write().await.remove(&stream_id);
        if let Some(stream) = &stream {
            self.metrics
                .record_stream_duration(stream.started_at.elapsed().as_millis() as u64);
        }
        stream
    }

    fn emit(&self, event: &str, payload: StreamEventPayload) {
        emit(self.emitter.as_ref(), event, &payload);
    }
}

impl Drop for StreamingSessionManager {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.try_write() {
            for stream in active.values_mut() {
                stream.token.cancel();
                if let Some(handle) = stream.handle.take() {
                    handle.abort();
                }
            }
            active.clear();
        }
    }
}

fn average(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}
