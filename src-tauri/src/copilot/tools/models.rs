use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::errors::DatabaseError;

pub const EVENT_TOOL_PROGRESS: &str = "tool_progress";

/// Request for a tool invocation.
#[derive(Debug, Clone)]
pub struct ToolInvocationRequest {
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub workspace_id: Option<Uuid>,
    pub cancellation_token: Option<CancellationToken>,
}

/// Available tools that the copilot can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub requires_confirmation: bool,
    pub category: ToolCategory,
    pub permission: ToolPermission,
    pub timeout_ms: u64,
    pub retry_policy: ToolRetryPolicy,
    pub supports_parallel: bool,
}

impl ToolDefinition {
    pub(crate) fn new(
        name: &str,
        description: &str,
        category: ToolCategory,
        parameters: Vec<ToolParameter>,
        permission: ToolPermission,
    ) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            parameters,
            requires_confirmation: permission.requires_confirmation,
            category,
            permission,
            timeout_ms: 10_000,
            retry_policy: ToolRetryPolicy::default(),
            supports_parallel: true,
        }
    }
}

/// Parameter definition for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub parameter_type: ToolParameterType,
    pub description: String,
    pub required: bool,
}

impl ToolParameter {
    pub(crate) fn required(
        name: &str,
        parameter_type: ToolParameterType,
        description: &str,
    ) -> Self {
        Self::new(name, parameter_type, description, true)
    }

    pub(crate) fn optional(
        name: &str,
        parameter_type: ToolParameterType,
        description: &str,
    ) -> Self {
        Self::new(name, parameter_type, description, false)
    }

    fn new(
        name: &str,
        parameter_type: ToolParameterType,
        description: &str,
        required: bool,
    ) -> Self {
        Self {
            name: name.to_string(),
            param_type: parameter_type.as_str().to_string(),
            parameter_type,
            description: description.to_string(),
            required,
        }
    }
}

/// JSON-level parameter types supported by validation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolParameterType {
    String,
    Number,
    Boolean,
    Object,
    Array,
}

impl ToolParameterType {
    fn as_str(self) -> &'static str {
        match self {
            ToolParameterType::String => "string",
            ToolParameterType::Number => "number",
            ToolParameterType::Boolean => "boolean",
            ToolParameterType::Object => "object",
            ToolParameterType::Array => "array",
        }
    }
}

/// Logical category used for discovery and future provider/MCP grouping.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Workspace,
    File,
    Search,
    Timeline,
    ContextMemory,
    External,
}

/// Permission metadata for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPermission {
    pub required_level: ToolPermissionLevel,
    pub requires_confirmation: bool,
    pub risk_level: ToolRiskLevel,
}

impl ToolPermission {
    pub(crate) fn read_only() -> Self {
        Self {
            required_level: ToolPermissionLevel::Read,
            requires_confirmation: false,
            risk_level: ToolRiskLevel::Low,
        }
    }

    pub(crate) fn write_with_confirmation() -> Self {
        Self {
            required_level: ToolPermissionLevel::Write,
            requires_confirmation: true,
            risk_level: ToolRiskLevel::Medium,
        }
    }
}

/// Permission level required to invoke a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolPermissionLevel {
    Read,
    Write,
    Destructive,
    External,
    Denied,
}

/// User-visible risk classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolRiskLevel {
    Low,
    Medium,
    High,
}

/// Retry policy applied by the invocation pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ToolRetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub retryable: bool,
}

impl Default for ToolRetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff_ms: 250,
            retryable: false,
        }
    }
}

/// Structured result produced by the tool invocation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInvocationResult {
    pub invocation_id: Uuid,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub status: ToolInvocationStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub attempts: u32,
}

