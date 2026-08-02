//! Tool Executor - Safe execution framework for copilot tools.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::services::WorkspaceService;
use crate::session::SessionEngine;
use crate::timeline::TimelineEngine;

const TOOL_PROGRESS_BUFFER: usize = 256;

type ToolFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, DatabaseError>> + Send + 'a>>;

#[derive(Clone)]
struct ToolHandler {
    definition: ToolDefinition,
    execute: for<'a> fn(&'a ToolExecutor, &'a serde_json::Value) -> ToolFuture<'a>,
}

/// Tool executor that safely invokes registered copilot tools.
pub struct ToolExecutor {
    workspace_service: Arc<WorkspaceService>,
    session_engine: Arc<SessionEngine>,
    timeline_engine: Arc<TimelineEngine>,
    registry: HashMap<String, ToolHandler>,
    progress_tx: broadcast::Sender<ToolProgressEvent>,
    metrics: Arc<ToolMetricsCollector>,
}

impl ToolExecutor {
    /// Creates a new tool executor.
    pub fn new(
        workspace_service: Arc<WorkspaceService>,
        session_engine: Arc<SessionEngine>,
        timeline_engine: Arc<TimelineEngine>,
    ) -> Self {
        let (progress_tx, _) = broadcast::channel(TOOL_PROGRESS_BUFFER);
        let registry = Self::build_registry();

        Self {
            workspace_service,
            session_engine,
            timeline_engine,
            registry,
            progress_tx,
            metrics: Arc::new(ToolMetricsCollector::default()),
        }
    }

