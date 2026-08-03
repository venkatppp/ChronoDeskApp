//! Tests for the RC-8 M3 context intelligence service against a real
//! migrated database. Seeds workspaces, files, executions + reports and
//! memory records the way the rest of the backend does, then drives the
//! service surface: context inference with a deterministic fake
//! embedder, workspace similarity + discovery + persistence, goal
//! clusters, knowledge summaries, snapshots + timeline deltas, memory +
//! KG fusion, planner retrieval, explanations, and cache invalidation.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::database::test_database;
use crate::models::kg::{GraphNodeType, GraphRelationshipType};
use crate::models::kg_context::{ContextSignalType, FusedHitSource};
use crate::models::kg_live::GraphEmbedder;
use crate::repositories::{
    ContextIntelRepository, KgLiveRepository, KgRepository, WorkspaceRepository,
};
use crate::services::{ContextIntelService, KgLiveService, KgService};

/// Deterministic fake embedder: any text mentioning "alpha" maps to a
/// shared vector, "beta" to an orthogonal one — so alpha/alpha pairs
/// score 1.0 and alpha/beta score 0.0. Unknown text scores 1/√2 against
/// both, which keeps below the 0.45 semantic threshold against them.
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

fn intel_service(pool: SqlitePool, with_embedder: bool) -> ContextIntelService {
    let kg = KgService::new(KgRepository::new(pool.clone()));
    let mut live = KgLiveService::new(kg.clone(), KgLiveRepository::new(pool.clone()));
    if with_embedder {
        live = live.with_embedder(Arc::new(FakeEmbedder));
    }
    let mut service = ContextIntelService::new(
        kg,
        live,
        WorkspaceRepository::new(pool.clone()),
        ContextIntelRepository::new(pool),
    );
    if with_embedder {
        service = service.with_embedder(Arc::new(FakeEmbedder));
    }
    service
}

async fn sync_graph(service: &ContextIntelService) {
    service.live.incremental_sync().await.unwrap();
}

async fn semantic_edges(service: &ContextIntelService) {
    service.live.rebuild_semantic_edges(None).await.unwrap();
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
    .bind("context intel service test")
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
         VALUES (?, ?, 'context intel conv', ?, ?)",
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

async fn seed_memory(pool: &SqlitePool, workspace_id: Uuid, goal: &str) -> Uuid {
    let id = Uuid::new_v4();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO execution_memory
             (id, kind, source_id, workspace_id, goal, status, created_at, updated_at)
         VALUES (?, 'execution', ?, ?, ?, 'success', ?, ?)",
    )
    .bind(id)
    .bind(id)
    .bind(workspace_id)
    .bind(goal)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn infer_context_ranks_semantic_and_structural_hits() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    let execution = seed_execution_and_report(&pool, workspace).await;
    sync_graph(&service).await;
    semantic_edges(&service).await;

    let inference = service
        .infer_context(GraphNodeType::Workspace, workspace, None, false)
        .await
        .unwrap();
    assert_eq!(inference.source.entity_id, workspace);
    assert!(
        inference
            .related
            .iter()
            .any(|hit| hit.signal == ContextSignalType::Semantic),
        "semantic related_to neighbors are ranked in"
    );
    assert!(
        inference
            .related
            .iter()
            .any(|hit| hit.signal == ContextSignalType::Structural),
        "structural neighbors are ranked in"
    );
    assert!(
        inference
            .related
            .iter()
            .any(|hit| hit.node.entity_id == execution),
        "execution neighbor included"
    );
    assert!(
        inference
            .related
            .windows(2)
            .all(|pair| pair[0].score >= pair[1].score),
        "hits arrive strongest-first"
    );
    assert!(inference.confidence.total > 0.0);

    let cached = service
        .infer_context(GraphNodeType::Workspace, workspace, None, true)
        .await
        .unwrap();
    assert_eq!(
        cached.related.len(),
        inference.related.len(),
        "cached round trip"
    );
}

