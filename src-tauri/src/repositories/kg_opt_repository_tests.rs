//! KgOptRepository tests (RC-8 M4): paginated node/edge/neighbor
//! loading, the four integrity scans (including rows seeded with FK
//! enforcement off, the only way orphan/dangling rows can exist), the
//! repair helpers, issue persistence, maintenance history, benchmark
//! persistence, and the query-metrics ledger.

use super::*;
use crate::database::test_database;
use crate::models::kg::{GraphRelationshipType, GraphSource};
use crate::repositories::KgRepository;
use serde_json::json;
use sqlx::ConnectOptions;

async fn setup() -> (
    KgOptRepository,
    KgRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (
        KgOptRepository::new(pool.clone()),
        KgRepository::new(pool.clone()),
        pool,
        temp_dir,
    )
}

fn source(id: Uuid, title: &str) -> GraphSource {
    GraphSource {
        entity_id: id,
        title: title.into(),
        workspace_id: None,
        summary: Some("test summary".into()),
        metadata: json!({}),
    }
}

async fn seed_nodes(kg: &KgRepository, count: usize) -> Vec<Uuid> {
    let mut ids = Vec::new();
    for i in 0..count {
        let id = Uuid::new_v4();
        kg.upsert_node(
            GraphNodeType::MemoryRecord,
            &source(id, &format!("goal {i}")),
        )
        .await
        .unwrap();
        ids.push(id);
    }
    ids
}

#[tokio::test]
async fn nodes_page_paginates_and_reports_has_more() {
    let (opt, kg, _pool, _guard) = setup().await;
    let ids = seed_nodes(&kg, 5).await;

    let first = opt.nodes_page(None, None, 0, 2).await.unwrap();
    assert_eq!(first.total, 5);
    assert_eq!(first.nodes.len(), 2);
    assert!(first.has_more);
    assert_eq!(first.nodes[0].entity_id, ids[4], "newest first");

    let last = opt.nodes_page(None, None, 4, 2).await.unwrap();
    assert_eq!(last.nodes.len(), 1);
    assert!(!last.has_more);

    let filtered = opt
        .nodes_page(Some(&[GraphNodeType::Workspace]), None, 0, 50)
        .await
        .unwrap();
    assert_eq!(filtered.total, 0);

    assert_eq!(
        opt.nodes_page_count(Some(&[GraphNodeType::MemoryRecord]), None)
            .await
            .unwrap(),
        5
    );
}

#[tokio::test]
async fn edges_and_neighbors_page_over_one_relationship() {
    let (opt, kg, _pool, _guard) = setup().await;
    let ws = Uuid::new_v4();
    let file = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::Workspace, &source(ws, "Alpha WS"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::File, &source(file, "alpha.rs"))
        .await
        .unwrap();
    kg.upsert_relationship(
        GraphNodeType::Workspace,
        ws,
        GraphNodeType::File,
        file,
        GraphRelationshipType::Contains,
        1.0,
        json!({}),
    )
    .await
    .unwrap();

    let edges = opt.edges_page(0, 50).await.unwrap();
    assert_eq!(edges.total, 1);
    assert_eq!(edges.edges.len(), 1);
    assert!(!edges.has_more);
    assert_eq!(opt.edges_page_count().await.unwrap(), 1);

    let neighbors = opt
        .neighbors_page(GraphNodeType::Workspace, ws, 0, 50)
        .await
        .unwrap();
    assert_eq!(neighbors.total, 1);
    assert_eq!(neighbors.neighbors.len(), 1);
    assert_eq!(neighbors.neighbors[0].neighbor.entity_id, file);
    assert_eq!(
        neighbors.neighbors[0].edge.relationship_type,
        GraphRelationshipType::Contains
    );
}

#[tokio::test]
async fn orphan_edges_scan_and_delete_helper() {
    let (opt, kg, _pool, _guard) = setup().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(a, "alpha"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(b, "beta"))
        .await
        .unwrap();

    // The edge cannot reference a missing node while FKs are on — that
    // state can only exist in legacy/corrupt data, simulated by seeding
    // through a connection with FK enforcement off.
    let orphan = Uuid::new_v4();
    let mut conn = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(_guard.path().join("test.db"))
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
    .bind(orphan)
    .bind(a)
    .bind(Uuid::new_v4())
    .execute(&mut conn)
    .await
    .unwrap();
    drop(conn);

    let found = opt.orphan_edge_ids().await.unwrap();
    assert_eq!(found, vec![orphan]);

    let removed = opt.delete_edges(&found).await.unwrap();
    assert_eq!(removed, 1);
    assert!(opt.orphan_edge_ids().await.unwrap().is_empty());
}

#[tokio::test]
async fn dangling_workspace_scan_and_delete_helper() {
    let (opt, kg, _pool, _guard) = setup().await;
    let valid = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::Workspace, &source(valid, "Kept WS"))
        .await
        .unwrap();

    let dangling = Uuid::new_v4();
    let mut conn = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(_guard.path().join("test.db"))
        .foreign_keys(false)
        .connect()
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO graph_nodes
             (node_type, entity_id, title, workspace_id, summary, metadata, created_at, updated_at)
         VALUES ('file', ?, 'ghost.rs', ?, 'gone', '{}', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
    )
    .bind(dangling)
    .bind(Uuid::new_v4())
    .execute(&mut conn)
    .await
    .unwrap();
    drop(conn);

    let found = opt.dangling_workspace_nodes().await.unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].entity_id, dangling);

    assert!(opt
        .delete_node(found[0].node_type.as_str(), found[0].entity_id)
        .await
        .unwrap());
    assert!(opt.dangling_workspace_nodes().await.unwrap().is_empty());
}

