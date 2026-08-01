//! AI diagnostics and monitoring.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::ai::models::{InferenceStats, ModelInfo};

/// System-wide AI diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIDiagnostics {
    pub models: Vec<ModelInfo>,
    pub inference_stats: HashMap<String, InferenceStats>,
    pub total_memory_usage_bytes: u64,
    pub cache_stats: CacheDiagnostics,
    pub system_info: SystemInfo,
}

/// Cache diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheDiagnostics {
    pub embedding_cache_size: usize,
    pub embedding_cache_capacity: usize,
    pub embedding_cache_hit_rate: f32,
    pub inference_cache_size: usize,
    pub inference_cache_capacity: usize,
    pub inference_cache_hit_rate: f32,
}

/// System information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub available_memory_bytes: u64,
    pub cpu_cores: usize,
    pub onnx_runtime_version: String,
}

impl AIDiagnostics {
    /// Creates a new diagnostics snapshot.
    pub fn new(models: Vec<ModelInfo>, inference_stats: HashMap<String, InferenceStats>) -> Self {
        let total_memory_usage_bytes = models.iter().filter_map(|m| m.memory_usage_bytes).sum();

        Self {
            models,
            inference_stats,
            total_memory_usage_bytes,
            cache_stats: CacheDiagnostics {
                embedding_cache_size: 0,
                embedding_cache_capacity: 0,
                embedding_cache_hit_rate: 0.0,
                inference_cache_size: 0,
                inference_cache_capacity: 0,
                inference_cache_hit_rate: 0.0,
            },
            system_info: SystemInfo {
                available_memory_bytes: Self::get_available_memory(),
                cpu_cores: num_cpus::get(),
                onnx_runtime_version: "2.0.0-rc.4".to_string(),
            },
        }
    }

    /// Updates cache statistics.
    pub fn with_cache_stats(mut self, cache_stats: CacheDiagnostics) -> Self {
        self.cache_stats = cache_stats;
        self
    }

    /// Gets available system memory.
    fn get_available_memory() -> u64 {
        // Platform-specific memory detection
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
                for line in content.lines() {
                    if line.starts_with("MemAvailable:") {
                        if let Some(kb) = line.split_whitespace().nth(1) {
                            if let Ok(kb_val) = kb.parse::<u64>() {
                                return kb_val * 1024;
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("sysctl").arg("hw.memsize").output() {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    if let Some(value) = stdout.split(':').nth(1) {
                        if let Ok(bytes) = value.trim().parse::<u64>() {
                            return bytes;
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            if let Ok(output) = Command::new("wmic")
                .args(&["OS", "get", "FreePhysicalMemory"])
                .output()
            {
                if let Ok(stdout) = String::from_utf8(output.stdout) {
                    for line in stdout.lines().skip(1) {
                        if let Ok(kb) = line.trim().parse::<u64>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }

        0 // Unknown
    }

    /// Generates a summary report.
    pub fn summary(&self) -> String {
        let loaded_models = self
            .models
            .iter()
            .filter(|m| matches!(m.status, crate::ai::models::ModelStatus::Loaded))
            .count();

        let total_inferences: u64 = self
            .inference_stats
            .values()
            .map(|s| s.total_inferences)
            .sum();

        let avg_cache_hit_rate = if !self.inference_stats.is_empty() {
            self.inference_stats
                .values()
                .map(|s| s.cache_hit_rate)
                .sum::<f32>()
                / self.inference_stats.len() as f32
        } else {
            0.0
        };

        format!(
            "AI Diagnostics:\n\
             - Models loaded: {}/{}\n\
             - Total memory: {:.2} MB\n\
             - Total inferences: {}\n\
             - Avg cache hit rate: {:.1}%\n\
             - System memory: {:.2} GB\n\
             - CPU cores: {}",
            loaded_models,
            self.models.len(),
            self.total_memory_usage_bytes as f64 / 1_000_000.0,
            total_inferences,
            avg_cache_hit_rate * 100.0,
            self.system_info.available_memory_bytes as f64 / 1_000_000_000.0,
            self.system_info.cpu_cores
        )
    }
}

/// Health check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheck {
    pub healthy: bool,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

impl HealthCheck {
    /// Performs a health check on the AI system.
    pub fn perform(diagnostics: &AIDiagnostics) -> Self {
        let mut issues = Vec::new();
        let mut warnings = Vec::new();

        // Check if any models are loaded
        let loaded_models = diagnostics
            .models
            .iter()
            .filter(|m| matches!(m.status, crate::ai::models::ModelStatus::Loaded))
            .count();

        if loaded_models == 0 {
            warnings.push("No models are currently loaded".to_string());
        }

        // Check for model errors
        for model in &diagnostics.models {
            if matches!(model.status, crate::ai::models::ModelStatus::Error) {
                if let Some(err) = &model.error_message {
                    issues.push(format!("Model {} error: {}", model.metadata.id, err));
                }
            }
        }

        // Check memory usage
        let memory_gb = diagnostics.total_memory_usage_bytes as f64 / 1_000_000_000.0;
        if memory_gb > 4.0 {
            warnings.push(format!("High memory usage: {:.2} GB", memory_gb));
        }

        // Check cache hit rates
        for (model_id, stats) in &diagnostics.inference_stats {
            if stats.total_inferences > 100 && stats.cache_hit_rate < 0.3 {
                warnings.push(format!(
                    "Low cache hit rate for {}: {:.1}%",
                    model_id,
                    stats.cache_hit_rate * 100.0
                ));
            }
        }

        let healthy = issues.is_empty();

        Self {
            healthy,
            issues,
            warnings,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_summary_generates() {
        let diagnostics = AIDiagnostics::new(Vec::new(), HashMap::new());
        let summary = diagnostics.summary();
        assert!(summary.contains("AI Diagnostics"));
    }

    #[test]
    fn health_check_warns_on_no_models() {
        let diagnostics = AIDiagnostics::new(Vec::new(), HashMap::new());
        let health = HealthCheck::perform(&diagnostics);
        assert!(!health.warnings.is_empty());
    }
}