#[tokio::test]
async fn workspace_similarity_detects_goal_overlap_and_persists() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), true);
    let alpha = seed_workspace(&pool, "Alpha Workspace").await;
    let beta = seed_workspace(&pool, "Beta Workspace").await;
    let gamma = seed_workspace(&pool, "Gamma Workspace").await;
    seed_memory(&pool, alpha, "fix login bug").await;
    seed_memory(&pool, alpha, "fix login issue").await;
    seed_memory(&pool, beta, "fix login bug").await;
    seed_memory(&pool, beta, "resolve login issue").await;
    seed_memory(&pool, gamma, "design marketing site").await;
    sync_graph(&service).await;

    let result = service.workspace_similarity(alpha, false).await.unwrap();
    assert!(!result.cached);
    assert_eq!(result.source_workspace_id, alpha);
    let beta_hit = result
        .related
        .iter()
        .find(|similarity| similarity.target_workspace_id == beta);
    assert!(beta_hit.is_some(), "shared login goals relate alpha~beta");
    let beta_hit = beta_hit.unwrap();
    assert!(beta_hit.similarity >= 0.18);
    assert!(beta_hit.persisted, "relationship written through");
    assert!(beta_hit
        .signals
        .iter()
        .any(|signal| { signal.signal == ContextSignalType::GoalOverlap && signal.score > 0.0 }));
    assert!(
        result
            .related
            .iter()
            .all(|similarity| similarity.target_workspace_id != gamma),
        "no goal/graph/semantic overlap with gamma → below the floor"
    );

    let cached = service.workspace_similarity(alpha, true).await.unwrap();
    assert!(cached.cached, "second call served from cache");

    // Persisted rows are queryable from either side of the pair.
    let stored = service
        .repository
        .list_workspace_similarity(alpha, None, None)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1);
}

#[tokio::test]
async fn discover_relationships_forces_refresh_and_persists() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), true);
    let alpha = seed_workspace(&pool, "Alpha Workspace").await;
    let beta = seed_workspace(&pool, "Beta Workspace").await;
    seed_memory(&pool, alpha, "fix login bug").await;
    seed_memory(&pool, beta, "fix login bug").await;
    sync_graph(&service).await;

    let first = service
        .discover_cross_workspace_relationships(alpha)
        .await
        .unwrap();
    assert_eq!(first.related.len(), 1);
    assert!(first.related[0].persisted);

    // A fresh goal in alpha changes the relationship on the next pass.
    seed_memory(&pool, alpha, "alpha crash triage").await;
    sync_graph(&service).await;
    let second = service
        .discover_cross_workspace_relationships(alpha)
        .await
        .unwrap();
    assert_eq!(second.related.len(), 1, "still one active workspace pair");
    let stored = service
        .repository
        .list_workspace_similarity(beta, None, None)
        .await
        .unwrap();
    assert_eq!(stored.len(), 1, "refresh rewrites the persisted pair");
}

#[tokio::test]
async fn goal_clusters_group_similar_goals_and_persist() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_memory(&pool, workspace, "fix login bug").await;
    seed_memory(&pool, workspace, "fix login issue").await;
    seed_memory(&pool, workspace, "setup ci pipeline").await;
    seed_memory(&pool, workspace, "build release notes").await;
    sync_graph(&service).await;

    let clusters = service.goal_clusters(Some(workspace), false).await.unwrap();
    let login = clusters
        .iter()
        .find(|cluster| cluster.member_count >= 2)
        .expect("login goals cluster together");
    assert!(login
        .members
        .iter()
        .all(|member| member.node_type == GraphNodeType::MemoryRecord));
    assert!(login.confidence >= 0.3, "cohesion reflects member scores");
    assert!(
        clusters
            .iter()
            .any(|cluster| cluster.member_count == 1
                && cluster.members[0].title == "setup ci pipeline"),
        "dissimilar goals stay singleton clusters"
    );

    let cached = service.goal_clusters(Some(workspace), true).await.unwrap();
    assert_eq!(cached.len(), clusters.len(), "cached round trip");

    let stored = service
        .repository
        .clusters_list(Some(workspace))
        .await
        .unwrap();
    assert_eq!(stored.len(), clusters.len(), "clusters persisted");
    let persisted_ids: Vec<i64> = stored.iter().map(|row| row.id).collect();
    assert!(
        persisted_ids.iter().all(|id| *id > 0),
        "real row ids returned"
    );
}

