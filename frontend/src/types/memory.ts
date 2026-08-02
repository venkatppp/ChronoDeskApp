// Execution Memory & Learning types (RC-6 M1)
// Mirrors the backend `copilot::memory` models: durable records of
// executions, planner reports, and autonomous sessions, plus the ranked
// hits / recommendations / avoid lists the retrieval and learning engines
// return.

import type { ExecutionPlan } from "@/types/proactive";

export type MemoryKind = "execution" | "planner_report" | "autonomous_session";

export type MemoryStatus = "success" | "failed" | "cancelled";

// --- RC-6 M4: retention policy ---

export type RetentionPolicy = "permanent" | "temporary" | "archived" | "expired";

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

  // --- RC-6 M4: lifecycle ---
  retention: RetentionPolicy;
  retention_until: string | null;
  archived_at: string | null;
  expired_at: string | null;
  summary: string | null;
  compressed_at: string | null;
  version: number;
  parent_id: string | null;
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

// --- RC-6 M4: memory lifecycle ---

export interface CleanupReport {
  expired_marked: number;
  removed_expired: number;
  removed_duplicate_archives: number;
  removed_orphaned_vectors: number;
  compressed: number;
  ran_at: string;
}

export interface CompressionResult {
  examined: number;
  compressed: number;
  already_compressed: number;
}

export interface MemoryStorageStats {
  database_size_bytes: number;
  vector_index_size_bytes: number;
  cache_entries: number;
  cache_size_bytes: number;
  cache_capacity: number;
  cache_occupancy: number;
  archived_memories: number;
  expired_memories: number;
  temporary_memories: number;
  permanent_memories: number;
  snapshots: number;
  snapshot_size_bytes: number;
  compressed_records: number;
  compression_archive_count: number;
}

export type LineageRelation = "parent" | "merged";

export interface LineageNode {
  id: string;
  goal: string;
  status: MemoryStatus;
  retention: RetentionPolicy;
  version: number;
  created_at: string;
  relation: LineageRelation | null;
}

export interface MemoryLineage {
  memory_id: string;
  root_id: string | null;
  version: number;
  ancestors: LineageNode[];
  children: LineageNode[];
  merged_into: LineageNode[];
  merged_into_id: string | null;
}

export interface MemorySnapshot {
  id: string;
  label: string;
  created_at: string;
  record_count: number;
}

export interface ImportResult {
  imported: number;
  skipped: number;
  acceptance_restored: number;
}

export interface RestoreResult {
  records_restored: number;
  acceptance_restored: number;
  snapshots_kept: number;
}
