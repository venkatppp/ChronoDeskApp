//! ContextIntelRepository tests (RC-8 M3): persisted cross-workspace
//! relationships, graph context snapshots, and goal clusters —
//! upsert/replace round trips, idempotency, ordering, and scoping.

use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::*;
use crate::database::test_database;
use crate::models::kg::GraphNodeType;
use crate::models::kg_context::ClusterMember;
use serde_json::json;

async fn setup() -> (ContextIntelRepository, SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    (ContextIntelRepository::new(pool.clone()), pool, temp_dir)
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
    .bind("context intel repository test")
    .bind(&now)
    .bind(&now)
    .bind(&now)
    .execute(pool)
    .await
    .unwrap();
    id
}

#[tokio::test]
async fn workspace_similarity_round_trips_both_directions() {
    let (repo, pool, _guard) = setup().await;
    let alpha = seed_workspace(&pool, "Alpha").await;
    let beta = seed_workspace(&pool, "Beta").await;

    let signals = vec![json!({ "signal": "goalOverlap", "score": 0.8, "detail": "2 shared" })];
    repo.upsert_workspace_similarity(alpha, beta, 0.75, 0.9, &signals)
        .await
        .unwrap();

    // Queried from the source side and the target side alike.
    for side in [alpha, beta] {
        let rows = repo
            .list_workspace_similarity(side, None, None)
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        let other = if side == alpha { beta } else { alpha };
        assert_eq!(rows[0].target_workspace_id, other);
        assert!((rows[0].similarity - 0.75).abs() < 1e-9);
        assert!((rows[0].confidence - 0.9).abs() < 1e-9);
        assert_eq!(rows[0].signals, serde_json::to_string(&signals).unwrap());
    }
}

#[tokio::test]
async fn workspace_similarity_upsert_is_idempotent_and_min_floor_filters() {
    let (repo, pool, _guard) = setup().await;
    let alpha = seed_workspace(&pool, "Alpha").await;
    let beta = seed_workspace(&pool, "Beta").await;

    repo.upsert_workspace_similarity(alpha, beta, 0.4, 0.6, &[])
        .await
        .unwrap();
    repo.upsert_workspace_similarity(alpha, beta, 0.9, 0.95, &[])
        .await
        .unwrap();

    let rows = repo
        .list_workspace_similarity(alpha, None, None)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1, "upsert refreshes, never duplicates");
    assert!((rows[0].similarity - 0.9).abs() < 1e-9);

    let strict = repo
        .list_workspace_similarity(alpha, Some(0.95), None)
        .await
        .unwrap();
    assert!(strict.is_empty(), "floor filters weak relationships");

    let limited = repo
        .list_workspace_similarity(alpha, Some(0.5), Some(1))
        .await
        .unwrap();
    assert_eq!(limited.len(), 1);
}

#[tokio::test]
async fn snapshot_insert_and_list_are_newest_first() {
    let (repo, pool, _guard) = setup().await;
    let workspace = seed_workspace(&pool, "Alpha").await;

    let first = repo
        .snapshot_insert(
            workspace,
            "manual",
            3,
            2,
            0.7,
            &[SummaryPoint {
                label: "Nodes".into(),
                value: "3".into(),
                detail: None,
            }],
            &json!({ "scope": "all" }),
        )
        .await
        .unwrap();
    let second = repo
        .snapshot_insert(workspace, "manual", 5, 4, 0.8, &[], &json!({}))
        .await
        .unwrap();
    assert!(first < second, "auto-increment ids grow");

    let rows = repo.snapshot_list(workspace, None).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, second, "newest snapshot first");
    assert_eq!(rows[0].node_count, 5);
    assert_eq!(rows[0].edge_count, 4);
    assert_eq!(rows[1].id, first);
    assert_eq!(rows[1].workspace_id, workspace);
    assert_eq!(rows[1].snapshot_type, "manual");

    let limited = repo.snapshot_list(workspace, Some(1)).await.unwrap();
    assert_eq!(limited.len(), 1);
}

#[tokio::test]
async fn clusters_replace_scopes_and_clears() {
    let (repo, pool, _guard) = setup().await;
    let workspace = seed_workspace(&pool, "Alpha").await;

    let clusters = vec![GoalCluster {
        id: 0,
        workspace_id: Some(workspace),
        name: "Bug fixing".into(),
        member_count: 2,
        members: vec![ClusterMember {
            node_type: GraphNodeType::Execution,
            entity_id: Uuid::new_v4(),
            title: "fix crash".into(),
            workspace_id: Some(workspace),
            score: 0.9,
        }],
        centroid_terms: vec!["fix".into(), "crash".into()],
        confidence: 0.8,
    }];
    repo.clusters_replace(Some(workspace), &clusters)
        .await
        .unwrap();

    let rows = repo.clusters_list(Some(workspace)).await.unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, "Bug fixing");
    assert_eq!(rows[0].member_count, 2);
    assert_eq!(rows[0].workspace_id, Some(workspace));

    // Replace purges the old set for the same scope.
    repo.clusters_replace(Some(workspace), &[]).await.unwrap();
    assert!(repo
        .clusters_list(Some(workspace))
        .await
        .unwrap()
        .is_empty());
    assert_eq!(
        repo.clusters_list(None).await.unwrap().len(),
        0,
        "whole-graph scope sees nothing too"
    );
}
