//! Core types for session intelligence.
//!
//! Sessions are derived from timeline events, not stored as canonical data.
//! All session metadata is computed on-demand from the timeline.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::TimelineEvent;

/// A work session: a continuous period of activity within a workspace.
///
/// Sessions are reconstructed from timeline events by grouping events
/// separated by less than the inactivity threshold (default 30 minutes).
/// They are not stored in the database — timeline events remain the
/// single source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    /// The workspace this session belongs to.
    pub workspace_id: Uuid,

    /// Session start time (first event's occurred_at).
    pub started_at: DateTime<Utc>,

    /// Session end time (last event's occurred_at).
    pub ended_at: DateTime<Utc>,

    /// Session duration in seconds.
    pub duration_seconds: i64,

    /// Total number of timeline events in this session.
    pub event_count: usize,

    /// Number of distinct files touched in this session.
    pub file_count: usize,

    /// Programming languages detected in edited files.
    pub languages: Vec<String>,

    /// Productivity score (0-100) with transparent scoring factors.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub productivity_score: Option<ProductivityScore>,

    /// The timeline events that comprise this session (for detailed analysis).
    #[serde(skip)]
    pub events: Vec<TimelineEvent>,
}

/// Context data used for scoring calculations.
///
/// Extracted from a Session to provide calculators with the information
/// they need without exposing the full event list.
#[derive(Debug, Clone)]
pub struct SessionContext {
    pub workspace_id: Uuid,
    pub duration_seconds: i64,
    pub event_count: usize,
    pub file_count: usize,
    pub events: Vec<TimelineEvent>,
}

impl From<&Session> for SessionContext {
    fn from(session: &Session) -> Self {
        Self {
            workspace_id: session.workspace_id,
            duration_seconds: session.duration_seconds,
            event_count: session.event_count,
            file_count: session.file_count,
            events: session.events.clone(),
        }
    }
}

/// Productivity score with transparent scoring factors.
///
/// The score is computed by a weighted combination of individual factors
/// (focus duration, deep editing, context switching, completion signals, etc.).
/// Each factor contributes to the final score and includes a human-readable
/// reason explaining its value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductivityScore {
    /// Final score (0-100), weighted combination of all factors.
    pub score: f64,

    /// Individual scoring factors that contributed to the final score.
    pub factors: Vec<ScoreFactor>,
}

/// A single scoring factor contributing to the productivity score.
///
/// Each factor has a name, weight, normalized value (0-1), and a
/// human-readable reason explaining why it received that value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreFactor {
    /// Factor name (e.g. "Focus Duration", "Deep Editing").
    pub name: String,

    /// Weight of this factor in the final score calculation (0-1).
    pub weight: f64,

    /// Normalized value for this factor (0-1).
    pub value: f64,

    /// Human-readable explanation of why this factor has this value.
    pub reason: String,
}

/// Summary view of a session for the Smart Resume feature.
///
/// Contains all the information needed to display a "Continue Working"
/// banner without exposing the full event list.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    /// Workspace ID this session belongs to.
    pub workspace_id: Uuid,

    /// Workspace name (for display).
    pub workspace_name: String,

    /// When the session started.
    pub started_at: DateTime<Utc>,

    /// When the session ended.
    pub ended_at: DateTime<Utc>,

    /// Session duration in seconds.
    pub duration_seconds: i64,

    /// Number of files edited in this session.
    pub file_count: usize,

    /// Programming languages detected.
    pub languages: Vec<String>,

    /// Productivity score (0-100).
    pub productivity_score: f64,

    /// Scoring factors (for transparency).
    pub score_factors: Vec<ScoreFactor>,

    /// Recent events for mini-timeline display (limited to ~5-10 events).
    pub recent_events: Vec<SessionEventSummary>,
}

/// Lightweight event summary for displaying in mini-timelines.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEventSummary {
    /// When the event occurred.
    pub occurred_at: DateTime<Utc>,

    /// Event type (e.g. "edit", "commit", "open").
    pub event_type: String,

    /// File name (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    /// Event description for display.
    pub description: String,
}
