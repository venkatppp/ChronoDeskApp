// Proactive AI types for frontend

export type NotificationType =
  | "resume_work"
  | "long_focus_session"
  | "workspace_switch"
  | "repeated_edits"
  | "build_failure"
  | "recurring_workflow"
  | "idle_period"
  | "project_completion"
  | "unfinished_work"
  | "health_warning"
  | "learning_insight"
  | "recommendation_update";

export type NotificationPriority = "low" | "medium" | "high" | "critical";

export type EvidenceSource =
  | "timeline"
  | "context_memory"
  | "predictive"
  | "learning"
  | "semantic"
  | "session"
  | "recommendation"
  | "health_monitor";

export type PermissionLevel = "always_allow" | "always_reject" | "ask_each_time";

export type PlanApprovalStatus = "pending" | "approved" | "rejected" | "executing" | "completed" | "failed";

export interface Evidence {
  source: EvidenceSource;
  description: string;
  confidence: number;
  timestamp: string;
  metadata: Record<string, unknown>;
}

export interface ProactiveNotification {
  id: string;
  workspace_id: string | null;
  notification_type: NotificationType;
  title: string;
  message: string;
  priority: NotificationPriority;
  evidence: Evidence[];
  suggested_actions: string[];
  dismissible: boolean;
  dismissed: boolean;
  created_at: string;
  expires_at: string | null;
}

export interface UnfinishedWork {
  description: string;
  file_path: string | null;
  detected_at: string;
  confidence: number;
  evidence: Evidence[];
}

export interface TimelineSummary {
  event_type: string;
  description: string;
  occurred_at: string;
}

export interface ResumeContext {
  workspace_id: string;
  last_active: string;
  unfinished_work: UnfinishedWork[];
  open_files: string[];
  active_branch: string | null;
  recent_timeline: TimelineSummary[];
  previous_conversation_id: string | null;
  context_snapshot: string | null;
}

export interface PlanTask {
  id: string;
  description: string;
  dependencies: string[];
  estimated_minutes: number;
  required_files: string[];
  tool_name: string | null;
  arguments: Record<string, unknown> | null;
  completed: boolean;
}

export interface ExecutionPlan {
  id: string;
  workspace_id: string | null;
  goal: string;
  tasks: PlanTask[];
  estimated_duration_minutes: number;
  required_files: string[];
  checkpoints: string[];
  confidence: number;
  reasoning: string;
  status: PlanApprovalStatus;
  created_at: string;
}

export interface AutomationPermission {
  id: string;
  workspace_id: string | null;
  action_type: string;
  permission: PermissionLevel;
  granted_at: string;
  expires_at: string | null;
}

export interface Priority {
  description: string;
  confidence: number;
  reasoning: string;
  estimated_minutes: number;
}

export interface HealthTrend {
  workspace_id: string;
  workspace_name: string;
  current_score: number;
  previous_score: number;
  change: number;
  trend: string;
}

export interface PredictionChange {
  prediction_type: string;
  previous_confidence: number;
  current_confidence: number;
  change: number;
  reasoning: string;
}

export interface LearningInsight {
  insight_type: string;
  description: string;
  confidence: number;
  discovered_at: string;
}

export interface SemanticDiscovery {
  discovery_type: string;
  description: string;
  files: string[];
  confidence: number;
}

export interface FocusBlock {
  time: string;
  activity: string;
  confidence: number;
}

export interface EnhancedBriefing {
  date: string;
  summary: string;
  yesterday_summary: string[];
  today_priorities: Priority[];
  unfinished_work: UnfinishedWork[];
  health_trends: HealthTrend[];
  prediction_changes: PredictionChange[];
  learning_insights: LearningInsight[];
  semantic_discoveries: SemanticDiscovery[];
  recommendations: string[];
  estimated_focus_schedule: FocusBlock[];
}

export interface TimelineIntelligence {
  query: string;
  answer: string;
  evidence: Evidence[];
  confidence: number;
  related_events: TimelineSummary[];
}
