//! Micro-benchmark engine (RC-10 M1).
//!
//! [`BenchmarkEngine`] measures representative read-only operations of
//! the five subsystems — planner, execution, memory, graph, vector —
//! by wall-clock over a fixed iteration count, persists every result to
//! the `benchmark_runs` ledger, and returns a [`BenchmarkSuiteResult`]
//! for the frontend. Nothing here writes application data: every
//! benchmarked operation is a pure read path so running a suite is
//! always side-effect free for the user's workspaces.

use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde_json::{json, Value};

use crate::copilot::execution_engine::ExecutionEngine;
use crate::copilot::memory::models::MemorySearchRequest;
use crate::copilot::memory::MemoryEngine;
use crate::copilot::planner::Planner;
use crate::errors::DatabaseError;
use crate::graph::GraphEngine;
use crate::models::performance::{BenchmarkCategory, BenchmarkResult, BenchmarkSuiteResult};
use crate::repositories::PerformanceRepository;
use crate::semantic::models::SemanticSearchRequest;
use crate::semantic::SemanticSearchEngine;

/// Iterations per micro-benchmark (small so a suite run stays snappy).
const ITERATIONS: u32 = 5;

/// Runs one benchmark suite and persists its results.
#[derive(Clone)]
pub struct BenchmarkEngine {
    repository: PerformanceRepository,
    planner: Option<Arc<Planner>>,
    execution_engine: Option<Arc<ExecutionEngine>>,
    memory_engine: Option<Arc<MemoryEngine>>,
    graph_engine: Option<GraphEngine>,
    semantic_search: Option<SemanticSearchEngine>,
}

