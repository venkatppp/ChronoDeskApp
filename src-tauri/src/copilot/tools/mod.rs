//! Tool calling framework for copilot and execution engines.

mod executor;
mod metrics;
mod models;
mod permissions;
mod registry;

pub use executor::ToolExecutor;
pub use metrics::ToolDiagnostics;
pub use models::{
    ToolCategory, ToolDefinition, ToolInvocationRequest, ToolInvocationResult,
    ToolInvocationStatus, ToolParameter, ToolParameterType, ToolPermission, ToolPermissionDecision,
    ToolPermissionLevel, ToolPermissionPolicy, ToolProgressEvent, ToolRetryPolicy, ToolRiskLevel,
    EVENT_TOOL_PROGRESS,
};
pub use permissions::ToolPermissionService;
