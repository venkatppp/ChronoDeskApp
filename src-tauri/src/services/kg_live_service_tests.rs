//! Tests for the RC-8 M2 live knowledge graph service against a real
//! migrated database. Every test seeds source rows the way the rest of
//! the backend does (workspaces, files, executions, planner reports) and
//! drives the service surface: incremental sync, semantic edges with a
//! deterministic fake embedder, confidence decay, multi-hop expansion,
//! recommendations, analytics + query cache, and the relationship
//! inspector.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::database::test_database;
use crate::models::kg::{GraphNodeType, GraphRelationshipType};
use crate::models::kg_live::{GraphEmbedder, MultiHopHit};
use crate::repositories::{KgLiveRepository, KgRepository};
use crate::services::{KgLiveService, KgService};

/// Deterministic fake embedder: any text mentioning "alpha" maps to a
/// shared vector, "beta" to an orthogonal one — so alpha/alpha pairs
/// score 1.0 (above `SEMANTIC_THRESHOLD`) and alpha/beta score 0.0.
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

fn live_service(pool: SqlitePool, with_embedder: bool) -> KgLiveService {
    let kg_service = KgService::new(KgRepository::new(pool.clone()));
    let service = KgLiveService::new(kg_service, KgLiveRepository::new(pool));
    if with_embedder {
        service.with_embedder(Arc::new(FakeEmbedder))
    } else {
        service
    }
}

async fn test_pool() -> (SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    (database.pool().clone(), temp_dir)
}

async fn seed_workspace(pool: &SqlitePool, name: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO workspaces
             (id, name, description, status, health_score, last_active_at, created_at, updated_at)
         VALUES (?, ?, ?, 'active', 80.0, ?, ?, ?)",
    )
    .bind(id)
    .bind(name)
    .bind("kg live test")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn seed_file(pool: &SqlitePool, workspace_id: Uuid, path: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO files (id, workspace_id, artifact_type, path_or_url, created_at, updated_at)
         VALUES (?, ?, 'file', ?, ?, ?)",
    )
    .bind(id)
    .bind(workspace_id)
    .bind(path)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn seed_execution_and_report(pool: &SqlitePool, workspace_id: Uuid) -> Uuid {
    let conversation_id = Uuid::new_v4();
    let execution_id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO copilot_conversations (id, workspace_id, title, created_at, updated_at)
         VALUES (?, ?, 'kg live conv', ?, ?)",
    )
    .bind(conversation_id)
    .bind(workspace_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO plan_executions
             (id, plan_id, conversation_id, status, current_step, total_steps, created_at, updated_at)
         VALUES (?, ?, ?, 'completed', 0, 3, ?, ?)",
    )
    .bind(execution_id)
    .bind(Uuid::new_v4())
    .bind(conversation_id)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO plan_execution_reports (execution_id, report) VALUES (?, 'alpha report body')",
    )
    .bind(execution_id)
    .execute(pool)
    .await
    .unwrap();
    execution_id
}

#[tokio::test]
async fn incremental_sync_builds_then_watermarks() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_execution_and_report(&pool, workspace).await;

    let first = service.incremental_sync().await.unwrap();
    assert_eq!(first.created_nodes, 4, "workspace, file, execution, report");
    assert_eq!(first.created_edges, 3, "contains + runs_in + reports_on");

    let second = service.incremental_sync().await.unwrap();
    assert_eq!(second.created_nodes, 0, "watermark advanced, nothing to do");
    assert_eq!(second.created_edges, 0);

    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    let third = service.incremental_sync().await.unwrap();
    assert_eq!(third.created_nodes, 1, "only the new file was synced");
    assert_eq!(third.created_edges, 1);

    let stats = service.kg.stats().await.unwrap();
    assert_eq!(stats.node_count, 5);
    assert_eq!(stats.edge_count, 4);
}

#[tokio::test]
async fn sync_entity_is_idempotent_and_drops_missing_sources() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let file = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;

    let ws_result = service
        .sync_entity(GraphNodeType::Workspace, workspace)
        .await
        .unwrap();
    assert!(ws_result.node_created);

    let first = service
        .sync_entity(GraphNodeType::File, file)
        .await
        .unwrap();
    assert!(first.node_created);
    assert_eq!(first.edges_created, 1, "file contains workspace");

    let again = service
        .sync_entity(GraphNodeType::File, file)
        .await
        .unwrap();
    assert!(!again.node_created);
    assert!(again.node_updated);
    assert_eq!(
        again.edges_updated, 1,
        "structural edge refreshed, not recreated"
    );

    let missing = service
        .sync_entity(GraphNodeType::File, Uuid::new_v4())
        .await
        .unwrap();
    assert!(!missing.node_created);

    // Deleting the source row makes the next sync drop the node.
    sqlx::query("DELETE FROM files WHERE id = ?")
        .bind(file)
        .execute(&pool)
        .await
        .unwrap();
    let removed = service
        .sync_entity(GraphNodeType::File, file)
        .await
        .unwrap();
    assert!(!removed.node_created);
    let node = service
        .kg
        .get_node(GraphNodeType::File, file)
        .await
        .unwrap();
    assert!(node.is_none(), "missing source row removes the node");
}

