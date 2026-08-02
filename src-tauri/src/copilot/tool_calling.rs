//! LLM-native tool calling.
//!
//! Wires the LLM's provider-native tool/function calls into the existing
//! tool framework: each call in an assistant response is parsed into a
//! [`ToolInvocationRequest`], executed through the shared [`ToolExecutor`]
//! (which enforces the persistent permission system), and the resulting
//! [`ToolInvocationResult`] is fed back into the conversation as a `tool`
//! message. The round-trip repeats until the model returns a plain
//! assistant answer or the iteration limit is reached.

use std::sync::Arc;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::copilot::tools::{
    ToolDefinition, ToolExecutor, ToolInvocationRequest, ToolInvocationResult,
    ToolInvocationStatus, ToolParameterType,
};
use crate::llm::{
    LLMMessage, LLMRequest, LLMResponse, LLMTool, LLMToolCall, LLMToolParameter,
    LLMToolParameterType, LLMToolParameters,
};

/// Default maximum number of model round-trips in a single tool loop.
pub const DEFAULT_MAX_TOOL_ITERATIONS: usize = 8;

/// Outcome when a tool loop finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallLoopStatus {
    /// The model returned a plain assistant answer.
    Completed,
    /// The loop stopped because the iteration limit was reached.
    MaxIterationsReached,
}

/// The result of an LLM-native tool calling loop.
#[derive(Debug, Clone)]
pub struct ToolCallLoopResult {
    /// The final assistant content (empty when the iteration limit cut the
    /// loop short).
    pub content: String,
    /// Number of completion round-trips performed.
    pub iterations: usize,
    pub status: ToolCallLoopStatus,
    /// Every tool invocation executed out of the loop, in order.
    pub executions: Vec<ToolInvocationResult>,
}

/// A completed (or failed) tool execution plus its LLM-facing feedback.
pub struct ToolCallFeedback {
    pub invocation: Option<ToolInvocationResult>,
    pub content: String,
}

/// Error surfaced by the tool calling loop.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ToolCallLoopError {
    #[error("tool call loop cancelled")]
    Cancelled,
    #[error("{0}")]
    Responder(String),
    #[error("{0}")]
    Execution(String),
}

/// Handles one completion round-trip for the loop. Implementations decide
/// whether the round is streamed or buffered; the loop only needs the final
/// [`LLMResponse`].
#[async_trait]
pub trait ToolCallResponder: Send + Sync {
    async fn respond(&self, request: LLMRequest) -> Result<LLMResponse, ToolCallLoopError>;
}

/// Drives the iterative provider-native tool calling loop.
#[derive(Clone)]
pub struct ToolCallLoop {
    tool_executor: Arc<ToolExecutor>,
    workspace_id: Option<Uuid>,
    cancellation_token: Option<CancellationToken>,
    max_iterations: usize,
}

