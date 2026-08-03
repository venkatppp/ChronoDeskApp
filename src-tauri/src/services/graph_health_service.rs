//! Graph Health service (RC-8 M4).
//!
//! Business logic for the knowledge graph's operational health,
//! composing [`KgOptRepository`](crate::repositories::KgOptRepository)
//! (integrity scans, repair SQL, operational ledger) with
//! [`KgOptService`](crate::services::KgOptService) (paginated/ranked/
//! vector/traversal surfaces used by the benchmark suite) and
//! [`KgLiveRepository`](crate::repositories::KgLiveRepository)
//! (query-cache invalidation after graph writes):
//!
//! - **Integrity checks** — the four scans (orphan edges, dangling
//!   workspaces, malformed nodes, invalid confidence), persisted as
//!   open issues with dedup.
//! - **Repair** — deletes orphan edges/dangling nodes, fixes or drops
//!   malformed nodes, clamps out-of-range confidence, resolves the
//!   corresponding issues.
//! - **Orphan detection + cleanup** — read-only bookkeeping and the
//!   destructive pass.
//! - **Consistency verification** — five pass/fail probes for the
//!   diagnostics panel.
//! - **Maintenance runs + benchmark suite + query metrics** — the
//!   persisted operational ledger behind the performance dashboard.
//!
//! All SQL lives in repositories; every repair/verification policy and
//! benchmark lives here.

use std::str::FromStr;
use std::time::Instant;

use chrono::Utc;
use uuid::Uuid;

use crate::errors::DatabaseError;
use crate::models::kg::GraphNodeType;
use crate::models::kg_opt::{
    BenchmarkSuiteResult, ConsistencyCheck, ConsistencyReport, GraphBenchmarkResult,
    GraphDiagnostics, IntegrityCheckResult, IssueSeverity, IssueType, MaintenanceRun,
    OrphanCleanupResult, OrphanSummary, QueryMetric, RepairResult,
};
use crate::repositories::{KgLiveRepository, KgOptRepository};
use crate::services::KgOptService;

/// Maintenance run types recorded in `graph_maintenance_runs`.
const RUN_INTEGRITY: &str = "integrity_check";
const RUN_REPAIR: &str = "repair";
const RUN_ORPHAN_CLEANUP: &str = "orphan_cleanup";
const RUN_CONSISTENCY: &str = "consistency";
const RUN_BENCHMARK: &str = "benchmark";

/// Default page sizes used by the benchmark suite.
const BENCH_PAGE_SIZE: u32 = 50;
/// Max roots sampled into the parallel-traversal benchmark.
const BENCH_TRAVERSAL_ROOTS: usize = 4;

/// Graph Health service.
#[derive(Clone)]
pub struct GraphHealthService {
    kg_opt: KgOptService,
    live: KgLiveRepository,
    opt: KgOptRepository,
}

impl GraphHealthService {
    pub fn new(kg_opt: KgOptService, live: KgLiveRepository, opt: KgOptRepository) -> Self {
        Self { kg_opt, live, opt }
    }

    // ------------------------------------------------------------------
    // Integrity checks
    // ------------------------------------------------------------------

