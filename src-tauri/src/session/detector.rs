//! Session detection algorithm.
//!
//! Reconstructs work sessions from timeline events by grouping events
//! separated by less than the inactivity threshold. Sessions are derived
//! from timeline events, not stored as canonical data.

use chrono::Duration;
use std::collections::HashSet;
use uuid::Uuid;

use crate::models::TimelineEvent;
use crate::session::types::Session;

/// Default inactivity threshold: 30 minutes.
/// If two events are more than 30 minutes apart, they belong to different sessions.
pub const DEFAULT_INACTIVITY_THRESHOLD_SECONDS: i64 = 30 * 60;

/// Detects work sessions from a list of timeline events.
///
/// Algorithm:
/// 1. Sort events by occurred_at (oldest first)
/// 2. Group events by inactivity gaps
/// 3. Each continuous activity period = one session
///
/// # Arguments
/// * `events` - Timeline events, should all be from the same workspace
/// * `threshold_seconds` - Inactivity threshold; events more than this many
///   seconds apart belong to different sessions
///
/// # Returns
/// List of detected sessions, ordered by start time (newest first)
pub fn detect_sessions(mut events: Vec<TimelineEvent>, threshold_seconds: i64) -> Vec<Session> {
    if events.is_empty() {
        return Vec::new();
    }

    // Sort events oldest first for sequential processing
    events.sort_by_key(|e| e.occurred_at);

    let threshold = Duration::seconds(threshold_seconds);
    let mut sessions = Vec::new();
    let mut current_session_events = Vec::new();

    for event in events {
        if current_session_events.is_empty() {
            // Start first session
            current_session_events.push(event);
        } else {
            let last_event = current_session_events.last().unwrap();
            let gap = event.occurred_at - last_event.occurred_at;

            if gap > threshold {
                // Gap too large, finalize current session and start new one
                sessions.push(build_session(current_session_events));
                current_session_events = vec![event];
            } else {
                // Continue current session
                current_session_events.push(event);
            }
        }
    }

    // Finalize last session
    if !current_session_events.is_empty() {
        sessions.push(build_session(current_session_events));
    }

    // Return sessions newest first
    sessions.reverse();
    sessions
}

/// Builds a Session from a list of timeline events.
///
/// Assumes all events are already sorted by occurred_at and belong to
/// the same continuous session.
fn build_session(events: Vec<TimelineEvent>) -> Session {
    let started_at = events.first().unwrap().occurred_at;
    let ended_at = events.last().unwrap().occurred_at;
    let duration = ended_at - started_at;

    // Count distinct files touched in this session
    let file_ids: HashSet<Uuid> = events.iter().filter_map(|e| e.file_id).collect();

    let workspace_id = events[0].workspace_id;

    Session {
        workspace_id,
        started_at,
        ended_at,
        duration_seconds: duration.num_seconds().max(0),
        event_count: events.len(),
        file_count: file_ids.len(),
        languages: Vec::new(),    // Will be populated by SessionEngine
        productivity_score: None, // Will be computed by SessionEngine
        events,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{TimelineEvent, TimelineEventType};
    use chrono::Utc;

    fn make_event(workspace_id: Uuid, occurred_at: chrono::DateTime<Utc>) -> TimelineEvent {
        TimelineEvent {
            id: Uuid::new_v4(),
            workspace_id,
            file_id: Some(Uuid::new_v4()),
            event_type: TimelineEventType::Edit,
            occurred_at,
            metadata: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn empty_events_returns_empty_sessions() {
        let sessions = detect_sessions(Vec::new(), DEFAULT_INACTIVITY_THRESHOLD_SECONDS);
        assert!(sessions.is_empty());
    }

    #[test]
    fn single_event_creates_single_session() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();
        let events = vec![make_event(workspace_id, now)];

        let sessions = detect_sessions(events, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].workspace_id, workspace_id);
        assert_eq!(sessions[0].event_count, 1);
        assert_eq!(sessions[0].duration_seconds, 0);
    }

    #[test]
    fn events_within_threshold_form_single_session() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();
        let events = vec![
            make_event(workspace_id, now),
            make_event(workspace_id, now + Duration::minutes(10)),
            make_event(workspace_id, now + Duration::minutes(20)),
        ];

        let sessions = detect_sessions(events, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].event_count, 3);
        assert_eq!(sessions[0].duration_seconds, 20 * 60);
    }

    #[test]
    fn events_exceeding_threshold_form_multiple_sessions() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();
        let events = vec![
            make_event(workspace_id, now),
            make_event(workspace_id, now + Duration::minutes(10)),
            // 40-minute gap (exceeds 30-minute threshold)
            make_event(workspace_id, now + Duration::minutes(50)),
            make_event(workspace_id, now + Duration::minutes(55)),
        ];

        let sessions = detect_sessions(events, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);

        assert_eq!(sessions.len(), 2);

        // Sessions are returned newest first
        assert_eq!(sessions[0].event_count, 2); // Second session (newer)
        assert_eq!(sessions[1].event_count, 2); // First session (older)
    }

    #[test]
    fn custom_threshold_is_respected() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();
        let events = vec![
            make_event(workspace_id, now),
            make_event(workspace_id, now + Duration::minutes(6)), // Just over 5-min threshold
        ];

        // With 5-minute threshold, these should be separate sessions
        let sessions = detect_sessions(events, 5 * 60);
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn unsorted_events_are_handled_correctly() {
        let workspace_id = Uuid::new_v4();
        let now = Utc::now();

        // Events provided out of order
        let events = vec![
            make_event(workspace_id, now + Duration::minutes(20)),
            make_event(workspace_id, now),
            make_event(workspace_id, now + Duration::minutes(10)),
        ];

        let sessions = detect_sessions(events, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].event_count, 3);
        // Should compute duration based on sorted order
        assert_eq!(sessions[0].duration_seconds, 20 * 60);
    }

    #[test]
    fn file_count_is_deduplicated() {
        let workspace_id = Uuid::new_v4();
        let file_id = Uuid::new_v4();
        let now = Utc::now();

        let mut event1 = make_event(workspace_id, now);
        event1.file_id = Some(file_id);

        let mut event2 = make_event(workspace_id, now + Duration::minutes(5));
        event2.file_id = Some(file_id); // Same file

        let mut event3 = make_event(workspace_id, now + Duration::minutes(10));
        event3.file_id = Some(Uuid::new_v4()); // Different file

        let events = vec![event1, event2, event3];
        let sessions = detect_sessions(events, DEFAULT_INACTIVITY_THRESHOLD_SECONDS);

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].event_count, 3);
        assert_eq!(sessions[0].file_count, 2); // Only 2 distinct files
    }
}
