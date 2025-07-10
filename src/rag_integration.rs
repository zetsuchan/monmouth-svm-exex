//! RAG Integration for SVM ExEx
//!
//! This module handles communication with the RAG Memory ExEx for context retrieval
//! and memory operations.

use crate::errors::*;
use crate::inter_exex::{
    InterExExCoordinator, SvmExExMessage, TransactionMetadata, ContextType,
    MemoryItem, MemoryType, MemoryPriority, MemoryQuery,
};
use crate::ai::traits::{ContextData, EmbeddingData, RoutingDecision};
use alloy_primitives::{Address, B256};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, debug, warn};
use std::collections::HashMap;

/// RAG integration service for the SVM ExEx
pub struct RagIntegrationService {
    /// Inter-ExEx coordinator
    coordinator: Arc<InterExExCoordinator>,
    /// Context cache
    context_cache: Arc<RwLock<HashMap<B256, Vec<ContextData>>>>,
    /// Pending requests
    pending_requests: Arc<RwLock<HashMap<B256, mpsc::Sender<Vec<ContextData>>>>>,
    /// Configuration
    config: RagIntegrationConfig,
}

/// Configuration for RAG integration
#[derive(Debug, Clone)]
pub struct RagIntegrationConfig {
    /// Maximum context cache size
    pub max_cache_size: usize,
    /// Context request timeout (ms)
    pub request_timeout_ms: u64,
    /// Default context limit
    pub default_context_limit: usize,
    /// Enable caching
    pub enable_caching: bool,
}

impl Default for RagIntegrationConfig {
    fn default() -> Self {
        Self {
            max_cache_size: 1000,
            request_timeout_ms: 100,
            default_context_limit: 10,
            enable_caching: true,
        }
    }
}

