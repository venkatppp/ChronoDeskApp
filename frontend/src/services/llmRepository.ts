import { invoke } from "@tauri-apps/api/core";
import type { LLMSettings } from "@/types/llm";

export const llmRepository = {
  async getSettings(): Promise<LLMSettings> {
    return invoke<LLMSettings>("llm_get_settings");
  },

  async updateSettings(settings: LLMSettings): Promise<void> {
    return invoke<void>("llm_update_settings", { settings });
  },

  async testConnection(): Promise<void> {
    return invoke<void>("llm_test_connection");
  },

  async isConfigured(): Promise<boolean> {
    return invoke<boolean>("llm_is_configured");
  },
};
