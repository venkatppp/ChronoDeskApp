use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use crate::copilot::tools::executor::ToolExecutor;
use crate::copilot::tools::models::{
    ToolCategory, ToolDefinition, ToolParameter, ToolParameterType, ToolPermission,
};
use crate::errors::DatabaseError;

pub(crate) type ToolFuture<'a> =
    Pin<Box<dyn Future<Output = Result<serde_json::Value, DatabaseError>> + Send + 'a>>;

#[derive(Clone)]
pub(crate) struct ToolHandler {
    pub(crate) definition: ToolDefinition,
    pub(crate) execute: for<'a> fn(&'a ToolExecutor, &'a serde_json::Value) -> ToolFuture<'a>,
}

pub(crate) fn build_registry() -> HashMap<String, ToolHandler> {
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

fn tool_handler(
    definition: ToolDefinition,
    execute: for<'a> fn(&'a ToolExecutor, &'a serde_json::Value) -> ToolFuture<'a>,
) -> ToolHandler {
    ToolHandler {
        definition,
        execute,
    }
}
