//! Copilot models - Domain types for AI assistant.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A copilot conversation with persistent history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i32,
}

/// A message in a copilot conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub reasoning: Option<String>,
    pub sources: Option<Vec<Source>>,
    pub created_at: DateTime<Utc>,
}

/// Role of a message author.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A tool call made by the assistant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Uuid,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub status: ToolExecutionStatus,
}

/// Status of a tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolExecutionStatus {
    Pending,
    Success,
    Failed,
    Cancelled,
}

/// Source of information for a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub source_type: SourceType,
    pub title: String,
    pub reference: String,
    pub relevance: f64,
}

/// Type of information source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    TimelineEvent,
    ContextMemory,
    WorkspaceFile,
    SessionHistory,
    KnowledgeGraph,
}

/// A tool execution record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub id: Uuid,
    pub message_id: Uuid,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub status: ToolExecutionStatus,
    pub requires_confirmation: bool,
    pub confirmed: bool,
    pub error: Option<String>,
    pub executed_at: DateTime<Utc>,
}

/// Context snapshot at the time of a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub active_files: Vec<String>,
    pub recent_events: Vec<String>,
    pub session_summary: Option<String>,
    pub captured_at: DateTime<Utc>,
}

/// A multi-step action plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: Uuid,
    pub message_id: Uuid,
    pub goal: String,
    pub steps: Vec<PlanStep>,
    pub current_step: usize,
    pub status: PlanStatus,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Status of a plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanStatus {
    Planning,
    Executing,
    Completed,
    Failed,
    Cancelled,
}

/// A step in a multi-step plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub description: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub completed: bool,
    pub result: Option<serde_json::Value>,
}

/// Request to send a message to the copilot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub conversation_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub message: String,
    pub include_context: bool,
}

/// Response from the copilot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotResponse {
    pub conversation_id: Uuid,
    pub message: Message,
    pub suggested_actions: Vec<SuggestedAction>,
}

/// Response returned after a streaming copilot request has started.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotStreamResponse {
    pub conversation_id: Uuid,
    pub stream_id: Uuid,
}

/// Request to search persisted copilot conversations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchRequest {
    pub query: Option<String>,
    pub workspace_id: Option<Uuid>,
    pub provider: Option<String>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub limit: Option<usize>,
}

/// Conversation search result with a short backend-generated match preview.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub conversation: Conversation,
    pub matched_message_id: Option<Uuid>,
    pub matched_at: Option<DateTime<Utc>>,
    pub snippet: Option<String>,
    pub provider: Option<String>,
}

/// An action suggested by the copilot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedAction {
    pub title: String,
    pub description: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
    pub requires_confirmation: bool,
}

/// Request to get a daily briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBriefingRequest {
    pub workspace_id: Option<Uuid>,
}

/// Daily briefing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyBriefing {
    pub date: DateTime<Utc>,
    pub summary: String,
    pub highlights: Vec<String>,
    pub pending_tasks: Vec<String>,
    pub recommendations: Vec<String>,
    pub workspace_stats: WorkspaceStats,
}

/// Workspace statistics for briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    pub active_workspaces: usize,
    pub files_modified: usize,
    pub time_tracked: i64,
    pub sessions_completed: usize,
}

/// Request to answer a question about workspace history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceQuestionRequest {
    pub workspace_id: Option<Uuid>,
    pub question: String,
}

/// Answer to a workspace question.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceAnswer {
    pub answer: String,
    pub reasoning: String,
    pub sources: Vec<Source>,
    pub confidence: f64,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "user" => Some(Self::User),
            "assistant" => Some(Self::Assistant),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

impl ToolExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(Self::Pending),
            "success" => Some(Self::Success),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl PlanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Planning => "planning",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "planning" => Some(Self::Planning),
            "executing" => Some(Self::Executing),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }
}
