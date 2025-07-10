//! Integration test modules for Monmouth SVM ExEx
//! 
//! This module contains comprehensive integration tests that validate
//! the end-to-end functionality of the integrated system.

pub mod cross_exex_tests;

// Re-export commonly used types for integration tests
pub use cross_exex_tests::{
    IntegrationTestConfig, IntegrationMetrics, MockExExInstance, IntegrationTestSuite,
};

/// Test utilities and helpers
pub mod utils {
    use std::time::Duration;
    use tracing_subscriber;

    /// Initialize test logging with appropriate filters
    pub fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug,hyper=warn,reqwest=warn")
            .with_test_writer()
            .try_init();
    }

    /// Initialize test logging with custom filter
    pub fn init_test_logging_with_filter(filter: &str) {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    }

    /// Create test timeout with reasonable defaults
    pub fn test_timeout() -> Duration {
        Duration::from_secs(30)
    }

    /// Create performance test timeout
    pub fn performance_timeout() -> Duration {
        Duration::from_secs(120)
    }

    /// Create stress test timeout
    pub fn stress_test_timeout() -> Duration {
        Duration::from_secs(300)
    }

    /// Validate latency meets SVM speed requirements
    pub fn validate_svm_latency(latency_ms: f64) -> bool {
        latency_ms < 50.0 // RAG queries must complete in <50ms
    }

    /// Validate memory operation latency
    pub fn validate_memory_latency(latency_ms: f64) -> bool {
        latency_ms < 10.0 // Memory operations must be <10ms
    }

    /// Validate communication latency
    pub fn validate_communication_latency(latency_ms: f64) -> bool {
        latency_ms < 5.0 // Cross-ExEx communication overhead must be <5ms
    }

    /// Calculate success rate
    pub fn calculate_success_rate(successful: u64, total: u64) -> f64 {
        if total == 0 {
            0.0
        } else {
            successful as f64 / total as f64
        }
    }

    /// Validate success rate meets requirements
    pub fn validate_success_rate(rate: f64) -> bool {
        rate >= 0.95 // Minimum 95% success rate
    }
}

/// Performance test categories
pub mod performance {
    use super::*;
    use std::time::{Duration, Instant};

    /// Performance test category
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TestCategory {
        /// RAG query performance
        RAGQuery,
        /// Memory operations
        Memory,
        /// Cross-ExEx communication
        Communication,
        /// AI decision making
        AIDecision,
        /// State synchronization
        StateSync,
        /// Batch processing
        BatchProcessing,
    }

    /// Performance test result
    #[derive(Debug, Clone)]
    pub struct PerformanceResult {
        /// Test category
        pub category: TestCategory,
        /// Average latency in milliseconds
        pub avg_latency_ms: f64,
        /// Maximum latency in milliseconds
        pub max_latency_ms: f64,
        /// Minimum latency in milliseconds
        pub min_latency_ms: f64,
        /// Success rate (0.0-1.0)
        pub success_rate: f64,
        /// Total operations performed
        pub total_operations: u64,
        /// Successful operations
        pub successful_operations: u64,
        /// Test duration
        pub duration: Duration,
        /// Throughput (operations per second)
        pub throughput: f64,
    }

    impl PerformanceResult {
        /// Check if result meets SVM speed requirements
        pub fn meets_requirements(&self) -> bool {
            match self.category {
                TestCategory::RAGQuery => utils::validate_svm_latency(self.avg_latency_ms),
                TestCategory::Memory => utils::validate_memory_latency(self.avg_latency_ms),
                TestCategory::Communication => utils::validate_communication_latency(self.avg_latency_ms),
                _ => self.avg_latency_ms < 100.0, // General requirement
            } && utils::validate_success_rate(self.success_rate)
        }

