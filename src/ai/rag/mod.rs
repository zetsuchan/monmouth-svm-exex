//! RAG (Retrieval-Augmented Generation) module
//! 
//! This module provides comprehensive RAG capabilities for the AI system.

pub mod agent_context;

pub use agent_context::{
    AgentContextProvider, AgentContextConfig, AgentContext,
    RankedContext, ContextSource, SourceType, FusionStrategy,
};

use crate::ai::traits::ContextData;
use std::sync::Arc;

/// Complete RAG system with all components
pub struct RAGSystem {
    /// Agent context provider
    pub agent_provider: Arc<AgentContextProvider>,
    /// Vector store interface
    pub vector_store: Arc<dyn VectorStore>,
    /// Query processor
    pub query_processor: Arc<QueryProcessor>,
}

/// Trait for vector store implementations
#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    /// Store embeddings
    async fn store(&self, id: String, embedding: Vec<f32>, metadata: serde_json::Value) -> Result<(), crate::errors::AIAgentError>;
    
    /// Search for similar embeddings
    async fn search(&self, query_embedding: Vec<f32>, limit: usize) -> Result<Vec<SearchResult>, crate::errors::AIAgentError>;
    
    /// Delete embeddings
    async fn delete(&self, id: &str) -> Result<(), crate::errors::AIAgentError>;
}

/// Vector search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

/// Query processor for RAG
pub struct QueryProcessor {
    /// Query expansion enabled
    pub expand_queries: bool,
    /// Reranking enabled
    pub enable_reranking: bool,
}

impl QueryProcessor {
    /// Process a query for RAG
    pub async fn process_query(&self, query: &str) -> ProcessedQuery {
        ProcessedQuery {
            original: query.to_string(),
            expanded: if self.expand_queries {
                self.expand_query(query)
            } else {
                vec![query.to_string()]
            },
            embeddings: vec![], // Would generate actual embeddings
        }
    }
    
    /// Expand query with synonyms and related terms
    fn expand_query(&self, query: &str) -> Vec<String> {
        // Simple expansion for now
        vec![
            query.to_string(),
            format!("{} transaction", query),
            format!("{} smart contract", query),
        ]
    }
}

/// Processed query with expansions
#[derive(Debug, Clone)]
pub struct ProcessedQuery {
    pub original: String,
    pub expanded: Vec<String>,
    pub embeddings: Vec<Vec<f32>>,
}

impl RAGSystem {
    /// Create a new RAG system
    pub fn new(
        agent_provider: Arc<AgentContextProvider>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            agent_provider,
            vector_store,
            query_processor: Arc::new(QueryProcessor {
                expand_queries: true,
                enable_reranking: true,
            }),
        }
    }
    
    /// Retrieve context for a query
    pub async fn retrieve(
        &self,
        query: &str,
        agent_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<ContextData>, crate::errors::AIAgentError> {
        // Process query
        let processed = self.query_processor.process_query(query).await;
        
        // Search vector store
        let mut all_results = Vec::new();
        for (i, embedding) in processed.embeddings.iter().enumerate() {
            let results = self.vector_store.search(embedding.clone(), limit).await?;
            all_results.extend(results);
        }
        
        // Convert to context data
        let contexts: Vec<ContextData> = all_results
            .into_iter()
            .map(|r| ContextData {
                id: r.id,
                content: r.metadata.get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                embedding: vec![], // Would include actual embedding
                metadata: serde_json::from_value(r.metadata).unwrap_or_default(),
                timestamp: 0,
            })
            .collect();
        
        // Get agent-specific context if agent ID provided
        if let Some(agent_id) = agent_id {
            let agent_context = self.agent_provider
                .get_agent_context(agent_id, query, contexts)
                .await?;
            
            Ok(agent_context.contexts
                .into_iter()
                .map(|rc| rc.context)
                .collect())
        } else {
            Ok(contexts)
        }
    }
}