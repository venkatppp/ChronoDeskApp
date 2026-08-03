//! Tests for the RC-8 M4 graph health service against a real migrated
//! database: integrity scans + issue persistence, repair, orphan
//! summary/cleanup, consistency verification, maintenance history,
//! the benchmark suite, and the combined diagnostics bundle.

use uuid::Uuid;

use crate::database::test_database;
use crate::models::kg::{GraphNodeType, GraphRelationshipType, GraphSource};
use crate::models::kg_opt::IssueType;
use crate::repositories::{KgLiveRepository, KgOptRepository, KgRepository};
use crate::services::{GraphHealthService, KgOptService, KgService};
use serde_json::json;
use sqlx::ConnectOptions;

async fn setup() -> (
    GraphHealthService,
    KgRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let kg_service = KgService::new(KgRepository::new(pool.clone()));
    let kg_opt_service = KgOptService::new(
        kg_service,
        KgOptRepository::new(pool.clone()),
        KgLiveRepository::new(pool.clone()),
    );
    let health = GraphHealthService::new(
        kg_opt_service,
        KgLiveRepository::new(pool.clone()),
        KgOptRepository::new(pool.clone()),
    );
    (health, KgRepository::new(pool.clone()), pool, temp_dir)
}

fn source(id: Uuid, title: &str) -> GraphSource {
    GraphSource {
        entity_id: id,
        title: title.into(),
        workspace_id: None,
        summary: Some("summary".into()),
        metadata: json!({}),
    }
}

async fn seed_pair(kg: &KgRepository) -> (Uuid, Uuid) {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(a, "alpha"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(b, "beta"))
        .await
        .unwrap();
    kg.upsert_relationship(
        GraphNodeType::MemoryRecord,
        a,
        GraphNodeType::MemoryRecord,
        b,
        GraphRelationshipType::RelatedTo,
        0.8,
        json!({}),
    )
    .await
    .unwrap();
    (a, b)
}