        /// Get performance grade
        pub fn get_grade(&self) -> PerformanceGrade {
            if !utils::validate_success_rate(self.success_rate) {
                return PerformanceGrade::F;
            }

            let latency_score = match self.category {
                TestCategory::RAGQuery => {
                    if self.avg_latency_ms < 25.0 { 4 }
                    else if self.avg_latency_ms < 35.0 { 3 }
                    else if self.avg_latency_ms < 45.0 { 2 }
                    else if self.avg_latency_ms < 50.0 { 1 }
                    else { 0 }
                }
                TestCategory::Memory => {
                    if self.avg_latency_ms < 2.0 { 4 }
                    else if self.avg_latency_ms < 5.0 { 3 }
                    else if self.avg_latency_ms < 8.0 { 2 }
                    else if self.avg_latency_ms < 10.0 { 1 }
                    else { 0 }
                }
                TestCategory::Communication => {
                    if self.avg_latency_ms < 1.0 { 4 }
                    else if self.avg_latency_ms < 2.0 { 3 }
                    else if self.avg_latency_ms < 3.0 { 2 }
                    else if self.avg_latency_ms < 5.0 { 1 }
                    else { 0 }
                }
                _ => {
                    if self.avg_latency_ms < 25.0 { 4 }
                    else if self.avg_latency_ms < 50.0 { 3 }
                    else if self.avg_latency_ms < 75.0 { 2 }
                    else if self.avg_latency_ms < 100.0 { 1 }
                    else { 0 }
                }
            };

            match latency_score {
                4 => PerformanceGrade::A,
                3 => PerformanceGrade::B,
                2 => PerformanceGrade::C,
                1 => PerformanceGrade::D,
                _ => PerformanceGrade::F,
            }
        }
    }

    /// Performance grade
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub enum PerformanceGrade {
        A, // Excellent
        B, // Good
        C, // Acceptable
        D, // Poor but passing
        F, // Failing
    }

    impl std::fmt::Display for PerformanceGrade {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                PerformanceGrade::A => write!(f, "A (Excellent)"),
                PerformanceGrade::B => write!(f, "B (Good)"),
                PerformanceGrade::C => write!(f, "C (Acceptable)"),
                PerformanceGrade::D => write!(f, "D (Poor)"),
                PerformanceGrade::F => write!(f, "F (Failing)"),
            }
        }
    }

    /// Performance benchmark runner
    pub struct BenchmarkRunner {
        /// Test configuration
        config: IntegrationTestConfig,
        /// Results
        results: Vec<PerformanceResult>,
    }

    impl BenchmarkRunner {
        /// Create new benchmark runner
        pub fn new(config: IntegrationTestConfig) -> Self {
            Self {
                config,
                results: Vec::new(),
            }
        }

        /// Run a performance benchmark
        pub async fn run_benchmark<F, Fut>(
            &mut self,
            category: TestCategory,
            operation: F,
        ) -> eyre::Result<PerformanceResult>
        where
            F: Fn() -> Fut,
            Fut: std::future::Future<Output = eyre::Result<()>>,
        {
            let start_time = Instant::now();
            let mut latencies = Vec::new();
            let mut successful = 0;

            for _ in 0..self.config.performance_iterations {
                let op_start = Instant::now();
                match operation().await {
                    Ok(_) => {
                        successful += 1;
                        latencies.push(op_start.elapsed().as_millis() as f64);
                    }
                    Err(_) => {
                        // Record failed operation with max latency
                        latencies.push(1000.0);
                    }
                }
            }

            let duration = start_time.elapsed();
            let total_operations = self.config.performance_iterations as u64;
            let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
            let max_latency = latencies.iter().fold(0.0, |acc, &x| acc.max(x));
            let min_latency = latencies.iter().fold(f64::INFINITY, |acc, &x| acc.min(x));
            let success_rate = successful as f64 / total_operations as f64;
            let throughput = total_operations as f64 / duration.as_secs_f64();

            let result = PerformanceResult {
                category,
                avg_latency_ms: avg_latency,
                max_latency_ms: max_latency,
                min_latency_ms: min_latency,
                success_rate,
                total_operations,
                successful_operations: successful,
                duration,
                throughput,
            };

            self.results.push(result.clone());
            Ok(result)
        }

        /// Get all results
        pub fn get_results(&self) -> &[PerformanceResult] {
            &self.results
        }

        /// Generate performance report
        pub fn generate_report(&self) -> String {
            let mut report = String::new();
            report.push_str("=== PERFORMANCE BENCHMARK REPORT ===\n\n");

            for result in &self.results {
                report.push_str(&format!(
                    "Category: {:?}\n\
                     Grade: {}\n\
                     Average Latency: {:.2}ms\n\
                     Max Latency: {:.2}ms\n\
                     Min Latency: {:.2}ms\n\
                     Success Rate: {:.2}%\n\
                     Throughput: {:.2} ops/sec\n\
                     Meets Requirements: {}\n\n",
                    result.category,
                    result.get_grade(),
                    result.avg_latency_ms,
                    result.max_latency_ms,
                    result.min_latency_ms,
                    result.success_rate * 100.0,
                    result.throughput,
                    result.meets_requirements()
                ));
            }

            let overall_grade = self.get_overall_grade();
            report.push_str(&format!("Overall Grade: {}\n", overall_grade));

            report
        }

        /// Get overall performance grade
        pub fn get_overall_grade(&self) -> PerformanceGrade {
            if self.results.is_empty() {
                return PerformanceGrade::F;
            }

            let failing_count = self.results.iter()
                .filter(|r| !r.meets_requirements())
                .count();

            if failing_count > 0 {
                return PerformanceGrade::F;
            }

            let grades: Vec<_> = self.results.iter()
                .map(|r| r.get_grade())
                .collect();

            let avg_grade_value = grades.iter()
                .map(|g| match g {
                    PerformanceGrade::A => 4.0,
                    PerformanceGrade::B => 3.0,
                    PerformanceGrade::C => 2.0,
                    PerformanceGrade::D => 1.0,
                    PerformanceGrade::F => 0.0,
                })
                .sum::<f64>() / grades.len() as f64;

            match avg_grade_value {
                x if x >= 3.5 => PerformanceGrade::A,
                x if x >= 2.5 => PerformanceGrade::B,
                x if x >= 1.5 => PerformanceGrade::C,
                x if x >= 0.5 => PerformanceGrade::D,
                _ => PerformanceGrade::F,
            }
        }
    }
}

