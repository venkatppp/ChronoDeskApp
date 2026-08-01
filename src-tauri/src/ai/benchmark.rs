//! Benchmarking utilities for AI inference.

use serde::{Deserialize, Serialize};
use std::time::Instant;

/// Benchmark result for inference operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BenchmarkResult {
    pub operation: String,
    pub iterations: usize,
    pub total_duration_ms: f64,
    pub avg_latency_ms: f64,
    pub min_latency_ms: f64,
    pub max_latency_ms: f64,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub throughput_ops_per_sec: f64,
    pub memory_used_bytes: Option<u64>,
    pub memory_peak_bytes: Option<u64>,
}

/// Benchmark runner for AI operations.
pub struct Benchmark {
    operation: String,
    latencies: Vec<f64>,
    start_memory: Option<u64>,
    peak_memory: Option<u64>,
}

impl Benchmark {
    /// Creates a new benchmark.
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            latencies: Vec::new(),
            start_memory: Self::current_memory_usage(),
            peak_memory: None,
        }
    }

    /// Records a single iteration.
    pub fn record<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = Instant::now();
        let result = f();
        let elapsed = start.elapsed();

        self.latencies.push(elapsed.as_secs_f64() * 1000.0);

        // Track peak memory
        if let Some(current) = Self::current_memory_usage() {
            if let Some(peak) = self.peak_memory {
                self.peak_memory = Some(peak.max(current));
            } else {
                self.peak_memory = Some(current);
            }
        }

        result
    }

    /// Records an async iteration.
    pub async fn record_async<F, Fut, R>(&mut self, f: F) -> R
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = R>,
    {
        let start = Instant::now();
        let result = f().await;
        let elapsed = start.elapsed();

        self.latencies.push(elapsed.as_secs_f64() * 1000.0);

        // Track peak memory
        if let Some(current) = Self::current_memory_usage() {
            if let Some(peak) = self.peak_memory {
                self.peak_memory = Some(peak.max(current));
            } else {
                self.peak_memory = Some(current);
            }
        }

        result
    }

    /// Finishes the benchmark and returns results.
    pub fn finish(mut self) -> BenchmarkResult {
        if self.latencies.is_empty() {
            return BenchmarkResult {
                operation: self.operation,
                iterations: 0,
                total_duration_ms: 0.0,
                avg_latency_ms: 0.0,
                min_latency_ms: 0.0,
                max_latency_ms: 0.0,
                p50_latency_ms: 0.0,
                p95_latency_ms: 0.0,
                p99_latency_ms: 0.0,
                throughput_ops_per_sec: 0.0,
                memory_used_bytes: None,
                memory_peak_bytes: None,
            };
        }

        // Sort for percentile calculations
        self.latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let iterations = self.latencies.len();
        let total_duration_ms: f64 = self.latencies.iter().sum();
        let avg_latency_ms = total_duration_ms / iterations as f64;
        let min_latency_ms = self.latencies[0];
        let max_latency_ms = self.latencies[iterations - 1];

        let p50_latency_ms = self.percentile(50.0);
        let p95_latency_ms = self.percentile(95.0);
        let p99_latency_ms = self.percentile(99.0);

        let throughput_ops_per_sec = if total_duration_ms > 0.0 {
            (iterations as f64 * 1000.0) / total_duration_ms
        } else {
            0.0
        };

        let end_memory = Self::current_memory_usage();
        let memory_used_bytes = match (self.start_memory, end_memory) {
            (Some(start), Some(end)) if end > start => Some(end - start),
            _ => None,
        };

        BenchmarkResult {
            operation: self.operation,
            iterations,
            total_duration_ms,
            avg_latency_ms,
            min_latency_ms,
            max_latency_ms,
            p50_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            throughput_ops_per_sec,
            memory_used_bytes,
            memory_peak_bytes: self.peak_memory,
        }
    }

    /// Calculates a percentile from sorted latencies.
    fn percentile(&self, p: f64) -> f64 {
        let index = ((p / 100.0) * (self.latencies.len() - 1) as f64).round() as usize;
        self.latencies[index]
    }

    /// Gets current memory usage (RSS) in bytes.
    /// Returns None if not available on this platform.
    fn current_memory_usage() -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(statm) = fs::read_to_string("/proc/self/statm") {
                let parts: Vec<&str> = statm.split_whitespace().collect();
                if let Some(rss_pages) = parts.get(1) {
                    if let Ok(pages) = rss_pages.parse::<u64>() {
                        let page_size = 4096u64; // Standard page size on Linux
                        return Some(pages * page_size);
                    }
                }
            }
            None
        }

        #[cfg(target_os = "macos")]
        {
            // Use mach task_info on macOS
            use std::mem;

            extern "C" {
                fn mach_task_self() -> u32;
                fn task_info(
                    target_task: u32,
                    flavor: u32,
                    task_info_out: *mut u8,
                    task_info_count: *mut u32,
                ) -> i32;
            }

            const MACH_TASK_BASIC_INFO: u32 = 20;
            const MACH_TASK_BASIC_INFO_COUNT: u32 = 10;

            #[repr(C)]
            struct MachTaskBasicInfo {
                virtual_size: u64,
                resident_size: u64,
                resident_size_max: u64,
                user_time: [u32; 2],
                system_time: [u32; 2],
                policy: u32,
                suspend_count: u32,
            }

            unsafe {
                let mut info: MachTaskBasicInfo = mem::zeroed();
                let mut count = MACH_TASK_BASIC_INFO_COUNT;
                let result = task_info(
                    mach_task_self(),
                    MACH_TASK_BASIC_INFO,
                    &mut info as *mut _ as *mut u8,
                    &mut count,
                );

                if result == 0 {
                    return Some(info.resident_size);
                }
            }
            None
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            None
        }
    }
}

