//! Individual scoring factor calculators.
//!
//! Each calculator implements one dimension of context scoring and is
//! independently testable.

mod completion_signals;
mod context_switching;
mod deep_editing;
mod focus_duration;
mod workspace_consistency;

pub use completion_signals::CompletionSignalsCalculator;
pub use context_switching::ContextSwitchingCalculator;
pub use deep_editing::DeepEditingCalculator;
pub use focus_duration::FocusDurationCalculator;
pub use workspace_consistency::WorkspaceConsistencyCalculator;
