// Predictive Intelligence types

export interface WorkspacePrediction {
  workspaceId: string;
  workspaceName: string;
  confidence: number;
  reason: string;
  predictedAt: string;
}

export interface FilePrediction {
  filePath: string;
  workspaceId: string;
  confidence: number;
  reason: string;
}

export interface ActionPrediction {
  actionType: string;
  description: string;
  confidence: number;
  reason: string;
}

export interface SessionContinuationPrediction {
  willContinue: boolean;
  confidence: number;
  estimatedDurationSeconds: number;
  reason: string;
}

export type WorkflowType =
  | "coding"
  | "debugging"
  | "documentation"
  | "research"
  | "meeting"
  | "custom";

export interface WorkflowState {
  workflowType: WorkflowType;
  startedAt: string;
  workspaceId: string;
  confidence: number;
  activeFiles: string[];
}

export interface PredictionsSummary {
  nextWorkspace: WorkspacePrediction | null;
  nextFiles: FilePrediction[];
  nextActions: ActionPrediction[];
  sessionContinuation: SessionContinuationPrediction | null;
  currentWorkflow: WorkflowState | null;
}

export interface LearningProfile {
  userId: string;
  preferredWorkHours: number[];
  avgSessionDurationSeconds: number;
  workspaceSwitchFrequency: number;
  technologyPreferences: TechPreference[];
  focusPatterns: FocusPattern;
  lastUpdated: string;
}

export interface TechPreference {
  technology: string;
  usagePercentage: number;
}

export interface FocusPattern {
  peakFocusHours: number[];
  avgFocusDurationMinutes: number;
  distractionFrequency: number;
}

export type TriggerType =
  | "workspace_activated"
  | "long_inactive"
  | "duplicates_exceed_threshold"
  | "productivity_drop"
  | "workflow_transition"
  | "time_of_day";

export type ActionType =
  | "restore_context"
  | "create_snapshot"
  | "recommend_cleanup"
  | "recommend_break"
  | "notify_user"
  | "switch_workspace";

export interface AutomationRule {
  id: number;
  name: string;
  enabled: boolean;
  triggerType: TriggerType;
  triggerConfig: Record<string, unknown>;
  actionType: ActionType;
  actionConfig: Record<string, unknown>;
  createdAt: string;
}

export interface CreateAutomationRuleRequest {
  name: string;
  enabled: boolean;
  triggerType: TriggerType;
  triggerConfig: Record<string, unknown>;
  actionType: ActionType;
  actionConfig: Record<string, unknown>;
}
