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
