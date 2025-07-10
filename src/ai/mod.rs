//! AI Module - Central AI coordination and decision-making system
//! 
//! This module provides:
//! - Enhanced AI decision engine with RAG context integration
//! - Context management for cross-ExEx coordination
//! - Learning and adaptation capabilities

pub mod traits;
pub mod decision_engine;
pub mod context_manager;
pub mod rag_adapter;

pub use traits::{
    AIDecisionEngine, RAGEnabledEngine, CrossExExCoordinator,
    TransactionContext, RoutingDecision, DecisionType, TransactionPriority,
    ExecutionFeedback, ConfidenceMetrics, AIEngineState,
};
pub use decision_engine::{EnhancedAIDecisionEngine, TransactionAnalysis, AnalysisFactors};
pub use context_manager::{ContextManager, AIContext, ContextUpdate};
pub use rag_adapter::{RAGAdapter, RAGQuery, RAGResponse, DocumentChunk};

use crate::errors::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Central AI coordinator that manages all AI subsystems
pub struct AICoordinator {
    /// Decision engine for routing and analysis
    pub decision_engine: Arc<EnhancedAIDecisionEngine>,
    
    /// Context manager for maintaining AI state
    pub context_manager: Arc<RwLock<ContextManager>>,
    
    /// RAG adapter for external knowledge integration
    pub rag_adapter: Arc<RAGAdapter>,
}

impl AICoordinator {
    /// Create a new AI coordinator
    pub fn new(rag_endpoint: Option<String>) -> Self {
        Self {
            decision_engine: Arc::new(EnhancedAIDecisionEngine::new()),
            context_manager: Arc::new(RwLock::new(ContextManager::new())),
            rag_adapter: Arc::new(RAGAdapter::new(rag_endpoint)),
        }
    }
    
    /// Initialize the AI system with configuration
    pub async fn initialize(&self, config: AIConfig) -> AIResult<()> {
        // Initialize decision engine with weights
        if let Some(weights) = config.decision_weights {
            self.decision_engine.update_weights_from_config(weights).await?;
        }
        
        // Initialize context manager
        self.context_manager.write().await.set_config(config.context_config);
        
        // Initialize RAG adapter
        if config.rag_enabled {
            self.rag_adapter.connect().await?;
        }
        
        Ok(())
    }
    
    /// Analyze transaction with full AI pipeline
    pub async fn analyze_with_context(
        &self,
        transaction_data: &[u8],
        agent_id: Option<String>,
    ) -> AIResult<TransactionAnalysis> {
        // Get relevant context from RAG
        let rag_context = if let Some(agent_id) = &agent_id {
            self.rag_adapter.query_agent_context(agent_id).await?
        } else {
            None
        };
        
        // Get historical context
        let historical_context = self.context_manager.read().await
            .get_transaction_context(transaction_data).await?;
        
        // Combine contexts
        let combined_context = self.merge_contexts(rag_context, historical_context);
        
        // Analyze with enhanced context
        let analysis = self.decision_engine
            .analyze_transaction_with_context(transaction_data, &combined_context)
            .await?;
        
        // Update context with results
        self.context_manager.write().await
            .update_from_analysis(&analysis, transaction_data)
            .await?;
        
        Ok(analysis)
    }
    
    /// Merge different context sources
    fn merge_contexts(
        &self,
        rag_context: Option<RAGResponse>,
        historical_context: AIContext,
    ) -> CombinedContext {
        CombinedContext {
            rag_insights: rag_context,
            historical_data: historical_context,
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Configuration for AI subsystems
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AIConfig {
    pub decision_weights: Option<DecisionWeights>,
    pub context_config: ContextConfig,
    pub rag_enabled: bool,
    pub learning_enabled: bool,
}

/// Decision weights configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DecisionWeights {
    pub complexity_weight: f64,
    pub safety_weight: f64,
    pub gas_price_weight: f64,
    pub congestion_weight: f64,
    pub history_weight: f64,
    pub anomaly_weight: f64,
    pub rag_context_weight: f64,
}

/// Context configuration
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContextConfig {
    pub max_history_size: usize,
    pub context_ttl_seconds: u64,
    pub enable_cross_exex_context: bool,
}

/// Combined context from multiple sources
#[derive(Debug, Clone)]
pub struct CombinedContext {
    pub rag_insights: Option<RAGResponse>,
    pub historical_data: AIContext,
    pub timestamp: std::time::SystemTime,
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            decision_weights: Some(DecisionWeights {
                complexity_weight: 0.25,
                safety_weight: 0.30,
                gas_price_weight: 0.15,
                congestion_weight: 0.10,
                history_weight: 0.15,
                anomaly_weight: 0.05,
                rag_context_weight: 0.0, // Disabled by default
            }),
            context_config: ContextConfig {
                max_history_size: 10000,
                context_ttl_seconds: 3600,
                enable_cross_exex_context: true,
            },
            rag_enabled: false,
            learning_enabled: true,
        }
    }
}