impl RagIntegrationService {
    /// Create a new RAG integration service
    pub fn new(coordinator: Arc<InterExExCoordinator>, config: RagIntegrationConfig) -> Self {
        Self {
            coordinator,
            context_cache: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Request context for a transaction
    pub async fn get_transaction_context(
        &self,
        tx_hash: B256,
        agent_id: &str,
        query: &str,
    ) -> Result<Vec<ContextData>> {
        // Check cache first
        if self.config.enable_caching {
            let cache = self.context_cache.read().await;
            if let Some(contexts) = cache.get(&tx_hash) {
                debug!("Returning cached context for tx {:?}", tx_hash);
                return Ok(contexts.clone());
            }
        }
        
        // Create response channel
        let (tx, mut rx) = mpsc::channel(1);
        
        // Store pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(tx_hash, tx);
        }
        
        // Send request to RAG Memory ExEx
        let request = SvmExExMessage::RequestRagContext {
            tx_hash,
            agent_id: agent_id.to_string(),
            query: query.to_string(),
            context_type: ContextType::Historical,
        };
        
        let msg = request.into();
        self.coordinator.broadcast(msg).await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to send RAG request: {}", e)))?;
        
        // Wait for response with timeout
        match tokio::time::timeout(
            tokio::time::Duration::from_millis(self.config.request_timeout_ms),
            rx.recv()
        ).await {
            Ok(Some(contexts)) => {
                // Cache the result
                if self.config.enable_caching {
                    let mut cache = self.context_cache.write().await;
                    if cache.len() >= self.config.max_cache_size {
                        // Simple eviction - remove first item
                        if let Some(first_key) = cache.keys().next().cloned() {
                            cache.remove(&first_key);
                        }
                    }
                    cache.insert(tx_hash, contexts.clone());
                }
                Ok(contexts)
            }
            Ok(None) => Err(SvmExExError::ProcessingError("No context received".to_string())),
            Err(_) => {
                warn!("RAG context request timed out for tx {:?}", tx_hash);
                // Remove pending request
                let mut pending = self.pending_requests.write().await;
                pending.remove(&tx_hash);
                
                // Return empty context on timeout
                Ok(vec![])
            }
        }
    }
    
    /// Handle incoming RAG context response
    pub async fn handle_context_response(
        &self,
        tx_hash: B256,
        contexts: Vec<ContextData>,
    ) -> Result<()> {
        let mut pending = self.pending_requests.write().await;
        if let Some(sender) = pending.remove(&tx_hash) {
            let _ = sender.send(contexts).await;
        }
        Ok(())
    }
    
    /// Store memory items
    pub async fn store_memory(
        &self,
        agent_id: &str,
        memories: Vec<MemoryItem>,
        priority: MemoryPriority,
    ) -> Result<()> {
        let msg = SvmExExMessage::StoreMemory {
            agent_id: agent_id.to_string(),
            memories,
            priority,
        };
        
        let exex_msg = msg.into();
        self.coordinator.broadcast(exex_msg).await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to store memory: {}", e)))?;
        
        Ok(())
    }
    
    /// Retrieve memories
    pub async fn retrieve_memories(
        &self,
        agent_id: &str,
        query: MemoryQuery,
        limit: usize,
    ) -> Result<Vec<MemoryItem>> {
        // For now, return empty - would implement full retrieval flow
        Ok(vec![])
    }
    
    /// Create a memory item from transaction execution
    pub fn create_memory_from_execution(
        &self,
        tx_hash: B256,
        result: &crate::svm::SvmExecutionResult,
        routing_decision: &RoutingDecision,
    ) -> MemoryItem {
        let mut metadata = HashMap::new();
        metadata.insert("tx_hash".to_string(), serde_json::json!(tx_hash.to_string()));
        metadata.insert("success".to_string(), serde_json::json!(result.success));
        metadata.insert("gas_used".to_string(), serde_json::json!(result.gas_used));
        metadata.insert("decision_type".to_string(), serde_json::json!(format!("{:?}", routing_decision.decision)));
        metadata.insert("confidence".to_string(), serde_json::json!(routing_decision.confidence));
        
        let content = format!(
            "SVM execution for tx {} - Success: {}, Gas: {}, Decision: {:?}",
            tx_hash, result.success, result.gas_used, routing_decision.decision
        );
        
        MemoryItem {
            id: format!("svm_exec_{}", tx_hash),
            content,
            embedding: vec![], // Would generate real embedding
            metadata,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            memory_type: MemoryType::Episodic,
        }
    }
    
    /// Clear context cache
    pub async fn clear_cache(&self) {
        let mut cache = self.context_cache.write().await;
        cache.clear();
        info!("Cleared RAG context cache");
    }
}

/// RAG-enhanced transaction processor
pub struct RagEnhancedProcessor {
    /// RAG integration service
    rag_service: Arc<RagIntegrationService>,
    /// AI decision engine
    decision_engine: Arc<crate::ai::decision_engine::EnhancedAIDecisionEngine>,
}

impl RagEnhancedProcessor {
    /// Create a new RAG-enhanced processor
    pub fn new(
        rag_service: Arc<RagIntegrationService>,
        decision_engine: Arc<crate::ai::decision_engine::EnhancedAIDecisionEngine>,
    ) -> Self {
        Self {
            rag_service,
            decision_engine,
        }
    }
    
    /// Process transaction with RAG context
    pub async fn process_with_context(
        &self,
        tx_hash: B256,
        tx_data: &[u8],
        sender: Address,
        agent_id: &str,
    ) -> Result<(RoutingDecision, Vec<ContextData>)> {
        // Get RAG context
        let query = format!("Transaction from {} with hash {}", sender, tx_hash);
        let contexts = self.rag_service
            .get_transaction_context(tx_hash, agent_id, &query)
            .await?;
        
        // Make routing decision with context
        let decision = self.decision_engine
            .analyze_transaction(tx_data)
            .await;
        
        // Store execution memory
        let memory = self.rag_service.create_memory_from_execution(
            tx_hash,
            &crate::svm::SvmExecutionResult {
                tx_hash,
                success: true,
                gas_used: 0,
                return_data: vec![],
                logs: vec![],
                state_changes: vec![],
            },
            &decision,
        );
        
        self.rag_service
            .store_memory(agent_id, vec![memory], MemoryPriority::Medium)
            .await?;
        
        Ok((decision, contexts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rag_service_creation() {
        let config = crate::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            InterExExCoordinator::new(config, "test-node".to_string())
                .expect("Failed to create coordinator")
        );
        
        let rag_config = RagIntegrationConfig::default();
        let service = RagIntegrationService::new(coordinator, rag_config);
        
        // Test cache is empty
        let cache = service.context_cache.read().await;
        assert!(cache.is_empty());
    }
}