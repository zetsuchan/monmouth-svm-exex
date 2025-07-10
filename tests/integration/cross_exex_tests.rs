//! Comprehensive Integration Tests for Cross-ExEx Communication
//! 
//! Tests the integrated functionality of multiple ExEx instances working together,
//! including inter-ExEx communication, AI coordination, state synchronization,
//! RAG integration, and performance validation.

use monmouth_svm_exex::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::time::timeout;
use tracing::{debug, info, warn, error};
use eyre::Result;

/// Test configuration for integration tests
#[derive(Debug, Clone)]
pub struct IntegrationTestConfig {
    /// Number of ExEx instances to simulate
    pub exex_count: usize,
    /// Test timeout duration
    pub test_timeout: Duration,
    /// Message processing timeout
    pub message_timeout: Duration,
    /// Performance test iterations
    pub performance_iterations: usize,
    /// Enable stress testing
    pub enable_stress_tests: bool,
}

impl Default for IntegrationTestConfig {
    fn default() -> Self {
        Self {
            exex_count: 3,
            test_timeout: Duration::from_secs(30),
            message_timeout: Duration::from_millis(100),
            performance_iterations: 1000,
            enable_stress_tests: true,
        }
    }
}

/// Mock ExEx instance for integration testing
pub struct MockExExInstance {
    /// Instance ID
    pub id: String,
    /// Inter-ExEx message bus
    pub message_bus: Arc<inter_exex::MessageBus>,
    /// AI decision engine
    pub ai_engine: Arc<ai::EnhancedAIDecisionEngine>,
    /// Optimization service
    pub optimization: Arc<OptimizationService>,
    /// Batch manager
    pub batch_manager: Arc<BatchManager>,
    /// Deployment manager
    pub deployment: Arc<DeploymentManager>,
    /// Health checker
    pub health_checker: Arc<HealthChecker>,
    /// Received messages
    pub received_messages: Arc<Mutex<Vec<inter_exex::MessageType>>>,
    /// Performance metrics
    pub metrics: Arc<RwLock<IntegrationMetrics>>,
}

/// Integration test metrics
#[derive(Debug, Default, Clone)]
pub struct IntegrationMetrics {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// AI decisions made
    pub ai_decisions: u64,
    /// RAG queries processed
    pub rag_queries: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Average latency (ms)
    pub avg_latency_ms: f64,
    /// Error count
    pub errors: u64,
}

