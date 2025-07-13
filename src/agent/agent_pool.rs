//! Agent transaction pool separate from regular mempool

use super::agent_tx::{AgentTx, AgentTxDecoder};
use crate::errors::*;
use dashmap::DashMap;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Agent transaction pool
pub struct AgentTransactionPool {
    /// Pool of pending transactions by ID
    transactions: Arc<DashMap<String, PooledAgentTx>>,
    
    /// Priority queue for transaction ordering
    priority_queue: Arc<RwLock<BinaryHeap<PriorityWrapper>>>,
    
    /// Index by agent ID
    by_agent: Arc<DashMap<String, Vec<String>>>,
    
    /// Index by intent type
    by_intent_type: Arc<DashMap<String, Vec<String>>>,
    
    /// Pool configuration
    config: PoolConfig,
    
    /// Pool metrics
    metrics: Arc<RwLock<PoolMetrics>>,
}

/// Pooled agent transaction
#[derive(Debug, Clone)]
pub struct PooledAgentTx {
    /// The agent transaction
    pub tx: AgentTx,
    
    /// Transaction ID
    pub id: String,
    
    /// Submission timestamp
    pub submitted_at: std::time::Instant,
    
    /// Priority score
    pub priority_score: f64,
    
    /// Validation status
    pub validated: bool,
    
    /// Retry count
    pub retry_count: u32,
}

/// Priority wrapper for heap ordering
#[derive(Debug, Clone)]
struct PriorityWrapper {
    tx_id: String,
    priority_score: f64,
}

impl PartialEq for PriorityWrapper {
    fn eq(&self, other: &Self) -> bool {
        self.priority_score == other.priority_score
    }
}

impl Eq for PriorityWrapper {}

impl PartialOrd for PriorityWrapper {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.priority_score.partial_cmp(&other.priority_score)
    }
}

impl Ord for PriorityWrapper {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority_score.partial_cmp(&other.priority_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Pool configuration
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum pool size
    pub max_size: usize,
    
    /// Maximum transactions per agent
    pub max_per_agent: usize,
    
    /// Transaction TTL in seconds
    pub ttl_seconds: u64,
    
    /// Enable priority scoring
    pub enable_priority_scoring: bool,
    
    /// Maximum retry count
    pub max_retries: u32,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            max_per_agent: 100,
            ttl_seconds: 300, // 5 minutes
            enable_priority_scoring: true,
            max_retries: 3,
        }
    }
}

/// Pool metrics
#[derive(Debug, Default)]
struct PoolMetrics {
    total_submitted: u64,
    total_accepted: u64,
    total_rejected: u64,
    total_executed: u64,
    total_expired: u64,
    current_size: usize,
}

impl AgentTransactionPool {
    /// Create a new agent transaction pool
    pub fn new(config: PoolConfig) -> Self {
        Self {
            transactions: Arc::new(DashMap::new()),
            priority_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            by_agent: Arc::new(DashMap::new()),
            by_intent_type: Arc::new(DashMap::new()),
            config,
            metrics: Arc::new(RwLock::new(PoolMetrics::default())),
        }
    }
    
    /// Submit a new agent transaction
    pub async fn submit(&self, tx: AgentTx) -> Result<String> {
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_submitted += 1;
        }
        
        // Validate transaction
        AgentTxDecoder::validate(&tx)?;
        
        // Check pool size
        if self.transactions.len() >= self.config.max_size {
            let mut metrics = self.metrics.write().await;
            metrics.total_rejected += 1;
            return Err(SvmExExError::ProcessingError(
                "Transaction pool is full".to_string()
            ));
        }
        
        // Check per-agent limit
        let agent_id = tx.agent.agent_id.clone();
        if let Some(agent_txs) = self.by_agent.get(&agent_id) {
            if agent_txs.len() >= self.config.max_per_agent {
                let mut metrics = self.metrics.write().await;
                metrics.total_rejected += 1;
                return Err(SvmExExError::ProcessingError(
                    format!("Agent {} has reached transaction limit", agent_id)
                ));
            }
        }
        
        // Generate transaction ID
        let tx_id = format!("agent-tx-{}", uuid::Uuid::new_v4());
        
        // Calculate priority score
        let priority_score = if self.config.enable_priority_scoring {
            self.calculate_priority_score(&tx)
        } else {
            tx.metadata.priority as f64
        };
        
        // Create pooled transaction
        let pooled_tx = PooledAgentTx {
            tx: tx.clone(),
            id: tx_id.clone(),
            submitted_at: std::time::Instant::now(),
            priority_score,
            validated: true,
            retry_count: 0,
        };
        
        // Add to pool
        self.transactions.insert(tx_id.clone(), pooled_tx);
        
        // Update indices
        self.by_agent
            .entry(agent_id)
            .or_insert_with(Vec::new)
            .push(tx_id.clone());
        
