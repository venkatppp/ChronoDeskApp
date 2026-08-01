//! Tool Executor - Safe execution framework for copilot tools.

use std::sync::Arc;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::services::WorkspaceService;
use crate::session::SessionEngine;
use crate::timeline::TimelineEngine;

/// Tool executor that safely invokes existing IPC commands.
pub struct ToolExecutor {
    workspace_service: Arc<WorkspaceService>,
    session_engine: Arc<SessionEngine>,
    timeline_engine: Arc<TimelineEngine>,
}

impl ToolExecutor {
    /// Creates a new tool executor.
    pub fn new(
        workspace_service: Arc<WorkspaceService>,
        session_engine: Arc<SessionEngine>,
        timeline_engine: Arc<TimelineEngine>,
    ) -> Self {
        Self {
            workspace_service,
            session_engine,
            timeline_engine,
        }
    }

    /// Executes a tool and returns the result.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<serde_json::Value, DatabaseError> {
        match tool_name {
            "list_workspaces" => self.list_workspaces(arguments).await,
            "get_workspace" => self.get_workspace(arguments).await,
            "get_active_workspace" => self.get_active_workspace(arguments).await,
            "get_recent_events" => self.get_recent_events(arguments).await,
            "search_timeline" => self.search_timeline(arguments).await,
            "get_session_summary" => self.get_session_summary(arguments).await,
            "resume_workspace" => self.resume_workspace(arguments).await,
            _ => Err(DatabaseError::InvalidInput(format!(
                "Unknown tool: {}",
                tool_name
            ))),
        }
    }

    /// Checks if a tool requires user confirmation.
    pub fn requires_confirmation(&self, tool_name: &str) -> bool {
        matches!(
            tool_name,
            "delete_workspace" | "clear_history" | "execute_action"
        )
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
        // Get the most recent active session
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

        // Simple search implementation - would use search engine in full implementation
        let events = if let Some(ws_id) = workspace_id {
            self.timeline_engine.recent_events(ws_id, Some(100)).await?
        } else {
            Vec::new()
        };

        let filtered: Vec<_> = events
            .into_iter()
            .filter(|e| {
                format!("{:?}", e.event_type)
                    .to_lowercase()
                    .contains(&query.to_lowercase())
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
            // Get the latest session for this workspace
            if let Some(session) = self.session_engine.get_latest_session(ws_id, None).await? {
                // Get workspace name
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

        // Open/switch to workspace
        self.workspace_service
            .open_workspace(workspace_uuid)
            .await?;

        Ok(serde_json::json!({
            "success": true,
            "message": "Workspace activated successfully"
        }))
    }
}

/// Available tools that the copilot can use.
#[derive(Debug, Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ToolParameter>,
    pub requires_confirmation: bool,
}

/// Parameter definition for a tool.
#[derive(Debug, Clone)]
pub struct ToolParameter {
    pub name: String,
    pub param_type: String,
    pub description: String,
    pub required: bool,
}

impl ToolExecutor {
    /// Returns all available tool definitions.
    pub fn get_available_tools() -> Vec<ToolDefinition> {
        vec![
            ToolDefinition {
                name: "list_workspaces".to_string(),
                description: "List all workspaces".to_string(),
                parameters: vec![],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "get_workspace".to_string(),
                description: "Get details of a specific workspace".to_string(),
                parameters: vec![ToolParameter {
                    name: "workspace_id".to_string(),
                    param_type: "string".to_string(),
                    description: "UUID of the workspace".to_string(),
                    required: true,
                }],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "get_active_workspace".to_string(),
                description: "Get the currently active workspace".to_string(),
                parameters: vec![],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "get_recent_events".to_string(),
                description: "Get recent timeline events".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "workspace_id".to_string(),
                        param_type: "string".to_string(),
                        description: "Optional workspace ID to filter events".to_string(),
                        required: false,
                    },
                    ToolParameter {
                        name: "limit".to_string(),
                        param_type: "number".to_string(),
                        description: "Maximum number of events to return (default: 50)".to_string(),
                        required: false,
                    },
                ],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "search_timeline".to_string(),
                description: "Search timeline events by query".to_string(),
                parameters: vec![
                    ToolParameter {
                        name: "query".to_string(),
                        param_type: "string".to_string(),
                        description: "Search query".to_string(),
                        required: true,
                    },
                    ToolParameter {
                        name: "workspace_id".to_string(),
                        param_type: "string".to_string(),
                        description: "Optional workspace ID to filter".to_string(),
                        required: false,
                    },
                ],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "get_session_summary".to_string(),
                description: "Get a summary of the current or recent session".to_string(),
                parameters: vec![ToolParameter {
                    name: "workspace_id".to_string(),
                    param_type: "string".to_string(),
                    description: "Optional workspace ID".to_string(),
                    required: false,
                }],
                requires_confirmation: false,
            },
            ToolDefinition {
                name: "resume_workspace".to_string(),
                description: "Resume work in a specific workspace".to_string(),
                parameters: vec![ToolParameter {
                    name: "workspace_id".to_string(),
                    param_type: "string".to_string(),
                    description: "UUID of the workspace to resume".to_string(),
                    required: true,
                }],
                requires_confirmation: false,
            },
        ]
    }
}
