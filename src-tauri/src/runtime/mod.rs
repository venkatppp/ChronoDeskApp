//! Real-time Intelligence Runtime
//!
//! Provides event-driven updates, caching, and background workers for
//! predictive intelligence, workflow detection, health monitoring, and
//! context memory.

pub mod cache;
pub mod diagnostics;
pub mod emitter;
pub mod health;
pub mod recovery;
pub mod settings;
pub mod shutdown;
pub mod workers;

pub use cache::IntelligenceCache;
pub use diagnostics::{DiagnosticsService, RuntimeDiagnostics};
pub use emitter::IntelligenceEmitter;
pub use health::{RuntimeHealth, RuntimeHealthService};
pub use recovery::{RecoveryService, RecoveryState};
pub use settings::RuntimeSettings;
pub use shutdown::ShutdownCoordinator;
pub use workers::RuntimeWorkers;
