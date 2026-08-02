// Autonomous Agent Runtime types (RC-5 M6)
// Mirrors the backend `copilot::autonomous::models` and the payloads
// streamed over `autonomous:session` / `autonomous:reasoning`.

import type { ExecutionPlan } from "@/types/proactive";

export type AutonomousStatus =
  | "running"
  | "paused"
  | "waiting_approval"
  | "completed"
  | "failed"
  | "cancelled";

export type ApprovalMode = "automatic" | "on_risk" | "manual";

export type ReasoningPhase =
  | "planning"
  | "executing"
  | "observed"
  | "replanning"
  | "awaiting_approval"
  | "approval_resolved"
  | "budget_update"
  | "pause"
  | "terminal";

export interface ReasoningEvent {
  session_id: string;
  phase: ReasoningPhase;
  message: string;
  detail: Record<string, unknown> | null;
  created_at: string;
}

export interface ExecutionBudget {
  max_steps: number;
  max_plans: number;
  max_replans: number;
  max_duration_seconds: number;
}

export interface RetryPolicy {
  max_attempts: number;
  backoff_ms: number;
  retry_on_timeout: boolean;
}

export interface TimeoutPolicy {
  step_timeout_ms: number;
  plan_timeout_seconds: number;
  approval_timeout_seconds: number;
}

export interface ApprovalPolicy {
  mode: ApprovalMode;
  gate_replans: boolean;
}

export interface ExecutionPolicy {
  budget: ExecutionBudget;
  retry: RetryPolicy;
  timeout: TimeoutPolicy;
  approval: ApprovalPolicy;
}

export interface ApprovalRequest {
  request_id: string;
  session_id: string;
  goal: string;
  plan: ExecutionPlan;
  reason: string;
  requested_at: string;
  decided_at: string | null;
  approved: boolean | null;
  note: string | null;
}

export interface AutonomousSessionProgress {
  session_id: string;
  workspace_id: string | null;
  goal: string;
  status: AutonomousStatus;
  policy: ExecutionPolicy;
  reasoning: ReasoningEvent[];
  current_plan: ExecutionPlan | null;
  execution_id: string | null;
  last_execution_id: string | null;
  plans_attempted: number;
  plans_completed: number;
  steps_completed: number;
  retries_used: number;
  replans_used: number;
  steps_left: number;
  error: string | null;
  pending_approval: ApprovalRequest | null;
  created_at: string;
  updated_at: string;
}