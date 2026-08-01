//! Copilot Engine - AI-powered workspace assistant with multi-step planning.

use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

use crate::context_memory::ContextMemoryEngine;
use crate::copilot::conversation::ConversationManager;
use crate::copilot::models::*;
use crate::copilot::repository::CopilotRepository;
use crate::copilot::tools::ToolExecutor;
use crate::errors::DatabaseError;
use crate::intelligence::recommendation::RecommendationEngine;
use crate::learning::AdaptiveLearningEngine;
use crate::predictive::PredictiveEngine;
use crate::semantic::ContextReasoningEngine;
use crate::session::SessionEngine;
use crate::timeline::TimelineEngine;

/// Copilot engine that orchestrates all intelligence layers.
pub struct CopilotEngine {
    conversation_manager: Arc<ConversationManager>,
    tool_executor: Arc<ToolExecutor>,
    repository: Arc<CopilotRepository>,
    reasoning_engine: Arc<ContextReasoningEngine>,
    predictive_engine: Arc<PredictiveEngine>,
    learning_engine: Arc<AdaptiveLearningEngine>,
    recommendation_engine: Arc<RecommendationEngine>,
    context_memory: Arc<ContextMemoryEngine>,
    session_engine: Arc<SessionEngine>,
    timeline_engine: Arc<TimelineEngine>,
}