impl MockExExInstance {
    /// Create a new mock ExEx instance
    pub async fn new(id: String, config: &IntegrationTestConfig) -> Result<Self> {
        // Initialize inter-ExEx message bus
        let message_bus_config = inter_exex::MessageBusConfig {
            node_id: id.clone(),
            heartbeat_interval: Duration::from_secs(1),
            discovery_interval: Duration::from_secs(5),
            max_message_size: 1024 * 1024, // 1MB
            enable_compression: true,
            enable_encryption: false, // Disabled for tests
        };
        let message_bus = Arc::new(inter_exex::MessageBus::new(message_bus_config).await?);

        // Initialize AI decision engine
        let ai_config = ai::EnhancedAIDecisionEngineConfig::default();
        let ai_engine = Arc::new(ai::EnhancedAIDecisionEngine::new(ai_config).await?);

        // Initialize optimization service
        let optimization_config = OptimizationServiceConfig::default();
        let optimization = Arc::new(OptimizationService::new(optimization_config));

        // Initialize batch manager
        let batch_config = BatchManagerConfig::default();
        let batch_manager = Arc::new(BatchManager::new(batch_config).await?);

        // Initialize deployment manager
        let deployment_config = DeploymentConfig {
            environment: Environment::Testing,
            service_config: deployment::ServiceConfig {
                name: format!("test-exex-{}", id),
                version: "0.8.0-test".to_string(),
                instance_id: id.clone(),
                bind_address: "127.0.0.1".to_string(),
                port: 8000 + id.parse::<u16>().unwrap_or(0),
                worker_threads: 4,
                request_timeout: Duration::from_secs(30),
                keep_alive_timeout: Duration::from_secs(60),
                max_connections: 1000,
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            resource_limits: deployment::ResourceLimits::default(),
            health_config: HealthCheckConfig::default(),
            monitoring_config: deployment::MonitoringConfig::default(),
            alert_config: deployment::AlertConfig::default(),
            shutdown_config: deployment::ShutdownConfig::default(),
            feature_flags: HashMap::new(),
        };
        let deployment = Arc::new(DeploymentManager::new(deployment_config).await?);

        // Initialize health checker
        let health_config = HealthCheckConfig::default();
        let health_checker = Arc::new(HealthChecker::new(health_config).await?);

        Ok(Self {
            id,
            message_bus,
            ai_engine,
            optimization,
            batch_manager,
            deployment,
            health_checker,
            received_messages: Arc::new(Mutex::new(Vec::new())),
            metrics: Arc::new(RwLock::new(IntegrationMetrics::default())),
        })
    }

    /// Start the ExEx instance
    pub async fn start(&self) -> Result<()> {
        info!("Starting ExEx instance: {}", self.id);

        // Initialize all services
        self.optimization.initialize().await?;
        self.batch_manager.start().await?;
        self.deployment.start().await?;
        self.health_checker.start().await?;

        // Start message bus
        self.message_bus.start().await?;

        // Set up message handling
        self.setup_message_handling().await?;

        info!("ExEx instance {} started successfully", self.id);
        Ok(())
    }

    /// Setup message handling for the ExEx instance
    async fn setup_message_handling(&self) -> Result<()> {
        let received_messages = self.received_messages.clone();
        let metrics = self.metrics.clone();
        let ai_engine = self.ai_engine.clone();
        let optimization = self.optimization.clone();

        // Subscribe to all message types
        let mut receiver = self.message_bus.subscribe().await?;

        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                let start_time = Instant::now();

                // Record received message
                {
                    let mut messages = received_messages.lock().await;
                    messages.push(message.clone());
                }

                // Process message based on type
                match &message {
                    inter_exex::MessageType::NodeAnnouncement(_) => {
                        debug!("Received node announcement");
                    }
                    inter_exex::MessageType::TransactionProposal(proposal) => {
                        debug!("Received transaction proposal: {:?}", proposal);
                        
                        // Process with AI engine
                        if let Ok(analysis) = ai_engine.analyze_transaction(&proposal.transaction_data).await {
                            debug!("AI analysis result: {:?}", analysis);
                        }
                    }
                    inter_exex::MessageType::StateSync(sync_msg) => {
                        debug!("Received state sync: {:?}", sync_msg);
                    }
                    inter_exex::MessageType::LoadInfo(_) => {
                        debug!("Received load info");
                    }
                    inter_exex::MessageType::ConsensusVote(_) => {
                        debug!("Received consensus vote");
                    }
                    inter_exex::MessageType::ALHUpdate(_) => {
                        debug!("Received ALH update");
                    }
                }

                // Update metrics
                {
                    let mut metrics = metrics.write().await;
                    metrics.messages_received += 1;
                    let latency = start_time.elapsed().as_millis() as f64;
                    metrics.avg_latency_ms = (metrics.avg_latency_ms * (metrics.messages_received - 1) as f64 + latency) / metrics.messages_received as f64;
                }
            }
        });

        Ok(())
    }

    /// Send a message to other ExEx instances
    pub async fn send_message(&self, message: inter_exex::MessageType) -> Result<()> {
        self.message_bus.broadcast(message).await?;
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.messages_sent += 1;
        }

        Ok(())
    }

    /// Process a RAG query
    pub async fn process_rag_query(&self, query: &str) -> Result<OptimizedExecutionResult> {
        let result = self.optimization.execute_optimized_query(
            query,
            Arc::new(vector_store::MockVectorStore::new()),
            10,
        ).await?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.rag_queries += 1;
            if result.cache_metadata.as_ref().map(|c| c.cache_hit).unwrap_or(false) {
                metrics.cache_hits += 1;
            } else {
                metrics.cache_misses += 1;
            }
        }

        Ok(result)
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> IntegrationMetrics {
        self.metrics.read().await.clone()
    }

    /// Stop the ExEx instance
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping ExEx instance: {}", self.id);

        // Stop all services
        self.health_checker.stop().await?;
        self.deployment.stop().await?;
        self.batch_manager.stop().await?;
        self.message_bus.stop().await?;

        info!("ExEx instance {} stopped successfully", self.id);
        Ok(())
    }
}

/// Integration test suite
pub struct IntegrationTestSuite {
    /// Test configuration
    config: IntegrationTestConfig,
    /// ExEx instances
    instances: Vec<Arc<MockExExInstance>>,
}

