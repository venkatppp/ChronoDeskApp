//! Tests for the RC-8 M4 knowledge graph optimization service against a
//! real migrated database: paginated loading, ranked search, vector
//! search with a deterministic fake embedder, parallel multi-root
//! traversal (rayon), cache trimming/expiry, and memory statistics.

use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::database::test_database;
use crate::models::kg::{GraphNodeType, GraphRelationshipType, GraphSource};
use crate::models::kg_live::GraphEmbedder;
use crate::repositories::{KgLiveRepository, KgOptRepository, KgRepository};
use crate::services::{KgOptService, KgService};
use serde_json::json;

/// Deterministic fake embedder: text mentioning "alpha" maps to a
/// shared vector, "beta" to an orthogonal one, anything else to the
/// "generic" vector — so alpha queries rank alpha titles first.
struct FakeEmbedder;

#[async_trait]
impl GraphEmbedder for FakeEmbedder {
    async fn embed(&self, text: &str) -> Option<Vec<f32>> {
        let lower = text.to_lowercase();
        if lower.contains("alpha") {
            Some(vec![1.0, 0.0])
        } else if lower.contains("beta") {
            Some(vec![0.0, 1.0])
        } else {
            Some(vec![1.0, 1.0])
        }
    }
}

async fn setup() -> (
    KgOptService,
    KgRepository,
    sqlx::SqlitePool,
    tempfile::TempDir,
) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let kg_service = KgService::new(KgRepository::new(pool.clone()));
    let service = KgOptService::new(
        kg_service,
        KgOptRepository::new(pool.clone()),
        KgLiveRepository::new(pool.clone()),
    )
    .with_embedder(Arc::new(FakeEmbedder));
    (service, KgRepository::new(pool.clone()), pool, temp_dir)
}

fn source(id: Uuid, title: &str) -> GraphSource {
    GraphSource {
        entity_id: id,
        title: title.into(),
        workspace_id: None,
        summary: Some("shared summary".into()),
        metadata: json!({}),
    }
}

async fn seed_alpha_beta(kg: &KgRepository) -> (Uuid, Uuid) {
    let alpha = Uuid::new_v4();
    let beta = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(alpha, "alpha feature"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(beta, "beta feature"))
        .await
        .unwrap();
    (alpha, beta)
}

#[tokio::test]
async fn pagination_lists_nodes_edges_and_neighbors() {
    let (service, kg, _pool, _guard) = setup().await;
    let (alpha, beta) = seed_alpha_beta(&kg).await;
    kg.upsert_relationship(
        GraphNodeType::MemoryRecord,
        alpha,
        GraphNodeType::MemoryRecord,
        beta,
        GraphRelationshipType::RelatedTo,
        0.9,
        json!({}),
    )
    .await
    .unwrap();

    let page = service.nodes_page(None, None, 0, Some(1)).await.unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.nodes.len(), 1);
    assert!(page.has_more);
    assert_eq!(service.nodes_total(None, None).await.unwrap(), 2);

    let edges = service.edges_page(0, Some(10)).await.unwrap();
    assert_eq!(edges.total, 1);

    let neighbors = service
        .neighbors_page(GraphNodeType::MemoryRecord, alpha, 0, Some(10))
        .await
        .unwrap();
    assert_eq!(neighbors.total, 1);
    assert_eq!(neighbors.neighbors[0].neighbor.entity_id, beta);
}