/// Runs a benchmark with multiple iterations.
pub fn run_benchmark<F, R>(operation: &str, iterations: usize, mut f: F) -> BenchmarkResult
where
    F: FnMut() -> R,
{
    let mut benchmark = Benchmark::new(operation);

    for _ in 0..iterations {
        benchmark.record(&mut f);
    }

    benchmark.finish()
}

/// Runs an async benchmark with multiple iterations.
pub async fn run_benchmark_async<F, Fut, R>(
    operation: &str,
    iterations: usize,
    mut f: F,
) -> BenchmarkResult
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = R>,
{
    let mut benchmark = Benchmark::new(operation);

    for _ in 0..iterations {
        benchmark.record_async(&mut f).await;
    }

    benchmark.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn benchmark_records_iterations() {
        let result = run_benchmark("test_op", 10, || {
            std::thread::sleep(Duration::from_millis(10));
        });

        assert_eq!(result.iterations, 10);
        assert!(result.avg_latency_ms >= 10.0);
        assert!(result.throughput_ops_per_sec > 0.0);
    }

    #[tokio::test]
    async fn async_benchmark_records_iterations() {
        let result = run_benchmark_async("test_async_op", 5, || async {
            tokio::time::sleep(Duration::from_millis(5)).await;
        })
        .await;

        assert_eq!(result.iterations, 5);
        assert!(result.avg_latency_ms >= 5.0);
    }

    #[test]
    fn benchmark_calculates_percentiles() {
        let mut benchmark = Benchmark::new("test");

        for i in 1..=100 {
            benchmark.latencies.push(i as f64);
        }
        benchmark
            .latencies
            .sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50 = benchmark.percentile(50.0);
        let p95 = benchmark.percentile(95.0);
        let p99 = benchmark.percentile(99.0);

        assert!((p50 - 50.5).abs() < 1.0);
        assert!((p95 - 95.0).abs() < 1.0);
        assert!((p99 - 99.0).abs() < 1.0);
    }
}