#[tokio::test]
async fn rebuild_semantic_edges_persists_confident_pairs_and_prunes_stale() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let _alpha_one = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    let _alpha_two = seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    let beta = seed_file(&pool, workspace, "/tmp/beta_thing.md").await;
    service.incremental_sync().await.unwrap();

    let result = service.rebuild_semantic_edges(None).await.unwrap();
    assert_eq!(
        result.candidate_pairs, 3,
        "4 nodes, 3 pairs clear the threshold"
    );
    assert_eq!(
        result.created, 3,
        "workspace-alpha1, workspace-alpha2, alpha1-alpha2"
    );
    assert_eq!(result.pruned, 0);

    let related: Vec<_> = service
        .repository
        .all_edges()
        .await
        .unwrap()
        .into_iter()
        .filter(|edge| edge.relationship_type == GraphRelationshipType::RelatedTo)
        .collect();
    assert_eq!(related.len(), 3);
    assert!(related.iter().all(|edge| edge.confidence == 1.0));
    assert!(related
        .iter()
        .all(|edge| { edge.source_entity_id != beta && edge.target_entity_id != beta }));

    // A stale low-confidence edge (e.g. workspace~beta at 0.3) is
    // removed by the next pass; the confident pairs are refreshed.
    service
        .repository
        .upsert_semantic_relationship(
            GraphNodeType::Workspace,
            workspace,
            GraphNodeType::File,
            beta,
            0.3,
            0.3,
            serde_json::json!({ "source": "test" }),
        )
        .await
        .unwrap();
    let second = service.rebuild_semantic_edges(None).await.unwrap();
    assert_eq!(second.created, 0);
    assert_eq!(second.updated, 3, "confident pairs refreshed");
    assert_eq!(second.pruned, 1, "stale workspace~beta edge removed");
}

#[tokio::test]
async fn apply_edge_decay_ages_semantic_edges_only() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    service.incremental_sync().await.unwrap();
    service.rebuild_semantic_edges(None).await.unwrap();

    sqlx::query("UPDATE graph_relationships SET updated_at = datetime('now', '-20 days')")
        .execute(&pool)
        .await
        .unwrap();
    let summary = service.apply_edge_decay().await.unwrap();
    assert_eq!(summary.decayed, 1, "only the semantic edge ages");
    assert_eq!(summary.pruned, 0, "0.92^20 stays above the 0.10 floor");

    let edges = service.repository.all_edges().await.unwrap();
    let related = edges
        .iter()
        .find(|edge| edge.relationship_type == GraphRelationshipType::RelatedTo)
        .unwrap();
    let expected = 0.92f64.powi(20);
    assert!(
        (related.confidence - expected).abs() < 0.002,
        "confidence {:.4} ~= 0.92^20 {:.4}",
        related.confidence,
        expected
    );
    let structural = edges
        .iter()
        .find(|edge| edge.relationship_type == GraphRelationshipType::Contains)
        .unwrap();
    assert_eq!(structural.confidence, 1.0, "structural edges never decay");

    sqlx::query("UPDATE graph_relationships SET updated_at = datetime('now', '-40 days')")
        .execute(&pool)
        .await
        .unwrap();
    let second = service.apply_edge_decay().await.unwrap();
    assert_eq!(second.decayed, 1);
    assert_eq!(
        second.pruned, 1,
        "below the 0.10 floor after ~60 days total"
    );
}

