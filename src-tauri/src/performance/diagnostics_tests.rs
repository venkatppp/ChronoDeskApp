//! Diagnostics tests (RC-10 M1): the snapshot carries machine,
//! database, cache, and worker facts without any subsystem attached,
//! and with the graph/runtime handles attached once the database is up.

use super::*;
use crate::database::test_database;
use crate::runtime::IntelligenceCache;

async fn setup() -> (Diagnostics, sqlx::SqlitePool, tempfile::TempDir) {
    let (database, temp_dir) = test_database().await;
    let pool = database.pool().clone();
    let diagnostics =
        Diagnostics::new(PerformanceRepository::new(pool.clone())).with_db_path("test.db");
    (diagnostics, pool, temp_dir)
}

#[tokio::test]
async fn capture_reports_machine_and_database_facts() {
    let (diagnostics, _pool, _guard) = setup().await;
    let snapshot = diagnostics.capture().await.unwrap();

    assert!(snapshot.cpu.cores >= 1);
    assert!(snapshot.cpu.cpu_parallelism >= 1);
    assert!((0.0..=100.0).contains(&snapshot.cpu.usage_percent));

    assert!(snapshot.memory.total_bytes > 0);
    assert!(snapshot.memory.used_bytes > 0);
    assert!((0.0..=100.0).contains(&snapshot.memory.percent));

    // A freshly migrated database is small but non-empty.
    assert!(snapshot.db.size_bytes > 0);
    assert_eq!(snapshot.db.path, "test.db");

    assert!(snapshot.threads.process_count >= 1);
    // Per-process thread counts depend on the OS: sysinfo only exposes
    // them on Linux/Windows (macOS reports 0). Either way the count is
    // bounded by a sane per-process ceiling.
    assert!(
        snapshot.threads.total_threads <= snapshot.threads.process_count.saturating_mul(10_000)
    );
}

#[tokio::test]
async fn capture_reports_cache_usage_when_wired() {
    let (database, _guard) = test_database().await;
    let cache = IntelligenceCache::new();
    let diagnostics = Diagnostics::new(PerformanceRepository::new(database.pool().clone()))
        .with_intelligence_cache(cache);
    let snapshot = diagnostics.capture().await.unwrap();
    assert_eq!(snapshot.cache.runtime_entries, 0);
    assert_eq!(snapshot.cache.graph_cache_entries, 0);
}

#[tokio::test]
async fn capture_without_handles_yields_empty_workers() {
    let (diagnostics, _pool, _guard) = setup().await;
    let snapshot = diagnostics.capture().await.unwrap();
    assert!(snapshot.workers.is_empty());
}

#[tokio::test]
async fn cpu_and_memory_usage_are_bounded() {
    let (diagnostics, _pool, _guard) = setup().await;
    let first = diagnostics.capture().await.unwrap();
    let second = diagnostics.capture().await.unwrap();
    assert!(second.captured_at >= first.captured_at);
    assert!((0.0..=100.0).contains(&second.cpu.usage_percent));
}
