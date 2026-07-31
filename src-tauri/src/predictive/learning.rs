//! Adaptive Learning system for user behavior analysis.

use chrono::{Timelike, Utc};

use crate::errors::DatabaseError;
use crate::predictive::models::{FocusPattern, LearningProfile, TechPreference};
use crate::predictive::repository::PredictiveRepository;
use crate::repositories::{TimelineRepository, WorkspaceRepository};
use crate::services::ContextService;

/// Adaptive learning system that learns user patterns without storing personal content.
#[derive(Clone)]
pub struct AdaptiveLearning {
    repository: PredictiveRepository,
    workspace_repo: WorkspaceRepository,
    timeline_repo: TimelineRepository,
    context_service: ContextService,
}

impl AdaptiveLearning {
    pub fn new(
        repository: PredictiveRepository,
        workspace_repo: WorkspaceRepository,
        timeline_repo: TimelineRepository,
        context_service: ContextService,
    ) -> Self {
        Self {
            repository,
            workspace_repo,
            timeline_repo,
            context_service,
        }
    }

    /// Updates the learning profile for a user based on recent activity.
    pub async fn update_learning_profile(&self, user_id: &str) -> Result<(), DatabaseError> {
        // Get historical data
        let workspaces = self.workspace_repo.list_active_workspaces().await?;

        let mut all_events = Vec::new();
        for workspace in &workspaces {
            let events = self
                .timeline_repo
                .list_by_workspace(workspace.id, Some(100))
                .await?;
            all_events.extend(events);
        }

        if all_events.is_empty() {
            return Ok(());
        }

        // Calculate preferred work hours
        let preferred_work_hours = self.calculate_preferred_hours(&all_events);

        // Calculate average session duration
        let avg_session_duration_seconds = self.calculate_avg_session_duration(&workspaces).await?;

        // Calculate workspace switch frequency
        let workspace_switch_frequency = self.calculate_switch_frequency(&all_events);

        // Calculate technology preferences
        let technology_preferences = self.calculate_tech_preferences(&all_events);

        // Calculate focus patterns
        let focus_patterns = self.calculate_focus_patterns(&all_events);

        let profile = LearningProfile {
            user_id: user_id.to_string(),
            preferred_work_hours,
            avg_session_duration_seconds,
            workspace_switch_frequency,
            technology_preferences,
            focus_patterns,
            last_updated: Utc::now(),
        };

        self.repository.upsert_learning_profile(&profile).await?;

        Ok(())
    }

    /// Gets the learning profile for a user.
    pub async fn get_learning_profile(
        &self,
        user_id: &str,
    ) -> Result<Option<LearningProfile>, DatabaseError> {
        self.repository.get_learning_profile(user_id).await
    }

    /// Calculates preferred work hours from timeline events.
    fn calculate_preferred_hours(&self, events: &[crate::models::TimelineEvent]) -> Vec<i32> {
        use std::collections::HashMap;

        let mut hour_counts: HashMap<i32, i32> = HashMap::new();

        for event in events {
            let hour = event.occurred_at.hour() as i32;
            *hour_counts.entry(hour).or_insert(0) += 1;
        }

        // Get top 8 hours
        let mut hours: Vec<(i32, i32)> = hour_counts.into_iter().collect();
        hours.sort_by(|a, b| b.1.cmp(&a.1));

        hours.into_iter().take(8).map(|(hour, _)| hour).collect()
    }

    /// Calculates average session duration.
    async fn calculate_avg_session_duration(
        &self,
        workspaces: &[crate::models::Workspace],
    ) -> Result<i64, DatabaseError> {
        let mut total_duration = 0i64;
        let mut session_count = 0;

        for workspace in workspaces {
            let sessions = self
                .context_service
                .get_workspace_sessions(workspace.id, Some(20))
                .await?;

            for session in sessions {
                total_duration += session.duration_seconds;
                session_count += 1;
            }
        }

        if session_count == 0 {
            return Ok(3600); // Default 1 hour
        }

        Ok(total_duration / session_count)
    }