    /// Runs the four integrity scans, persists the new findings
    /// (deduplicated against open issues), and records a maintenance
    /// run. Returns the current open issues + per-type counts.
    pub async fn integrity_check(&self) -> Result<IntegrityCheckResult, DatabaseError> {
        let started = Instant::now();

        let orphan_ids = self.opt.orphan_edge_ids().await?;
        let dangling = self.opt.dangling_workspace_nodes().await?;
        let malformed = self.opt.malformed_nodes().await?;
        let invalid = self.opt.invalid_confidence_edges().await?;

        // Dedup against issues already open: a repeated scan must not
        // duplicate rows for the same (type, entity).
        let mut open: std::collections::HashSet<(IssueType, Option<Uuid>)> = self
            .opt
            .open_issues(10_000)
            .await?
            .into_iter()
            .map(|issue| (issue.issue_type, issue.entity_id))
            .collect();

        for id in &orphan_ids {
            if open.insert((IssueType::OrphanEdge, Some(*id))) {
                self.opt
                    .insert_issue(
                        IssueType::OrphanEdge,
                        IssueSeverity::Critical,
                        None,
                        Some(*id),
                        "Edge references a node that no longer exists".to_string(),
                    )
                    .await?;
            }
        }
        for node in &dangling {
            if open.insert((IssueType::DanglingWorkspace, Some(node.entity_id))) {
                self.opt
                    .insert_issue(
                        IssueType::DanglingWorkspace,
                        IssueSeverity::Warning,
                        Some(node.node_type.as_str()),
                        Some(node.entity_id),
                        format!("Workspace-linked node '{}' has no workspace", node.title),
                    )
                    .await?;
            }
        }
        for (node_type, entity_id, title, _summary) in &malformed {
            if open.insert((IssueType::MalformedNode, Some(*entity_id))) {
                let severity = if GraphNodeType::from_str(node_type).is_ok() {
                    IssueSeverity::Warning
                } else {
                    IssueSeverity::Critical
                };
                self.opt
                    .insert_issue(
                        IssueType::MalformedNode,
                        severity,
                        Some(node_type),
                        Some(*entity_id),
                        format!(
                            "Node '{}' has an empty title/summary or unknown type",
                            title
                        ),
                    )
                    .await?;
            }
        }
        for edge in &invalid {
            if open.insert((IssueType::InvalidConfidence, Some(edge.id))) {
                self.opt
                    .insert_issue(
                        IssueType::InvalidConfidence,
                        IssueSeverity::Info,
                        None,
                        Some(edge.id),
                        format!(
                            "Edge confidence {:.3} or weight {:.3} outside [0, 1]",
                            edge.confidence, edge.weight
                        ),
                    )
                    .await?;
            }
        }

        let issues = self.opt.open_issues(200).await?;
        let issue_type_counts = self.opt.open_issue_type_counts().await?;
        let issues_found = orphan_ids.len() as u64
            + dangling.len() as u64
            + malformed.len() as u64
            + invalid.len() as u64;

        self.record_maintenance(
            RUN_INTEGRITY,
            issues_found,
            0,
            started,
            serde_json::json!({
                "orphan_edges": orphan_ids.len(),
                "dangling_workspaces": dangling.len(),
                "malformed_nodes": malformed.len(),
                "invalid_confidence": invalid.len(),
            }),
        )
        .await;

        self.record_metric("integrity_check", None, started, issues.len() as u64)
            .await;

        Ok(IntegrityCheckResult {
            issues,
            issue_type_counts,
            checked_at: Utc::now(),
        })
    }

    // ------------------------------------------------------------------
    // Repair
    // ------------------------------------------------------------------

