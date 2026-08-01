//! Graceful Shutdown Coordinator
//!
//! Coordinates clean shutdown of all runtime components.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Shutdown signal coordinator for graceful termination.
#[derive(Clone)]
pub struct ShutdownCoordinator {
    shutdown_flag: Arc<AtomicBool>,
    shutdown_tx: broadcast::Sender<()>,
}

impl ShutdownCoordinator {
    /// Creates a new shutdown coordinator.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(16);
        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            shutdown_tx: tx,
        }
    }

    /// Signals shutdown to all components.
    pub fn shutdown(&self) {
        self.shutdown_flag.store(true, Ordering::SeqCst);
        let _ = self.shutdown_tx.send(());
        tracing::info!("Shutdown signal sent to all components");
    }

    /// Returns true if shutdown has been requested.
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Subscribes to shutdown notifications.
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }
}

impl Default for ShutdownCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_flag_is_initially_false() {
        let coordinator = ShutdownCoordinator::new();
        assert!(!coordinator.is_shutdown());
    }

    #[tokio::test]
    async fn shutdown_sets_flag() {
        let coordinator = ShutdownCoordinator::new();
        coordinator.shutdown();
        assert!(coordinator.is_shutdown());
    }

    #[tokio::test]
    async fn subscribers_receive_shutdown_signal() {
        let coordinator = ShutdownCoordinator::new();
        let mut rx = coordinator.subscribe();

        coordinator.shutdown();

        // Should receive shutdown signal
        assert!(rx.recv().await.is_ok());
    }
}
