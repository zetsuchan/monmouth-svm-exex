//! gRPC client for remote ExEx communication

use super::proto::*;
use super::connection_pool::ConnectionPool;
use crate::errors::*;
use std::sync::Arc;
use tonic::transport::Channel;
use tracing::{debug, error, info};

/// gRPC client for ExEx communication
pub struct ExExGrpcClient {
    /// Connection pool for managing connections
    connection_pool: Arc<ConnectionPool>,
    
    /// Default timeout for requests
    request_timeout: std::time::Duration,
}

impl ExExGrpcClient {
    /// Create a new gRPC client
    pub fn new(connection_pool: Arc<ConnectionPool>) -> Self {
        Self {
            connection_pool,
            request_timeout: std::time::Duration::from_secs(5),
        }
    }
    
    /// Create a new gRPC client with custom configuration
    pub fn with_config(
        connection_pool: Arc<ConnectionPool>,
        request_timeout: std::time::Duration,
    ) -> Self {
        Self {
            connection_pool,
            request_timeout,
        }
    }
    
    /// Get a client for a specific endpoint
    async fn get_client(&self, endpoint: &str) -> Result<ex_ex_service_client::ExExServiceClient<Channel>> {
        let channel = self.connection_pool.get_connection(endpoint).await?;
        Ok(ex_ex_service_client::ExExServiceClient::new(channel))
    }
    