#[tokio::test]
async fn malformed_node_scan_finds_empty_title_and_repair_fixes_it() {
    let (opt, kg, _pool, _guard) = setup().await;
    let good = Uuid::new_v4();
    let bad = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(good, "fine goal"))
        .await
        .unwrap();
    kg.upsert_node(
        GraphNodeType::MemoryRecord,
        &GraphSource {
            entity_id: bad,
            title: "".into(),
            workspace_id: None,
            summary: Some("summary only".into()),
            metadata: json!({}),
        },
    )
    .await
    .unwrap();

    let malformed = opt.malformed_nodes().await.unwrap();
    assert_eq!(malformed.len(), 1);
    assert_eq!(malformed[0].1, bad);

    assert!(opt
        .fix_malformed_node(GraphNodeType::MemoryRecord, bad)
        .await
        .unwrap());
    assert!(opt.malformed_nodes().await.unwrap().is_empty());

    let (title,): (String,) = sqlx::query_as("SELECT title FROM graph_nodes WHERE entity_id = ?")
        .bind(bad)
        .fetch_one(&opt.pool)
        .await
        .unwrap();
    assert_eq!(title, "(untitled)");
}

#[tokio::test]
async fn invalid_confidence_scan_is_empty_and_clamp_is_harmless() {
    let (opt, kg, _pool, _guard) = setup().await;
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

    // Post-migration CHECKs make out-of-range rows impossible to
    // insert; the scan is a legacy/corruption safety net.
    assert!(opt.invalid_confidence_edges().await.unwrap().is_empty());

    let (edge_id,): (Uuid,) = sqlx::query_as("SELECT id FROM graph_relationships LIMIT 1")
        .fetch_one(&opt.pool)
        .await
        .unwrap();
    assert!(opt.clamp_edge_values(edge_id).await.unwrap());
}

#[tokio::test]
async fn issue_persistence_round_trip_and_resolution() {
    let (opt, _kg, _pool, _guard) = setup().await;
    let entity = Uuid::new_v4();
    let id = opt
        .insert_issue(
            IssueType::OrphanEdge,
            IssueSeverity::Critical,
            None,
            Some(entity),
            "missing endpoint".into(),
        )
        .await
        .unwrap();

    assert_eq!(opt.open_issue_count().await.unwrap(), 1);
    let issues = opt.open_issues(10).await.unwrap();
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].issue_type, IssueType::OrphanEdge);
    assert_eq!(issues[0].severity, IssueSeverity::Critical);
    assert_eq!(issues[0].entity_id, Some(entity));
    assert_eq!(issues[0].status, "open");

    let counts = opt.open_issue_type_counts().await.unwrap();
    assert_eq!(counts.len(), 1);
    assert_eq!(counts[0].name, "orphan_edge");
    assert_eq!(counts[0].count, 1);

    assert_eq!(
        opt.resolve_issues(IssueType::OrphanEdge, &[entity])
            .await
            .unwrap(),
        1
    );
    assert_eq!(opt.open_issue_count().await.unwrap(), 0);
    let resolved = opt.recent_issues(10).await.unwrap();
    assert_eq!(resolved[0].status, "resolved");
    assert!(resolved[0].resolved_at.is_some());

    let id2 = opt
        .insert_issue(
            IssueType::MalformedNode,
            IssueSeverity::Warning,
            Some("memory_record"),
            Some(Uuid::new_v4()),
            "empty title".into(),
        )
        .await
        .unwrap();
    assert!(opt.mark_issues_resolved(&[id, id2]).await.unwrap() >= 1);
}

#[tokio::test]
async fn maintenance_runs_round_trip() {
    let (opt, _kg, _pool, _guard) = setup().await;
    opt.insert_maintenance_run(
        "integrity_check",
        "completed",
        3,
        0,
        12,
        json!({ "orphan_edges": 3 }),
    )
    .await
    .unwrap();

    let runs = opt.recent_maintenance_runs(10).await.unwrap();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].run_type, "integrity_check");
    assert_eq!(runs[0].issues_found, 3);
    assert_eq!(runs[0].duration_ms, 12);
    assert_eq!(runs[0].summary["orphan_edges"], 3);
    assert!(runs[0].finished_at.is_some());
}

#[tokio::test]
async fn benchmarks_and_metrics_round_trip() {
    let (opt, _kg, _pool, _guard) = setup().await;
    opt.insert_benchmark(
        "suite_1",
        "nodes_page_50",
        "paginate_nodes",
        10,
        5,
        8,
        json!({ "rows": 10 }),
    )
    .await
    .unwrap();
    let benchmarks = opt.recent_benchmarks(10).await.unwrap();
    assert_eq!(benchmarks.len(), 1);
    assert_eq!(benchmarks[0].name, "nodes_page_50");
    assert_eq!(benchmarks[0].node_count, 10);
    assert_eq!(benchmarks[0].duration_ms, 8);

    opt.insert_query_metric("ranked_search", Some("all"), Some("alpha"), 4, 3, false)
        .await
        .unwrap();
    opt.insert_query_metric("paginate_nodes", None, None, 2, 50, true)
        .await
        .unwrap();
    let metrics = opt.recent_query_metrics(10).await.unwrap();
    assert_eq!(metrics.len(), 2);
    assert_eq!(metrics[0].operation, "paginate_nodes");
    assert!(metrics[0].hit_cache);
    assert_eq!(metrics[1].operation, "ranked_search");
    assert!(!metrics[1].hit_cache);
    assert_eq!(metrics[1].query.as_deref(), Some("alpha"));
}