    /// Calculates workspace switch frequency (switches per hour).
    fn calculate_switch_frequency(&self, events: &[crate::models::TimelineEvent]) -> f64 {
        let workspace_switches = events
            .iter()
            .filter(|e| e.event_type.as_str() == "workspace_switch")
            .count();

        if events.is_empty() {
            return 0.0;
        }

        // Calculate time span
        let first = events.first().unwrap();
        let last = events.last().unwrap();
        let duration_hours = (last.occurred_at - first.occurred_at).num_hours().max(1) as f64;

        workspace_switches as f64 / duration_hours
    }

    /// Calculates technology preferences from file patterns.
    fn calculate_tech_preferences(
        &self,
        events: &[crate::models::TimelineEvent],
    ) -> Vec<TechPreference> {
        use std::collections::HashMap;

        let mut tech_counts: HashMap<String, i32> = HashMap::new();
        let mut total = 0;

        for event in events {
            if let Some(metadata) = &event.metadata {
                if let Some(path) = metadata.get("path").and_then(|v| v.as_str()) {
                    if let Some(ext) = std::path::Path::new(path)
                        .extension()
                        .and_then(|e| e.to_str())
                    {
                        let tech = self.extension_to_tech(ext);
                        *tech_counts.entry(tech.to_string()).or_insert(0) += 1;
                        total += 1;
                    }
                }
            }
        }

        if total == 0 {
            return Vec::new();
        }

        let mut prefs: Vec<TechPreference> = tech_counts
            .into_iter()
            .map(|(tech, count)| TechPreference {
                technology: tech,
                usage_percentage: (count as f64 / total as f64) * 100.0,
            })
            .collect();

        prefs.sort_by(|a, b| b.usage_percentage.partial_cmp(&a.usage_percentage).unwrap());
        prefs.truncate(10);

        prefs
    }

    /// Calculates focus patterns.
    fn calculate_focus_patterns(&self, events: &[crate::models::TimelineEvent]) -> FocusPattern {
        let preferred_hours = self.calculate_preferred_hours(events);
        let peak_focus_hours = preferred_hours.into_iter().take(3).collect();

        // Calculate average focus duration (time between workspace switches)
        let mut focus_durations = Vec::new();
        let mut last_switch: Option<chrono::DateTime<chrono::Utc>> = None;

        for event in events {
            if event.event_type.as_str() == "workspace_switch" {
                if let Some(prev) = last_switch {
                    let duration = (event.occurred_at.timestamp() - prev.timestamp()) / 60;
                    if duration > 0 && duration < 300 {
                        // Less than 5 hours
                        focus_durations.push(duration as i32);
                    }
                }
                last_switch = Some(event.occurred_at);
            }
        }

        let avg_focus_duration_minutes = if focus_durations.is_empty() {
            60
        } else {
            focus_durations.iter().sum::<i32>() / focus_durations.len() as i32
        };

        // Calculate distraction frequency (events per hour)
        let distraction_frequency = if events.len() > 1 {
            let first = events.first().unwrap();
            let last = events.last().unwrap();
            let hours = (last.occurred_at - first.occurred_at).num_hours().max(1) as f64;
            events.len() as f64 / hours
        } else {
            0.0
        };

        FocusPattern {
            peak_focus_hours,
            avg_focus_duration_minutes,
            distraction_frequency,
        }
    }

    /// Maps file extension to technology name.
    fn extension_to_tech(&self, ext: &str) -> &'static str {
        match ext {
            "rs" => "Rust",
            "js" | "jsx" => "JavaScript",
            "ts" | "tsx" => "TypeScript",
            "py" => "Python",
            "java" => "Java",
            "go" => "Go",
            "cpp" | "cc" | "cxx" => "C++",
            "c" | "h" => "C",
            "rb" => "Ruby",
            "php" => "PHP",
            "swift" => "Swift",
            "kt" => "Kotlin",
            "md" => "Markdown",
            "html" => "HTML",
            "css" | "scss" | "sass" => "CSS",
            "json" => "JSON",
            "yaml" | "yml" => "YAML",
            "toml" => "TOML",
            "sql" => "SQL",
            _ => "Other",
        }
    }
}
