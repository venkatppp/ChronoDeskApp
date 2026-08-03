//! BenchmarkEngine tests (RC-10 M1): the harness measures, persists,
//! and reports; every subsystem suite runs against a disposable test
//! database with the real (lightweight) engine stack.

use std::sync::Arc;

use super::*;
use crate::copilot::execution_engine::ExecutionEngine;
use crate::copilot::memory::vector::LocalVectorProvider;
use crate::copilot::memory::MemoryEngine;
use crate::copilot::planner::Planner;
use crate::copilot::tools::ToolExecutor;
use crate::copilot::MemoryRepository;
use crate::database::test_database;
use crate::graph::GraphEngine;
use crate::repositories::{
    FileRepository, GraphRepository, KgRepository, TimelineRepository, WorkspaceRepository,
};
use crate::semantic::embeddings::LocalEmbeddingProvider;
use crate::semantic::repository::SemanticRepository;
use crate::semantic::{SemanticMemoryEngine, SemanticSearchEngine};
use crate::services::{GraphService, KgService, TimelineService, WorkspaceService};
use crate::session::SessionEngine;
use crate::timeline::recorder::TimelineRecorder;
use crate::timeline::TimelineEngine;

async fn setup() -> (BenchmarkEngine, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();

    let workspace_repository = WorkspaceRepository::new(pool.clone());
    let file_repository = FileRepository::new(pool.clone());
    let timeline_repository = TimelineRepository::new(pool.clone());

    let workspace_service =
        WorkspaceService::new(workspace_repository.clone(), timeline_repository.clone());
    let timeline_recorder =
        TimelineRecorder::new(file_repository.clone(), timeline_repository.clone());
    let timeline_service = TimelineService::new(timeline_recorder, timeline_repository.clone());
    let timeline_engine = TimelineEngine::new(timeline_service);
    let session_engine = SessionEngine::new(timeline_repository.clone(), file_repository.clone());
    let tool_executor = Arc::new(ToolExecutor::new(
        Arc::new(workspace_service),
        Arc::new(session_engine),
        Arc::new(timeline_engine),
    ));

    let planner = Arc::new(Planner::new(tool_executor.clone(), None));
    let execution_engine = Arc::new(ExecutionEngine::new(
        Arc::new(crate::copilot::ExecutionRepository::new(pool.clone())),
        tool_executor,
    ));

    let memory_engine = Arc::new(MemoryEngine::new(
        MemoryRepository::new(pool.clone()),
        Arc::new(LocalVectorProvider::default()),
    ));

    let graph_service = GraphService::new(GraphRepository::new(pool.clone()));
    let graph_engine = GraphEngine::new(graph_service)
        .with_kg_service(KgService::new(KgRepository::new(pool.clone())));

    let semantic_repository = SemanticRepository::new(pool.clone());
    semantic_repository.initialize().await.unwrap();
    let semantic_memory = SemanticMemoryEngine::new(
        semantic_repository.clone(),
        Arc::new(LocalEmbeddingProvider::default()),
    );
    let semantic_search = SemanticSearchEngine::new(semantic_memory, semantic_repository);

    let benchmark = BenchmarkEngine::new(
        PerformanceRepository::new(pool.clone()),
        Some(planner),
        Some(execution_engine),
        Some(memory_engine),
        Some(graph_engine),
        Some(semantic_search),
    );
    (benchmark, pool, temp_dir)
}

#[tokio::test]
async fn planner_suite_measures_and_persists() {
    let (benchmark, _pool, _guard) = setup().await;
    let result = benchmark
        .run(Some(BenchmarkCategory::Planner))
        .await
        .unwrap();
    assert_eq!(result.suite_name, "planner");
    assert!(!result.benchmarks.is_empty());
    assert!(
        result.benchmarks.iter().all(|b| b.ok),
        "plan should build a deterministic DAG"
    );
    let recent = benchmark.recent(10).await.unwrap();
    assert!(recent.iter().any(|b| b.name == "planner_plan"));
}

#[tokio::test]
async fn all_suites_run_and_persist_history() {
    let (benchmark, _pool, _guard) = setup().await;
    let result = benchmark.run(None).await.unwrap();
    let categories: Vec<String> = result
        .benchmarks
        .iter()
        .map(|b| b.category.as_str().to_string())
        .collect();
    for expected in ["planner", "execution", "memory", "graph", "vector"] {
        assert!(
            categories.contains(&expected.to_string()),
            "missing {expected} benchmark"
        );
    }
    let recent = benchmark.recent(50).await.unwrap();
    assert!(recent.len() >= result.benchmarks.len());
}

#[tokio::test]
async fn unconfigured_subsystem_reports_skipped_not_crash() {
    let (database, _guard) = test_database().await;
    let empty = BenchmarkEngine::new(
        PerformanceRepository::new(database.pool().clone()),
        None,
        None,
        None,
        None,
        None,
    );
    let result = empty.run(Some(BenchmarkCategory::Memory)).await.unwrap();
    assert_eq!(result.benchmarks.len(), 1);
    assert!(!result.benchmarks[0].ok);
    let _ = _guard;
}

#[tokio::test]
async fn graph_suite_exercises_all_graph_operations() {
    let (benchmark, _pool, _guard) = setup().await;
    let result = benchmark.run(Some(BenchmarkCategory::Graph)).await.unwrap();
    let names: Vec<&str> = result.benchmarks.iter().map(|b| b.name.as_str()).collect();
    for expected in [
        "graph_nodes_page",
        "graph_nodes_total",
        "graph_ranked_search",
        "graph_memory_stats",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}