/// Failure scenarios for testing resilience
pub mod failure_scenarios {
    use super::*;
    use std::time::Duration;

    /// Type of failure to simulate
    #[derive(Debug, Clone, Copy)]
    pub enum FailureType {
        /// Network partition
        NetworkPartition,
        /// Component crash
        ComponentCrash,
        /// Memory leak
        MemoryLeak,
        /// CPU overload
        CPUOverload,
        /// Disk full
        DiskFull,
        /// Byzantine behavior
        Byzantine,
    }

    /// Failure injection configuration
    #[derive(Debug, Clone)]
    pub struct FailureConfig {
        /// Type of failure
        pub failure_type: FailureType,
        /// Duration of failure
        pub duration: Duration,
        /// Affected components
        pub affected_components: Vec<String>,
        /// Severity (0.0-1.0)
        pub severity: f64,
    }

    /// Failure scenario test
    pub struct FailureScenario {
        /// Scenario name
        pub name: String,
        /// Failure configuration
        pub config: FailureConfig,
        /// Expected recovery time
        pub expected_recovery_time: Duration,
        /// Minimum functionality during failure
        pub min_functionality_percent: f64,
    }

    impl FailureScenario {
        /// Create network partition scenario
        pub fn network_partition() -> Self {
            Self {
                name: "Network Partition".to_string(),
                config: FailureConfig {
                    failure_type: FailureType::NetworkPartition,
                    duration: Duration::from_secs(10),
                    affected_components: vec!["inter_exex".to_string()],
                    severity: 0.8,
                },
                expected_recovery_time: Duration::from_secs(5),
                min_functionality_percent: 0.5,
            }
        }

        /// Create component crash scenario
        pub fn component_crash() -> Self {
            Self {
                name: "Component Crash".to_string(),
                config: FailureConfig {
                    failure_type: FailureType::ComponentCrash,
                    duration: Duration::from_secs(15),
                    affected_components: vec!["ai_engine".to_string()],
                    severity: 1.0,
                },
                expected_recovery_time: Duration::from_secs(10),
                min_functionality_percent: 0.3,
            }
        }

        /// Create memory leak scenario
        pub fn memory_leak() -> Self {
            Self {
                name: "Memory Leak".to_string(),
                config: FailureConfig {
                    failure_type: FailureType::MemoryLeak,
                    duration: Duration::from_secs(30),
                    affected_components: vec!["optimization".to_string()],
                    severity: 0.6,
                },
                expected_recovery_time: Duration::from_secs(20),
                min_functionality_percent: 0.7,
            }
        }
    }
}