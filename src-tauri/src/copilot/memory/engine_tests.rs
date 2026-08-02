use super::*;
use crate::copilot::memory::vector::LocalVectorProvider;
use crate::database::test_database;

async fn engine() -> (MemoryEngine, tempfile::TempDir) {
    let (database, guard) = test_database().await;
    let repo = MemoryRepository::new(database.pool().clone());
    let provider: Arc<dyn VectorProvider> = Arc::new(LocalVectorProvider::default());
    (MemoryEngine::new(repo, provider), guard)
}

fn step(tool: Option<&str>, description: &str, error: Option<&str>) -> ExecutionStep {
    ExecutionStep {
        id: Uuid::new_v4(),
        execution_id: Uuid::new_v4(),
        step_number: 0,
        description: description.to_string(),
        tool_name: tool.map(String::from),
        arguments: None,
        status: if error.is_some() {
            crate::copilot::StepStatus::Failed
        } else {
            crate::copilot::StepStatus::Completed
        },
        result: None,
        error: error.map(String::from),
        started_at: None,
        completed_at: None,
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn record_execution_and_search_find_it() {
    let (engine, _guard) = engine().await;
    let execution_id = Uuid::new_v4();
    engine
        .record_execution(
            execution_id,
            None,
            "resume my focus session",
            None,
            &[
                step(Some("list_workspaces"), "List workspaces", None),
                step(Some("resume_workspace"), "Resume focused work", None),
            ],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .expect("capture should succeed");

    let hits = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .expect("search should succeed");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.source_id, execution_id);
    assert!(matches!(hits[0].record.status, MemoryStatus::Success));
    assert!(hits[0].similarity > 0.9);
    assert_eq!(
        hits[0].record.tools_used,
        vec!["list_workspaces", "resume_workspace"]
    );
}

#[tokio::test]
async fn failed_executions_are_retrievable_as_avoid_strategies() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(
                Some("get_recent_events"),
                "Gather activity",
                Some("permission denied"),
            )],
            ExecutionStatus::Failed,
            Some("permission denied".into()),
        )
        .await
        .expect("capture should succeed");

    let avoided = engine
        .avoid("resume my focus session", None, 5)
        .await
        .expect("avoid should succeed");
    assert_eq!(avoided.len(), 1);
    assert!(avoided[0].failure.contains("permission denied"));
}

#[tokio::test]
async fn recommend_prefers_successful_workflows() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(
                Some("get_recent_events"),
                "Gather activity",
                Some("denied"),
            )],
            ExecutionStatus::Failed,
            Some("denied".into()),
        )
        .await
        .unwrap();

    let recommendations = engine
        .recommend("resume my focus session", None, 5)
        .await
        .expect("recommend should succeed");
    assert_eq!(recommendations.len(), 1);
    assert!(matches!(
        recommendations[0].record.status,
        MemoryStatus::Success
    ));
    assert!(recommendations[0].score > 0.5);
}

#[tokio::test]
async fn planner_report_recording_round_trips() {
    let (engine, _guard) = engine().await;
    let report = PlannerReport {
        plan: ExecutionPlan {
            id: Uuid::new_v4(),
            workspace_id: None,
            goal: "recover after step failure".into(),
            tasks: vec![],
            estimated_duration_minutes: 0,
            required_files: vec![],
            checkpoints: vec![],
            confidence: 0.8,
            reasoning: "r".into(),
            status: crate::copilot::proactive_models::PlanApprovalStatus::Pending,
            created_at: Utc::now(),
        },
        execution_id: Some(Uuid::new_v4()),
        completed: vec![Uuid::new_v4()],
        skipped: vec![],
        replaced: vec![],
        replan_count: 0,
        error: None,
    };
    engine
        .record_planner_report(&report)
        .await
        .expect("report capture should succeed");

    let hits = engine
        .search(&MemorySearchRequest {
            query: "recover after step failure".into(),
            kind: Some(MemoryKind::PlannerReport),
            workspace_id: None,
            status: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.outcome.completed, 1);
}

#[tokio::test]
async fn autonomous_session_recording_round_trips() {
    let (engine, _guard) = engine().await;
    let progress = AutonomousSessionProgress {
        session_id: Uuid::new_v4(),
        workspace_id: None,
        goal: "resume the most recent workspace".into(),
        status: AutonomousStatus::Completed,
        policy: Default::default(),
        reasoning: vec![crate::copilot::autonomous::models::ReasoningEvent::new(
            Uuid::new_v4(),
            crate::copilot::autonomous::models::ReasoningPhase::Terminal,
            "Goal reached: 3 steps completed",
            None,
        )],
        current_plan: None,
        execution_id: None,
        last_execution_id: None,
        plans_attempted: 1,
        plans_completed: 1,
        steps_completed: 3,
        retries_used: 0,
        replans_used: 0,
        steps_left: 0,
        error: None,
        pending_approval: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    engine
        .record_autonomous_session(&progress)
        .await
        .expect("session capture should succeed");

    let hits = engine
        .search(&MemorySearchRequest {
            query: "resume the most recent workspace".into(),
            kind: Some(MemoryKind::AutonomousSession),
            workspace_id: None,
            status: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.outcome.steps, 3);
    assert_eq!(hits[0].record.outcome.plans_attempted, 1);
}

#[tokio::test]
async fn stats_and_learned_workflows_aggregate() {
    let (engine, _guard) = engine().await;
    for _ in 0..2 {
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                "resume my focus session",
                None,
                &[step(Some("list_workspaces"), "List workspaces", None)],
                ExecutionStatus::Completed,
                None,
            )
            .await
            .unwrap();
    }
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(
                Some("get_recent_events"),
                "Gather activity",
                Some("denied"),
            )],
            ExecutionStatus::Failed,
            Some("denied".into()),
        )
        .await
        .unwrap();

    let stats = engine.stats().await.unwrap();
    assert_eq!(stats.total_records, 3);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.failed, 1);

    let workflows = engine.learned_workflows().await.unwrap();
    assert_eq!(workflows.len(), 1);
    let focus = &workflows[0];
    assert_eq!(focus.success_count, 2);
    assert_eq!(focus.failure_count, 1);
    assert_eq!(focus.goal_fingerprint, "resume my focus session");
}