impl BenchmarkEngine {
    /// Creates the engine; each subsystem is optional so the engine can
    /// be wired incrementally and constructed in tests without the full
    /// application stack.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repository: PerformanceRepository,
        planner: Option<Arc<Planner>>,
        execution_engine: Option<Arc<ExecutionEngine>>,
        memory_engine: Option<Arc<MemoryEngine>>,
        graph_engine: Option<GraphEngine>,
        semantic_search: Option<SemanticSearchEngine>,
    ) -> Self {
        Self {
            repository,
            planner,
            execution_engine,
            memory_engine,
            graph_engine,
            semantic_search,
        }
    }

    /// Attaches the graph engine for the graph benchmark suite.
    pub fn with_graph_engine(mut self, graph_engine: GraphEngine) -> Self {
        self.graph_engine = Some(graph_engine);
        self
    }

    /// Runs every registered suite and returns the combined result.
    pub async fn run_all(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let started = Instant::now();
        let mut benchmarks = Vec::new();
        for suite in [
            self.run_planner_suite().await?,
            self.run_execution_suite().await?,
            self.run_memory_suite().await?,
            self.run_graph_suite().await?,
            self.run_vector_suite().await?,
        ] {
            benchmarks.extend(suite.benchmarks);
        }
        Ok(BenchmarkSuiteResult {
            suite_name: "all".to_string(),
            benchmarks,
            total_duration_ms: started.elapsed().as_millis() as u64,
            ran_at: Utc::now(),
        })
    }

    /// Runs one suite by category (or every suite when `None`).
    pub async fn run(
        &self,
        category: Option<BenchmarkCategory>,
    ) -> Result<BenchmarkSuiteResult, DatabaseError> {
        match category {
            Some(BenchmarkCategory::Planner) => self.run_planner_suite().await,
            Some(BenchmarkCategory::Execution) => self.run_execution_suite().await,
            Some(BenchmarkCategory::Memory) => self.run_memory_suite().await,
            Some(BenchmarkCategory::Graph) => self.run_graph_suite().await,
            Some(BenchmarkCategory::Vector) => self.run_vector_suite().await,
            None => self.run_all().await,
        }
    }

    // ------------------------------------------------------------------
    // Suites
    // ------------------------------------------------------------------

    /// Planning pipeline: tool discovery + deterministic plan DAG build.
    async fn run_planner_suite(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let mut benchmarks = Vec::new();
        if let Some(planner) = &self.planner {
            let planner = planner.clone();
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Planner,
                    "planner_plan",
                    "plan",
                    || async {
                        planner
                            .plan(None, None, "Summarize recent workspace activity")
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
        } else {
            benchmarks.push(
                self.skipped(
                    BenchmarkCategory::Planner,
                    "planner_plan",
                    "plan",
                    "planner not wired",
                )
                .await?,
            );
        }
        Ok(self.suite("planner", benchmarks))
    }

    /// Execution pipeline: recent execution listing (pure read path).
    async fn run_execution_suite(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let mut benchmarks = Vec::new();
        if let Some(execution_engine) = &self.execution_engine {
            let execution_engine = execution_engine.clone();
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Execution,
                    "execution_list_recent",
                    "list_recent",
                    || async {
                        execution_engine
                            .list_recent(20)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
        } else {
            benchmarks.push(
                self.skipped(
                    BenchmarkCategory::Execution,
                    "execution_list_recent",
                    "list_recent",
                    "execution engine not wired",
                )
                .await?,
            );
        }
        Ok(self.suite("execution", benchmarks))
    }

    /// Memory pipeline: goal search over the execution memory store.
    async fn run_memory_suite(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let mut benchmarks = Vec::new();
        if let Some(memory_engine) = &self.memory_engine {
            let memory_engine = memory_engine.clone();
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Memory,
                    "memory_search",
                    "search",
                    || async {
                        let request = MemorySearchRequest {
                            query: "handle git rebase conflicts".to_string(),
                            kind: None,
                            workspace_id: None,
                            status: None,
                            limit: 10,
                        };
                        memory_engine
                            .search(&request)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
        } else {
            benchmarks.push(
                self.skipped(
                    BenchmarkCategory::Memory,
                    "memory_search",
                    "search",
                    "memory engine not wired",
                )
                .await?,
            );
        }
        Ok(self.suite("memory", benchmarks))
    }

    /// Graph pipeline: pagination, count, ranked search, memory stats.
    async fn run_graph_suite(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let mut benchmarks = Vec::new();
        if let Some(graph_engine) = &self.graph_engine {
            let graph_engine = graph_engine.clone();
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Graph,
                    "graph_nodes_page",
                    "nodes_page",
                    || async {
                        graph_engine
                            .graph_nodes_page(None, None, 0, Some(50))
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Graph,
                    "graph_nodes_total",
                    "nodes_total",
                    || async {
                        graph_engine
                            .graph_nodes_total(None, None)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Graph,
                    "graph_ranked_search",
                    "ranked_search",
                    || async {
                        graph_engine
                            .graph_ranked_search("contextsphere", None, Some(10))
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Graph,
                    "graph_memory_stats",
                    "memory_stats",
                    || async {
                        graph_engine
                            .graph_memory_stats()
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
        } else {
            benchmarks.push(
                self.skipped(
                    BenchmarkCategory::Graph,
                    "graph_nodes_page",
                    "nodes_page",
                    "graph engine not wired",
                )
                .await?,
            );
        }
        Ok(self.suite("graph", benchmarks))
    }

    /// Vector pipeline: semantic search over indexed documents.
    async fn run_vector_suite(&self) -> Result<BenchmarkSuiteResult, DatabaseError> {
        let mut benchmarks = Vec::new();
        if let Some(semantic_search) = &self.semantic_search {
            let semantic_search = semantic_search.clone();
            benchmarks.push(
                self.measure(
                    BenchmarkCategory::Vector,
                    "semantic_search",
                    "search",
                    || async {
                        let request = SemanticSearchRequest {
                            query: "workspace timeline activity".to_string(),
                            ..SemanticSearchRequest::default()
                        };
                        semantic_search
                            .search(request)
                            .await
                            .map(|_| ())
                            .map_err(|e| e.to_string())
                    },
                )
                .await?,
            );
        } else {
            benchmarks.push(
                self.skipped(
                    BenchmarkCategory::Vector,
                    "semantic_search",
                    "search",
                    "semantic search not wired",
                )
                .await?,
            );
        }
        Ok(self.suite("vector", benchmarks))
    }

    // ------------------------------------------------------------------
    // Measurement helpers
    // ------------------------------------------------------------------

    /// Runs `op` `ITERATIONS` times, averages the wall-clock duration,
    /// persists, and returns the result. Failures are recorded as
    /// `ok = false` (with the error in `payload`) so a flaky operation
    /// still shows up in history.
    async fn measure<F, Fut>(
        &self,
        category: BenchmarkCategory,
        name: &str,
        operation: &str,
        mut op: F,
    ) -> Result<BenchmarkResult, DatabaseError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(), String>>,
    {
        let mut durations = Vec::with_capacity(ITERATIONS as usize);
        let mut last_error: Option<String> = None;
        for _ in 0..ITERATIONS {
            let started = Instant::now();
            match op().await {
                Ok(()) => durations.push(started.elapsed()),
                Err(error) => last_error = Some(error),
            }
        }

        let ok = last_error.is_none();
        let mean_ms = if durations.is_empty() {
            0
        } else {
            durations.iter().map(|d| d.as_millis() as u64).sum::<u64>() / durations.len() as u64
        };
        let throughput = if ok && mean_ms > 0 {
            Some(1000.0 / mean_ms as f64)
        } else {
            None
        };
        let payload = match last_error {
            Some(error) => {
                json!({ "error": error, "samples_ms": durations.iter().map(|d| d.as_millis() as u64).collect::<Vec<_>>() })
            }
            None => {
                json!({ "samples_ms": durations.iter().map(|d| d.as_millis() as u64).collect::<Vec<_>>() })
            }
        };

        let result = BenchmarkResult {
            id: 0,
            name: name.to_string(),
            operation: operation.to_string(),
            category,
            iterations: ITERATIONS,
            duration_ms: mean_ms,
            throughput_per_sec: throughput,
            ok,
            payload,
            created_at: Utc::now(),
        };
        self.repository
            .record_benchmark(category.as_str(), &result)
            .await?;
        Ok(result)
    }

    /// A suite entry for a subsystem that was not wired into the engine;
    /// persisted like a measured benchmark so history shows the gap.
    async fn skipped(
        &self,
        category: BenchmarkCategory,
        name: &str,
        operation: &str,
        reason: &str,
    ) -> Result<BenchmarkResult, DatabaseError> {
        let result = BenchmarkResult {
            id: 0,
            name: name.to_string(),
            operation: operation.to_string(),
            category,
            iterations: 0,
            duration_ms: 0,
            throughput_per_sec: None,
            ok: false,
            payload: Value::String(reason.to_string()),
            created_at: Utc::now(),
        };
        self.repository
            .record_benchmark(category.as_str(), &result)
            .await?;
        Ok(result)
    }

    fn suite(&self, name: &str, benchmarks: Vec<BenchmarkResult>) -> BenchmarkSuiteResult {
        BenchmarkSuiteResult {
            suite_name: name.to_string(),
            total_duration_ms: benchmarks.iter().map(|b| b.duration_ms).sum(),
            benchmarks,
            ran_at: Utc::now(),
        }
    }

    /// Most recent persisted benchmark results (history panel).
    pub async fn recent(&self, limit: u32) -> Result<Vec<BenchmarkResult>, DatabaseError> {
        self.repository.recent_benchmarks(limit).await
    }
}

#[cfg(test)]
#[path = "benchmark_tests.rs"]
mod tests;