    /// Executes a tool and returns only the raw tool payload.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let result = self.invoke_tool(tool_name, arguments.clone()).await?;
        result.into_database_result()
    }

    /// Runs a complete invocation pipeline with validation, permission checks,
    /// timeout, retries, progress events, and structured results.
    pub async fn invoke_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<ToolInvocationResult, DatabaseError> {
        self.invoke_tool_with_context(ToolInvocationRequest {
            tool_name: tool_name.to_string(),
            arguments,
            workspace_id: None,
            cancellation_token: None,
        })
        .await
    }

    /// Runs a tool invocation with caller-provided execution context.
    pub async fn invoke_tool_with_context(
        &self,
        request: ToolInvocationRequest,
    ) -> Result<ToolInvocationResult, DatabaseError> {
        let handler = self.handler(&request.tool_name)?;
        let invocation_id = Uuid::new_v4();
        let started_at = Utc::now();
        let started_instant = Instant::now();

        self.validate_arguments(&handler.definition, &request.arguments)?;
        self.ensure_permission(&handler.definition, request.workspace_id)?;
        self.metrics.record_invocation();
        self.emit_progress(ToolProgressEvent::started(
            invocation_id,
            &handler.definition.name,
            started_at,
        ));

        let max_attempts = handler.definition.retry_policy.max_attempts.max(1);
        let mut attempt = 1;
        let mut last_error = None;

        while attempt <= max_attempts {
            if let Some(token) = &request.cancellation_token {
                if token.is_cancelled() {
                    let result = ToolInvocationResult::cancelled(
                        invocation_id,
                        handler.definition.name.clone(),
                        request.arguments.clone(),
                        started_at,
                        started_instant.elapsed(),
                    );
                    self.metrics.record_cancelled();
                    self.emit_progress(ToolProgressEvent::from_result(&result));
                    return Ok(result);
                }
            }

            self.emit_progress(ToolProgressEvent::running(
                invocation_id,
                &handler.definition.name,
                attempt,
                max_attempts,
            ));

            let execute = (handler.execute)(self, &request.arguments);
            let timeout = Duration::from_millis(handler.definition.timeout_ms);
            let attempt_result = if let Some(token) = &request.cancellation_token {
                tokio::select! {
                    _ = token.cancelled() => Err(DatabaseError::IoError("tool invocation cancelled".to_string())),
                    result = tokio::time::timeout(timeout, execute) => result.map_err(|_| DatabaseError::IoError(format!("tool '{}' timed out after {}ms", handler.definition.name, handler.definition.timeout_ms)))?,
                }
            } else {
                tokio::time::timeout(timeout, execute).await.map_err(|_| {
                    DatabaseError::IoError(format!(
                        "tool '{}' timed out after {}ms",
                        handler.definition.name, handler.definition.timeout_ms
                    ))
                })?
            };

            match attempt_result {
                Ok(value) => {
                    let result = ToolInvocationResult::success(
                        invocation_id,
                        handler.definition.name.clone(),
                        request.arguments,
                        value,
                        started_at,
                        started_instant.elapsed(),
                        attempt,
                    );
                    self.metrics.record_success(started_instant.elapsed());
                    self.emit_progress(ToolProgressEvent::from_result(&result));
                    return Ok(result);
                }
                Err(error)
                    if attempt < max_attempts && handler.definition.retry_policy.retryable =>
                {
                    last_error = Some(error.to_string());
                    self.metrics.record_retry();
                    tokio::time::sleep(Duration::from_millis(
                        handler.definition.retry_policy.backoff_ms * attempt as u64,
                    ))
                    .await;
                }
                Err(error) => {
                    last_error = Some(error.to_string());
                    break;
                }
            }

            attempt += 1;
        }

        let result = ToolInvocationResult::failed(
            invocation_id,
            handler.definition.name.clone(),
            request.arguments,
            last_error.unwrap_or_else(|| "tool invocation failed".to_string()),
            started_at,
            started_instant.elapsed(),
            attempt.min(max_attempts),
        );
        self.metrics.record_failure(started_instant.elapsed());
        self.emit_progress(ToolProgressEvent::from_result(&result));
        Ok(result)
    }

    /// Executes independent tools concurrently and returns structured results
    /// in the same order as the requests.
    pub async fn invoke_tools_parallel(
        &self,
        requests: Vec<ToolInvocationRequest>,
    ) -> Vec<Result<ToolInvocationResult, DatabaseError>> {
        futures::future::join_all(
            requests
                .into_iter()
                .map(|request| self.invoke_tool_with_context(request)),
        )
        .await
    }

    /// Checks if a tool requires user confirmation.
    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        self.registry
            .get(tool_name)
            .map(|handler| handler.definition.requires_confirmation)
            .unwrap_or(false)
    }

    /// Returns registry-backed tool definitions.
    pub fn available_tools(&self) -> Vec<ToolDefinition> {
        let mut tools: Vec<_> = self
            .registry
            .values()
            .map(|handler| handler.definition.clone())
            .collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    /// Subscribes to tool progress events for future streaming integrations.
    pub fn subscribe_progress(&self) -> broadcast::Receiver<ToolProgressEvent> {
        self.progress_tx.subscribe()
    }

    /// Returns aggregate tool diagnostics.
    pub fn diagnostics(&self) -> ToolDiagnostics {
        self.metrics.diagnostics(self.registry.len())
    }

    /// Backwards-compatible static metadata for IPC callers that do not hold state.
    pub fn get_available_tools() -> Vec<ToolDefinition> {
        let mut tools: Vec<_> = Self::build_registry()
            .into_values()
            .map(|handler| handler.definition)
            .collect();
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        tools
    }

    fn handler(&self, tool_name: &str) -> Result<&ToolHandler, DatabaseError> {
        self.registry
            .get(tool_name)
            .ok_or_else(|| DatabaseError::InvalidInput(format!("Unknown tool: {}", tool_name)))
    }

    fn validate_arguments(
        &self,
        definition: &ToolDefinition,
        arguments: &serde_json::Value,
    ) -> Result<(), DatabaseError> {
        let object = arguments.as_object().ok_or_else(|| {
            DatabaseError::InvalidInput(format!(
                "tool '{}' arguments must be a JSON object",
                definition.name
            ))
        })?;

        for parameter in &definition.parameters {
            let value = object.get(&parameter.name);
            if parameter.required && value.is_none() {
                return Err(DatabaseError::InvalidInput(format!(
                    "tool '{}' missing required argument '{}'",
                    definition.name, parameter.name
                )));
            }

            if let Some(value) = value {
                validate_parameter_type(&definition.name, parameter, value)?;
            }
        }

        Ok(())
    }

    fn ensure_permission(
        &self,
        definition: &ToolDefinition,
        _workspace_id: Option<Uuid>,
    ) -> Result<(), DatabaseError> {
        if definition.permission.required_level == ToolPermissionLevel::Denied {
            return Err(DatabaseError::InvalidInput(format!(
                "tool '{}' is not permitted",
                definition.name
            )));
        }
        Ok(())
    }

    fn emit_progress(&self, event: ToolProgressEvent) {
        let _ = self.progress_tx.send(event);
    }

    fn build_registry() -> HashMap<String, ToolHandler> {
        [
            tool_handler(
                ToolDefinition::new(
                    "list_workspaces",
                    "List all active workspaces",
                    ToolCategory::Workspace,
                    vec![],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.list_workspaces(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "get_workspace",
                    "Get details of a specific workspace",
                    ToolCategory::Workspace,
                    vec![ToolParameter::required(
                        "workspace_id",
                        ToolParameterType::String,
                        "UUID of the workspace",
                    )],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.get_workspace(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "get_active_workspace",
                    "Get the currently active workspace",
                    ToolCategory::Workspace,
                    vec![],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.get_active_workspace(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "get_recent_events",
                    "Get recent timeline events",
                    ToolCategory::Timeline,
                    vec![
                        ToolParameter::optional(
                            "workspace_id",
                            ToolParameterType::String,
                            "Optional workspace ID to filter events",
                        ),
                        ToolParameter::optional(
                            "limit",
                            ToolParameterType::Number,
                            "Maximum number of events to return",
                        ),
                    ],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.get_recent_events(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "search_timeline",
                    "Search timeline events by query",
                    ToolCategory::Search,
                    vec![
                        ToolParameter::required("query", ToolParameterType::String, "Search query"),
                        ToolParameter::optional(
                            "workspace_id",
                            ToolParameterType::String,
                            "Optional workspace ID to filter",
                        ),
                    ],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.search_timeline(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "get_session_summary",
                    "Get a summary of the current or recent session",
                    ToolCategory::ContextMemory,
                    vec![ToolParameter::optional(
                        "workspace_id",
                        ToolParameterType::String,
                        "Optional workspace ID",
                    )],
                    ToolPermission::read_only(),
                ),
                |executor, arguments| Box::pin(executor.get_session_summary(arguments)),
            ),
            tool_handler(
                ToolDefinition::new(
                    "resume_workspace",
                    "Resume work in a specific workspace",
                    ToolCategory::Workspace,
                    vec![ToolParameter::required(
                        "workspace_id",
                        ToolParameterType::String,
                        "UUID of the workspace to resume",
                    )],
                    ToolPermission::write_with_confirmation(),
                ),
                |executor, arguments| Box::pin(executor.resume_workspace(arguments)),
            ),
        ]
        .into_iter()
        .map(|handler| (handler.definition.name.clone(), handler))
        .collect()
    }

    /// Lists all workspaces.
    async fn list_workspaces(
        &self,
        _arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let workspaces = self.workspace_service.list_active_workspaces().await?;
        serde_json::to_value(workspaces).map_err(|e| DatabaseError::IoError(e.to_string()))
    }

    /// Gets a specific workspace.
    async fn get_workspace(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let workspace_id = arguments
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DatabaseError::InvalidInput("Missing workspace_id".to_string()))?;

        let workspace_uuid = Uuid::parse_str(workspace_id)
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

        let workspace = self.workspace_service.get_workspace(workspace_uuid).await?;

        serde_json::to_value(workspace).map_err(|e| DatabaseError::IoError(e.to_string()))
    }

    /// Gets the currently active workspace.
    async fn get_active_workspace(
        &self,
        _arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let session = self
            .session_engine
            .get_most_recent_active_session(None)
            .await?;

        if let Some(sess) = session {
            let workspace = self
                .workspace_service
                .get_workspace(sess.workspace_id)
                .await?;
            Ok(serde_json::to_value(workspace)
                .map_err(|e| DatabaseError::IoError(e.to_string()))?)
        } else {
            Ok(serde_json::json!(null))
        }
    }

    /// Gets recent timeline events.
    async fn get_recent_events(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let workspace_id = arguments
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| l as i64);

        let events = if let Some(ws_id) = workspace_id {
            self.timeline_engine.recent_events(ws_id, limit).await?
        } else {
            Vec::new()
        };

        serde_json::to_value(events).map_err(|e| DatabaseError::IoError(e.to_string()))
    }

    /// Searches timeline events.
    async fn search_timeline(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DatabaseError::InvalidInput("Missing query".to_string()))?;

        let workspace_id = arguments
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

        let events = if let Some(ws_id) = workspace_id {
            self.timeline_engine.recent_events(ws_id, Some(100)).await?
        } else {
            Vec::new()
        };

        let query_lower = query.to_lowercase();
        let filtered: Vec<_> = events
            .into_iter()
            .filter(|event| {
                format!("{:?}", event.event_type)
                    .to_lowercase()
                    .contains(&query_lower)
            })
            .take(20)
            .collect();

        serde_json::to_value(filtered).map_err(|e| DatabaseError::IoError(e.to_string()))
    }

    /// Gets a session summary.
    async fn get_session_summary(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let workspace_id = arguments
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

        if let Some(ws_id) = workspace_id {
            if let Some(session) = self.session_engine.get_latest_session(ws_id, None).await? {
                let workspace = self.workspace_service.get_workspace(ws_id).await?;
                let summary = self
                    .session_engine
                    .get_session_summary(&session, workspace.name)
                    .await?;
                Ok(serde_json::to_value(summary)
                    .map_err(|e| DatabaseError::IoError(e.to_string()))?)
            } else {
                Ok(serde_json::json!({
                    "message": "No session found for this workspace"
                }))
            }
        } else {
            Ok(serde_json::json!({
                "message": "No workspace specified"
            }))
        }
    }

    /// Resumes a workspace session.
    async fn resume_workspace(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        let workspace_id = arguments
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DatabaseError::InvalidInput("Missing workspace_id".to_string()))?;

        let workspace_uuid = Uuid::parse_str(workspace_id)
            .map_err(|e| DatabaseError::InvalidInput(e.to_string()))?;

        self.workspace_service
            .open_workspace(workspace_uuid)
            .await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Workspace activated successfully"
        }))
    }
}

