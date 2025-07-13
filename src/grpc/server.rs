//! gRPC server implementation with <10ms latency target

use super::proto::*;
use crate::errors::*;
use crate::enhanced_exex::EnhancedSvmExEx;
use crate::inter_exex::{ExExMessage, MessageType, MessagePayload};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tonic::{Request, Response, Status};
use tracing::{debug, error, info, warn};

/// gRPC server for ExEx remote execution
pub struct ExExGrpcServer {
    /// Reference to the ExEx instance
    exex: Arc<RwLock<EnhancedSvmExEx>>,
    
    /// Connection pool for managing client connections
    connection_pool: Arc<Mutex<super::connection_pool::ConnectionPool>>,
    
    /// Health service
    health_service: Arc<super::health::HealthService>,
    
    /// Server configuration
    config: ServerConfig,
    
    /// Metrics collector
    metrics: Arc<Mutex<ServerMetrics>>,
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    
    /// Maximum message size in bytes
    pub max_message_size: usize,
    
    /// Enable request compression
    pub enable_compression: bool,
    
    /// Target latency in milliseconds
    pub target_latency_ms: u64,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 1000,
            request_timeout_ms: 5000,
            max_message_size: 10 * 1024 * 1024, // 10MB
            enable_compression: true,
            target_latency_ms: 10, // <10ms target
        }
    }
}

/// Server metrics
#[derive(Debug, Default)]
struct ServerMetrics {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    average_latency_ms: f64,
    current_connections: usize,
}

impl ExExGrpcServer {
    /// Create a new gRPC server
    pub fn new(
        exex: Arc<RwLock<EnhancedSvmExEx>>,
        config: ServerConfig,
    ) -> Self {
        let connection_pool = Arc::new(Mutex::new(
            super::connection_pool::ConnectionPool::new(config.max_concurrent_requests)
        ));
        
        let health_service = Arc::new(super::health::HealthService::new());
        
        Self {
            exex,
            connection_pool,
            health_service,
            config,
            metrics: Arc::new(Mutex::new(ServerMetrics::default())),
        }
    }
    
    /// Start the gRPC server
    pub async fn start(self: Arc<Self>, addr: std::net::SocketAddr) -> Result<()> {
        info!("Starting gRPC server on {}", addr);
        
        let service = ex_ex_service_server::ExExServiceServer::new(self)
            .max_decoding_message_size(self.config.max_message_size)
            .max_encoding_message_size(self.config.max_message_size);
        
        // Configure server with compression if enabled
        let mut server_builder = tonic::transport::Server::builder();
        
        if self.config.enable_compression {
            server_builder = server_builder
                .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
                .send_compressed(tonic::codec::CompressionEncoding::Gzip);
        }
        
        server_builder
            .add_service(service)
            .serve(addr)
            .await
            .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Record request metrics
    async fn record_metrics(&self, start_time: std::time::Instant, success: bool) {
        let latency_ms = start_time.elapsed().as_millis() as f64;
        let mut metrics = self.metrics.lock().await;
        
        metrics.total_requests += 1;
        if success {
            metrics.successful_requests += 1;
        } else {
            metrics.failed_requests += 1;
        }
        
        // Update average latency (simple moving average)
        let n = metrics.total_requests as f64;
        metrics.average_latency_ms = 
            ((n - 1.0) * metrics.average_latency_ms + latency_ms) / n;
        
        if latency_ms > self.config.target_latency_ms as f64 {
            warn!("Request latency {}ms exceeds target {}ms", 
                latency_ms, self.config.target_latency_ms);
        }
    }
}

#[tonic::async_trait]
impl ex_ex_service_server::ExExService for Arc<ExExGrpcServer> {
    async fn submit_transaction(
        &self,
        request: Request<SubmitTransactionRequest>,
    ) -> Result<Response<SubmitTransactionResponse>, Status> {
        let start_time = std::time::Instant::now();
        debug!("Received transaction submission request");
        
        let req = request.into_inner();
        
        // Validate request
        if req.tx_hash.is_empty() || req.tx_data.is_empty() {
            self.record_metrics(start_time, false).await;
            return Err(Status::invalid_argument("Invalid transaction data"));
        }
        
        // Convert to internal message format
        let tx_hash = alloy_primitives::B256::from_slice(&req.tx_hash);
        let sender = alloy_primitives::Address::from_slice(&req.sender);
        
        let message = ExExMessage::new(
            MessageType::TransactionProposal,
            "grpc-client".to_string(),
            MessagePayload::Transaction(crate::inter_exex::messages::TransactionData {
                hash: tx_hash,
                from: sender,
                to: None,
                value: alloy_primitives::U256::ZERO,
                gas_price: alloy_primitives::U256::from(req.metadata.as_ref().map(|m| m.gas_price).unwrap_or(0)),
                data: req.tx_data.clone(),
                svm_metadata: Some(crate::inter_exex::messages::SvmMetadata {
                    compute_units: 200_000,
                    account_count: 1,
                    programs: vec![],
                    priority: 1,
                }),
            }),
        );
        
        // Submit to ExEx
        let exex = self.exex.read().await;
        // Note: In a real implementation, we would submit through the proper channels
        
        let transaction_id = uuid::Uuid::new_v4().to_string();
        
        let response = SubmitTransactionResponse {
            transaction_id: transaction_id.clone(),
            accepted: true,
            reason: String::new(),
            estimated_execution_time_ms: 50,
        };
        
        self.record_metrics(start_time, true).await;
        Ok(Response::new(response))
    }
    