impl IntegrationTestSuite {
    /// Create a new integration test suite
    pub async fn new(config: IntegrationTestConfig) -> Result<Self> {
        let mut instances = Vec::new();

        // Create ExEx instances
        for i in 0..config.exex_count {
            let instance = Arc::new(MockExExInstance::new(
                format!("exex-{}", i),
                &config,
            ).await?);
            instances.push(instance);
        }

        Ok(Self { config, instances })
    }

    /// Start all ExEx instances
    pub async fn start_all(&self) -> Result<()> {
        info!("Starting {} ExEx instances", self.instances.len());

        for instance in &self.instances {
            instance.start().await?;
        }

        // Wait for instances to discover each other
        tokio::time::sleep(Duration::from_secs(2)).await;

        info!("All ExEx instances started successfully");
        Ok(())
    }

    /// Stop all ExEx instances
    pub async fn stop_all(&self) -> Result<()> {
        info!("Stopping all ExEx instances");

        for instance in &self.instances {
            instance.stop().await?;
        }

        info!("All ExEx instances stopped successfully");
        Ok(())
    }
}

/// Mock vector store for testing
mod vector_store {
    use super::*;
    use ai::rag::VectorStore;

    pub struct MockVectorStore;

    impl MockVectorStore {
        pub fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl VectorStore for MockVectorStore {
        async fn search(&self, query: &str, limit: usize) -> Result<Vec<ai::rag::SearchResult>> {
            // Mock search results
            let results = (0..limit.min(5))
                .map(|i| ai::rag::SearchResult {
                    id: format!("result-{}", i),
                    content: format!("Mock content for query '{}' - result {}", query, i),
                    score: 0.9 - (i as f64 * 0.1),
                    metadata: std::collections::HashMap::new(),
                })
                .collect();

            Ok(results)
        }

        async fn add_document(&self, _document: ai::rag::Document) -> Result<String> {
            Ok("mock-doc-id".to_string())
        }

        async fn get_document(&self, _id: &str) -> Result<Option<ai::rag::Document>> {
            Ok(None)
        }

        async fn delete_document(&self, _id: &str) -> Result<bool> {
            Ok(true)
        }
    }
}

// Integration tests
#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test;

    /// Initialize test logging
    fn init_test_logging() {
        let _ = tracing_subscriber::fmt()
            .with_env_filter("debug")
            .try_init();
    }

    #[tokio::test]
    async fn test_cross_exex_basic_communication() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 2,
            test_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        // Test basic message passing
        let sender = &test_suite.instances[0];
        let receiver = &test_suite.instances[1];

        // Send a transaction proposal
        let proposal = inter_exex::TransactionProposal {
            proposer_id: sender.id.clone(),
            transaction_data: b"test_transaction".to_vec(),
            priority: inter_exex::MessagePriority::Normal,
            estimated_gas: 21000,
            max_fee: 1000000000,
            timestamp: std::time::SystemTime::now(),
        };

        sender.send_message(inter_exex::MessageType::TransactionProposal(proposal)).await?;

        // Wait for message to be received
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify message was received
        let received_messages = receiver.received_messages.lock().await;
        assert!(!received_messages.is_empty(), "No messages received");
        
        match &received_messages[0] {
            inter_exex::MessageType::TransactionProposal(received_proposal) => {
                assert_eq!(received_proposal.proposer_id, sender.id);
                assert_eq!(received_proposal.transaction_data, b"test_transaction");
            }
            _ => panic!("Unexpected message type received"),
        }

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_ai_coordination_between_exexs() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 3,
            test_timeout: Duration::from_secs(15),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        // Test AI coordination scenario
        let coordinator = &test_suite.instances[0];
        let participants = &test_suite.instances[1..];

        // Simulate a complex transaction requiring AI coordination
        for (i, participant) in participants.iter().enumerate() {
            let consensus_vote = inter_exex::ConsensusVote {
                voter_id: participant.id.clone(),
                transaction_hash: format!("tx_hash_{}", i),
                decision: inter_exex::ConsensusDecision::Approve,
                confidence: 0.85 + (i as f64 * 0.05),
                reasoning: format!("AI analysis result for participant {}", i),
                timestamp: std::time::SystemTime::now(),
            };

            participant.send_message(inter_exex::MessageType::ConsensusVote(consensus_vote)).await?;
        }

        // Wait for coordination to complete
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Verify all participants sent votes
        for participant in participants {
            let metrics = participant.get_metrics().await;
            assert!(metrics.messages_sent > 0, "Participant {} didn't send messages", participant.id);
        }