fn tool_handler(
    definition: ToolDefinition,
    execute: for<'a> fn(&'a ToolExecutor, &'a serde_json::Value) -> ToolFuture<'a>,
) -> ToolHandler {
    ToolHandler {
        definition,
        execute,
    }
}

fn validate_parameter_type(
    tool_name: &str,
    parameter: &ToolParameter,
    value: &serde_json::Value,
) -> Result<(), DatabaseError> {
    let valid = match parameter.parameter_type {
        ToolParameterType::String => value.is_string() || value.is_null() && !parameter.required,
        ToolParameterType::Number => value.is_number() || value.is_null() && !parameter.required,
        ToolParameterType::Boolean => value.is_boolean() || value.is_null() && !parameter.required,
        ToolParameterType::Object => value.is_object() || value.is_null() && !parameter.required,
        ToolParameterType::Array => value.is_array() || value.is_null() && !parameter.required,
    };

    if valid {
        Ok(())
    } else {
        Err(DatabaseError::InvalidInput(format!(
            "tool '{}' argument '{}' must be {}",
            tool_name, parameter.name, parameter.param_type
        )))
    }
}

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
    fn new(
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
    fn required(name: &str, parameter_type: ToolParameterType, description: &str) -> Self {
        Self::new(name, parameter_type, description, true)
    }

    fn optional(name: &str, parameter_type: ToolParameterType, description: &str) -> Self {
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
    fn read_only() -> Self {
        Self {
            required_level: ToolPermissionLevel::Read,
            requires_confirmation: false,
            risk_level: ToolRiskLevel::Low,
        }
    }

    fn write_with_confirmation() -> Self {
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
    fn success(
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

    fn failed(
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

    fn cancelled(
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

    fn into_database_result(self) -> Result<serde_json::Value, DatabaseError> {
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
    fn started(invocation_id: Uuid, tool_name: &str, created_at: DateTime<Utc>) -> Self {
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

    fn running(invocation_id: Uuid, tool_name: &str, attempt: u32, max_attempts: u32) -> Self {
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

    fn from_result(result: &ToolInvocationResult) -> Self {
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

#[derive(Default)]
struct ToolMetricsCollector {
    invocations: AtomicU64,
    successes: AtomicU64,
    failures: AtomicU64,
    cancellations: AtomicU64,
    retries: AtomicU64,
    total_duration_ms: AtomicU64,
    duration_samples: AtomicU64,
}

impl ToolMetricsCollector {
    fn record_invocation(&self) {
        self.invocations.fetch_add(1, Ordering::Relaxed);
    }

    fn record_success(&self, duration: Duration) {
        self.successes.fetch_add(1, Ordering::Relaxed);
        self.record_duration(duration);
    }

    fn record_failure(&self, duration: Duration) {
        self.failures.fetch_add(1, Ordering::Relaxed);
        self.record_duration(duration);
    }

    fn record_cancelled(&self) {
        self.cancellations.fetch_add(1, Ordering::Relaxed);
    }

    fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_duration(&self, duration: Duration) {
        self.total_duration_ms
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        self.duration_samples.fetch_add(1, Ordering::Relaxed);
    }

    fn diagnostics(&self, registered_tools: usize) -> ToolDiagnostics {
        let samples = self.duration_samples.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);
        let invocations = self.invocations.load(Ordering::Relaxed);
        let successes = self.successes.load(Ordering::Relaxed);

        ToolDiagnostics {
            registered_tools,
            total_invocations: invocations,
            successful_invocations: successes,
            failed_invocations: self.failures.load(Ordering::Relaxed),
            cancelled_invocations: self.cancellations.load(Ordering::Relaxed),
            retried_invocations: self.retries.load(Ordering::Relaxed),
            average_duration_ms: if samples == 0 {
                0.0
            } else {
                total_duration as f64 / samples as f64
            },
            success_rate: if invocations == 0 {
                1.0
            } else {
                successes as f64 / invocations as f64
            },
        }
    }
}

/// Aggregate tool framework diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDiagnostics {
    pub registered_tools: usize,
    pub total_invocations: u64,
    pub successful_invocations: u64,
    pub failed_invocations: u64,
    pub cancelled_invocations: u64,
    pub retried_invocations: u64,
    pub average_duration_ms: f64,
    pub success_rate: f64,
}