impl ToolCallLoop {
    pub fn new(
        tool_executor: Arc<ToolExecutor>,
        workspace_id: Option<Uuid>,
        cancellation_token: Option<CancellationToken>,
    ) -> Self {
        Self {
            tool_executor,
            workspace_id,
            cancellation_token,
            max_iterations: DEFAULT_MAX_TOOL_ITERATIONS,
        }
    }

    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations.max(1);
        self
    }

    /// Runs the loop over `messages` until the model answers without tool
    /// calls or the iteration limit is hit. Each round appends the
    /// assistant tool-call message and per-call `tool` result messages so
    /// execution history stays in the conversation sent back to the model.
    pub async fn run(
        &self,
        responder: &dyn ToolCallResponder,
        mut messages: Vec<LLMMessage>,
        tools: Vec<LLMTool>,
    ) -> Result<ToolCallLoopResult, ToolCallLoopError> {
        let mut executions = Vec::new();
        let mut iterations = 0usize;

        loop {
            if let Some(token) = &self.cancellation_token {
                if token.is_cancelled() {
                    return Err(ToolCallLoopError::Cancelled);
                }
            }

            if iterations >= self.max_iterations {
                return Ok(ToolCallLoopResult {
                    content: String::new(),
                    iterations,
                    executions,
                    status: ToolCallLoopStatus::MaxIterationsReached,
                });
            }

            let request = LLMRequest {
                messages: messages.clone(),
                tools: if tools.is_empty() {
                    None
                } else {
                    Some(tools.clone())
                },
                ..LLMRequest::default()
            };

            let response = responder.respond(request).await?;
            iterations += 1;

            let Some(tool_calls) = response.tool_calls.filter(|calls| !calls.is_empty()) else {
                return Ok(ToolCallLoopResult {
                    content: response.content,
                    iterations,
                    executions,
                    status: ToolCallLoopStatus::Completed,
                });
            };

            messages.push(LLMMessage::assistant_tool_calls(
                response.content,
                tool_calls.clone(),
            ));

            for call in tool_calls {
                let feedback = self.execute(&call).await;
                if let Some(invocation) = feedback.invocation {
                    executions.push(invocation);
                }
                messages.push(LLMMessage::tool_result(call.id, feedback.content));
            }
        }
    }

    /// Parses a provider tool call into a [`ToolInvocationRequest`] and
    /// executes it through the shared [`ToolExecutor`], which enforces the
    /// persistent permission system. Failures (permission denial, tool
    /// errors) are captured in the feedback content instead of aborting the
    /// loop, so the model can observe and react to them.
    async fn execute(&self, call: &LLMToolCall) -> ToolCallFeedback {
        let request = ToolInvocationRequest {
            tool_name: call.name.clone(),
            arguments: call.arguments.clone(),
            workspace_id: self.workspace_id,
            cancellation_token: self.cancellation_token.clone(),
        };

        match self.tool_executor.invoke_tool_with_context(request).await {
            Ok(invocation) => ToolCallFeedback {
                content: feedback_content(&invocation),
                invocation: Some(invocation),
            },
            Err(error) => ToolCallFeedback {
                content: serde_json::json!({ "error": error.to_string() }).to_string(),
                invocation: None,
            },
        }
    }
}

/// Renders a [`ToolInvocationResult`] compactly for the model to read.
fn feedback_content(invocation: &ToolInvocationResult) -> String {
    match invocation.status {
        ToolInvocationStatus::Success => invocation
            .result
            .as_ref()
            .and_then(|result| serde_json::to_string(result).ok())
            .unwrap_or_else(|| "null".to_string()),
        ToolInvocationStatus::Failed | ToolInvocationStatus::Cancelled => serde_json::json!({
            "status": invocation.status,
            "error": invocation.error,
        })
        .to_string(),
        ToolInvocationStatus::Pending | ToolInvocationStatus::Running => "null".to_string(),
    }
}