        // Verify coordinator received votes
        let coordinator_messages = coordinator.received_messages.lock().await;
        let vote_count = coordinator_messages.iter()
            .filter(|msg| matches!(msg, inter_exex::MessageType::ConsensusVote(_)))
            .count();
        assert_eq!(vote_count, participants.len(), "Coordinator didn't receive all votes");

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_rag_query_performance() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 1,
            performance_iterations: 100,
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        let instance = &test_suite.instances[0];

        // Performance test: RAG queries should complete under 50ms
        let start_time = Instant::now();
        let mut successful_queries = 0;

        for i in 0..config.performance_iterations {
            let query = format!("test query {}", i);
            let query_start = Instant::now();

            match timeout(
                Duration::from_millis(50),
                instance.process_rag_query(&query)
            ).await {
                Ok(Ok(_result)) => {
                    successful_queries += 1;
                    let query_time = query_start.elapsed();
                    debug!("Query {} completed in {:?}", i, query_time);
                }
                Ok(Err(e)) => {
                    warn!("Query {} failed: {}", i, e);
                }
                Err(_) => {
                    warn!("Query {} timed out (>50ms)", i);
                }
            }
        }

        let total_time = start_time.elapsed();
        let success_rate = successful_queries as f64 / config.performance_iterations as f64;

        info!(
            "RAG Performance Test Results: {}/{} queries successful ({:.2}%) in {:?}",
            successful_queries, config.performance_iterations, success_rate * 100.0, total_time
        );

        // Verify performance requirements
        assert!(success_rate >= 0.95, "Success rate too low: {:.2}%", success_rate * 100.0);

        let avg_time_per_query = total_time / config.performance_iterations as u32;
        assert!(
            avg_time_per_query < Duration::from_millis(50),
            "Average query time too high: {:?}",
            avg_time_per_query
        );

        // Verify metrics
        let metrics = instance.get_metrics().await;
        assert_eq!(metrics.rag_queries, successful_queries as u64);
        assert!(metrics.avg_latency_ms < 50.0, "Average latency too high: {:.2}ms", metrics.avg_latency_ms);

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_state_synchronization() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 2,
            test_timeout: Duration::from_secs(20),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        let source = &test_suite.instances[0];
        let target = &test_suite.instances[1];

        // Test state synchronization
        let sync_message = inter_exex::StateSync {
            sync_type: inter_exex::StateSyncType::Full,
            source_id: source.id.clone(),
            target_id: Some(target.id.clone()),
            block_height: 12345,
            state_hash: [0u8; 32],
            data: b"mock_state_data".to_vec(),
            checksum: 0x12345678,
            timestamp: std::time::SystemTime::now(),
        };

        source.send_message(inter_exex::MessageType::StateSync(sync_message)).await?;

        // Wait for synchronization
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Verify sync message was received
        let target_messages = target.received_messages.lock().await;
        let sync_received = target_messages.iter()
            .any(|msg| matches!(msg, inter_exex::MessageType::StateSync(_)));
        assert!(sync_received, "State sync message not received");

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_health_monitoring_integration() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 1,
            test_timeout: Duration::from_secs(10),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        let instance = &test_suite.instances[0];

        // Test health monitoring
        let health_status = instance.health_checker.check_overall_health().await?;
        assert_eq!(health_status.status, HealthStatus::Healthy, "Instance should be healthy");

        // Test individual component health
        let component_health = instance.health_checker.check_component_health("optimization").await?;
        assert!(component_health.is_some(), "Optimization component should have health status");

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_failure_recovery() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 2,
            test_timeout: Duration::from_secs(15),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        // Simulate failure and recovery
        let instance = &test_suite.instances[0];

        // Stop one component (simulate failure)
        instance.health_checker.stop().await?;

        // Verify degraded status
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Restart component (simulate recovery)
        let health_config = HealthCheckConfig::default();
        let new_health_checker = Arc::new(HealthChecker::new(health_config).await?);
        new_health_checker.start().await?;

        // Verify recovery
        let recovery_status = new_health_checker.check_overall_health().await?;
        assert_ne!(recovery_status.status, HealthStatus::Down, "System should recover");

