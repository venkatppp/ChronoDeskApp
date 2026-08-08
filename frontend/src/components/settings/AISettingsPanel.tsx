import { useState, useEffect } from "react";
import { Bot, Check, X, Loader2, AlertCircle, Eye, EyeOff, Zap } from "lucide-react";
import { llmRepository } from "@/services/llmRepository";
import type { LLMSettings, LLMProviderType } from "@/types/llm";

export function AISettingsPanel() {
  const [settings, setSettings] = useState<LLMSettings | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<{ success: boolean; message: string } | null>(null);
  const [showApiKey, setShowApiKey] = useState(false);
  const [hasChanges, setHasChanges] = useState(false);

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    setIsLoading(true);
    setError(null);
    try {
      const data = await llmRepository.getSettings();
      setSettings(data);
    } catch (err) {
      setError("Failed to load AI settings");
      console.error("Failed to load settings:", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSave = async () => {
    if (!settings) return;

    setIsSaving(true);
    setError(null);
    setTestResult(null);

    try {
      await llmRepository.updateSettings(settings);
      setHasChanges(false);
      setTestResult({ success: true, message: "Settings saved successfully" });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save settings");
    } finally {
      setIsSaving(false);
    }
  };

  const handleTestConnection = async () => {
    setIsTesting(true);
    setTestResult(null);
    setError(null);

    try {
      await llmRepository.testConnection();
      setTestResult({ success: true, message: "Connection successful" });
    } catch (err) {
      setTestResult({
        success: false,
        message: err instanceof Error ? err.message : "Connection failed",
      });
    } finally {
      setIsTesting(false);
    }
  };

  const updateField = <K extends keyof LLMSettings>(field: K, value: LLMSettings[K]) => {
    if (!settings) return;
    setSettings({ ...settings, [field]: value });
    setHasChanges(true);
    setTestResult(null);
  };

  const providerOptions: { value: LLMProviderType; label: string; description: string }[] = [
    { value: "openai", label: "OpenAI", description: "GPT-4, GPT-3.5" },
    { value: "ollama", label: "Ollama", description: "Local models" },
    { value: "custom", label: "Custom", description: "OpenAI-compatible API" },
  ];

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-20">
        <Loader2 className="h-8 w-8 animate-spin text-(--color-accent)" />
      </div>
    );
  }

  if (!settings) {
    return (
      <div className="text-center py-20 text-(--color-muted-foreground)">
        <AlertCircle className="h-12 w-12 mx-auto mb-4 opacity-50" />
        <p>Failed to load AI settings</p>
      </div>
    );
  }

  return (
    <section className="bg-(--color-surface-hover) border border-(--color-border) rounded-3xl overflow-hidden">
      <div className="p-8 border-b border-(--color-border) bg-(--color-background)-tertiary/30">
        <div className="flex items-center gap-3">
          <div className="p-2.5 bg-(--color-accent)/10 text-(--color-accent) rounded-xl">
            <Bot className="h-6 w-6" />
          </div>
          <div>
            <h2 className="text-xl font-bold text-(--color-foreground)">AI Settings</h2>
            <p className="text-sm text-(--color-muted-foreground)">
              Configure your LLM provider for AI Copilot features.
            </p>
          </div>
        </div>
      </div>

      <div className="p-8 space-y-6">
        {error && (
          <div className="p-4 bg-(--color-danger)/10 border border-(--color-danger)/20 rounded-xl text-(--color-danger) text-sm flex items-center gap-2">
            <AlertCircle className="h-4 w-4 flex-shrink-0" />
            <span>{error}</span>
          </div>
        )}

        {testResult && (
          <div
            className={`p-4 border rounded-xl text-sm flex items-center gap-2 ${
              testResult.success
                ? "bg-emerald-500/10 border-emerald-500/20 text-emerald-600 text-emerald-400"
                : "bg-(--color-danger)/10 border-(--color-danger)/20 text-(--color-danger)"
            }`}
          >
            {testResult.success ? (
              <Check className="h-4 w-4 flex-shrink-0" />
            ) : (
              <X className="h-4 w-4 flex-shrink-0" />
            )}
            <span>{testResult.message}</span>
          </div>
        )}

        {/* Provider Selection */}
        <div>
          <label className="block text-sm font-bold text-(--color-foreground) mb-3">Provider</label>
          <div className="grid grid-cols-3 gap-3">
            {providerOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                onClick={() => updateField("provider", option.value)}
                className={`p-4 rounded-xl border-2 text-left transition-all ${
                  settings.provider === option.value
                    ? "bg-(--color-accent)/5 border-(--color-accent)"
                    : "bg-(--color-background)-tertiary border-transparent hover:border-(--color-border)"
                }`}
              >
                <div className="font-bold text-sm text-(--color-foreground)">{option.label}</div>
                <div className="text-xs text-(--color-muted-foreground) mt-1">{option.description}</div>
              </button>
            ))}
          </div>
        </div>

        {/* Base URL */}
        <div>
          <label htmlFor="base_url" className="block text-sm font-bold text-(--color-foreground) mb-2">
            Base URL
          </label>
          <input
            id="base_url"
            type="url"
            value={settings.base_url}
            onChange={(e) => updateField("base_url", e.target.value)}
            placeholder="https://api.openai.com/v1"
            className="w-full px-4 py-3 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) placeholder-muted-foreground focus:outline-none focus:border-(--color-accent) transition-colors"
          />
          <p className="text-xs text-(--color-muted-foreground) mt-2">
            {settings.provider === "ollama"
              ? "Typically http://localhost:11434/v1"
              : "API endpoint for the provider"}
          </p>
        </div>

        {/* API Key */}
        <div>
          <label htmlFor="api_key" className="block text-sm font-bold text-(--color-foreground) mb-2">
            API Key
          </label>
          <div className="relative">
            <input
              id="api_key"
              type={showApiKey ? "text" : "password"}
              value={settings.api_key}
              onChange={(e) => updateField("api_key", e.target.value)}
              placeholder={settings.provider === "ollama" ? "Not required for Ollama" : "sk-..."}
              className="w-full px-4 py-3 pr-12 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) placeholder-muted-foreground focus:outline-none focus:border-(--color-accent) transition-colors font-mono text-sm"
            />
            <button
              type="button"
              onClick={() => setShowApiKey(!showApiKey)}
              className="absolute right-3 top-1/2 -translate-y-1/2 p-2 text-(--color-muted-foreground) hover:text-(--color-foreground) transition-colors"
            >
              {showApiKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
            </button>
          </div>
          {settings.provider !== "ollama" && (
            <p className="text-xs text-(--color-muted-foreground) mt-2">
              Your API key is stored locally and never shared.
            </p>
          )}
        </div>

        {/* Model */}
        <div>
          <label htmlFor="model" className="block text-sm font-bold text-(--color-foreground) mb-2">
            Model
          </label>
          <input
            id="model"
            type="text"
            value={settings.model}
            onChange={(e) => updateField("model", e.target.value)}
            placeholder="gpt-4o-mini"
            className="w-full px-4 py-3 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) placeholder-muted-foreground focus:outline-none focus:border-(--color-accent) transition-colors"
          />
          <p className="text-xs text-(--color-muted-foreground) mt-2">
            {settings.provider === "openai" && "Examples: gpt-4o, gpt-4o-mini, gpt-3.5-turbo"}
            {settings.provider === "ollama" && "Examples: llama3, mistral, codellama"}
            {settings.provider === "custom" && "Model identifier for your provider"}
          </p>
        </div>

        {/* Advanced Settings */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label htmlFor="temperature" className="block text-sm font-bold text-(--color-foreground) mb-2">
              Temperature
            </label>
            <input
              id="temperature"
              type="number"
              min="0"
              max="2"
              step="0.1"
              value={settings.temperature}
              onChange={(e) => updateField("temperature", parseFloat(e.target.value))}
              className="w-full px-4 py-3 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) focus:outline-none focus:border-(--color-accent) transition-colors"
            />
            <p className="text-xs text-(--color-muted-foreground) mt-2">0.0 = deterministic, 2.0 = creative</p>
          </div>

          <div>
            <label htmlFor="max_tokens" className="block text-sm font-bold text-(--color-foreground) mb-2">
              Max Tokens
            </label>
            <input
              id="max_tokens"
              type="number"
              min="100"
              max="32000"
              step="100"
              value={settings.max_tokens}
              onChange={(e) => updateField("max_tokens", parseInt(e.target.value))}
              className="w-full px-4 py-3 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) focus:outline-none focus:border-(--color-accent) transition-colors"
            />
            <p className="text-xs text-(--color-muted-foreground) mt-2">Maximum response length</p>
          </div>
        </div>

        <div>
          <label htmlFor="context_window" className="block text-sm font-bold text-(--color-foreground) mb-2">
            Context Window
          </label>
          <input
            id="context_window"
            type="number"
            min="2048"
            max="200000"
            step="1024"
            value={settings.context_window}
            onChange={(e) => updateField("context_window", parseInt(e.target.value))}
            className="w-full px-4 py-3 bg-(--color-background)-tertiary border border-(--color-border) rounded-xl text-(--color-foreground) focus:outline-none focus:border-(--color-accent) transition-colors"
          />
          <p className="text-xs text-(--color-muted-foreground) mt-2">
            Total tokens available for context (model-dependent)
          </p>
        </div>

        {/* Action Buttons */}
        <div className="flex items-center gap-3 pt-4">
          <button
            onClick={handleSave}
            disabled={isSaving || !hasChanges}
            className="flex items-center gap-2 px-6 py-3 bg-(--color-accent) text-(--color-accent-foreground) rounded-xl font-bold text-sm hover:scale-[1.02] active:scale-[0.98] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100"
          >
            {isSaving ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Saving...
              </>
            ) : (
              <>
                <Check className="h-4 w-4" />
                Save Settings
              </>
            )}
          </button>

          <button
            onClick={handleTestConnection}
            disabled={isTesting || !settings.api_key || !settings.model}
            className="flex items-center gap-2 px-6 py-3 bg-(--color-background)-tertiary text-(--color-foreground) border border-(--color-border) rounded-xl font-bold text-sm hover:bg-(--color-surface-hover) transition-all disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isTesting ? (
              <>
                <Loader2 className="h-4 w-4 animate-spin" />
                Testing...
              </>
            ) : (
              <>
                <Zap className="h-4 w-4" />
                Test Connection
              </>
            )}
          </button>
        </div>
      </div>
    </section>
  );
}