#[tokio::test]
async fn knowledge_summary_counts_connections_and_topics() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    seed_execution_and_report(&pool, workspace).await;
    sync_graph(&service).await;

    let summary = service
        .knowledge_summary(GraphNodeType::Workspace, workspace, false)
        .await
        .unwrap();
    assert_eq!(summary.node.entity_id, workspace);
    let connections = summary
        .points
        .iter()
        .find(|point| point.label == "Graph connections")
        .expect("connection count point");
    assert_eq!(connections.value, "3", "2 files + 1 execution");
    assert!(connections.detail.as_ref().unwrap().contains("contains"));
    assert!(summary.confidence > 0.5);
    assert!(summary
        .points
        .iter()
        .any(|point| point.label == "Last updated"));

    let cached = service
        .knowledge_summary(GraphNodeType::Workspace, workspace, true)
        .await
        .unwrap();
    assert_eq!(
        cached.points.len(),
        summary.points.len(),
        "cached round trip"
    );
}

#[tokio::test]
async fn snapshots_and_timeline_report_deltas() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_execution_and_report(&pool, workspace).await;
    sync_graph(&service).await;

    let first = service
        .context_snapshot_create(workspace, "manual")
        .await
        .unwrap();
    assert_eq!(
        first.node_count, 3,
        "workspace + file + execution — the planner report node is intentionally workspace-less"
    );
    assert_eq!(
        first.edge_count, 2,
        "contains + runs_in — the reports_on edge's report endpoint has no workspace"
    );
    assert!(first.id > 0);
    assert!(!first.summary.is_empty());

    seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    sync_graph(&service).await;
    let second = service
        .context_snapshot_create(workspace, "manual")
        .await
        .unwrap();
    assert_eq!(second.node_count, 4);

    let snapshots = service
        .context_snapshot_list(workspace, None)
        .await
        .unwrap();
    assert_eq!(snapshots.len(), 2, "newest first");
    assert_eq!(snapshots[0].id, second.id);

    let timeline = service.context_timeline(workspace, None).await.unwrap();
    assert_eq!(timeline.len(), 2);
    assert_eq!(
        timeline[0].nodes_delta, 1,
        "one file added since first capture"
    );
    assert_eq!(timeline[0].edges_delta, 1);
    assert!(timeline[0].confidence_delta >= 0.0);
    assert_eq!(
        timeline[1].nodes_delta, 0,
        "oldest entry has no predecessor"
    );
}

#[tokio::test]
async fn fused_context_merges_memory_and_kg_hits() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_execution_and_report(&pool, workspace).await;
    seed_memory(&pool, workspace, "alpha crash fix").await;
    sync_graph(&service).await;

    let fused = service
        .fused_context(GraphNodeType::Workspace, workspace, false)
        .await
        .unwrap();
    assert!(!fused.memory_hits.is_empty(), "memory record fused in");
    assert!(
        fused
            .memory_hits
            .iter()
            .all(|hit| hit.signal == ContextSignalType::Memory),
        "memory hits carry the memory signal"
    );
    assert!(!fused.kg_hits.is_empty(), "kg hits present");
    assert!(
        fused
            .fused
            .iter()
            .any(|hit| hit.source == FusedHitSource::Memory),
        "fused list labels memory provenance"
    );
    assert!(
        fused
            .fused
            .iter()
            .any(|hit| hit.source == FusedHitSource::KnowledgeGraph),
        "fused list labels kg provenance"
    );
    assert!(
        fused
            .fused
            .windows(2)
            .all(|pair| pair[0].score >= pair[1].score),
        "fused hits arrive strongest-first"
    );
    assert!(fused.confidence.memory > 0.0, "memory confidence reported");

    let cached = service
        .fused_context(GraphNodeType::Workspace, workspace, true)
        .await
        .unwrap();
    assert_eq!(cached.fused.len(), fused.fused.len(), "cached round trip");
}