    /// Repairs every detectable problem: removes orphan edges, removes
    /// dangling workspace nodes, fixes (or drops) malformed nodes,
    /// clamps out-of-range confidence, resolves the affected open
    /// issues, and invalidates the query cache.
    pub async fn repair(&self) -> Result<RepairResult, DatabaseError> {
        let started = Instant::now();

        let orphan_ids = self.opt.orphan_edge_ids().await?;
        let orphan_edges_removed = self.opt.delete_edges(&orphan_ids).await?;

        let dangling = self.opt.dangling_workspace_nodes().await?;
        let mut dangling_removed = 0u64;
        let dangling_ids: Vec<Uuid> = dangling.iter().map(|node| node.entity_id).collect();
        for node in &dangling {
            if self
                .opt
                .delete_node(node.node_type.as_str(), node.entity_id)
                .await?
            {
                dangling_removed += 1;
            }
        }

        let malformed = self.opt.malformed_nodes().await?;
        let mut malformed_fixed = 0u64;
        let malformed_ids: Vec<Uuid> = malformed.iter().map(|(_, id, _, _)| *id).collect();
        for (node_type, entity_id, _title, _summary) in &malformed {
            if let Ok(parsed) = GraphNodeType::from_str(node_type) {
                if self.opt.fix_malformed_node(parsed, *entity_id).await? {
                    malformed_fixed += 1;
                }
            } else {
                // Unknown node types cannot be decoded — drop the row.
                let _ = self.opt.delete_node(node_type, *entity_id).await?;
            }
        }

        let invalid = self.opt.invalid_confidence_edges().await?;
        let mut invalid_fixed = 0u64;
        let invalid_ids: Vec<Uuid> = invalid.iter().map(|edge| edge.id).collect();
        for edge in &invalid {
            if self.opt.clamp_edge_values(edge.id).await? {
                invalid_fixed += 1;
            }
        }

        let issues_resolved = self
            .opt
            .resolve_issues(IssueType::OrphanEdge, &orphan_ids)
            .await?
            + self
                .opt
                .resolve_issues(IssueType::DanglingWorkspace, &dangling_ids)
                .await?
            + self
                .opt
                .resolve_issues(IssueType::MalformedNode, &malformed_ids)
                .await?
            + self
                .opt
                .resolve_issues(IssueType::InvalidConfidence, &invalid_ids)
                .await?;

        let _ = self.live.query_cache_clear().await?;

        let result = RepairResult {
            orphan_edges_removed,
            dangling_workspaces_removed: dangling_removed,
            malformed_nodes_fixed: malformed_fixed,
            invalid_confidence_fixed: invalid_fixed,
            issues_resolved,
        };
        self.record_maintenance(
            RUN_REPAIR,
            0,
            issues_resolved,
            started,
            serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Orphan detection + cleanup
    // ------------------------------------------------------------------

    /// Read-only orphan bookkeeping (edges + dangling workspace nodes).
    pub async fn orphan_summary(&self) -> Result<OrphanSummary, DatabaseError> {
        let started = Instant::now();
        let orphan_edges = self.opt.orphan_edge_ids().await?.len() as u64;
        let dangling_workspaces = self.opt.dangling_workspace_nodes().await?.len() as u64;
        self.record_metric("orphan_summary", None, started, orphan_edges)
            .await;
        Ok(OrphanSummary {
            orphan_edges,
            dangling_workspaces,
        })
    }

    /// Removes every orphan edge and dangling workspace node, resolves
    /// the corresponding open issues, invalidates the cache, and records
    /// a maintenance run.
    pub async fn orphan_cleanup(&self) -> Result<OrphanCleanupResult, DatabaseError> {
        let started = Instant::now();

        let orphan_ids = self.opt.orphan_edge_ids().await?;
        let orphan_edges_removed = self.opt.delete_edges(&orphan_ids).await?;

        let dangling = self.opt.dangling_workspace_nodes().await?;
        let dangling_ids: Vec<Uuid> = dangling.iter().map(|node| node.entity_id).collect();
        let mut dangling_removed = 0u64;
        for node in &dangling {
            if self
                .opt
                .delete_node(node.node_type.as_str(), node.entity_id)
                .await?
            {
                dangling_removed += 1;
            }
        }

        let issues_resolved = self
            .opt
            .resolve_issues(IssueType::OrphanEdge, &orphan_ids)
            .await?
            + self
                .opt
                .resolve_issues(IssueType::DanglingWorkspace, &dangling_ids)
                .await?;
        let _ = self.live.query_cache_clear().await?;

        let result = OrphanCleanupResult {
            orphan_edges_removed,
            dangling_workspaces_removed: dangling_removed,
            issues_resolved,
        };
        self.record_maintenance(
            RUN_ORPHAN_CLEANUP,
            orphan_ids.len() as u64 + dangling.len() as u64,
            issues_resolved,
            started,
            serde_json::to_value(&result).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(result)
    }

    // ------------------------------------------------------------------
    // Consistency verification
    // ------------------------------------------------------------------

    /// Runs the five consistency probes and records a maintenance run.
    pub async fn consistency_report(&self) -> Result<ConsistencyReport, DatabaseError> {
        let started = Instant::now();

        let orphan_edges = self.opt.orphan_edge_ids().await?.len() as u64;
        let dangling = self.opt.dangling_workspace_nodes().await?.len() as u64;
        let malformed = self.opt.malformed_nodes().await?.len() as u64;
        let invalid = self.opt.invalid_confidence_edges().await?.len() as u64;
        let duplicates = self.opt.duplicate_node_count().await?;

        let checks = vec![
            ConsistencyCheck {
                name: "Node uniqueness".into(),
                passed: duplicates == 0,
                detail: format!("{duplicates} duplicate (type, id) pairs"),
            },
            ConsistencyCheck {
                name: "Forward references".into(),
                passed: orphan_edges == 0,
                detail: format!("{orphan_edges} edges reference a missing node"),
            },
            ConsistencyCheck {
                name: "Workspace references".into(),
                passed: dangling == 0,
                detail: format!("{dangling} nodes reference a missing workspace"),
            },
            ConsistencyCheck {
                name: "Node well-formedness".into(),
                passed: malformed == 0,
                detail: format!("{malformed} nodes with empty/unknown fields"),
            },
            ConsistencyCheck {
                name: "Confidence bounds".into(),
                passed: invalid == 0,
                detail: format!("{invalid} edges with out-of-range confidence"),
            },
        ];
        let passed = checks.iter().all(|check| check.passed);

        self.record_maintenance(
            RUN_CONSISTENCY,
            checks.iter().filter(|check| !check.passed).count() as u64,
            0,
            started,
            serde_json::to_value(&checks).unwrap_or_else(|_| serde_json::json!({})),
        )
        .await;
        Ok(ConsistencyReport {
            checks,
            passed,
            checked_at: Utc::now(),
        })
    }

    // ------------------------------------------------------------------
    // Maintenance history
    // ------------------------------------------------------------------

    /// Most recent maintenance runs, newest first.
    pub async fn recent_maintenance_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<MaintenanceRun>, DatabaseError> {
        self.opt.recent_maintenance_runs(limit).await
    }

    /// Drops every cached query entry past its own TTL (the background
    /// sweep; returns rows removed).
    pub async fn sweep_expired_cache(&self) -> Result<u64, DatabaseError> {
        let removed = self.live.cache_clear_expired().await?;
        tracing::info!(removed, "expired graph cache entries swept");
        Ok(removed)
    }

    // ------------------------------------------------------------------
    // Benchmark suite
    // ------------------------------------------------------------------

    /// Runs the micro-benchmark suite over the real service surfaces
    /// (node/edge/neighbor pagination, ranked search, vector search,
    /// memory stats, cache stats, parallel traversal), persists every
    /// result, and records a `benchmark` maintenance run.
    pub async fn benchmark_suite(
        &self,
        suite_name: Option<String>,
    ) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let started = Instant::now();
        let suite_name =
            suite_name.unwrap_or_else(|| format!("suite_{}", Utc::now().format("%Y%m%d%H%M%S")));
        let memory = self.kg_opt.memory_stats().await?;
        let mut benchmarks = Vec::new();

        // 1. Node page (50).
        let timed = Instant::now();
        let page = self
            .kg_opt
            .nodes_page(None, None, 0, Some(BENCH_PAGE_SIZE))
            .await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "nodes_page_50",
            "paginate_nodes",
            memory.node_count,
            memory.edge_count,
            page.nodes.len() as u64,
            timed,
        )
        .await;

        // 2. Edge page (50).
        let timed = Instant::now();
        let edge_page = self.kg_opt.edges_page(0, Some(BENCH_PAGE_SIZE)).await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "edges_page_50",
            "paginate_edges",
            memory.node_count,
            memory.edge_count,
            edge_page.edges.len() as u64,
            timed,
        )
        .await;

        // 3. Neighbor page for the newest node (or a miss when empty).
        let timed = Instant::now();
        let anchor = page.nodes.first();
        let neighbors = match anchor {
            Some(node) => {
                self.kg_opt
                    .neighbors_page(node.node_type, node.entity_id, 0, Some(BENCH_PAGE_SIZE))
                    .await?
            }
            None => crate::models::kg_opt::NeighborPage {
                neighbors: Vec::new(),
                total: 0,
                offset: 0,
                limit: BENCH_PAGE_SIZE,
                has_more: false,
            },
        };
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "neighbors_page_50",
            "paginate_neighbors",
            memory.node_count,
            memory.edge_count,
            neighbors.neighbors.len() as u64,
            timed,
        )
        .await;