        self.by_intent_type
            .entry(format!("{:?}", tx.intent.intent_type))
            .or_insert_with(Vec::new)
            .push(tx_id.clone());
        
        // Add to priority queue
        let mut queue = self.priority_queue.write().await;
        queue.push(PriorityWrapper {
            tx_id: tx_id.clone(),
            priority_score,
        });
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_accepted += 1;
            metrics.current_size = self.transactions.len();
        }
        
        info!("Accepted agent transaction {} with priority {}", tx_id, priority_score);
        Ok(tx_id)
    }
    
    /// Get next transaction for execution
    pub async fn get_next(&self) -> Option<PooledAgentTx> {
        let mut queue = self.priority_queue.write().await;
        
        while let Some(wrapper) = queue.pop() {
            if let Some(pooled_tx) = self.transactions.get(&wrapper.tx_id) {
                // Check if transaction has expired
                let age = pooled_tx.submitted_at.elapsed().as_secs();
                if age > self.config.ttl_seconds {
                    // Remove expired transaction
                    self.remove_transaction(&wrapper.tx_id).await;
                    let mut metrics = self.metrics.write().await;
                    metrics.total_expired += 1;
                    continue;
                }
                
                return Some(pooled_tx.clone());
            }
        }
        
        None
    }
    
    /// Get transactions by agent ID
    pub fn get_by_agent(&self, agent_id: &str) -> Vec<PooledAgentTx> {
        if let Some(tx_ids) = self.by_agent.get(agent_id) {
            tx_ids.iter()
                .filter_map(|id| self.transactions.get(id).map(|tx| tx.clone()))
                .collect()
        } else {
            vec![]
        }
    }
    
    /// Get transactions by intent type
    pub fn get_by_intent_type(&self, intent_type: &str) -> Vec<PooledAgentTx> {
        if let Some(tx_ids) = self.by_intent_type.get(intent_type) {
            tx_ids.iter()
                .filter_map(|id| self.transactions.get(id).map(|tx| tx.clone()))
                .collect()
        } else {
            vec![]
        }
    }
    
    /// Mark transaction as executed
    pub async fn mark_executed(&self, tx_id: &str) -> Result<()> {
        if let Some((_, pooled_tx)) = self.transactions.remove(tx_id) {
            // Remove from indices
            self.remove_from_indices(&pooled_tx.tx).await;
            
            // Update metrics
            let mut metrics = self.metrics.write().await;
            metrics.total_executed += 1;
            metrics.current_size = self.transactions.len();
            
            info!("Transaction {} marked as executed", tx_id);
            Ok(())
        } else {
            Err(SvmExExError::ProcessingError(
                format!("Transaction {} not found", tx_id)
            ))
        }
    }
    
    /// Retry a failed transaction
    pub async fn retry_transaction(&self, tx_id: &str) -> Result<()> {
        if let Some(mut pooled_tx) = self.transactions.get_mut(tx_id) {
            pooled_tx.retry_count += 1;
            
            if pooled_tx.retry_count > self.config.max_retries {
                // Remove transaction after max retries
                drop(pooled_tx);
                self.remove_transaction(tx_id).await;
                return Err(SvmExExError::ProcessingError(
                    format!("Transaction {} exceeded max retries", tx_id)
                ));
            }
            
            // Re-add to priority queue with lower priority
            let new_priority = pooled_tx.priority_score * 0.9;
            pooled_tx.priority_score = new_priority;
            
            let mut queue = self.priority_queue.write().await;
            queue.push(PriorityWrapper {
                tx_id: tx_id.to_string(),
                priority_score: new_priority,
            });
            
            info!("Retrying transaction {} (attempt {})", tx_id, pooled_tx.retry_count);
            Ok(())
        } else {
            Err(SvmExExError::ProcessingError(
                format!("Transaction {} not found", tx_id)
            ))
        }
    }
    
    /// Remove a transaction from the pool
    async fn remove_transaction(&self, tx_id: &str) {
        if let Some((_, pooled_tx)) = self.transactions.remove(tx_id) {
            self.remove_from_indices(&pooled_tx.tx).await;
        }
    }
    
    /// Remove transaction from indices
    async fn remove_from_indices(&self, tx: &AgentTx) {
        // Remove from agent index
        if let Some(mut agent_txs) = self.by_agent.get_mut(&tx.agent.agent_id) {
            agent_txs.retain(|id| id != &tx.agent.agent_id);
        }
        
        // Remove from intent type index
        let intent_type = format!("{:?}", tx.intent.intent_type);
        if let Some(mut intent_txs) = self.by_intent_type.get_mut(&intent_type) {
            intent_txs.retain(|id| id != &tx.agent.agent_id);
        }
    }
    
    /// Calculate priority score for a transaction
    fn calculate_priority_score(&self, tx: &AgentTx) -> f64 {
        let mut score = tx.metadata.priority as f64;
        
        // Factor in resource requirements
        score += (tx.resources.total_gas_estimate as f64 / 1_000_000.0).min(10.0);
        
        // Factor in intent type
        score += match tx.intent.intent_type {
            super::agent_tx::IntentType::Bridge => 5.0,
            super::agent_tx::IntentType::Swap => 3.0,
            super::agent_tx::IntentType::Transfer => 2.0,
            _ => 1.0,
        };
        
        // Factor in cross-chain complexity
        score += tx.cross_chain_plans.len() as f64 * 2.0;
        
        // Factor in agent type
        score += match tx.agent.agent_type {
            super::agent_tx::AgentType::Orchestrator => 10.0,
            super::agent_tx::AgentType::Validator => 5.0,
            super::agent_tx::AgentType::Analyzer => 3.0,
            super::agent_tx::AgentType::Executor => 1.0,
        };
        
        score
    }
    
    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let metrics = self.metrics.read().await;
        
        PoolStats {
            total_submitted: metrics.total_submitted,
            total_accepted: metrics.total_accepted,
            total_rejected: metrics.total_rejected,
            total_executed: metrics.total_executed,
            total_expired: metrics.total_expired,
            current_size: metrics.current_size,
            by_agent_count: self.by_agent.len(),
            by_intent_count: self.by_intent_type.len(),
        }
    }
    
    /// Start cleanup task for expired transactions
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                self.cleanup_expired().await;
            }
        });
    }
    
    /// Clean up expired transactions
    async fn cleanup_expired(&self) {
        let now = std::time::Instant::now();
        let mut expired_ids = Vec::new();
        
        // Find expired transactions
        for entry in self.transactions.iter() {
            let age = now.duration_since(entry.submitted_at).as_secs();
            if age > self.config.ttl_seconds {
                expired_ids.push(entry.id.clone());
            }
        }
        
        // Remove expired transactions
        for tx_id in expired_ids {
            self.remove_transaction(&tx_id).await;
            
            let mut metrics = self.metrics.write().await;
            metrics.total_expired += 1;
            metrics.current_size = self.transactions.len();
        }
        
        if !expired_ids.is_empty() {
            info!("Cleaned up {} expired agent transactions", expired_ids.len());
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_submitted: u64,
    pub total_accepted: u64,
    pub total_rejected: u64,
    pub total_executed: u64,
    pub total_expired: u64,
    pub current_size: usize,
    pub by_agent_count: usize,
    pub by_intent_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agent_tx::*;
    
    #[tokio::test]
    async fn test_pool_creation() {
        let pool = AgentTransactionPool::new(PoolConfig::default());
        let stats = pool.get_stats().await;
        
        assert_eq!(stats.current_size, 0);
        assert_eq!(stats.total_submitted, 0);
    }
    
    #[tokio::test]
    async fn test_transaction_submission() {
        let pool = AgentTransactionPool::new(PoolConfig::default());
        
        let tx = create_test_agent_tx();
        let result = pool.submit(tx).await;
        
        assert!(result.is_ok());
        
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_submitted, 1);
        assert_eq!(stats.total_accepted, 1);
        assert_eq!(stats.current_size, 1);
    }
    
    #[tokio::test]
    async fn test_get_next_transaction() {
        let pool = AgentTransactionPool::new(PoolConfig::default());
        
        // Submit multiple transactions with different priorities
        for i in 0..3 {
            let mut tx = create_test_agent_tx();
            tx.metadata.priority = (3 - i) as u32;
            pool.submit(tx).await.unwrap();
        }
        
        // Should get highest priority first
        let next = pool.get_next().await;
        assert!(next.is_some());
        
        let next_tx = next.unwrap();
        assert!(next_tx.priority_score > 0.0);
    }
    
    fn create_test_agent_tx() -> AgentTx {
        AgentTx {
            agent: AgentIdentity {
                agent_id: "test-agent".to_string(),
                public_key: vec![0u8; 32],
                agent_type: AgentType::Executor,
                capabilities: vec!["execute".to_string()],
                attributes: HashMap::new(),
            },
            intent: Intent {
                intent_id: uuid::Uuid::new_v4().to_string(),
                intent_type: IntentType::Transfer,
                description: "Test transfer".to_string(),
                parameters: vec![],
                constraints: vec![],
                expiration_timestamp: 0,
            },
            memory_context: MemoryContext {
                required_memories: vec![],
                memory_root: alloy_primitives::B256::ZERO,
                requirements: vec![],
                max_memory_age_seconds: 3600,
            },
            cross_chain_plans: vec![],
            resources: ResourceRequirements {
                total_gas_estimate: 100_000,
                total_compute_units: 50_000,
                per_chain_allocation: HashMap::new(),
                requires_gpu: false,
                requires_tee: false,
            },
            metadata: AgentTxMetadata {
                nonce: 0,
                timestamp: 0,
                correlation_id: uuid::Uuid::new_v4().to_string(),
                tags: vec![],
                labels: HashMap::new(),
                priority: 1,
            },
            signature: vec![0u8; 64],
        }
    }
}