#[tokio::test]
async fn planner_context_anchors_on_goal_and_degrades_gracefully() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), true);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    seed_memory(&pool, workspace, "alpha crash fix").await;
    sync_graph(&service).await;

    let found = service
        .planner_context("alpha crash fix", false)
        .await
        .unwrap();
    let anchor = found.anchor.expect("goal matches the memory record");
    assert_eq!(anchor.node_type, GraphNodeType::MemoryRecord);
    let fused = found.context.expect("fused context around the anchor");
    assert!(!fused.fused.is_empty());
    assert!(found.summary.starts_with("Anchored"));

    let cached = service
        .planner_context("alpha crash fix", true)
        .await
        .unwrap();
    assert_eq!(cached.goal, found.goal, "cached round trip");

    let missing = service
        .planner_context("zzz no such goal", false)
        .await
        .unwrap();
    assert!(missing.anchor.is_none());
    assert!(missing.context.is_none());
    assert!(missing.summary.contains("No knowledge-graph anchor"));
}

#[tokio::test]
async fn explain_finds_paths_and_falls_back_to_topic_overlap() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    let file = seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    let execution = seed_execution_and_report(&pool, workspace).await;
    sync_graph(&service).await;

    // workspace → execution → report: a real 2-hop path.
    let explanation = service
        .explain(
            GraphNodeType::Workspace,
            workspace,
            GraphNodeType::PlannerReport,
            execution,
        )
        .await
        .unwrap();
    assert_eq!(explanation.chain.len(), 2);
    assert_eq!(
        explanation.chain[0].relationship_type,
        GraphRelationshipType::RunsIn
    );
    assert_eq!(
        explanation.chain[1].relationship_type,
        GraphRelationshipType::ReportsOn
    );
    assert!(explanation.summary.contains("Connected in 2 hop(s)"));
    assert!(explanation.confidence > 0.0);

    // file1 → workspace → file1's sibling: also 2 hops.
    let second_file = seed_file(&pool, workspace, "/tmp/alpha_two.rs").await;
    sync_graph(&service).await;
    let sibling = service
        .explain(GraphNodeType::File, file, GraphNodeType::File, second_file)
        .await
        .unwrap();
    assert_eq!(sibling.chain.len(), 2);
    assert!(sibling
        .chain
        .iter()
        .all(|link| link.relationship_type == GraphRelationshipType::Contains));

    // Unrelated workspace pair with disjoint vocabulary: no path, no
    // shared terms — the weakest possible explanation.
    let other = seed_workspace(&pool, "Qux Workspace").await;
    let zzz = seed_file(&pool, other, "/tmp/zzz_thing.rs").await;
    sync_graph(&service).await;
    let isolated = service
        .explain(GraphNodeType::File, file, GraphNodeType::File, zzz)
        .await
        .unwrap();
    assert!(isolated.chain.is_empty());
    assert!(isolated.summary.contains("No graph path within 4 hops"));
    assert!(isolated.confidence < 0.5);
}

#[tokio::test]
async fn graph_writes_invalidate_context_cache() {
    let (pool, _temp_dir) = test_pool().await;
    let service = intel_service(pool.clone(), false);
    let workspace = seed_workspace(&pool, "Alpha Workspace").await;
    seed_file(&pool, workspace, "/tmp/alpha_one.rs").await;
    sync_graph(&service).await;
    service
        .infer_context(GraphNodeType::Workspace, workspace, None, true)
        .await
        .unwrap();
    assert!(
        service.cache_stats().await.unwrap().cached_queries >= 1,
        "inference cached in the shared query cache"
    );

    sync_graph(&service).await;
    assert_eq!(
        service.cache_stats().await.unwrap().cached_queries,
        0,
        "any graph write clears the context cache too"
    );
}
