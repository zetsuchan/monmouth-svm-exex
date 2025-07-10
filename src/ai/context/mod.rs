//! Context management for RAG ExEx
//! 
//! This module provides context preprocessing and caching for efficient RAG operations.

pub mod preprocessing;
pub mod cache;

pub use preprocessing::{
    ContextPreprocessor, PreprocessingConfig, PreprocessedContext,
    ContextChunk, ContextMetadata, TransactionType,
};
pub use cache::{
    ContextCache, CacheConfig, CacheKey, CacheOperations,
};

use std::sync::Arc;

/// Combined context manager with preprocessing and caching
pub struct ContextPipeline {
    preprocessor: Arc<ContextPreprocessor>,
    cache: Arc<ContextCache>,
}

impl ContextPipeline {
    /// Create a new context pipeline
    pub fn new(
        preprocessing_config: PreprocessingConfig,
        cache_config: CacheConfig,
    ) -> Self {
        let cache = Arc::new(ContextCache::new(cache_config));
        cache.start_cleanup_task();
        
        Self {
            preprocessor: Arc::new(ContextPreprocessor::new(preprocessing_config)),
            cache,
        }
    }
    
    /// Process transaction with caching
    pub async fn process_transaction(
        &self,
        tx_hash: [u8; 32],
        tx_data: &[u8],
        agent_id: Option<String>,
        additional_context: Option<&str>,
    ) -> Result<Arc<PreprocessedContext>, crate::errors::AIAgentError> {
        // Check cache first
        let cache_key = CacheKey {
            tx_hash,
            version: 0,
            agent_id: agent_id.clone(),
        };
        
        if let Some(cached) = self.cache.get(&cache_key).await {
            return Ok(cached);
        }
        
        // Process if not cached
        let context = self.preprocessor
            .process_transaction(tx_hash, tx_data, additional_context)
            .await
            .map_err(|e| crate::errors::AIAgentError::ContextRetrievalFailed(e.to_string()))?;
        
        // Cache the result
        self.cache.put(cache_key, context.clone()).await
            .map_err(|e| crate::errors::AIAgentError::ProcessingError(e.to_string()))?;
        
        Ok(Arc::new(context))
    }
    
    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> cache::CacheStats {
        self.cache.get_stats().await
    }
    
    /// Get preprocessing metrics
    pub async fn get_preprocessing_metrics(&self) -> preprocessing::PreprocessingMetrics {
        self.preprocessor.get_metrics().await
    }
}