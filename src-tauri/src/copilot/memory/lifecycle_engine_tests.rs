//! Memory lifecycle engine tests (RC-6 M4) — end-to-end lifecycle
//! behavior through the `MemoryEngine` facade: retention transitions,
//! the cleanup pass, compression, versioning + lineage, import/export,
//! snapshots, and storage statistics.

use std::sync::Arc;

use chrono::Utc;
use uuid::Uuid;

use crate::copilot::execution::{ExecutionStatus, ExecutionStep, StepStatus};
use crate::copilot::memory::engine::MemoryEngine;
use crate::copilot::memory::models::{MemoryKind, MemoryStatus, RetentionPolicy};
use crate::copilot::memory::repository::MemoryRepository;
use crate::copilot::memory::vector::LocalVectorProvider;
use crate::database::test_database;

async fn setup() -> (tempfile::TempDir, MemoryEngine) {
    let (database, guard) = test_database().await;
    let engine = MemoryEngine::new(
        MemoryRepository::new(database.pool().clone()),
        Arc::new(LocalVectorProvider::default()),
    );
    (guard, engine)
}

fn step(execution_id: Uuid, description: &str, completed: bool) -> ExecutionStep {
    ExecutionStep {
        id: Uuid::new_v4(),
        execution_id,
        step_number: 1,
        description: description.to_string(),
        tool_name: Some("list_workspaces".into()),
        arguments: None,
        status: if completed {
            StepStatus::Completed
        } else {
            StepStatus::Failed
        },
        result: None,
        error: completed.then(|| "boom".into()),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        created_at: Utc::now(),
    }
}

#[tokio::test]
async fn retention_transitions_through_the_engine() {
    let (_guard, engine) = setup().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume my focus session",
            None,
            &[step(Uuid::new_v4(), "list workspaces", true)],
            ExecutionStatus::Completed,
            None,
            Some(12),
        )
        .await
        .unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);
    assert!(matches!(record.retention, RetentionPolicy::Permanent));

    // Archive, then revive to permanent, then expire.
    engine.archive(record.id).await.unwrap();
    let archived = engine.repository.get(record.id).await.unwrap().unwrap();
    assert!(matches!(archived.retention, RetentionPolicy::Archived));
    assert!(archived.archived_at.is_some());

    engine
        .set_retention(record.id, RetentionPolicy::Permanent, None)
        .await
        .unwrap();
    let revived = engine.repository.get(record.id).await.unwrap().unwrap();
    assert!(matches!(revived.retention, RetentionPolicy::Permanent));
    assert!(revived.archived_at.is_none(), "revive clears the state");

    engine.expire(record.id).await.unwrap();
    let expired = engine.repository.get(record.id).await.unwrap().unwrap();
    assert!(matches!(expired.retention, RetentionPolicy::Expired));

    // Temporary without a deadline is rejected.
    assert!(engine
        .set_retention(record.id, RetentionPolicy::Temporary, None)
        .await
        .is_err());
}

#[tokio::test]
async fn versioning_chains_reused_workflows_with_lineage() {
    let (_guard, engine) = setup().await;
    // First run: no ancestor yet → version 1, no parent.
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "organize tax receipts",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let v1 = engine.repository.list_all().await.unwrap().remove(0);
    assert_eq!(v1.version, 1);
    assert!(v1.parent_id.is_none());

    // The workflow is reused (replayed) and a new successful run of the
    // same goal appears → version 2, chained to v1.
    engine.mark_replayed(v1.id).await.unwrap();
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "  ORGANIZE Tax Receipts ",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let all = engine.repository.list_all().await.unwrap();
    let v2 = all
        .iter()
        .find(|r| r.version == 2)
        .expect("version 2 exists");
    assert_eq!(v2.parent_id, Some(v1.id), "chained to the reused workflow");

    // Lineage walks the chain.
    let lineage = engine
        .lineage(v2.id)
        .await
        .unwrap()
        .expect("lineage exists");
    assert_eq!(lineage.root_id, Some(v1.id));
    assert_eq!(lineage.ancestors.len(), 1);
    assert_eq!(lineage.ancestors[0].id, v1.id);
    assert_eq!(lineage.version, 2);

    let v1_lineage = engine.lineage(v1.id).await.unwrap().unwrap();
    assert_eq!(v1_lineage.children.len(), 1);
    assert_eq!(v1_lineage.children[0].id, v2.id);
}