        // 4. Ranked search ("a", top 10).
        let timed = Instant::now();
        let hits = self.kg_opt.ranked_search("a", None, Some(10)).await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "ranked_search_top10",
            "ranked_search",
            memory.node_count,
            memory.edge_count,
            hits.len() as u64,
            timed,
        )
        .await;

        // 5. Vector search (only when an embedder is attached).
        if self.kg_opt.vector_search_available() {
            let timed = Instant::now();
            let hits = self.kg_opt.vector_search("query", None, Some(10)).await?;
            self.push_benchmark(
                &mut benchmarks,
                &suite_name,
                "vector_search_top10",
                "vector_search",
                memory.node_count,
                memory.edge_count,
                hits.len() as u64,
                timed,
            )
            .await;
        }

        // 6. Memory statistics.
        let timed = Instant::now();
        let _ = self.kg_opt.memory_stats().await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "memory_stats",
            "memory_stats",
            memory.node_count,
            memory.edge_count,
            1,
            timed,
        )
        .await;

        // 7. Cache statistics.
        let timed = Instant::now();
        let _ = self.kg_opt.cache_stats().await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "cache_stats",
            "cache_stats",
            memory.node_count,
            memory.edge_count,
            1,
            timed,
        )
        .await;

        // 8. Parallel multi-root traversal (up to 4 roots, depth 2).
        let timed = Instant::now();
        let roots: Vec<(GraphNodeType, Uuid)> = page
            .nodes
            .iter()
            .take(BENCH_TRAVERSAL_ROOTS)
            .map(|node| (node.node_type, node.entity_id))
            .collect();
        let walk = self
            .kg_opt
            .parallel_traversal(roots, Some(2), Some(200))
            .await?;
        self.push_benchmark(
            &mut benchmarks,
            &suite_name,
            "parallel_traversal_d2",
            "parallel_traversal",
            memory.node_count,
            memory.edge_count,
            walk.node_count,
            timed,
        )
        .await;

        let result = BenchmarkSuiteResult {
            suite_name: suite_name.clone(),
            total_duration_ms: started.elapsed().as_millis() as u64,
            ran_at: Utc::now(),
            benchmarks,
        };
        self.record_maintenance(
            RUN_BENCHMARK,
            result.benchmarks.len() as u64,
            0,
            started,
            serde_json::json!({ "suite": suite_name, "benchmarks": result.benchmarks.len() }),
        )
        .await;
        Ok(result)
    }

    /// Times, persists, and appends one benchmark result.
    #[allow(clippy::too_many_arguments)]
    async fn push_benchmark(
        &self,
        benchmarks: &mut Vec<GraphBenchmarkResult>,
        suite_name: &str,
        name: &str,
        operation: &str,
        node_count: u64,
        edge_count: u64,
        rows: u64,
        timed: Instant,
    ) {
        let duration_ms = timed.elapsed().as_millis() as u64;
        let throughput_per_sec = if duration_ms > 0 && rows > 0 {
            Some(rows * 1000 / duration_ms)
        } else {
            None
        };
        let result = GraphBenchmarkResult {
            name: name.to_string(),
            operation: operation.to_string(),
            node_count,
            edge_count,
            duration_ms,
            throughput_per_sec,
            suite_name: suite_name.to_string(),
            created_at: Utc::now(),
        };
        let _ = self
            .opt
            .insert_benchmark(
                suite_name,
                name,
                operation,
                node_count,
                edge_count,
                duration_ms,
                serde_json::json!({
                    "rows": rows,
                    "throughput_per_sec": throughput_per_sec,
                }),
            )
            .await;
        benchmarks.push(result);
    }

    // ------------------------------------------------------------------
    // Diagnostics bundle
    // ------------------------------------------------------------------

    /// The full performance/health bundle for the dashboard: a fresh
    /// integrity pass, consistency report, memory stats, and the recent
    /// maintenance/benchmark/metric history.
    pub async fn diagnostics(&self) -> Result<GraphDiagnostics, DatabaseError> {
        let integrity = self.integrity_check().await?;
        let consistency = self.consistency_report().await?;
        let memory = self.kg_opt.memory_stats().await?;
        let recent_maintenance = self.opt.recent_maintenance_runs(10).await?;
        let recent_benchmarks = self.opt.recent_benchmarks(10).await?;
        let recent_metrics = self.opt.recent_query_metrics(20).await?;
        Ok(GraphDiagnostics {
            integrity,
            consistency,
            memory,
            recent_maintenance,
            recent_benchmarks,
            recent_metrics,
        })
    }

    /// Most recent recorded operation metrics.
    pub async fn recent_metrics(&self, limit: u32) -> Result<Vec<QueryMetric>, DatabaseError> {
        self.opt.recent_query_metrics(limit).await
    }

    // ------------------------------------------------------------------
    // Ledger helpers
    // ------------------------------------------------------------------

    /// Records one maintenance run with pass timing.
    async fn record_maintenance(
        &self,
        run_type: &str,
        issues_found: u64,
        issues_resolved: u64,
        started: Instant,
        summary: serde_json::Value,
    ) {
        let duration_ms = started.elapsed().as_millis() as u64;
        let _ = self
            .opt
            .insert_maintenance_run(
                run_type,
                "completed",
                issues_found,
                issues_resolved,
                duration_ms,
                summary,
            )
            .await;
    }

    /// Records one operation metric via the shared ledger (best-effort).
    async fn record_metric(
        &self,
        operation: &str,
        scope: Option<String>,
        started: Instant,
        rows: u64,
    ) {
        self.kg_opt
            .record_metric(operation, scope, None, started, rows, false)
            .await;
    }
}

#[cfg(test)]
#[path = "graph_health_service_tests.rs"]
mod tests;