/// Seeds a legacy orphan edge/dangling node through a connection with
/// foreign keys off (the only way those rows can exist).
async fn seed_legacy_problems(kg: &KgRepository, guard: &tempfile::TempDir) {
    let a = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(a, "kept"))
        .await
        .unwrap();

    let mut conn = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(guard.path().join("test.db"))
        .foreign_keys(false)
        .connect()
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO graph_relationships
             (id, source_node_type, source_entity_id, target_node_type, target_entity_id,
              relationship_type, weight, metadata, created_at, updated_at)
         VALUES (?, 'memory_record', ?, 'memory_record', ?, 'related_to', 0.5, '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .bind(Uuid::new_v4())
    .bind(a)
    .bind(Uuid::new_v4())
    .execute(&mut conn)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO graph_nodes
             (node_type, entity_id, title, workspace_id, summary, metadata, created_at, updated_at)
         VALUES ('file', ?, 'ghost.rs', ?, 'gone', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(&mut conn)
    .await
    .unwrap();
    drop(conn);
}

#[tokio::test]
async fn integrity_check_persists_and_deduplicates_findings() {
    let (health, kg, _pool, guard) = setup().await;
    seed_pair(&kg).await;
    let clean = health.integrity_check().await.unwrap();
    assert!(clean.issues.is_empty());

    seed_legacy_problems(&kg, &guard).await;
    let first = health.integrity_check().await.unwrap();
    assert!(
        first
            .issues
            .iter()
            .any(|issue| issue.issue_type == IssueType::OrphanEdge),
        "orphan edge finding expected"
    );
    assert!(
        first
            .issues
            .iter()
            .any(|issue| issue.issue_type == IssueType::DanglingWorkspace),
        "dangling workspace finding expected"
    );
    let total: i64 = first.issue_type_counts.iter().map(|t| t.count).sum();
    assert_eq!(total, 2);

    // A second pass must not duplicate the findings.
    let second = health.integrity_check().await.unwrap();
    let total2: i64 = second.issue_type_counts.iter().map(|t| t.count).sum();
    assert_eq!(total2, 2);

    let runs = health.recent_maintenance_runs(10).await.unwrap();
    assert_eq!(runs.len(), 3, "clean pass + two scan passes");
    assert!(runs.iter().all(|run| run.run_type == "integrity_check"));
    assert!(runs.iter().all(|run| run.run_type == "integrity_check"));
}

#[tokio::test]
async fn repair_removes_problems_and_resolves_issues() {
    let (health, kg, _pool, guard) = setup().await;
    seed_pair(&kg).await;
    seed_legacy_problems(&kg, &guard).await;
    let check = health.integrity_check().await.unwrap();
    assert!(!check.issues.is_empty());

    let repaired = health.repair().await.unwrap();
    assert_eq!(repaired.orphan_edges_removed, 1);
    assert_eq!(repaired.dangling_workspaces_removed, 1);
    assert!(repaired.issues_resolved >= 2);

    let after = health.integrity_check().await.unwrap();
    assert!(
        after.issues.is_empty(),
        "no findings should remain after repair"
    );
}

#[tokio::test]
async fn orphan_summary_and_cleanup_round_trip() {
    let (health, kg, _pool, guard) = setup().await;
    seed_pair(&kg).await;
    seed_legacy_problems(&kg, &guard).await;
    let check = health.integrity_check().await.unwrap();
    assert!(!check.issues.is_empty());

    let summary = health.orphan_summary().await.unwrap();
    assert_eq!(summary.orphan_edges, 1);
    assert_eq!(summary.dangling_workspaces, 1);

    let cleaned = health.orphan_cleanup().await.unwrap();
    assert_eq!(cleaned.orphan_edges_removed, 1);
    assert_eq!(cleaned.dangling_workspaces_removed, 1);
    assert!(cleaned.issues_resolved >= 2);

    let after = health.orphan_summary().await.unwrap();
    assert_eq!(after.orphan_edges, 0);
    assert_eq!(after.dangling_workspaces, 0);
}

#[tokio::test]
async fn consistency_report_passes_clean_graph_and_fails_corrupt_one() {
    let (health, kg, _pool, _guard) = setup().await;
    seed_pair(&kg).await;
    let clean = health.consistency_report().await.unwrap();
    assert!(clean.passed);
    assert_eq!(clean.checks.len(), 5);

    // Simulate a legacy orphan: verify the forward-reference check flips.
    seed_legacy_problems(&kg, &_guard).await;
    let dirty = health.consistency_report().await.unwrap();
    assert!(!dirty.passed);
    assert!(dirty
        .checks
        .iter()
        .any(|check| !check.passed && check.name == "Forward references"));
}

#[tokio::test]
async fn benchmark_suite_runs_and_persists_results() {
    let (health, _kg, _pool, _guard) = setup().await;
    let suite = health
        .benchmark_suite(Some("m4_test_suite".into()))
        .await
        .unwrap();
    assert_eq!(suite.suite_name, "m4_test_suite");
    assert!(
        !suite.benchmarks.is_empty(),
        "suite should run benchmarks even on an empty graph"
    );
    assert!(suite.total_duration_ms > 0);
    assert!(suite.benchmarks.iter().all(|b| !b.name.is_empty()));

    // Re-read through the ledger (exercises persistence).
    let recent = health.recent_metrics(20).await.unwrap();
    assert!(
        recent.iter().any(|m| m.operation == "paginate_nodes"),
        "benchmark operations record metrics"
    );
    let runs = health.recent_maintenance_runs(10).await.unwrap();
    assert!(runs.iter().any(|run| run.run_type == "benchmark"));
}

async fn setup_health() -> (GraphHealthService, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let kg_service = KgService::new(KgRepository::new(pool.clone()));
    let kg_opt_service = KgOptService::new(
        kg_service,
        KgOptRepository::new(pool.clone()),
        KgLiveRepository::new(pool.clone()),
    );
    let health = GraphHealthService::new(
        kg_opt_service,
        KgLiveRepository::new(pool.clone()),
        KgOptRepository::new(pool.clone()),
    );
    (health, pool, temp_dir)
}

#[tokio::test]
async fn diagnostics_bundle_aggregates_every_ledger() {
    let (health, _pool, _guard) = setup_health().await;
    let diagnostics = health.diagnostics().await.unwrap();
    assert_eq!(diagnostics.consistency.checks.len(), 5);
    assert_eq!(diagnostics.memory.node_count, 0);
    assert!(
        diagnostics
            .recent_maintenance
            .iter()
            .any(|run| run.run_type == "integrity_check"),
        "diagnostics runs an integrity pass"
    );
}