/// Maps registry tool definitions into provider-native tool schemas.
pub fn build_tool_schemas(definitions: Vec<ToolDefinition>) -> Vec<LLMTool> {
    definitions
        .into_iter()
        .map(|definition| {
            let mut parameters = LLMToolParameters::default();
            for parameter in definition.parameters {
                parameters.add(LLMToolParameter {
                    name: parameter.name,
                    description: parameter.description,
                    param_type: match parameter.parameter_type {
                        ToolParameterType::String => LLMToolParameterType::String,
                        ToolParameterType::Number => LLMToolParameterType::Number,
                        ToolParameterType::Boolean => LLMToolParameterType::Boolean,
                        ToolParameterType::Object => LLMToolParameterType::Object,
                        ToolParameterType::Array => LLMToolParameterType::Array,
                    },
                    required: parameter.required,
                });
            }
            LLMTool::new(definition.name, definition.description, parameters)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use crate::copilot::tools::{
        ToolInvocationStatus, ToolPermissionDecision, ToolPermissionService,
    };
    use crate::database::test_database;
    use crate::repositories::{
        FileRepository, SettingsRepository, TimelineRepository, WorkspaceRepository,
    };
    use crate::services::{TimelineService, WorkspaceService};
    use crate::session::SessionEngine;
    use crate::timeline::recorder::TimelineRecorder;
    use crate::timeline::TimelineEngine;

    fn tool_call(n: usize, name: &str, arguments: serde_json::Value) -> LLMToolCall {
        LLMToolCall {
            id: format!("call-{n}"),
            name: name.to_string(),
            arguments,
        }
    }

    fn final_response(content: &str) -> LLMResponse {
        LLMResponse {
            content: content.to_string(),
            usage: Default::default(),
            model: "mock".to_string(),
            finish_reason: Some("stop".to_string()),
            tool_calls: None,
        }
    }

    fn tool_response(calls: Vec<LLMToolCall>) -> LLMResponse {
        LLMResponse {
            content: String::new(),
            usage: Default::default(),
            model: "mock".to_string(),
            finish_reason: Some("tool_calls".to_string()),
            tool_calls: Some(calls),
        }
    }

    struct ScriptedResponder {
        responses: Mutex<VecDeque<LLMResponse>>,
    }

    impl ScriptedResponder {
        fn new(responses: Vec<LLMResponse>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
            }
        }
    }

    #[async_trait]
    impl ToolCallResponder for ScriptedResponder {
        async fn respond(&self, _request: LLMRequest) -> Result<LLMResponse, ToolCallLoopError> {
            let mut queue = self
                .responses
                .lock()
                .map_err(|_| ToolCallLoopError::Execution("lock poisoned".to_string()))?;
            Ok(queue.pop_front().unwrap_or_else(|| final_response("done.")))
        }
    }

    struct AlwaysToolCalls;

    #[async_trait]
    impl ToolCallResponder for AlwaysToolCalls {
        async fn respond(&self, _request: LLMRequest) -> Result<LLMResponse, ToolCallLoopError> {
            Ok(tool_response(vec![tool_call(
                0,
                "list_workspaces",
                serde_json::json!({}),
            )]))
        }
    }

    async fn executor() -> (
        Arc<ToolExecutor>,
        Arc<ToolPermissionService>,
        tempfile::TempDir,
    ) {
        let (database, guard) = test_database().await;
        let pool = database.pool().clone();
        let workspace_repo = WorkspaceRepository::new(pool.clone());
        let file_repo = FileRepository::new(pool.clone());
        let timeline_repo = TimelineRepository::new(pool.clone());

        let workspace_service =
            Arc::new(WorkspaceService::new(workspace_repo, timeline_repo.clone()));
        let session_engine = Arc::new(SessionEngine::new(
            TimelineRepository::new(pool.clone()),
            FileRepository::new(pool.clone()),
        ));
        let timeline_engine = Arc::new(TimelineEngine::new(TimelineService::new(
            TimelineRecorder::new(file_repo, timeline_repo.clone()),
            timeline_repo,
        )));

        let permission_service = Arc::new(
            ToolPermissionService::new(SettingsRepository::new(pool.clone()))
                .await
                .expect("permission service should initialize"),
        );

        let executor = Arc::new(
            ToolExecutor::new(workspace_service, session_engine, timeline_engine)
                .with_permission_service(permission_service.clone()),
        );

        (executor, permission_service, guard)
    }

    fn user_message(content: &str) -> LLMMessage {
        LLMMessage::new("user", content)
    }

    #[tokio::test]
    async fn single_tool_call_executes_then_answers() {
        let (executor, _permissions, _guard) = executor().await;
        let responder = ScriptedResponder::new(vec![
            tool_response(vec![tool_call(1, "list_workspaces", serde_json::json!({}))]),
            final_response("Here are your workspaces."),
        ]);
        let schemas = build_tool_schemas(executor.available_tools());

        let runner = ToolCallLoop::new(executor, None, None);
        let result = runner
            .run(&responder, vec![user_message("list workspaces")], schemas)
            .await
            .expect("loop should complete");

        assert_eq!(result.status, ToolCallLoopStatus::Completed);
        assert_eq!(result.content, "Here are your workspaces.");
        assert_eq!(result.iterations, 2);
        assert_eq!(result.executions.len(), 1);
        assert_eq!(result.executions[0].tool_name, "list_workspaces");
        assert_eq!(result.executions[0].status, ToolInvocationStatus::Success);
    }

    #[tokio::test]
    async fn multiple_sequential_tool_calls_execute_in_order() {
        let (executor, _permissions, _guard) = executor().await;
        let responder = ScriptedResponder::new(vec![
            tool_response(vec![tool_call(1, "list_workspaces", serde_json::json!({}))]),
            tool_response(vec![tool_call(2, "list_workspaces", serde_json::json!({}))]),
            final_response("Both lookups done."),
        ]);
        let schemas = build_tool_schemas(executor.available_tools());

        let runner = ToolCallLoop::new(executor, None, None);
        let result = runner
            .run(&responder, vec![user_message("look up twice")], schemas)
            .await
            .expect("loop should complete");

        assert_eq!(result.status, ToolCallLoopStatus::Completed);
        assert_eq!(result.content, "Both lookups done.");
        assert_eq!(result.iterations, 3);
        assert_eq!(result.executions.len(), 2);
        assert_eq!(result.executions[0].tool_name, "list_workspaces");
        assert_eq!(result.executions[1].tool_name, "list_workspaces");
        assert_eq!(result.executions[0].status, ToolInvocationStatus::Success);
        assert_eq!(result.executions[1].status, ToolInvocationStatus::Success);
    }

    #[tokio::test]
    async fn permission_denied_feeds_error_back_but_loop_continues() {
        let (executor, permissions, _guard) = executor().await;
        permissions
            .set_policy("resume_workspace", None, ToolPermissionDecision::Deny)
            .await
            .expect("policy should persist");

        let responder = ScriptedResponder::new(vec![
            tool_response(vec![tool_call(
                1,
                "resume_workspace",
                serde_json::json!({
                    "workspace_id": "00000000-0000-0000-0000-000000000001"
                }),
            )]),
            final_response("I cannot do that."),
        ]);
        let schemas = build_tool_schemas(executor.available_tools());

        let runner = ToolCallLoop::new(executor, None, None);
        let result = runner
            .run(&responder, vec![user_message("resume workspace")], schemas)
            .await
            .expect("denied tools must not abort the loop");

        assert_eq!(result.status, ToolCallLoopStatus::Completed);
        assert_eq!(result.executions.len(), 1);
        assert_eq!(result.executions[0].tool_name, "resume_workspace");
        assert_eq!(result.executions[0].status, ToolInvocationStatus::Failed);
        let error = result.executions[0].error.as_deref().unwrap_or_default();
        assert!(
            error.contains("denied"),
            "expected a permission-denied error, got: {error}"
        );
    }

    #[tokio::test]
    async fn tool_failure_does_not_abort_loop() {
        let (executor, _permissions, _guard) = executor().await;
        // A syntactically valid but unknown workspace id makes
        // `get_workspace` fail at runtime.
        let responder = ScriptedResponder::new(vec![
            tool_response(vec![tool_call(
                1,
                "get_workspace",
                serde_json::json!({
                    "workspace_id": "00000000-0000-0000-0000-000000000001"
                }),
            )]),
            final_response("That workspace does not exist."),
        ]);
        let schemas = build_tool_schemas(executor.available_tools());

        let runner = ToolCallLoop::new(executor, None, None);
        let result = runner
            .run(
                &responder,
                vec![user_message("fetch missing workspace")],
                schemas,
            )
            .await
            .expect("failed tools must not abort the loop");

        assert_eq!(result.status, ToolCallLoopStatus::Completed);
        assert_eq!(result.executions.len(), 1);
        assert_eq!(result.executions[0].status, ToolInvocationStatus::Failed);
        assert!(result.executions[0].error.is_some());
    }

    #[tokio::test]
    async fn iteration_limit_protection_stops_the_loop() {
        let (executor, _permissions, _guard) = executor().await;
        let responder = AlwaysToolCalls;
        let schemas = build_tool_schemas(executor.available_tools());

        let runner = ToolCallLoop::new(executor, None, None).with_max_iterations(2);
        let result = runner
            .run(
                &responder,
                vec![user_message("keep going forever")],
                schemas,
            )
            .await
            .expect("loop should stop at the iteration limit");

        assert_eq!(result.status, ToolCallLoopStatus::MaxIterationsReached);
        assert_eq!(result.iterations, 2);
        assert_eq!(result.executions.len(), 2);
    }
}