#[tokio::test]
async fn mark_replayed_increments_replay_count() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    let hits = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .unwrap();
    let id = hits[0].record.id;
    engine.mark_replayed(id).await.unwrap();
    engine.mark_replayed(id).await.unwrap();

    let stats = engine.stats().await.unwrap();
    assert_eq!(stats.total_replays, 2);
}

// ------------------------------------------------------------------
// RC-6 M2: vector memory system
// ------------------------------------------------------------------

#[tokio::test]
async fn index_pending_embeds_captures_and_search_uses_the_index() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();

    // Before indexing the vector index is empty (cold start).
    let status = engine.vector_status().await.unwrap();
    assert_eq!(status.total_records, 1);
    assert_eq!(status.indexed, 0);
    assert_eq!(status.pending, 1);
    assert_eq!(status.provider, "local-ngram");
    assert_eq!(status.dimensions, 384);

    // One pass embeds the pending record everywhere.
    let result = engine.index_pending(10).await.unwrap();
    assert_eq!(result.indexed, 1);
    let status = engine.vector_status().await.unwrap();
    assert_eq!(status.indexed, 1);
    assert_eq!(status.pending, 0);
    assert!(status.last_indexed_at.is_some());

    let hits = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert!(
        (hits[0].similarity - 1.0).abs() < 1e-6,
        "indexed identical match"
    );
    assert!(
        engine.vector_system().index_len() == 1,
        "in-memory k-NN index populated"
    );
}

#[tokio::test]
async fn vector_ranking_prefers_similar_over_unrelated_goals() {
    let (engine, _guard) = engine().await;
    for goal in [
        "resume my focus session",
        "resume my last focus session",
        "organize tax receipts",
    ] {
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                goal,
                None,
                &[step(Some("list_workspaces"), "List workspaces", None)],
                ExecutionStatus::Completed,
                None,
            )
            .await
            .unwrap();
    }
    engine.index_pending(10).await.unwrap();

    let hits = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .unwrap();
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].record.goal, "resume my focus session");
    assert_eq!(hits[1].record.goal, "resume my last focus session");
    assert_eq!(hits[2].record.goal, "organize tax receipts");
    assert!(
        hits[0].similarity > hits[1].similarity && hits[1].similarity > hits[2].similarity,
        "embedding + token blend must separate similar from unrelated"
    );
}

#[tokio::test]
async fn recommend_finds_similar_goals_via_knn_after_indexing() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    engine.index_pending(10).await.unwrap();

    let recommendations = engine
        .recommend("resume my last focus session", None, 5)
        .await
        .unwrap();
    assert_eq!(recommendations.len(), 1);
    assert!(recommendations[0].score > 0.5);
}

#[tokio::test]
async fn reindex_rebuilds_after_records_change() {
    let (engine, _guard) = engine().await;
    let execution_id = Uuid::new_v4();
    engine
        .record_execution(
            execution_id,
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    engine.index_pending(10).await.unwrap();
    assert_eq!(engine.vector_status().await.unwrap().indexed, 1);

    // The same source is re-captured under a different goal; the
    // record updates and the indexer must re-embed it.
    engine
        .record_execution(
            execution_id,
            None,
            "plan a long vacation",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    engine.reindex().await.unwrap();

    let hits = engine
        .search(&MemorySearchRequest::new("plan a long vacation"))
        .await
        .unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.goal, "plan a long vacation");
    assert!((hits[0].similarity - 1.0).abs() < 1e-6);
    assert_eq!(engine.vector_status().await.unwrap().indexed, 1);
}

#[tokio::test]
async fn embedding_cache_serves_repeated_queries() {
    let (engine, _guard) = engine().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Some("list_workspaces"), "List workspaces", None)],
            ExecutionStatus::Completed,
            None,
        )
        .await
        .unwrap();
    engine.index_pending(10).await.unwrap();

    let _ = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .unwrap();
    let _ = engine
        .search(&MemorySearchRequest::new("resume my focus session"))
        .await
        .unwrap();
    let status = engine.vector_status().await.unwrap();
    assert!(
        status.cache_hits >= 1,
        "repeated queries must hit the embedding cache"
    );
    assert!(status.cache_size >= 1);
}