#[tokio::test]
async fn expand_context_walks_hops_and_caches() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let file_one = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    let file_two = seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    let execution = seed_execution_and_report(&pool, workspace).await;
    service.incremental_sync().await.unwrap();

    let context = service
        .expand_context(GraphNodeType::Workspace, workspace, Some(3), None, false)
        .await
        .unwrap();
    assert_eq!(context.source.entity_id, workspace);
    assert_eq!(context.related.len(), 4, "2 files + execution + its report");

    let by_key: HashMap<(GraphNodeType, Uuid), &MultiHopHit> = context
        .related
        .iter()
        .map(|hit| ((hit.node.node_type, hit.node.entity_id), hit))
        .collect();
    assert_eq!(by_key[&(GraphNodeType::File, file_one)].hop, 1);
    assert_eq!(by_key[&(GraphNodeType::File, file_two)].hop, 1);
    assert_eq!(by_key[&(GraphNodeType::Execution, execution)].hop, 1);
    let report = by_key[&(GraphNodeType::PlannerReport, execution)];
    assert_eq!(report.hop, 2, "report sits behind its execution");
    assert!(
        report.weight < 1.0,
        "multi-hop paths are discounted (weight {})",
        report.weight
    );

    let mut sorted = context.related.clone();
    sorted.sort_by(|a, b| b.weight.total_cmp(&a.weight));
    assert_eq!(context.related, sorted, "hits arrive strongest-path-first");

    let cached = service
        .expand_context(GraphNodeType::Workspace, workspace, Some(3), None, true)
        .await
        .unwrap();
    assert_eq!(cached.related.len(), context.related.len());
}

#[tokio::test]
async fn recommendations_skip_direct_neighbors_and_explain_via() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let file_one = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    let execution = seed_execution_and_report(&pool, workspace).await;
    service.incremental_sync().await.unwrap();

    let recs = service
        .recommendations(GraphNodeType::Workspace, workspace, None, false)
        .await
        .unwrap();
    assert!(!recs.is_empty());
    assert!(
        recs.iter().all(|rec| rec.hop >= 2),
        "direct neighbors excluded"
    );
    assert!(recs.iter().any(|rec| {
        rec.hop == 2
            && rec.node.node_type == GraphNodeType::PlannerReport
            && rec.node.entity_id == execution
            && rec.via.is_some()
    }));
    assert!(
        recs.iter().all(|rec| rec.via.is_some()),
        "2-hop via explained"
    );

    let from_file = service
        .recommendations(GraphNodeType::File, file_one, None, false)
        .await
        .unwrap();
    assert!(from_file.iter().all(|rec| rec.node.entity_id != workspace));
    assert!(from_file.iter().any(|rec| {
        rec.hop == 2
            && rec.node.node_type == GraphNodeType::Execution
            && rec.node.entity_id == execution
    }));
}

#[tokio::test]
async fn analytics_payload_and_query_cache_round_trip() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    service.incremental_sync().await.unwrap();

    let first = service.analytics(None, false).await.unwrap();
    assert!(!first.cached);
    assert_eq!(first.node_count, 3);
    assert_eq!(first.edge_count, 2);
    assert_eq!(first.components.len(), 1, "single connected component");
    assert_eq!(first.components[0].size, 3);
    assert_eq!(first.workspace_importance.len(), 1);
    assert_eq!(first.workspace_importance[0].workspace_id, workspace);
    assert_eq!(first.top_central_nodes.len(), 3);
    assert!((first.density - 2.0 / 3.0).abs() < 1e-9);

    let cached = service.analytics(None, true).await.unwrap();
    assert!(cached.cached, "served from the persisted cache");
    assert_eq!(cached.node_count, 3);
    assert!(service.cache_stats().await.unwrap().cached_queries >= 1);

    // Any graph write clears the cache.
    service.incremental_sync().await.unwrap();
    assert_eq!(service.cache_stats().await.unwrap().cached_queries, 0);
    let fresh = service.analytics(None, true).await.unwrap();
    assert!(!fresh.cached, "cache miss recomputes");
}

#[tokio::test]
async fn relationship_details_surface_incident_edges() {
    let (pool, _temp_dir) = test_pool().await;
    let service = live_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let file_one = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    let file_two = seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    service.incremental_sync().await.unwrap();

    let details = service
        .relationship_details(GraphNodeType::Workspace, workspace)
        .await
        .unwrap();
    assert_eq!(details.node.entity_id, workspace);
    assert_eq!(details.relationships.len(), 2);
    let neighbors: HashSet<Uuid> = details
        .relationships
        .iter()
        .map(|relationship| relationship.neighbor.entity_id)
        .collect();
    assert_eq!(neighbors, HashSet::from([file_one, file_two]));
    assert!(details.relationships.iter().all(|relationship| {
        relationship.edge.relationship_type == GraphRelationshipType::Contains
            && relationship.edge.confidence == 1.0
    }));

    let file_details = service
        .relationship_details(GraphNodeType::File, file_one)
        .await
        .unwrap();
    assert_eq!(file_details.relationships.len(), 1);
    assert_eq!(file_details.relationships[0].neighbor.entity_id, workspace);
}
