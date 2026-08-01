//! Real-time Intelligence Runtime
//!
//! Provides event-driven updates, caching, and background workers for
//! predictive intelligence, workflow detection, health monitoring, and
//! context memory.

pub mod cache;
pub mod emitter;
pub mod workers;

pub use cache::IntelligenceCache;
pub use emitter::IntelligenceEmitter;
pub use workers::RuntimeWorkers;
