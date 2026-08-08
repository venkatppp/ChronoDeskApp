import { invoke } from "@tauri-apps/api/core";
import type {
  PredictionsSummary,
  WorkflowState,
  LearningProfile,
  AutomationRule,
  CreateAutomationRuleRequest,
} from "@/types/predictive";

function normalizePredictionsSummary(payload: Partial<PredictionsSummary>): PredictionsSummary {
  return {
    nextWorkspace: payload.nextWorkspace ?? null,
    nextFiles: Array.isArray(payload.nextFiles) ? payload.nextFiles : [],
    nextActions: Array.isArray(payload.nextActions) ? payload.nextActions : [],
    sessionContinuation: payload.sessionContinuation ?? null,
    currentWorkflow: payload.currentWorkflow ?? null,
  };
}

class PredictiveRepository {
  async getPredictionsSummary(): Promise<PredictionsSummary> {
    const payload = await invoke<PredictionsSummary>("get_predictions_summary");
    return normalizePredictionsSummary(payload);
  }

  async getCurrentWorkflow(workspaceId: string): Promise<WorkflowState | null> {
    return invoke("get_current_workflow", { workspaceId });
  }

  async getLearningProfile(userId: string): Promise<LearningProfile | null> {
    return invoke("get_learning_profile", { userId });
  }

  async updateLearningProfile(userId: string): Promise<void> {
    return invoke("update_learning_profile", { userId });
  }

  async createAutomationRule(request: CreateAutomationRuleRequest): Promise<AutomationRule> {
    return invoke("create_automation_rule", { request });
  }

  async listAutomationRules(): Promise<AutomationRule[]> {
    return invoke("list_automation_rules");
  }

  async updateAutomationRuleEnabled(ruleId: number, enabled: boolean): Promise<void> {
    return invoke("update_automation_rule_enabled", { ruleId, enabled });
  }

  async deleteAutomationRule(ruleId: number): Promise<void> {
    return invoke("delete_automation_rule", { ruleId });
  }
}

export const predictiveRepository = new PredictiveRepository();
