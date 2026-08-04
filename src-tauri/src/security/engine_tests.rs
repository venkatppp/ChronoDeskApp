//! Engine-level tests: the `SecurityEngine` facade composed over real
//! repositories, the validator, and an in-memory secret store.

use std::sync::Arc;

use crate::database::test_database;
use crate::llm::InMemorySecretStore;
use crate::models::security::SecurityRecommendationStatus;
use crate::repositories::{LLMRepository, MaintenanceRepository, SecurityRepository};
use crate::security::{policy, SecurityEngine};

/// An engine over a disposable temp database. The maintenance ledger and
/// backup dir stay empty, so backup checks pass trivially.
async fn build_engine() -> (tempfile::TempDir, SecurityEngine) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();

    let security_repository = SecurityRepository::new(pool.clone());
    let maintenance_repository = Arc::new(MaintenanceRepository::new(pool.clone()));
    let store = Arc::new(InMemorySecretStore::new());
    let llm_repository = Arc::new(LLMRepository::new(pool.clone(), store.clone()));

    let engine = SecurityEngine::new(
        security_repository,
        maintenance_repository,
        llm_repository,
        store,
        temp_dir.path().join("chronodesk.db"),
        temp_dir.path().join("backups"),
    );
    (temp_dir, engine)
}

#[tokio::test]
async fn status_with_no_history_is_a_full_score() {
    let (_temp, engine) = build_engine().await;

    let report = engine.status().await.expect("status");
    assert_eq!(report.score, 100.0);
    assert_eq!(report.status, "excellent");
    assert_eq!(report.total_checks, 0);
    assert!(report.findings.is_empty());
}

#[tokio::test]
async fn startup_validation_persists_findings_and_audits_the_run() {
    let (_temp, engine) = build_engine().await;

    let report = engine.startup_validation().await.expect("startup");
    assert_eq!(report.score, 100.0, "info advisory does not move the score");
    assert_eq!(
        report.failed_checks, 1,
        "only the secure_delete advisory fails"
    );
    assert!(!report.run_id.is_empty());
    assert!(report.total_checks >= 14, "broad battery expected");

    let history = engine.history(100).await.expect("history");
    assert!(!history.is_empty(), "findings persisted");
    assert!(
        history.iter().any(|f| f.run_id == report.run_id),
        "findings carry the startup run id"
    );

    let audit = engine.audit_log(10).await.expect("audit");
    assert!(
        audit.iter().any(|e| e.action == "startup_validation"),
        "startup pass is audited"
    );
}

#[tokio::test]
async fn status_replays_the_latest_run_into_a_score_report() {
    let (_temp, engine) = build_engine().await;

    engine.startup_validation().await.expect("startup");
    let report = engine.status().await.expect("status");

    assert_eq!(report.score, 100.0, "info advisory is score-neutral");
    assert_eq!(report.status, "excellent");
    assert_eq!(report.failed_checks, 1, "the secure_delete advisory fails");
    assert!(report.findings.len() >= 14);
}

#[tokio::test]
async fn diagnostics_runs_are_audited_and_never_fatal() {
    let (_temp, engine) = build_engine().await;

    let report = engine.diagnostics().await.expect("diagnostics");
    assert!(report.total_checks >= 14);
    assert!(report.db_path.contains("chronodesk.db"));

    let audit = engine.audit_log(10).await.expect("audit");
    assert!(audit.iter().any(|e| e.action == "diagnostics_run"));
}

#[tokio::test]
async fn secrets_and_permissions_sub_batteries_are_coherent() {
    let (_temp, engine) = build_engine().await;

    let secrets = engine.secrets().await.expect("secrets");
    assert_eq!(secrets.checks.len(), 2);
    assert!(secrets.ok);

    let permissions = engine.permissions().await.expect("permissions");
    assert!(permissions
        .checks
        .iter()
        .any(|c| c.check_name == "db_file_permissions"));
}

#[tokio::test]
async fn config_set_validates_before_persisting() {
    let (_temp, engine) = build_engine().await;

    engine
        .set_config(policy::KEY_MONITOR_INTERVAL_SECONDS, "600")
        .await
        .expect("valid value");
    assert_eq!(engine.monitor_interval_seconds().await, 600);

    let err = engine
        .set_config(policy::KEY_MONITOR_INTERVAL_SECONDS, "fast")
        .await
        .expect_err("invalid value rejected");
    assert!(err.to_string().contains("seconds"), "{err}");

    let err = engine
        .set_config("security.not_a_key", "1")
        .await
        .expect_err("unknown key rejected");
    assert!(err.to_string().contains("unknown"), "{err}");

    let all = engine.config().await.expect("config");
    assert!(
        all.iter()
            .any(|c| c.key == policy::KEY_MONITOR_INTERVAL_SECONDS),
        "the accepted value is listed"
    );
}

#[tokio::test]
async fn recommendations_apply_and_dismiss_round_trip() {
    let (_temp, engine) = build_engine().await;

    engine.startup_validation().await.expect("startup");
    let recommendations = engine.recommendations().await.expect("recommendations");
    assert!(!recommendations.is_empty(), "a battery produces candidates");
    let first = &recommendations[0];
    assert_eq!(first.status, SecurityRecommendationStatus::Open);

    let applied = engine.apply_recommendation(first.id).await.expect("apply");
    assert_eq!(applied.status, SecurityRecommendationStatus::Applied);
    assert!(engine
        .recommendations()
        .await
        .expect("list")
        .iter()
        .any(|r| r.id == first.id && r.status == SecurityRecommendationStatus::Applied));

    let dismissed = engine
        .dismiss_recommendation(first.id)
        .await
        .expect("dismiss");
    assert_eq!(dismissed.status, SecurityRecommendationStatus::Dismissed);
}

#[tokio::test]
async fn apply_unknown_recommendation_is_not_found() {
    let (_temp, engine) = build_engine().await;

    let err = engine
        .apply_recommendation(999_999)
        .await
        .expect_err("unknown id");
    assert!(err.to_string().contains("not found"), "{err}");
}

#[tokio::test]
async fn monitor_tick_produces_a_scored_report_and_prunes_ledgers() {
    let (_temp, engine) = build_engine().await;

    let report = engine.monitor_tick().await.expect("tick");
    assert_eq!(report.score, 100.0, "clean machine stays at 100");
    assert!(report.total_checks >= 14);

    let audit = engine.audit_log(10).await.expect("audit");
    assert!(audit.iter().any(|e| e.action == "monitor_tick"));
}

#[tokio::test]
async fn history_is_bounded_and_newest_first() {
    let (_temp, engine) = build_engine().await;

    engine.startup_validation().await.expect("startup");
    engine.monitor_tick().await.expect("tick");

    let limited = engine.history(2).await.expect("limited");
    assert_eq!(limited.len(), 2);
    assert!(
        limited[0].checked_at >= limited[1].checked_at,
        "newest first"
    );
}

#[tokio::test]
async fn audit_log_respects_the_limit() {
    let (_temp, engine) = build_engine().await;

    engine.startup_validation().await.expect("startup");
    engine.diagnostics().await.expect("diagnostics");
    engine.monitor_tick().await.expect("tick");

    let entries = engine.audit_log(2).await.expect("audit");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].action, "monitor_tick");
}