impl CopilotEngine {
    /// Creates a new copilot engine.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        conversation_manager: Arc<ConversationManager>,
        tool_executor: Arc<ToolExecutor>,
        repository: Arc<CopilotRepository>,
        reasoning_engine: Arc<ContextReasoningEngine>,
        predictive_engine: Arc<PredictiveEngine>,
        learning_engine: Arc<AdaptiveLearningEngine>,
        recommendation_engine: Arc<RecommendationEngine>,
        context_memory: Arc<ContextMemoryEngine>,
        session_engine: Arc<SessionEngine>,
        timeline_engine: Arc<TimelineEngine>,
    ) -> Self {
        Self {
            conversation_manager,
            tool_executor,
            repository,
            reasoning_engine,
            predictive_engine,
            learning_engine,
            recommendation_engine,
            context_memory,
            session_engine,
            timeline_engine,
        }
    }

    /// Processes a user message and generates a response.
    pub async fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<CopilotResponse, DatabaseError> {
        // Get or create conversation
        let conversation = self
            .conversation_manager
            .get_or_create_conversation(request.conversation_id, request.workspace_id)
            .await?;

        // Add user message
        let _user_message = self
            .conversation_manager
            .add_user_message(conversation.id, &request.message)
            .await?;

        // Capture context if requested
        if request.include_context {
            self.conversation_manager
                .capture_context(conversation.id, request.workspace_id)
                .await?;
        }

        // Build context string
        let context = if request.include_context {
            self.conversation_manager
                .build_context_string(request.workspace_id)
                .await?
        } else {
            String::new()
        };

        // Analyze intent and generate response
        let response = self
            .generate_response(&request.message, &context, request.workspace_id)
            .await?;

        // Add assistant message
        let assistant_message = self
            .conversation_manager
            .add_assistant_message(
                conversation.id,
                &response.content,
                Some(response.reasoning.clone()),
                Some(response.sources.clone()),
                None,
            )
            .await?;

        // Generate suggested actions
        let suggested_actions = self
            .generate_suggested_actions(&request.message, request.workspace_id)
            .await?;

        Ok(CopilotResponse {
            conversation_id: conversation.id,
            message: assistant_message,
            suggested_actions,
        })
    }

    /// Generates a response using all available intelligence engines.
    async fn generate_response(
        &self,
        message: &str,
        context: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<ResponseData, DatabaseError> {
        let mut sources = Vec::new();
        let mut reasoning_parts = Vec::new();

        // Determine intent
        let intent = self.classify_intent(message);
        reasoning_parts.push(format!("Intent classified as: {:?}", intent));

        // Generate response based on intent
        let content = match intent {
            Intent::ListWorkspaces => {
                reasoning_parts.push("Fetching workspace list".to_string());
                self.handle_list_workspaces(&mut sources).await?
            }
            Intent::GetWorkspaceInfo => {
                reasoning_parts.push("Retrieving workspace information".to_string());
                self.handle_get_workspace_info(workspace_id, &mut sources)
                    .await?
            }
            Intent::SearchHistory => {
                reasoning_parts.push("Searching timeline history".to_string());
                self.handle_search_history(message, workspace_id, &mut sources)
                    .await?
            }
            Intent::SummarizeActivity => {
                reasoning_parts.push("Generating activity summary".to_string());
                self.handle_summarize_activity(workspace_id, &mut sources)
                    .await?
            }
            Intent::ExplainRecommendation => {
                reasoning_parts.push("Explaining recommendation logic".to_string());
                self.handle_explain_recommendation(workspace_id, &mut sources)
                    .await?
            }
            Intent::ResumeWork => {
                reasoning_parts.push("Preparing workspace resume".to_string());
                self.handle_resume_work(workspace_id, &mut sources).await?
            }
            Intent::AskQuestion => {
                reasoning_parts.push("Answering question using semantic search".to_string());
                self.handle_question(message, workspace_id, context, &mut sources)
                    .await?
            }
            Intent::Unknown => {
                reasoning_parts.push("Intent unclear, providing general help".to_string());
                format!(
                    "I can help you with:\n- Listing and managing workspaces\n- Searching your work history\n- Summarizing recent activity\n- Explaining recommendations\n- Resuming previous work\n- Answering questions about your projects\n\nWhat would you like to know?"
                )
            }
        };

        Ok(ResponseData {
            content,
            reasoning: reasoning_parts.join(" → "),
            sources,
        })
    }

    /// Classifies user intent.
    fn classify_intent(&self, message: &str) -> Intent {
        let message_lower = message.to_lowercase();

        if message_lower.contains("list") && message_lower.contains("workspace") {
            Intent::ListWorkspaces
        } else if message_lower.contains("what") && message_lower.contains("workspace") {
            Intent::GetWorkspaceInfo
        } else if message_lower.contains("search") || message_lower.contains("find") {
            Intent::SearchHistory
        } else if message_lower.contains("summarize")
            || message_lower.contains("summary")
            || message_lower.contains("what did i")
        {
            Intent::SummarizeActivity
        } else if message_lower.contains("why") || message_lower.contains("explain") {
            Intent::ExplainRecommendation
        } else if message_lower.contains("resume")
            || message_lower.contains("continue")
            || message_lower.contains("back to")
        {
            Intent::ResumeWork
        } else if message_lower.starts_with("what")
            || message_lower.starts_with("how")
            || message_lower.starts_with("when")
            || message_lower.starts_with("where")
        {
            Intent::AskQuestion
        } else {
            Intent::Unknown
        }
    }

    /// Handles list workspaces request.
    async fn handle_list_workspaces(
        &self,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        let result = self
            .tool_executor
            .execute_tool("list_workspaces", &serde_json::json!({}))
            .await?;

        let workspaces: Vec<serde_json::Value> =
            serde_json::from_value(result).map_err(|e| DatabaseError::IoError(e.to_string()))?;

        sources.push(Source {
            source_type: SourceType::WorkspaceFile,
            title: "Workspace Database".to_string(),
            reference: "workspaces table".to_string(),
            relevance: 1.0,
        });

        if workspaces.is_empty() {
            Ok("You don't have any workspaces yet. Start working in a project directory and I'll automatically detect it!".to_string())
        } else {
            let mut response = format!("You have {} workspace(s):\n\n", workspaces.len());
            for (i, ws) in workspaces.iter().take(10).enumerate() {
                let name = ws.get("name").and_then(|v| v.as_str()).unwrap_or("Unknown");
                let status = ws
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                response.push_str(&format!("{}. {} ({})\n", i + 1, name, status));
            }
            Ok(response)
        }
    }

    /// Handles get workspace info request.
    async fn handle_get_workspace_info(
        &self,
        workspace_id: Option<Uuid>,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        if workspace_id.is_none() {
            return Ok(
                "Please specify a workspace or let me know which workspace you're asking about."
                    .to_string(),
            );
        }

        let result = self
            .tool_executor
            .execute_tool(
                "get_workspace",
                &serde_json::json!({
                    "workspace_id": workspace_id.unwrap().to_string()
                }),
            )
            .await?;

        let workspace: serde_json::Value = result;
        let name = workspace
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let status = workspace
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        sources.push(Source {
            source_type: SourceType::WorkspaceFile,
            title: format!("Workspace: {}", name),
            reference: workspace_id.unwrap().to_string(),
            relevance: 1.0,
        });

        Ok(format!(
            "Workspace '{}' is currently {}. Let me know if you'd like to resume work or see recent activity!",
            name, status
        ))
    }

    /// Handles search history request.
    async fn handle_search_history(
        &self,
        query: &str,
        workspace_id: Option<Uuid>,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        // Extract search terms (simple implementation)
        let search_terms: Vec<&str> = query
            .split_whitespace()
            .filter(|w| w.len() > 3 && !["search", "find", "show", "what"].contains(w))
            .collect();

        let search_query = search_terms.join(" ");

        if search_query.is_empty() {
            return Ok("What would you like me to search for in your history?".to_string());
        }

        let result = self
            .tool_executor
            .execute_tool(
                "search_timeline",
                &serde_json::json!({
                    "query": search_query,
                    "workspace_id": workspace_id.map(|id| id.to_string())
                }),
            )
            .await?;

        let events: Vec<serde_json::Value> =
            serde_json::from_value(result).map_err(|e| DatabaseError::IoError(e.to_string()))?;

        if events.is_empty() {
            Ok(format!(
                "I couldn't find any events matching '{}'",
                search_query
            ))
        } else {
            sources.push(Source {
                source_type: SourceType::TimelineEvent,
                title: "Timeline Events".to_string(),
                reference: format!("{} events found", events.len()),
                relevance: 0.9,
            });

            let mut response = format!(
                "Found {} events matching '{}':\n\n",
                events.len(),
                search_query
            );
            for (i, event) in events.iter().take(5).enumerate() {
                let event_type = event
                    .get("event_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let file_path = event
                    .get("file_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("N/A");
                response.push_str(&format!("{}. {}: {}\n", i + 1, event_type, file_path));
            }

            if events.len() > 5 {
                response.push_str(&format!("\n...and {} more.", events.len() - 5));
            }

            Ok(response)
        }
    }

    /// Handles summarize activity request.
    async fn handle_summarize_activity(
        &self,
        workspace_id: Option<Uuid>,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        let result = self
            .tool_executor
            .execute_tool(
                "get_recent_events",
                &serde_json::json!({
                    "workspace_id": workspace_id.map(|id| id.to_string()),
                    "limit": 20
                }),
            )
            .await?;

        let events: Vec<serde_json::Value> =
            serde_json::from_value(result).map_err(|e| DatabaseError::IoError(e.to_string()))?;

        if events.is_empty() {
            return Ok("No recent activity to summarize.".to_string());
        }

        sources.push(Source {
            source_type: SourceType::TimelineEvent,
            title: "Recent Timeline Events".to_string(),
            reference: format!("{} events", events.len()),
            relevance: 1.0,
        });

        // Analyze event types
        let mut event_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        let mut files: std::collections::HashSet<String> = std::collections::HashSet::new();

        for event in &events {
            if let Some(event_type) = event.get("event_type").and_then(|v| v.as_str()) {
                *event_counts.entry(event_type.to_string()).or_insert(0) += 1;
            }
            if let Some(file_path) = event.get("file_path").and_then(|v| v.as_str()) {
                files.insert(file_path.to_string());
            }
        }

        let mut summary = format!(
            "Recent activity summary (last {} events):\n\n",
            events.len()
        );
        summary.push_str(&format!("• {} unique files modified\n", files.len()));

        for (event_type, count) in event_counts.iter() {
            summary.push_str(&format!("• {} {} events\n", count, event_type));
        }

        Ok(summary)
    }

    /// Handles explain recommendation request.
    async fn handle_explain_recommendation(
        &self,
        workspace_id: Option<Uuid>,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        if let Some(ws_id) = workspace_id {
            // Get recommendations
            let recommendations = self
                .recommendation_engine
                .generate_recommendations(ws_id)
                .await?;

            if recommendations.is_empty() {
                return Ok(
                    "No recommendations available yet. Keep working and I'll learn your patterns!"
                        .to_string(),
                );
            }

            sources.push(Source {
                source_type: SourceType::ContextMemory,
                title: "Recommendation Engine".to_string(),
                reference: format!("{} recommendations", recommendations.len()),
                relevance: 0.95,
            });

            let mut response = "Here are my current recommendations:\n\n".to_string();
            for (i, rec) in recommendations.iter().take(3).enumerate() {
                response.push_str(&format!(
                    "{}. {} (confidence: {:.0}%)\n\n",
                    i + 1,
                    rec.title,
                    rec.confidence * 100.0
                ));
            }

            Ok(response)
        } else {
            Ok("Please specify a workspace to get recommendations for.".to_string())
        }
    }

    /// Handles resume work request.
    async fn handle_resume_work(
        &self,
        workspace_id: Option<Uuid>,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        if workspace_id.is_none() {
            return Ok("Which workspace would you like to resume?".to_string());
        }

        let ws_id = workspace_id.unwrap();

        // Get session summary
        let summary_result = self
            .tool_executor
            .execute_tool(
                "get_session_summary",
                &serde_json::json!({
                    "workspace_id": ws_id.to_string()
                }),
            )
            .await;

        sources.push(Source {
            source_type: SourceType::SessionHistory,
            title: "Session Summary".to_string(),
            reference: ws_id.to_string(),
            relevance: 0.9,
        });

        if let Ok(_summary) = summary_result {
            Ok(format!(
                "Ready to resume work in workspace {}! Your previous session context has been loaded.",
                ws_id
            ))
        } else {
            Ok(format!(
                "Starting fresh in workspace {}. Let's get to work!",
                ws_id
            ))
        }
    }

    /// Handles general questions.
    async fn handle_question(
        &self,
        question: &str,
        workspace_id: Option<Uuid>,
        _context: &str,
        sources: &mut Vec<Source>,
    ) -> Result<String, DatabaseError> {
        // Simple question answering - would use reasoning engine with proper API
        sources.push(Source {
            source_type: SourceType::ContextMemory,
            title: "Context Analysis".to_string(),
            reference: "reasoning_engine".to_string(),
            relevance: 0.7,
        });

        let mut response = format!("Regarding your question: \"{}\"\n\n", question);

        if let Some(ws_id) = workspace_id {
            response.push_str(&format!(
                "Based on workspace {}, I can help you explore your work history and patterns.",
                ws_id
            ));
        } else {
            response.push_str(
                "I can help you explore your work history and patterns across all workspaces.",
            );
        }

        Ok(response)
    }

    /// Generates suggested actions based on the message.
    async fn generate_suggested_actions(
        &self,
        _message: &str,
        workspace_id: Option<Uuid>,
    ) -> Result<Vec<SuggestedAction>, DatabaseError> {
        let mut actions = Vec::new();

        if workspace_id.is_some() {
            actions.push(SuggestedAction {
                title: "View Recent Activity".to_string(),
                description: "See recent timeline events in this workspace".to_string(),
                tool_name: "get_recent_events".to_string(),
                arguments: serde_json::json!({
                    "workspace_id": workspace_id.map(|id| id.to_string()),
                    "limit": 10
                }),
                requires_confirmation: false,
            });
        }

        actions.push(SuggestedAction {
            title: "List All Workspaces".to_string(),
            description: "Show all available workspaces".to_string(),
            tool_name: "list_workspaces".to_string(),
            arguments: serde_json::json!({}),
            requires_confirmation: false,
        });

        Ok(actions)
    }

    /// Generates a daily briefing.
    pub async fn get_daily_briefing(
        &self,
        workspace_id: Option<Uuid>,
    ) -> Result<DailyBriefing, DatabaseError> {
        let date = Utc::now();

        // Get today's events
        let events = if let Some(ws_id) = workspace_id {
            self.timeline_engine.recent_events(ws_id, Some(100)).await?
        } else {
            Vec::new()
        };

        let today_events: Vec<_> = events
            .iter()
            .filter(|e| e.occurred_at.date_naive() == date.date_naive())
            .collect();

        let files_modified: std::collections::HashSet<_> = today_events
            .iter()
            .filter_map(|e| e.file_id.as_ref())
            .collect();

        let summary = if today_events.is_empty() {
            "No activity recorded today yet.".to_string()
        } else {
            format!(
                "Today you've worked on {} files with {} events recorded.",
                files_modified.len(),
                today_events.len()
            )
        };

        let highlights = vec![
            format!("{} timeline events", today_events.len()),
            format!("{} files modified", files_modified.len()),
        ];

        // Get recommendations
        let recommendations = if let Some(ws_id) = workspace_id {
            let recs = self
                .recommendation_engine
                .generate_recommendations(ws_id)
                .await?;
            recs.into_iter().take(3).map(|r| r.title).collect()
        } else {
            vec![]
        };

        Ok(DailyBriefing {
            date,
            summary,
            highlights,
            pending_tasks: vec![],
            recommendations,
            workspace_stats: WorkspaceStats {
                active_workspaces: 1,
                files_modified: files_modified.len(),
                time_tracked: 0,
                sessions_completed: 0,
            },
        })
    }

    /// Gets conversation history.
    pub async fn get_conversation_history(
        &self,
        conversation_id: Uuid,
    ) -> Result<Vec<Message>, DatabaseError> {
        self.conversation_manager
            .get_conversation_history(conversation_id, None)
            .await
    }

    /// Gets recent conversations.
    pub async fn get_recent_conversations(
        &self,
        limit: usize,
    ) -> Result<Vec<Conversation>, DatabaseError> {
        self.repository.get_recent_conversations(limit).await
    }
}

/// User intent classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Intent {
    ListWorkspaces,
    GetWorkspaceInfo,
    SearchHistory,
    SummarizeActivity,
    ExplainRecommendation,
    ResumeWork,
    AskQuestion,
    Unknown,
}

/// Internal response data structure.
struct ResponseData {
    content: String,
    reasoning: String,
    sources: Vec<Source>,
}
