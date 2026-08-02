// Execution types for the Execution Dashboard (RC-5 M5)
// No process mirrors the backend `copilot::execution` types and the planner
// report streaming payload (`execution:progress` events).

import type { ExecutionPlan } from "@/types/proactive";

export type ExecutionStatus =
  | "pending"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export type StepStatus = "pending" | "running" | "completed" | "failed" | "skipped";

export type ExecutionEventType =
  | "started"
  | "step_started"
  | "step_completed"
  | "step_failed"
  | "paused"
  | "resumed"
  | "checkpoint_saved"
  | "checkpoint_loaded"
  | "completed"
  | "failed"
  | "cancelled";

export interface ExecutionStep {
  id: string;
  execution_id: string;
  step_number: number;
  description: string;
  tool_name: string | null;
  arguments: Record<string, unknown> | null;
  status: StepStatus;
  result: string | null;
  error: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
}

export interface ExecutionEvent {
  id: string;
  execution_id: string;
  event_type: ExecutionEventType;
  step_number: number | null;
  message: string;
  metadata: Record<string, unknown> | null;
  created_at: string;
}

export interface PlannerReport {
  plan: ExecutionPlan;
  execution_id: string | null;
  completed: string[];
  skipped: string[];
  replaced: string[];
  replan_count: number;
  error: string | null;
}

export interface ExecutionProgress {
  execution_id: string;
  status: ExecutionStatus;
  current_step: number;
  total_steps: number;
  progress_percentage: number;
  steps: ExecutionStep[];
  recent_events: ExecutionEvent[];
  plan: ExecutionPlan | null;
  planner_report: PlannerReport | null;
}