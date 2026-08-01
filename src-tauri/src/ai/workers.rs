//! Background workers for embedding generation.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::time;

use crate::ai::onnx_provider::ONNXEmbeddingProvider;
use crate::errors::DatabaseError;
use crate::semantic::embeddings::EmbeddingProvider;
use crate::semantic::models::IndexDocumentRequest;
use crate::semantic::repository::SemanticRepository;

/// Request to generate embeddings in the background.
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    pub document: IndexDocumentRequest,
}

/// Background worker for generating embeddings.
pub struct EmbeddingWorker {
    provider: Arc<ONNXEmbeddingProvider>,
    repository: SemanticRepository,
    receiver: mpsc::Receiver<EmbeddingRequest>,
    batch_size: usize,
}

impl EmbeddingWorker {
    /// Creates a new embedding worker.
    pub fn new(
        provider: Arc<ONNXEmbeddingProvider>,
        repository: SemanticRepository,
        receiver: mpsc::Receiver<EmbeddingRequest>,
        batch_size: usize,
    ) -> Self {
        Self {
            provider,
            repository,
            receiver,
            batch_size,
        }
    }

    /// Starts the worker loop.
    pub async fn run(mut self) {
        let mut batch = Vec::new();

        loop {
            tokio::select! {
                // Receive new requests
                request = self.receiver.recv() => {
                    match request {
                        Some(req) => {
                            batch.push(req);

                            // Process batch if full
                            if batch.len() >= self.batch_size {
                                self.process_batch(&mut batch).await;
                            }
                        }
                        None => {
                            // Channel closed - process remaining and exit
                            if !batch.is_empty() {
                                self.process_batch(&mut batch).await;
                            }
                            break;
                        }
                    }
                }

                // Process batch periodically even if not full
                _ = time::sleep(Duration::from_secs(5)) => {
                    if !batch.is_empty() {
                        self.process_batch(&mut batch).await;
                    }
                }
            }
        }
    }

    /// Processes a batch of embedding requests.
    async fn process_batch(&self, batch: &mut Vec<EmbeddingRequest>) {
        for request in batch.drain(..) {
            if let Err(e) = self.process_request(request).await {
                log::error!("Failed to process embedding request: {}", e);
            }
        }
    }

    /// Processes a single embedding request.
    async fn process_request(&self, request: EmbeddingRequest) -> Result<(), DatabaseError> {
        // Generate embedding
        let combined_text = format!("{} {}", request.document.title, request.document.content);
        let embedding = self.provider.embed(&combined_text).await?;

        // Store in repository
        self.repository
            .index_document(request.document, Some(embedding))
            .await?;

        Ok(())
    }
}

/// Manager for embedding workers.
pub struct EmbeddingWorkerPool {
    sender: mpsc::Sender<EmbeddingRequest>,
}

impl EmbeddingWorkerPool {
    /// Creates a new worker pool.
    pub fn new(
        provider: Arc<ONNXEmbeddingProvider>,
        repository: SemanticRepository,
        _num_workers: usize,
        batch_size: usize,
        queue_size: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(queue_size);

        // Spawn the first worker with the receiver
        let worker = EmbeddingWorker::new(
            provider.clone(),
            repository.clone(),
            receiver,
            batch_size,
        );

        tokio::spawn(async move {
            worker.run().await;
        });

        // Note: For multiple workers, we would need a more sophisticated
        // work distribution mechanism (e.g., multiple channels or a shared queue)
        // For now, we spawn a single worker

        Self { sender }
    }

    /// Submits a document for background embedding generation.
    pub async fn submit(&self, document: IndexDocumentRequest) -> Result<(), DatabaseError> {
        self.sender
            .send(EmbeddingRequest { document })
            .await
            .map_err(|e| {
                DatabaseError::InvalidInput(format!("Failed to submit embedding request: {}", e))
            })
    }

    /// Creates a simplified single-worker pool.
    pub fn single_worker(
        provider: Arc<ONNXEmbeddingProvider>,
        repository: SemanticRepository,
        batch_size: usize,
    ) -> Self {
        let (sender, receiver) = mpsc::channel(1000);

        let worker = EmbeddingWorker::new(provider, repository, receiver, batch_size);

        tokio::spawn(async move {
            worker.run().await;
        });

        Self { sender }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_worker_compiles() {
        // Placeholder test to ensure module compiles
        // Real tests would require ONNX models
    }
}
