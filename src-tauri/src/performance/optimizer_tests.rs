//! Optimizer tests (RC-10 M1): every rule fires on the right
//! observations, severities are ordered, and actions are attached only
//! where the engine can apply them.

use super::*;
use crate::models::performance::{
    CacheUsage, CpuUsage, DbUsage, MemoryUsage, OptimizationAction, ProfileAggregate,
    ProfileCategory, ProfileSample, StartupStage, ThreadUsage, WorkerInfo,
};

fn diagnostics() -> DiagnosticsSnapshot {
    DiagnosticsSnapshot {
        captured_at: chrono::Utc::now(),
        cpu: CpuUsage {
            usage_percent: 12.0,
            cores: 8,
            cpu_parallelism: 8,
        },
        memory: MemoryUsage {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 8 * 1024 * 1024 * 1024,
            percent: 50.0,
        },
        db: DbUsage {
            size_bytes: 1_000_000,
            path: "test.db".into(),
        },
        cache: CacheUsage {
            runtime_entries: 4,
            runtime_hit_rate: 0.9,
            graph_cache_entries: 0,
            graph_cache_size_bytes: 0,
        },
        workers: vec![],
        threads: ThreadUsage {
            total_threads: 64,
            process_count: 12,
        },
    }
}

fn empty_profile() -> ProfileSnapshot {
    let sample = ProfileSample {
        id: 0,
        category: ProfileCategory::Command,
        name: "performance_profile".into(),
        duration_ms: 1,
        metadata: serde_json::Value::Null,
        occurred_at: chrono::Utc::now(),
    };
    ProfileSnapshot {
        captured_at: chrono::Utc::now(),
        aggregates: vec![],
        recent: vec![sample],
        slowest: vec![],
    }
}

#[test]
fn slow_operations_raise_query_recommendations() {
    let mut profile = empty_profile();
    profile.aggregates.push(ProfileAggregate {
        category: ProfileCategory::Repository,
        name: "plan_reports_page".into(),
        count: 10,
        avg_ms: 900.0,
        min_ms: 10,
        max_ms: 1200,
        p95_ms: 1100.0,
    });
    let recommendations = Optimizer::analyze(&profile, &diagnostics(), None, 0);
    let query = recommendations
        .iter()
        .find(|r| r.category == OptimizationCategory::Query)
        .expect("query recommendation");
    assert_eq!(query.severity, "warning");
    assert!(query.detail.contains("900"));
}

#[test]
fn heavy_startup_stages_flag_lazy_initialization() {
    let profile = empty_profile();
    let startup = StartupProfile {
        run_id: uuid::Uuid::new_v4(),
        total_ms: 2500,
        stages: vec![StartupStage {
            name: "graph_sync".into(),
            label: "Knowledge graph sync".into(),
            duration_ms: 800,
            started_at: chrono::Utc::now(),
        }],
        recorded_at: chrono::Utc::now(),
    };
    let recommendations = Optimizer::analyze(&profile, &diagnostics(), Some(&startup), 100);
    let lazy = recommendations
        .iter()
        .find(|r| r.category == OptimizationCategory::LazyInit)
        .expect("lazy-init recommendation");
    assert_eq!(lazy.severity, "warning");
    assert!(lazy.detail.contains("800"));
}

#[test]
fn cache_trim_action_is_offered_for_large_graph_cache() {
    let profile = empty_profile();
    let mut snap = diagnostics();
    snap.cache.graph_cache_entries = 6000;
    snap.cache.graph_cache_size_bytes = 80 * 1024 * 1024;
    let recommendations = Optimizer::analyze(&profile, &snap, None, 100);
    let trim = recommendations
        .iter()
        .find(|r| r.action.is_some())
        .expect("an actionable cache recommendation");
    assert_eq!(trim.category, OptimizationCategory::Cache);
}

#[test]
fn low_hit_rate_triggers_cache_warning() {
    let profile = empty_profile();
    let mut snap = diagnostics();
    snap.cache.runtime_hit_rate = 0.2;
    let recommendations = Optimizer::analyze(&profile, &snap, None, 100);
    let hit = recommendations
        .iter()
        .find(|r| r.id == "cache:hit_rate")
        .expect("hit-rate recommendation");
    assert_eq!(hit.severity, "warning");
}

#[test]
fn worker_errors_raise_worker_recommendation() {
    let profile = empty_profile();
    let mut snap = diagnostics();
    snap.workers.push(WorkerInfo {
        name: "learning_worker".into(),
        status: "healthy".into(),
        execution_count: 10,
        error_count: 3,
        avg_execution_time_ms: 12.0,
        last_execution: None,
    });
    let recommendations = Optimizer::analyze(&profile, &snap, None, 100);
    let worker = recommendations
        .iter()
        .find(|r| r.category == OptimizationCategory::Worker)
        .expect("worker recommendation");
    assert_eq!(worker.severity, "warning");
}

#[test]
fn memory_pressure_is_critical_and_ordered_first() {
    let profile = empty_profile();
    let mut snap = diagnostics();
    snap.memory.percent = 92.0;
    let recommendations = Optimizer::analyze(&profile, &snap, None, 100);
    assert_eq!(recommendations[0].id, "memory:pressure");
    assert_eq!(recommendations[0].severity, "critical");
}

#[test]
fn nothing_to_say_returns_no_recommendations() {
    let recommendations = Optimizer::analyze(&empty_profile(), &diagnostics(), None, 100);
    assert!(recommendations.is_empty());
}

#[test]
fn profile_ledger_prune_action_offered_when_history_is_large() {
    let profile = empty_profile();
    let recommendations = Optimizer::analyze(&profile, &diagnostics(), None, 12_000);
    let prune = recommendations
        .iter()
        .find(|r| r.id == "memory:profile_ledger")
        .expect("prune recommendation");
    assert_eq!(
        prune.action,
        Some(OptimizationAction::PruneProfileHistory(30))
    );
}
