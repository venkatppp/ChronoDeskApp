//! KgLiveRepository tests (RC-8 M2): semantic edge upserts with
//! confidence, confidence decay, low-confidence pruning, the persisted
//! query cache, and the analytics fetches.

use super::*;
use crate::database::test_database;
use crate::models::kg::GraphSource;
use crate::repositories::KgRepository;
use serde_json::json;

async fn setup() -> (
    KgLiveRepository,
    KgRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (
        KgLiveRepository::new(pool.clone()),
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

async fn seed_pair(live: &KgLiveRepository, kg: &KgRepository, a: Uuid, b: Uuid) -> Uuid {
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(a, "alpha goal"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(b, "beta goal"))
        .await
        .unwrap();
    live.upsert_semantic_relationship(
        GraphNodeType::MemoryRecord,
        a,
        GraphNodeType::MemoryRecord,
        b,
        0.8,
        0.8,
        json!({ "source": "semantic", "similarity": 0.8 }),
    )
    .await
    .unwrap();
    // The edge's id (stored as BLOB by sqlx's Uuid encoding), so tests
    // can bind it back through the same Uuid path.
    let (edge_id,): (Uuid,) =
        sqlx::query_as("SELECT id FROM graph_relationships ORDER BY rowid DESC LIMIT 1")
            .fetch_one(&live.pool)
            .await
            .unwrap();
    edge_id
}

#[tokio::test]
async fn semantic_upsert_stores_confidence_and_refreshes_it() {
    let (live, kg, _pool, _guard) = setup().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    let created = live
        .upsert_semantic_relationship(
            GraphNodeType::MemoryRecord,
            a,
            GraphNodeType::MemoryRecord,
            b,
            0.9,
            0.9,
            json!({ "source": "semantic" }),
        )
        .await;
    assert!(
        matches!(created, Err(DatabaseError::Constraint(_))),
        "edge without endpoint nodes fails the FK"
    );

    kg.upsert_node(GraphNodeType::MemoryRecord, &source(a, "alpha"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(b, "beta"))
        .await
        .unwrap();

    let created = live
        .upsert_semantic_relationship(
            GraphNodeType::MemoryRecord,
            a,
            GraphNodeType::MemoryRecord,
            b,
            0.9,
            0.9,
            json!({ "source": "semantic" }),
        )
        .await
        .unwrap();
    assert!(created, "first semantic edge inserts");

    let refreshed = live
        .upsert_semantic_relationship(
            GraphNodeType::MemoryRecord,
            a,
            GraphNodeType::MemoryRecord,
            b,
            0.95,
            0.95,
            json!({ "source": "semantic", "similarity": 0.95 }),
        )
        .await
        .unwrap();
    assert!(!refreshed, "second upsert refreshes in place");

    let edges = live.all_edges().await.unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].confidence, 0.95);
    assert_eq!(edges[0].weight, 0.95);
}

#[tokio::test]
async fn low_confidence_edges_are_pruned() {
    let (live, kg, _pool, _guard) = setup().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    seed_pair(&live, &kg, a, b).await;
    let edge_id = seed_pair(&live, &kg, a, c).await;

    // Drop the second edge's confidence below the floor.
    sqlx::query("UPDATE graph_relationships SET confidence = 0.05 WHERE id = ?")
        .bind(edge_id)
        .execute(&live.pool)
        .await
        .unwrap();

    let pruned = live.prune_low_confidence_edges(0.10).await.unwrap();
    assert_eq!(pruned, 1);
    assert_eq!(live.all_edges().await.unwrap().len(), 1);
}

#[tokio::test]
async fn decay_candidates_select_aged_semantic_edges_and_writes_back() {
    let (live, kg, _pool, _guard) = setup().await;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let edge_id = seed_pair(&live, &kg, a, b).await;

    // Backdate the semantic edge by 20 days so decay applies.
    let old = chrono::DateTime::parse_from_rfc3339("2026-07-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    sqlx::query("UPDATE graph_relationships SET updated_at = ?, confidence = 1.0 WHERE id = ?")
        .bind(old.to_rfc3339())
        .bind(edge_id)
        .execute(&live.pool)
        .await
        .unwrap();

    let now = chrono::DateTime::parse_from_rfc3339("2026-07-21T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let candidates = live.decay_candidates(now, 0.5).await.unwrap();
    assert_eq!(candidates.len(), 1);
    assert!((candidates[0].age_days - 20.0).abs() < 0.01);
    assert_eq!(
        candidates[0].confidence, 1.0,
        "semantic edges at 1.0 still decay"
    );

    let expected = 1.0_f64 * 0.92_f64.powi(20);
    live.update_edge_confidence(edge_id, expected, now)
        .await
        .unwrap();
    let edges = live.all_edges().await.unwrap();
    assert!(
        (edges[0].confidence - expected).abs() < 0.01,
        "confidence should be ~ {expected}, got {}",
        edges[0].confidence
    );

    // A second pass shortly after leaves the edge alone (age < 0.5 day).
    let later = now + chrono::Duration::hours(6);
    assert!(live.decay_candidates(later, 0.5).await.unwrap().is_empty());

    // A fresh structural edge (confidence 1.0) is never a decay candidate.
    let c = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(c, "gamma"))
        .await
        .unwrap();
    kg.upsert_relationship(
        GraphNodeType::MemoryRecord,
        c,
        GraphNodeType::MemoryRecord,
        a,
        GraphRelationshipType::DerivedFrom,
        1.0,
        json!({ "source": "structural" }),
    )
    .await
    .unwrap();
    assert!(
        live.decay_candidates(now, 0.5).await.unwrap().is_empty(),
        "structural edges are exempt from decay"
    );
}

#[tokio::test]
async fn query_cache_round_trips_with_ttl_and_clears() {
    let (live, _kg, _pool, _guard) = setup().await;
    assert!(live.query_cache_get("a").await.unwrap().is_none());

    live.query_cache_put("a", r#"{"nodeCount":3}"#, 60)
        .await
        .unwrap();
    live.query_cache_put("b", r#"{"nodeCount":9}"#, 120)
        .await
        .unwrap();
    assert_eq!(live.query_cache_count().await.unwrap(), 2);

    let (created_at, payload, ttl) = live.query_cache_get("a").await.unwrap().unwrap();
    assert_eq!(payload, r#"{"nodeCount":3}"#);
    assert_eq!(ttl, 60);
    assert!(created_at <= Utc::now());

    // Upserting the same key overwrites in place (no dupes).
    live.query_cache_put("a", r#"{"nodeCount":4}"#, 30)
        .await
        .unwrap();
    assert_eq!(live.query_cache_count().await.unwrap(), 2);

    let cleared = live.query_cache_clear().await.unwrap();
    assert_eq!(cleared, 2);
    assert_eq!(live.query_cache_count().await.unwrap(), 0);
}

#[tokio::test]
async fn node_fetches_respect_workspace_scope() {
    let (live, kg, _pool, _guard) = setup().await;
    let ws = Uuid::new_v4();
    let one = Uuid::new_v4();
    let two = Uuid::new_v4();

    // The workspace node carries its own id as workspace_id, so the row
    // must exist in `workspaces` for the graph FK to resolve.
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO workspaces
             (id, name, description, status, health_score, last_active_at, created_at, updated_at)
         VALUES (?, 'WS', NULL, 'active', 80.0, ?, ?, ?)",
    )
    .bind(ws)
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(&live.pool)
    .await
    .unwrap();

    kg.upsert_node(
        GraphNodeType::Workspace,
        &GraphSource {
            entity_id: ws,
            title: "WS".into(),
            workspace_id: Some(ws),
            summary: None,
            metadata: json!({}),
        },
    )
    .await
    .unwrap();
    kg.upsert_node(
        GraphNodeType::File,
        &GraphSource {
            entity_id: one,
            title: "a.rs".into(),
            workspace_id: Some(ws),
            summary: None,
            metadata: json!({}),
        },
    )
    .await
    .unwrap();
    kg.upsert_node(
        GraphNodeType::MemoryRecord,
        &GraphSource {
            entity_id: two,
            title: "a goal".into(),
            workspace_id: None,
            summary: None,
            metadata: json!({}),
        },
    )
    .await
    .unwrap();

    let scoped = live.all_nodes(Some(ws)).await.unwrap();
    assert_eq!(scoped.len(), 2);
    assert!(scoped.iter().all(|n| n.workspace_id == Some(ws)));

    let all = live.all_nodes(None).await.unwrap();
    assert_eq!(all.len(), 3);
}
