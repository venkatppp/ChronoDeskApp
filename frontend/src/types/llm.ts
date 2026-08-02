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

export type CircuitBreakerState = 'Closed' | 'Open' | 'HalfOpen';

export interface LLMProviderDiagnostics {
  provider: string;
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  retries: number;
  retry_rate: number;
  rate_limited_requests: number;
  circuit_breaker_state: CircuitBreakerState;
  average_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  provider_uptime_seconds: number;
}
