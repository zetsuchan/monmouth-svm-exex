//! Embedding generation and management
//! 
//! This module provides real-time and batch embedding generation capabilities.

pub mod realtime;
pub mod batch;

pub use realtime::{
    RealtimeEmbeddingPipeline, PipelineConfig, ModelStrategy,
    EmbeddingRequest, EmbeddingResult, Priority,
};
pub use batch::{
    BatchEmbeddingProcessor, BatchConfig, Batch, BatchItem,
    BatchType, CompletedBatch, OutputFormat,
};

use std::sync::Arc;

/// Unified embedding service combining real-time and batch processing
pub struct EmbeddingService {
    /// Real-time pipeline
    pub realtime: Arc<RealtimeEmbeddingPipeline>,
    /// Batch processor
    pub batch: Arc<BatchEmbeddingProcessor>,
    /// Service configuration
    config: ServiceConfig,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Enable auto-batching for real-time overflow
    pub auto_batching: bool,
    /// Threshold for auto-batching (requests/sec)
    pub batch_threshold: f64,
    /// Prefer real-time for small requests
    pub realtime_size_limit: usize,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(
        realtime_config: PipelineConfig,
        batch_config: BatchConfig,
        service_config: ServiceConfig,
    ) -> Self {
        Self {
            realtime: Arc::new(RealtimeEmbeddingPipeline::new(realtime_config)),
            batch: Arc::new(BatchEmbeddingProcessor::new(batch_config)),
            config: service_config,
        }
    }
    
    /// Start the embedding service
    pub async fn start(&self) -> Result<(), crate::errors::AIAgentError> {
        // Start real-time pipeline
        self.realtime.start().await?;
        
        // Start batch processor in background
        let batch = self.batch.clone();
        tokio::spawn(async move {
            loop {
                if let Err(e) = batch.process_queue().await {
                    tracing::error!("Batch processing error: {}", e);
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        });
        
        Ok(())
    }
    
    /// Process embedding request (auto-routes to real-time or batch)
    pub async fn process(&self, text: String, priority: Priority) -> Result<EmbeddingResult, crate::errors::AIAgentError> {
        // Route based on size and current load
        if text.len() <= self.config.realtime_size_limit {
            // Use real-time for small texts
            self.realtime.embed_realtime(text, priority).await
        } else {
            // Create single-item batch for large texts
            let batch = Batch {
                id: uuid::Uuid::new_v4().to_string(),
                items: vec![BatchItem {
                    id: uuid::Uuid::new_v4().to_string(),
                    text,
                    metadata: serde_json::json!({}),
                    priority,
                }],
                metadata: batch::BatchMetadata {
                    source: "auto_batch".to_string(),
                    batch_type: BatchType::RealtimeOverflow,
                    total_items: 1,
                    custom: serde_json::json!({}),
                },
                created_at: std::time::Instant::now(),
            };
            
            let batch_id = self.batch.submit_batch(batch).await?;
            
            // Wait for completion (simplified - in production would use better mechanism)
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            
            // Return placeholder result
            Ok(EmbeddingResult {
                id: batch_id,
                embedding: vec![0.0; 384], // Placeholder
                model: "batch".to_string(),
                processing_time: std::time::Duration::from_millis(100),
                cache_hit: false,
            })
        }
    }
    
    /// Get service metrics
    pub async fn get_metrics(&self) -> ServiceMetrics {
        ServiceMetrics {
            realtime_metrics: self.realtime.get_metrics().await,
            batch_stats: self.batch.get_global_stats().await,
        }
    }
}

/// Combined service metrics
#[derive(Debug, Clone)]
pub struct ServiceMetrics {
    pub realtime_metrics: realtime::PipelineMetrics,
    pub batch_stats: batch::GlobalStats,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            auto_batching: true,
            batch_threshold: 1000.0,
            realtime_size_limit: 1024,
        }
    }
}