impl ToolInvocationResult {
    pub(crate) fn success(
        invocation_id: Uuid,
        tool_name: String,
        arguments: serde_json::Value,
        result: serde_json::Value,
        started_at: DateTime<Utc>,
        duration: Duration,
        attempts: u32,
    ) -> Self {
        Self {
            invocation_id,
            tool_name,
            arguments,
            status: ToolInvocationStatus::Success,
            result: Some(result),
            error: None,
            started_at,
            completed_at: Utc::now(),
            duration_ms: duration.as_millis() as u64,
            attempts,
        }
    }

    pub(crate) fn failed(
        invocation_id: Uuid,
        tool_name: String,
        arguments: serde_json::Value,
        error: String,
        started_at: DateTime<Utc>,
        duration: Duration,
        attempts: u32,
    ) -> Self {
        Self {
            invocation_id,
            tool_name,
            arguments,
            status: ToolInvocationStatus::Failed,
            result: None,
            error: Some(error),
            started_at,
            completed_at: Utc::now(),
            duration_ms: duration.as_millis() as u64,
            attempts,
        }
    }

    pub(crate) fn cancelled(
        invocation_id: Uuid,
        tool_name: String,
        arguments: serde_json::Value,
        started_at: DateTime<Utc>,
        duration: Duration,
    ) -> Self {
        Self {
            invocation_id,
            tool_name,
            arguments,
            status: ToolInvocationStatus::Cancelled,
            result: None,
            error: Some("tool invocation cancelled".to_string()),
            started_at,
            completed_at: Utc::now(),
            duration_ms: duration.as_millis() as u64,
            attempts: 0,
        }
    }

    pub(crate) fn into_database_result(self) -> Result<serde_json::Value, DatabaseError> {
        match self.status {
            ToolInvocationStatus::Success => Ok(self.result.unwrap_or(serde_json::Value::Null)),
            ToolInvocationStatus::Cancelled => Err(DatabaseError::IoError(
                self.error
                    .unwrap_or_else(|| "tool invocation cancelled".to_string()),
            )),
            ToolInvocationStatus::Failed => Err(DatabaseError::IoError(
                self.error
                    .unwrap_or_else(|| "tool invocation failed".to_string()),
            )),
            ToolInvocationStatus::Pending | ToolInvocationStatus::Running => Err(
                DatabaseError::IoError("tool invocation incomplete".to_string()),
            ),
        }
    }
}

/// Invocation lifecycle status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolInvocationStatus {
    Pending,
    Running,
    Success,
    Failed,
    Cancelled,
}

/// Progress event emitted by the tool pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProgressEvent {
    pub invocation_id: Uuid,
    pub tool_name: String,
    pub status: ToolInvocationStatus,
    pub message: String,
    pub attempt: u32,
    pub max_attempts: u32,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl ToolProgressEvent {
    pub(crate) fn started(invocation_id: Uuid, tool_name: &str, created_at: DateTime<Utc>) -> Self {
        Self {
            invocation_id,
            tool_name: tool_name.to_string(),
            status: ToolInvocationStatus::Pending,
            message: format!("Starting tool '{}'", tool_name),
            attempt: 0,
            max_attempts: 0,
            result: None,
            error: None,
            created_at,
        }
    }

    pub(crate) fn running(
        invocation_id: Uuid,
        tool_name: &str,
        attempt: u32,
        max_attempts: u32,
    ) -> Self {
        Self {
            invocation_id,
            tool_name: tool_name.to_string(),
            status: ToolInvocationStatus::Running,
            message: format!("Running tool '{}'", tool_name),
            attempt,
            max_attempts,
            result: None,
            error: None,
            created_at: Utc::now(),
        }
    }

    pub(crate) fn from_result(result: &ToolInvocationResult) -> Self {
        Self {
            invocation_id: result.invocation_id,
            tool_name: result.tool_name.clone(),
            status: result.status,
            message: format!(
                "Tool '{}' finished with {:?}",
                result.tool_name, result.status
            ),
            attempt: result.attempts,
            max_attempts: result.attempts,
            result: result.result.clone(),
            error: result.error.clone(),
            created_at: result.completed_at,
        }
    }
}