#[tokio::test]
async fn ranked_search_scores_title_prefix_above_contains() {
    let (service, kg, _pool, _guard) = setup().await;
    let exact = Uuid::new_v4();
    let partial = Uuid::new_v4();
    let other = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::File, &source(exact, "login gate"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::File, &source(partial, "gatekeeper notes"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::File, &source(other, "unrelated file"))
        .await
        .unwrap();

    let hits = service.ranked_search("gate", None, Some(10)).await.unwrap();
    assert_eq!(hits.len(), 2);
    assert_eq!(
        hits[0].node.entity_id, partial,
        "title-prefix match ranks first"
    );
    assert_eq!(hits[0].method, "keyword");
    assert!(hits[0].score > hits[1].score);

    let scoped = service
        .ranked_search("gate", Some(vec![GraphNodeType::Workspace]), Some(10))
        .await
        .unwrap();
    assert!(scoped.is_empty());
}

#[tokio::test]
async fn vector_search_ranks_semantically_similar_titles() {
    let (service, kg, _pool, _guard) = setup().await;
    seed_alpha_beta(&kg).await;

    let hits = service
        .vector_search("alpha", None, Some(10))
        .await
        .unwrap();
    assert!(!hits.is_empty());
    assert_eq!(hits[0].node.title, "alpha feature");
    assert_eq!(hits[0].method, "vector");
    assert!(hits[0].score >= 0.9);
}

#[tokio::test]
async fn parallel_traversal_reaches_neighborhoods_from_multiple_roots() {
    let (service, kg, _pool, _guard) = setup().await;
    let ws = Uuid::new_v4();
    let f1 = Uuid::new_v4();
    let f2 = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::Workspace, &source(ws, "WS"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::File, &source(f1, "one.rs"))
        .await
        .unwrap();
    kg.upsert_node(GraphNodeType::File, &source(f2, "two.rs"))
        .await
        .unwrap();
    kg.upsert_relationship(
        GraphNodeType::Workspace,
        ws,
        GraphNodeType::File,
        f1,
        GraphRelationshipType::Contains,
        1.0,
        json!({}),
    )
    .await
    .unwrap();
    kg.upsert_relationship(
        GraphNodeType::Workspace,
        ws,
        GraphNodeType::File,
        f2,
        GraphRelationshipType::Contains,
        1.0,
        json!({}),
    )
    .await
    .unwrap();

    let walk = service
        .parallel_traversal(vec![(GraphNodeType::Workspace, ws)], Some(2), Some(100))
        .await
        .unwrap();
    assert_eq!(walk.roots, 1);
    assert_eq!(walk.node_count, 3, "workspace + both files");
    assert_eq!(walk.edge_count, 2);
    assert_eq!(walk.max_depth, 1);

    // Union across roots: a second root in the same component adds no
    // new nodes (deduplicated), but disjoint roots do.
    let far = Uuid::new_v4();
    kg.upsert_node(GraphNodeType::MemoryRecord, &source(far, "far memory"))
        .await
        .unwrap();
    let walk = service
        .parallel_traversal(
            vec![
                (GraphNodeType::Workspace, ws),
                (GraphNodeType::MemoryRecord, far),
            ],
            Some(1),
            Some(100),
        )
        .await
        .unwrap();
    assert_eq!(walk.node_count, 4);

    let empty = service
        .parallel_traversal(vec![], None, None)
        .await
        .unwrap();
    assert_eq!(empty.node_count, 0);
}

#[tokio::test]
async fn cache_trim_and_expiry_evict_oldest_and_expired() {
    let (service, _kg, pool, _guard) = setup().await;
    // Seed the cache directly with an old entry and a fresh one.
    sqlx::query(
        "INSERT INTO graph_query_cache (cache_key, payload, created_at, ttl_seconds)
         VALUES ('old', '{}', '2026-01-01T00:00:00Z', 60), ('fresh', '{}', ?, 60)",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let removed = service.clear_expired_cache().await.unwrap();
    assert_eq!(removed, 1, "the old entry is past its 60s TTL");

    sqlx::query(
        "INSERT INTO graph_query_cache (cache_key, payload, created_at, ttl_seconds)
         VALUES ('a', '{}', '2026-01-01T00:00:00Z', 999999), ('b', '{}', '2026-01-01T00:00:00Z', 999999)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let trimmed = service.trim_cache(1).await.unwrap();
    assert_eq!(trimmed, 1);

    let stats = service.cache_stats().await.unwrap();
    assert_eq!(stats.cached_queries, 2);
}

#[tokio::test]
async fn memory_stats_aggregate_registry_and_cache_footprint() {
    let (service, kg, _pool, _guard) = setup().await;
    seed_alpha_beta(&kg).await;

    let stats = service.memory_stats().await.unwrap();
    assert_eq!(stats.node_count, 2);
    assert_eq!(stats.edge_count, 0);
    assert_eq!(stats.cache_entries, 0);
    assert!(
        stats.estimated_bytes >= 2 * 512,
        "registry estimate scales with node count"
    );
}

#[tokio::test]
async fn recent_metrics_reflects_tracked_operations() {
    let (service, kg, _pool, _guard) = setup().await;
    seed_alpha_beta(&kg).await;
    let _ = service.nodes_page(None, None, 0, Some(10)).await.unwrap();
    let _ = service.ranked_search("alpha", None, Some(5)).await.unwrap();

    let metrics = service.recent_metrics(10).await.unwrap();
    let operations: Vec<&str> = metrics.iter().map(|m| m.operation.as_str()).collect();
    assert!(operations.contains(&"paginate_nodes"));
    assert!(operations.contains(&"ranked_search"));
    assert!(metrics.iter().all(|m| m.duration_ms >= 0));
}
