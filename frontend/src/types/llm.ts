// LLM Types
export type LLMProviderType = 'openai' | 'ollama' | 'custom';

export interface LLMSettings {
  provider: LLMProviderType;
  base_url: string;
  api_key: string;
  model: string;
  temperature: number;
  max_tokens: number;
  context_window: number;
}

export interface LLMConnectionStatus {
  is_configured: boolean;
  is_connected?: boolean;
  last_test?: string;
  error?: string;
}