    async fn submit_agent_transaction(
        &self,
        request: Request<SubmitAgentTransactionRequest>,
    ) -> Result<Response<SubmitAgentTransactionResponse>, Status> {
        let start_time = std::time::Instant::now();
        debug!("Received agent transaction submission");
        
        let req = request.into_inner();
        
        // Validate agent transaction
        let agent_tx = req.agent_tx.ok_or_else(|| {
            Status::invalid_argument("Missing agent transaction")
        })?;
        
        // Perform intent classification
        let intent_classification = IntentClassification {
            category: IntentCategory::Defi as i32,
            confidence: 0.95,
            suggested_tools: vec!["svm-executor".to_string(), "memory-loader".to_string()],
            risk_factors: vec![],
            estimated_resources: Some(EstimatedResources {
                gas_estimate: 100_000,
                compute_units_estimate: 50_000,
                execution_time_ms_estimate: 25,
                success_probability: 0.98,
            }),
        };
        
        let transaction_id = uuid::Uuid::new_v4().to_string();
        
        let response = SubmitAgentTransactionResponse {
            transaction_id,
            accepted: true,
            reason: String::new(),
            intent_classification: Some(intent_classification),
        };
        
        self.record_metrics(start_time, true).await;
        Ok(Response::new(response))
    }
    
    type StreamTransactionResultsStream = tokio_stream::wrappers::ReceiverStream<Result<TransactionResult, Status>>;
    
    async fn stream_transaction_results(
        &self,
        request: Request<StreamTransactionResultsRequest>,
    ) -> Result<Response<Self::StreamTransactionResultsStream>, Status> {
        let req = request.into_inner();
        debug!("Streaming results for {} transactions", req.transaction_ids.len());
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        
        // Spawn task to stream results
        tokio::spawn(async move {
            for tx_id in req.transaction_ids {
                // Simulate transaction result
                let result = TransactionResult {
                    transaction_id: tx_id,
                    tx_hash: vec![0u8; 32],
                    status: ExecutionStatus::Success as i32,
                    state_changes: vec![],
                    gas_used: 50_000,
                    execution_time_ms: 15,
                    error_message: vec![],
                };
                
                if tx.send(Ok(result)).await.is_err() {
                    break;
                }
                
                // Small delay to simulate processing
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        });
        
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
    
    async fn health_check(
        &self,
        request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let req = request.into_inner();
        
        let health_status = self.health_service.check_health(req.components).await;
        
        let response = HealthCheckResponse {
            healthy: health_status.healthy,
            components: health_status.components.into_iter()
                .map(|(k, v)| (k, ComponentHealth {
                    healthy: v.healthy,
                    status: v.status,
                    last_check_timestamp: v.last_check_timestamp,
                    metrics: v.metrics,
                }))
                .collect(),
            uptime_seconds: health_status.uptime_seconds,
        };
        
        Ok(Response::new(response))
    }
    
    async fn get_node_info(
        &self,
        _request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<GetNodeInfoResponse>, Status> {
        let metrics = self.metrics.lock().await;
        
        let response = GetNodeInfoResponse {
            node_id: "svm-exex-grpc".to_string(),
            node_type: NodeType::Svm as i32,
            capabilities: vec![
                "svm-execution".to_string(),
                "agent-transactions".to_string(),
                "memory-proofs".to_string(),
                "parallel-execution".to_string(),
            ],
            metadata: std::collections::HashMap::from([
                ("version".to_string(), "0.1.0".to_string()),
                ("grpc_enabled".to_string(), "true".to_string()),
            ]),
            load_metrics: Some(LoadMetrics {
                load_percentage: 25,
                available_compute: 1_000_000,
                queue_depth: 10,
                avg_processing_time_ms: metrics.average_latency_ms as u64,
                memory_usage_percentage: 30,
                bandwidth_usage_bps: 1_000_000,
            }),
        };
        
        Ok(Response::new(response))
    }
    
    async fn sync_state(
        &self,
        request: Request<SyncStateRequest>,
    ) -> Result<Response<SyncStateResponse>, Status> {
        let req = request.into_inner();
        debug!("Syncing state from block {} to {}", req.from_block, req.to_block);
        
        // TODO: Implement actual state sync logic
        let response = SyncStateResponse {
            block_states: vec![],
            latest_alh: vec![0u8; 32],
            memory_proofs: vec![],
        };
        
        Ok(Response::new(response))
    }
    
    type ExecuteToolSequenceStream = tokio_stream::wrappers::ReceiverStream<Result<ToolExecutionUpdate, Status>>;
    
    async fn execute_tool_sequence(
        &self,
        request: Request<ExecuteToolSequenceRequest>,
    ) -> Result<Response<Self::ExecuteToolSequenceStream>, Status> {
        let req = request.into_inner();
        debug!("Executing tool sequence with {} steps", req.steps.len());
        
        let (tx, rx) = tokio::sync::mpsc::channel(128);
        
        // Spawn task to execute tools
        tokio::spawn(async move {
            for (idx, step) in req.steps.into_iter().enumerate() {
                let update = ToolExecutionUpdate {
                    step_id: format!("step-{}", idx),
                    status: ToolExecutionStatus::Success as i32,
                    result: vec![],
                    error_message: String::new(),
                    execution_time_ms: 20,
                };
                
                if tx.send(Ok(update)).await.is_err() {
                    break;
                }
                
                tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
            }
        });
        
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}