    /// Submit a transaction to a remote ExEx
    pub async fn submit_transaction(
        &self,
        endpoint: &str,
        tx_hash: Vec<u8>,
        tx_data: Vec<u8>,
        sender: Vec<u8>,
        metadata: Option<TransactionMetadata>,
    ) -> Result<SubmitTransactionResponse> {
        debug!("Submitting transaction to {}", endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(SubmitTransactionRequest {
            tx_hash,
            tx_data,
            sender,
            metadata,
        });
        
        let response = tokio::time::timeout(
            self.request_timeout,
            client.submit_transaction(request)
        )
        .await
        .map_err(|_| SvmExExError::NetworkError("Request timeout".to_string()))?
        .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        self.connection_pool.release_connection(endpoint, true).await;
        
        Ok(response.into_inner())
    }
    
    /// Submit an agent transaction
    pub async fn submit_agent_transaction(
        &self,
        endpoint: &str,
        agent_tx: AgentTx,
        priority: ProcessingPriority,
    ) -> Result<SubmitAgentTransactionResponse> {
        debug!("Submitting agent transaction to {}", endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(SubmitAgentTransactionRequest {
            agent_tx: Some(agent_tx),
            priority: priority as i32,
        });
        
        let response = tokio::time::timeout(
            self.request_timeout,
            client.submit_agent_transaction(request)
        )
        .await
        .map_err(|_| SvmExExError::NetworkError("Request timeout".to_string()))?
        .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        self.connection_pool.release_connection(endpoint, true).await;
        
        Ok(response.into_inner())
    }
    
    /// Stream transaction results
    pub async fn stream_transaction_results(
        &self,
        endpoint: &str,
        transaction_ids: Vec<String>,
        include_state_changes: bool,
    ) -> Result<impl futures::Stream<Item = Result<TransactionResult>>> {
        debug!("Streaming results for {} transactions from {}", transaction_ids.len(), endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(StreamTransactionResultsRequest {
            transaction_ids,
            include_state_changes,
        });
        
        let response = client.stream_transaction_results(request)
            .await
            .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        let stream = response.into_inner();
        
        Ok(stream.map(|result| {
            result.map_err(|e| SvmExExError::NetworkError(e.to_string()))
        }))
    }
    
    /// Check health of a remote ExEx
    pub async fn health_check(
        &self,
        endpoint: &str,
        components: Vec<String>,
    ) -> Result<HealthCheckResponse> {
        debug!("Checking health of {}", endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(HealthCheckRequest { components });
        
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(2), // Shorter timeout for health checks
            client.health_check(request)
        )
        .await
        .map_err(|_| SvmExExError::NetworkError("Health check timeout".to_string()))?
        .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        self.connection_pool.release_connection(endpoint, true).await;
        
        Ok(response.into_inner())
    }
    
    /// Get node information
    pub async fn get_node_info(&self, endpoint: &str) -> Result<GetNodeInfoResponse> {
        debug!("Getting node info from {}", endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(GetNodeInfoRequest {});
        
        let response = client.get_node_info(request)
            .await
            .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        self.connection_pool.release_connection(endpoint, true).await;
        
        Ok(response.into_inner())
    }
    
    /// Sync state with a remote ExEx
    pub async fn sync_state(
        &self,
        endpoint: &str,
        from_block: u64,
        to_block: u64,
        include_alh: bool,
        include_memory_proofs: bool,
    ) -> Result<SyncStateResponse> {
        info!("Syncing state from {} (blocks {} to {})", endpoint, from_block, to_block);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(SyncStateRequest {
            from_block,
            to_block,
            include_alh,
            include_memory_proofs,
        });
        
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(30), // Longer timeout for state sync
            client.sync_state(request)
        )
        .await
        .map_err(|_| SvmExExError::NetworkError("State sync timeout".to_string()))?
        .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        self.connection_pool.release_connection(endpoint, true).await;
        
        Ok(response.into_inner())
    }
    
    /// Execute a tool sequence
    pub async fn execute_tool_sequence(
        &self,
        endpoint: &str,
        steps: Vec<ToolStep>,
        strategy: ExecutionStrategy,
        context: std::collections::HashMap<String, String>,
    ) -> Result<impl futures::Stream<Item = Result<ToolExecutionUpdate>>> {
        info!("Executing tool sequence with {} steps on {}", steps.len(), endpoint);
        
        let mut client = self.get_client(endpoint).await?;
        
        let request = tonic::Request::new(ExecuteToolSequenceRequest {
            steps,
            strategy: strategy as i32,
            context,
        });
        
        let response = client.execute_tool_sequence(request)
            .await
            .map_err(|e| SvmExExError::NetworkError(e.to_string()))?;
        
        let stream = response.into_inner();
        
        Ok(stream.map(|result| {
            result.map_err(|e| SvmExExError::NetworkError(e.to_string()))
        }))
    }
    
    /// Batch submit multiple transactions
    pub async fn batch_submit_transactions(
        &self,
        endpoint: &str,
        transactions: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>, // (tx_hash, tx_data, sender)
    ) -> Result<Vec<SubmitTransactionResponse>> {
        info!("Batch submitting {} transactions to {}", transactions.len(), endpoint);
        
        let mut results = Vec::new();
        
        // Use concurrent futures for better performance
        let futures: Vec<_> = transactions.into_iter()
            .map(|(tx_hash, tx_data, sender)| {
                let endpoint = endpoint.to_string();
                let client = self.clone();
                
                async move {
                    client.submit_transaction(
                        &endpoint,
                        tx_hash,
                        tx_data,
                        sender,
                        None,
                    ).await
                }
            })
            .collect();
        
        let responses = futures::future::join_all(futures).await;
        
        for response in responses {
            match response {
                Ok(resp) => results.push(resp),
                Err(e) => {
                    error!("Failed to submit transaction: {}", e);
                    return Err(e);
                }
            }
        }
        
        Ok(results)
    }
}

impl Clone for ExExGrpcClient {
    fn clone(&self) -> Self {
        Self {
            connection_pool: self.connection_pool.clone(),
            request_timeout: self.request_timeout,
        }
    }
}

/// Builder for ExExGrpcClient
pub struct ExExGrpcClientBuilder {
    max_connections: usize,
    request_timeout: std::time::Duration,
    connection_timeout: std::time::Duration,
}

impl Default for ExExGrpcClientBuilder {
    fn default() -> Self {
        Self {
            max_connections: 100,
            request_timeout: std::time::Duration::from_secs(5),
            connection_timeout: std::time::Duration::from_secs(10),
        }
    }
}

impl ExExGrpcClientBuilder {
    /// Set maximum connections
    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }
    
    /// Set request timeout
    pub fn request_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.request_timeout = timeout;
        self
    }
    
    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }
    
    /// Build the client
    pub fn build(self) -> ExExGrpcClient {
        let pool = Arc::new(ConnectionPool::new(self.max_connections));
        ExExGrpcClient::with_config(pool, self.request_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_client_builder() {
        let client = ExExGrpcClientBuilder::default()
            .max_connections(50)
            .request_timeout(std::time::Duration::from_secs(10))
            .build();
        
        assert_eq!(client.request_timeout, std::time::Duration::from_secs(10));
    }
}