#[tokio::test]
async fn expired_records_are_hidden_from_retrieval_and_cleanup_removes_them() {
    let (_guard, engine) = setup().await;
    let execution_id = Uuid::new_v4();
    engine
        .record_execution(
            execution_id,
            None,
            "resume my focus session",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);

    // Mark it expired: searches/recommendations must not surface it.
    engine.expire(record.id).await.unwrap();
    let hits = engine
        .search(&crate::copilot::memory::models::MemorySearchRequest {
            query: "resume".into(),
            kind: None,
            workspace_id: None,
            status: None,
            limit: 10,
        })
        .await
        .unwrap();
    assert!(hits.is_empty(), "expired memories never surface");
    assert!(engine
        .recommend("resume my focus session", None, 5)
        .await
        .unwrap()
        .is_empty());

    // The cleanup pass deletes it, vectors included.
    let report = engine.run_cleanup().await.unwrap();
    assert_eq!(report.removed_expired, 1);
    assert!(engine.repository.list_all().await.unwrap().is_empty());
    assert_eq!(engine.vector_system().index_len(), 0);
}

#[tokio::test]
async fn compression_round_trips_through_the_engine() {
    let (_guard, engine) = setup().await;
    use crate::copilot::autonomous::models::{
        AutonomousSessionProgress, ExecutionPolicy, ReasoningEvent, ReasoningPhase,
    };
    use crate::copilot::autonomous::AutonomousStatus;
    let reasoning = (0..90)
        .map(|i| ReasoningEvent {
            session_id: Uuid::new_v4(),
            phase: ReasoningPhase::Planning,
            message: format!("step {i}"),
            detail: None,
            created_at: Utc::now(),
        })
        .collect::<Vec<_>>();
    let progress = AutonomousSessionProgress {
        session_id: Uuid::new_v4(),
        workspace_id: None,
        goal: "deep research task".into(),
        status: AutonomousStatus::Completed,
        policy: ExecutionPolicy::default(),
        reasoning,
        current_plan: None,
        execution_id: None,
        last_execution_id: None,
        plans_attempted: 1,
        plans_completed: 0,
        steps_completed: 5,
        retries_used: 0,
        replans_used: 0,
        steps_left: 0,
        error: None,
        pending_approval: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    engine.record_autonomous_session(&progress).await.unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);
    assert_eq!(record.reasoning.len(), 90);

    let result = engine.compress_oversized(10).await.unwrap();
    assert_eq!(result.compressed, 1);
    let compressed = engine.repository.get(record.id).await.unwrap().unwrap();
    assert!(compressed.compressed_at.is_some());
    assert_eq!(compressed.reasoning.len(), 1, "history replaced by summary");
    assert!(compressed.summary.as_deref().unwrap().contains("90"));

    // Already compressed → not re-compressed; restore brings originals back.
    assert!(!engine.compress_memory(record.id).await.unwrap());
    assert!(engine.restore_compressed(record.id).await.unwrap());
    let restored = engine.repository.get(record.id).await.unwrap().unwrap();
    assert!(restored.compressed_at.is_none());
    assert_eq!(restored.reasoning.len(), 90);
}

#[tokio::test]
async fn export_import_round_trips_idempotently() {
    let (_guard, engine) = setup().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume focus",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);
    engine.record_acceptance(record.id, true).await.unwrap();

    let exported = engine.export_json().await.unwrap();
    assert!(exported.contains("resume focus"));
    assert!(exported.contains("schema_version"));

    // Import into an empty store restores everything.
    let (_guard2, engine2) = setup().await;
    let result = engine2.import_json(&exported).await.unwrap();
    assert_eq!(result.imported, 1);
    assert_eq!(result.acceptance_restored, 1);
    let imported = engine2.repository.get(record.id).await.unwrap().unwrap();
    assert_eq!(imported.goal, record.goal);
    assert_eq!(engine2.acceptance().await.unwrap()[&record.id].accepted, 1);

    // Re-import is idempotent (skipped, no duplicates).
    let again = engine2.import_json(&exported).await.unwrap();
    assert_eq!(again.imported, 0);
    assert_eq!(again.skipped, 1);
    assert_eq!(engine2.repository.list_all().await.unwrap().len(), 1);
}

