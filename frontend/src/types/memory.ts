// Execution Memory & Learning types (RC-6 M1)
// Mirrors the backend `copilot::memory` models: durable records of
// executions, planner reports, and autonomous sessions, plus the ranked
// hits / recommendations / avoid lists the retrieval and learning engines
// return.

import type { ExecutionPlan } from "@/types/proactive";

export type MemoryKind = "execution" | "planner_report" | "autonomous_session";

export type MemoryStatus = "success" | "failed" | "cancelled";

export interface MemoryOutcome {
  steps: number;
  completed: number;
  replaced: number;
  replan_count: number;
  retries_used: number;
  plans_attempted: number;
  duration_seconds: number;
}

export interface ExecutionMemoryRecord {
  id: string;
  kind: MemoryKind;
  source_id: string;
  workspace_id: string | null;
  goal: string;
  status: MemoryStatus;
  plan: ExecutionPlan | null;
  steps: string[];
  reasoning: string[];
  tools_used: string[];
  failed_steps: string[];
  error: string | null;
  outcome: MemoryOutcome;
  goal_embedding: number[] | null;
  replay_count: number;
  created_at: string;
  updated_at: string;
}

export interface MemoryHit {
  record: ExecutionMemoryRecord;
  similarity: number;
}

export interface MemoryRecommendation {
  record: ExecutionMemoryRecord;
  score: number;
  replay_count: number;
  /** Confidence Engine score (RC-6 M3): 0..1 */
  confidence_score: number;
  /** Why the confidence is what it is, per factor (RC-6 M3) */
  explanation: RecommendationExplanation[];
}

export interface RecommendationExplanation {
  factor: string;
  impact: number;
  description: string;
}

export interface AvoidedStrategy {
  record: ExecutionMemoryRecord;
  similarity: number;
  failure: string;
}

export interface LearnedWorkflow {
  goal_fingerprint: string;
  goal: string;
  success_count: number;
  failure_count: number;
  best_plan: ExecutionPlan | null;
  last_success_at: string | null;
}

export interface MemoryStats {
  total_records: number;
  successful: number;
  failed: number;
  cancelled: number;
  executions: number;
  planner_reports: number;
  autonomous_sessions: number;
  total_replays: number;
  learned_workflows: number;
}

// --- RC-6 M2: vector memory system ---

export interface VectorIndexStatus {
  total_records: number;
  indexed: number;
  pending: number;
  provider: string;
  dimensions: number;
  last_indexed_at: string | null;
  cache_size: number;
  cache_capacity: number;
  cache_hits: number;
  cache_misses: number;
  cache_hit_rate: number;
}

export interface IndexResult {
  requested: number;
  indexed: number;
  failed: number;
  skipped: number;
}

// --- RC-6 M3: adaptive learning ---

export interface SuccessTrend {
  date: string;
  successes: number;
  failures: number;
  success_rate: number;
}

export interface WorkflowQuality {
  workflow_count: number;
  avg_success_rate: number;
  avg_plan_confidence: number;
  avg_duration_seconds: number;
  replay_adoption_rate: number;
  replay_per_run: number;
}

export interface MemoryUtilization {
  total_records: number;
  active_records: number;
  aging_records: number;
  archived_records: number;
  avg_freshness: number;
  utilization_ratio: number;
  workflows_per_record: number;
}

export interface LearningHealth {
  confidence_average: number;
  confidence_successful: number;
  acceptance_rate: number;
  workflow_quality: WorkflowQuality;
  success_trends: SuccessTrend[];
  memory_utilization: MemoryUtilization;
  score_average: number;
}

export type FailurePatternType =
  | "repeated_failure"
  | "unstable_workflow"
  | "low_confidence_plan";

export interface FailurePattern {
  pattern_type: FailurePatternType;
  goal: string;
  goal_fingerprint: string;
  description: string;
  severity: number;
  occurrences: number;
  last_seen: string;
  avg_plan_confidence: number | null;
}

export interface WorkflowFamily {
  family_id: number;
  name: string;
  member_count: number;
  goals: string[];
  shared_tools: string[];
  total_successes: number;
  total_failures: number;
  avg_duration_seconds: number;
  avg_confidence: number;
}

export interface MemoryAgingSummary {
  total_records: number;
  fresh_records: number;
  aging_records: number;
  archived_records: number;
  avg_freshness: number;
  oldest_days: number;
  newest_days: number;
}

export interface DuplicateGroup {
  goal_fingerprint: string;
  records: ExecutionMemoryRecord[];
  keep_id: string;
  reason: string;
}

export interface MergeResult {
  groups_merged: number;
  records_merged: number;
}