        new_health_checker.stop().await?;
        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_processing_coordination() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 2,
            test_timeout: Duration::from_secs(20),
            ..Default::default()
        };

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        // Test batch processing coordination
        let coordinator = &test_suite.instances[0];
        let worker = &test_suite.instances[1];

        // Create batch jobs
        let mut jobs = Vec::new();
        for i in 0..10 {
            let job = batch::BatchJob {
                id: format!("job-{}", i),
                job_type: batch::BatchJobType::RAGQuery,
                data: batch::BatchJobData::RAGQuery {
                    query: format!("batch query {}", i),
                    context: HashMap::new(),
                    limit: 10,
                },
                priority: batch::BatchPriority::Normal,
                retry_config: batch::RetryConfig::default(),
                created_at: std::time::SystemTime::now(),
                deadline: Some(std::time::SystemTime::now() + Duration::from_secs(60)),
            };
            jobs.push(job);
        }

        // Submit batch for processing
        let batch_id = coordinator.batch_manager.submit_batch(jobs).await?;
        info!("Submitted batch: {}", batch_id);

        // Wait for processing
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Check batch status
        let status = coordinator.batch_manager.get_batch_status(&batch_id).await?;
        assert!(status.is_some(), "Batch status should be available");

        test_suite.stop_all().await?;
        Ok(())
    }

    #[tokio::test]
    async fn test_comprehensive_stress_scenario() -> Result<()> {
        init_test_logging();
        
        let config = IntegrationTestConfig {
            exex_count: 3,
            test_timeout: Duration::from_secs(60),
            performance_iterations: 500,
            enable_stress_tests: true,
            ..Default::default()
        };

        if !config.enable_stress_tests {
            return Ok(());
        }

        let test_suite = IntegrationTestSuite::new(config.clone()).await?;
        test_suite.start_all().await?;

        info!("Starting comprehensive stress test with {} instances", test_suite.instances.len());

        // Simulate high load scenario
        let mut handles = Vec::new();

        for (i, instance) in test_suite.instances.iter().enumerate() {
            let instance = instance.clone();
            let iterations = config.performance_iterations / test_suite.instances.len();

            let handle = tokio::spawn(async move {
                let mut local_metrics = IntegrationMetrics::default();

                for j in 0..iterations {
                    let start_time = Instant::now();

                    // Send messages
                    let load_info = inter_exex::LoadInfo {
                        reporter_id: instance.id.clone(),
                        cpu_usage: 50.0 + (j as f64 % 40.0),
                        memory_usage: 60.0 + (j as f64 % 30.0),
                        queue_size: j as u32 % 100,
                        avg_response_time: Duration::from_millis(10 + (j as u64 % 50)),
                        active_connections: 50 + (j as u32 % 200),
                        timestamp: std::time::SystemTime::now(),
                    };

                    if let Err(e) = instance.send_message(inter_exex::MessageType::LoadInfo(load_info)).await {
                        local_metrics.errors += 1;
                        warn!("Failed to send message: {}", e);
                    }

                    // Process RAG queries
                    let query = format!("stress test query {} from instance {}", j, i);
                    match instance.process_rag_query(&query).await {
                        Ok(_) => local_metrics.rag_queries += 1,
                        Err(e) => {
                            local_metrics.errors += 1;
                            warn!("RAG query failed: {}", e);
                        }
                    }

                    let latency = start_time.elapsed().as_millis() as f64;
                    local_metrics.avg_latency_ms = (local_metrics.avg_latency_ms * j as f64 + latency) / (j + 1) as f64;

                    // Small delay to prevent overwhelming
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }

                (i, local_metrics)
            });

            handles.push(handle);
        }

        // Wait for all stress tests to complete
        let results = futures_util::future::join_all(handles).await;

        // Analyze results
        let mut total_queries = 0;
        let mut total_errors = 0;
        let mut max_latency = 0.0;

        for result in results {
            match result {
                Ok((instance_id, metrics)) => {
                    info!("Instance {} metrics: {:?}", instance_id, metrics);
                    total_queries += metrics.rag_queries;
                    total_errors += metrics.errors;
                    max_latency = max_latency.max(metrics.avg_latency_ms);
                }
                Err(e) => {
                    error!("Stress test task failed: {}", e);
                    total_errors += 1;
                }
            }
        }

        // Verify stress test results
        let error_rate = total_errors as f64 / (total_queries + total_errors) as f64;
        info!(
            "Stress test completed - Queries: {}, Errors: {}, Error rate: {:.2}%, Max latency: {:.2}ms",
            total_queries, total_errors, error_rate * 100.0, max_latency
        );

        assert!(error_rate < 0.05, "Error rate too high: {:.2}%", error_rate * 100.0);
        assert!(max_latency < 100.0, "Maximum latency too high: {:.2}ms", max_latency);

        test_suite.stop_all().await?;
        Ok(())
    }
}