#[tokio::test]
async fn snapshots_create_list_and_restore() {
    let (_guard, engine) = setup().await;
    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume focus",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);
    engine.record_acceptance(record.id, true).await.unwrap();

    let snapshot = engine.create_snapshot(Some("pre-test")).await.unwrap();
    assert_eq!(snapshot.record_count, 1);
    let listed = engine.list_snapshots().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].label, "pre-test");

    // Wipe the store (simulate drift), then restore the snapshot.
    engine.expire(record.id).await.unwrap();
    engine.run_cleanup().await.unwrap();
    assert!(engine.repository.list_all().await.unwrap().is_empty());

    let restore = engine.restore_snapshot(snapshot.id).await.unwrap();
    assert_eq!(restore.records_restored, 1);
    assert_eq!(restore.acceptance_restored, 1);
    let restored = engine.repository.get(record.id).await.unwrap().unwrap();
    assert_eq!(restored.goal, "resume focus");
    assert_eq!(engine.acceptance().await.unwrap()[&record.id].accepted, 1);
    assert_eq!(engine.vector_system().index_len(), 1, "index rebuilt");
}

#[tokio::test]
async fn storage_stats_report_sizes_and_counts() {
    let (_guard, engine) = setup().await;
    let stats = engine.storage_stats().await.unwrap();
    assert!(stats.database_size_bytes > 0);
    assert_eq!(stats.permanent_memories, 0);
    assert_eq!(stats.archived_memories, 0);
    assert_eq!(stats.expired_memories, 0);
    assert_eq!(stats.snapshots, 0);

    engine
        .record_execution(
            Uuid::new_v4(),
            None,
            "resume focus",
            None,
            &[step(Uuid::new_v4(), "a", true)],
            ExecutionStatus::Completed,
            None,
            None,
        )
        .await
        .unwrap();
    let record = engine.repository.list_all().await.unwrap().remove(0);
    engine.archive(record.id).await.unwrap();

    let stats = engine.storage_stats().await.unwrap();
    assert_eq!(stats.permanent_memories, 0);
    assert_eq!(stats.archived_memories, 1);

    engine.create_snapshot(Some("manual")).await.unwrap();
    let stats = engine.storage_stats().await.unwrap();
    assert_eq!(stats.snapshots, 1);
    assert!(stats.snapshot_size_bytes > 0);
}

#[tokio::test]
async fn cleanup_removes_duplicate_archives_and_compresses() {
    let (_guard, engine) = setup().await;
    // Two identical archived records + a live record.
    for goal in ["resume focus", "resume focus", "resume focus"] {
        engine
            .record_execution(
                Uuid::new_v4(),
                None,
                goal,
                None,
                &[step(Uuid::new_v4(), "a", true)],
                ExecutionStatus::Completed,
                None,
                None,
            )
            .await
            .unwrap();
    }
    let all = engine.repository.list_all().await.unwrap();
    assert_eq!(all.len(), 3);
    engine.archive(all[0].id).await.unwrap();
    engine.archive(all[1].id).await.unwrap();

    // Make one record compressible and archive another duplicate.
    let mut compressible = all[2].clone();
    compressible.goal = "deep research".into();
    compressible.source_id = Uuid::new_v4();
    compressible.id = Uuid::new_v4();
    compressible.kind = MemoryKind::AutonomousSession;
    compressible.reasoning = (0..90).map(|i| format!("event {i}")).collect();
    engine.repository.upsert(&compressible).await.unwrap();

    let report = engine.run_cleanup().await.unwrap();
    assert_eq!(
        report.removed_duplicate_archives, 2,
        "live copy wins: every archived duplicate is removed"
    );
    assert_eq!(report.compressed, 1, "oversized history compressed");

    let remaining = engine.repository.list_all().await.unwrap();
    let kept_archived = remaining
        .iter()
        .filter(|r| matches!(r.retention, RetentionPolicy::Archived))
        .count();
    let compressed = remaining
        .iter()
        .filter(|r| r.compressed_at.is_some())
        .count();
    assert_eq!(kept_archived, 0, "the live copy is the one that survives");
    assert_eq!(compressed, 1);
    let _ = MemoryKind::Execution;
    let _ = MemoryStatus::Success;
}
