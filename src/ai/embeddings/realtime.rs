//! Real-time Embedding Pipeline for SVM Transactions
//! 
//! This module provides optimized embedding generation for real-time
//! SVM transaction processing with streaming capabilities.

use crate::errors::*;
use crate::ai::context::preprocessing::{PreprocessedContext, ContextChunk};
use crate::ai::traits::EmbeddingData;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tokio::time::timeout;
use tracing::{debug, info, warn, error};

/// Real-time embedding pipeline with streaming support
pub struct RealtimeEmbeddingPipeline {
    /// Embedding models
    models: Arc<ModelManager>,
    /// Streaming processor
    stream_processor: Arc<StreamProcessor>,
    /// Embedding cache
    cache: Arc<EmbeddingCache>,
    /// Pipeline configuration
    config: PipelineConfig,
    /// Performance monitor
    monitor: Arc<PerformanceMonitor>,
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Maximum concurrent embeddings
    pub max_concurrent: usize,
    /// Embedding timeout (ms)
    pub timeout_ms: u64,
    /// Enable streaming mode
    pub streaming_enabled: bool,
    /// Batch size for micro-batching
    pub micro_batch_size: usize,
    /// Cache TTL
    pub cache_ttl: Duration,
    /// Model selection strategy
    pub model_strategy: ModelStrategy,
}

/// Model selection strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelStrategy {
    /// Use fastest available model
    Fastest,
    /// Use most accurate model
    MostAccurate,
    /// Balance speed and accuracy
    Balanced,
    /// Adaptive based on load
    Adaptive,
}

/// Embedding model manager
pub struct ModelManager {
    /// Available models
    models: DashMap<String, Arc<dyn EmbeddingModel>>,
    /// Model performance stats
    stats: Arc<RwLock<ModelStats>>,
    /// Active model
    active_model: Arc<RwLock<String>>,
}

/// Model performance statistics
#[derive(Debug, Clone, Default)]
pub struct ModelStats {
    /// Stats per model
    model_stats: HashMap<String, ModelPerformance>,
}

/// Individual model performance
#[derive(Debug, Clone, Default)]
pub struct ModelPerformance {
    /// Average embedding time
    avg_time_ms: f64,
    /// Total embeddings generated
    total_embeddings: u64,
    /// Error rate
    error_rate: f64,
    /// Quality score
    quality_score: f64,
}

/// Trait for embedding models
#[async_trait]
pub trait EmbeddingModel: Send + Sync {
    /// Generate embedding for text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Generate embeddings for batch
    async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>>;
    
    /// Get model info
    fn info(&self) -> ModelInfo;
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name
    pub name: String,
    /// Model version
    pub version: String,
    /// Embedding dimension
    pub dimension: usize,
    /// Maximum input length
    pub max_length: usize,
    /// Model type
    pub model_type: ModelType,
}

/// Model types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    /// Transformer-based model
    Transformer,
    /// Lightweight model
    Lightweight,
    /// Specialized for blockchain
    Blockchain,
}

/// Stream processor for continuous embedding
pub struct StreamProcessor {
    /// Input stream
    input_stream: Arc<Mutex<mpsc::Receiver<EmbeddingRequest>>>,
    /// Output stream
    output_stream: Arc<mpsc::Sender<EmbeddingResult>>,
    /// Processing semaphore
    semaphore: Arc<Semaphore>,
    /// Micro-batch buffer
    batch_buffer: Arc<Mutex<VecDeque<EmbeddingRequest>>>,
}

/// Embedding request
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    /// Request ID
    pub id: String,
    /// Text to embed
    pub text: String,
    /// Priority level
    pub priority: Priority,
    /// Timestamp
    pub timestamp: Instant,
    /// Callback channel
    pub callback: Option<mpsc::Sender<EmbeddingResult>>,
}

/// Priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

/// Embedding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    /// Request ID
    pub id: String,
    /// Generated embedding
    pub embedding: Vec<f32>,
    /// Model used
    pub model: String,
    /// Processing time
    pub processing_time: Duration,
    /// Cache hit
    pub cache_hit: bool,
}

/// Embedding cache
pub struct EmbeddingCache {
    /// Cache storage
    cache: DashMap<u64, CachedEmbedding>,
    /// TTL tracking
    ttl_tracker: Arc<Mutex<HashMap<u64, Instant>>>,
}

/// Cached embedding
#[derive(Debug, Clone)]
pub struct CachedEmbedding {
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Model used
    pub model: String,
    /// Creation time
    pub created_at: Instant,
    /// Hit count
    pub hits: u32,
}

/// Performance monitor
pub struct PerformanceMonitor {
    /// Current metrics
    metrics: Arc<RwLock<PipelineMetrics>>,
    /// Metric history
    history: Arc<Mutex<VecDeque<MetricSnapshot>>>,
}

/// Pipeline metrics
#[derive(Debug, Clone, Default)]
pub struct PipelineMetrics {
    /// Total requests processed
    pub total_requests: u64,
    /// Average latency
    pub avg_latency_ms: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Current queue depth
    pub queue_depth: usize,
    /// Throughput (embeddings/sec)
    pub throughput: f64,
}

/// Metric snapshot
#[derive(Debug, Clone)]
pub struct MetricSnapshot {
    /// Timestamp
    pub timestamp: Instant,
    /// Metrics at time
    pub metrics: PipelineMetrics,
}

use std::collections::HashMap;

impl RealtimeEmbeddingPipeline {
    /// Create a new real-time embedding pipeline
    pub fn new(config: PipelineConfig) -> Self {
        let (input_tx, input_rx) = mpsc::channel(1000);
        let (output_tx, output_rx) = mpsc::channel(1000);
        
        Self {
            models: Arc::new(ModelManager::new()),
            stream_processor: Arc::new(StreamProcessor {
                input_stream: Arc::new(Mutex::new(input_rx)),
                output_stream: Arc::new(output_tx),
                semaphore: Arc::new(Semaphore::new(config.max_concurrent)),
                batch_buffer: Arc::new(Mutex::new(VecDeque::with_capacity(config.micro_batch_size))),
            }),
            cache: Arc::new(EmbeddingCache::new()),
            config,
            monitor: Arc::new(PerformanceMonitor::new()),
        }
    }
    
    /// Start the pipeline
    pub async fn start(&self) -> Result<()> {
        info!("Starting real-time embedding pipeline");
        
        // Start stream processor
        self.start_stream_processor().await?;
        
        // Start cache cleanup
        self.start_cache_cleanup().await?;
        
        // Start performance monitoring
        self.start_monitoring().await?;
        
        Ok(())
    }
    
    /// Embed a single text in real-time
    pub async fn embed_realtime(&self, text: String, priority: Priority) -> Result<EmbeddingResult> {
        let start = Instant::now();
        
        // Check cache first
        let cache_key = self.compute_cache_key(&text);
        if let Some(cached) = self.cache.get(&cache_key) {
            self.monitor.record_cache_hit().await;
            return Ok(EmbeddingResult {
                id: uuid::Uuid::new_v4().to_string(),
                embedding: cached.embedding.clone(),
                model: cached.model.clone(),
                processing_time: start.elapsed(),
                cache_hit: true,
            });
        }
        
        // Create embedding request
        let (callback_tx, mut callback_rx) = mpsc::channel(1);
        let request = EmbeddingRequest {
            id: uuid::Uuid::new_v4().to_string(),
            text: text.clone(),
            priority,
            timestamp: Instant::now(),
            callback: Some(callback_tx),
        };
        
        // Submit to stream processor
        self.submit_request(request).await?;
        
        // Wait for result with timeout
        match timeout(Duration::from_millis(self.config.timeout_ms), callback_rx.recv()).await {
            Ok(Some(result)) => {
                // Cache the result
                self.cache.insert(cache_key, CachedEmbedding {
                    embedding: result.embedding.clone(),
                    model: result.model.clone(),
                    created_at: Instant::now(),
                    hits: 0,
                });
                
                self.monitor.record_request(start.elapsed()).await;
                Ok(result)
            }
            Ok(None) => Err(AIAgentError::ProcessingError("Channel closed".to_string())),
            Err(_) => Err(AIAgentError::ProcessingError("Embedding timeout".to_string())),
        }
    }
    
    /// Embed preprocessed context
    pub async fn embed_context(&self, context: &PreprocessedContext) -> Result<Vec<EmbeddingResult>> {
        let mut futures = Vec::new();
        
        for chunk in &context.chunks {
            let text = chunk.content.clone();
            let priority = if chunk.relevance_score.unwrap_or(0.5) > 0.8 {
                Priority::High
            } else {
                Priority::Normal
            };
            
            futures.push(self.embed_realtime(text, priority));
        }
        
        // Process embeddings concurrently
        let results = futures::future::join_all(futures).await;
        
        // Collect successful results
        let embeddings: Result<Vec<_>> = results.into_iter().collect();
        embeddings
    }
    
    /// Submit embedding request
    async fn submit_request(&self, request: EmbeddingRequest) -> Result<()> {
        // Add to batch buffer for micro-batching
        let mut buffer = self.stream_processor.batch_buffer.lock().await;
        buffer.push_back(request);
        
        // Process if buffer is full
        if buffer.len() >= self.config.micro_batch_size {
            self.process_batch().await?;
        }
        
        Ok(())
    }
    
    /// Process a batch of requests
    async fn process_batch(&self) -> Result<()> {
        let mut buffer = self.stream_processor.batch_buffer.lock().await;
        if buffer.is_empty() {
            return Ok(());
        }
        
        // Extract batch
        let batch: Vec<EmbeddingRequest> = buffer.drain(..).collect();
        let batch_size = batch.len();
        
        // Get active model
        let model_name = self.models.active_model.read().await.clone();
        let model = self.models.get_model(&model_name)?;
        
        // Prepare texts
        let texts: Vec<&str> = batch.iter().map(|r| r.text.as_str()).collect();
        
        // Generate embeddings
        match model.embed_batch(texts).await {
            Ok(embeddings) => {
                // Send results
                for (request, embedding) in batch.into_iter().zip(embeddings.into_iter()) {
                    let result = EmbeddingResult {
                        id: request.id,
                        embedding,
                        model: model_name.clone(),
                        processing_time: request.timestamp.elapsed(),
                        cache_hit: false,
                    };
                    
                    // Send to callback if available
                    if let Some(callback) = request.callback {
                        let _ = callback.send(result).await;
                    }
                }
                
                self.models.update_stats(&model_name, batch_size, true).await;
            }
            Err(e) => {
                error!("Batch embedding failed: {}", e);
                self.models.update_stats(&model_name, batch_size, false).await;
                
                // Fallback to individual processing
                for request in batch {
                    let _ = self.process_single_request(request).await;
                }
            }
        }
        
        Ok(())
    }
    
    /// Process single request (fallback)
    async fn process_single_request(&self, request: EmbeddingRequest) -> Result<()> {
        let model_name = self.models.active_model.read().await.clone();
        let model = self.models.get_model(&model_name)?;
        
        match model.embed(&request.text).await {
            Ok(embedding) => {
                let result = EmbeddingResult {
                    id: request.id,
                    embedding,
                    model: model_name,
                    processing_time: request.timestamp.elapsed(),
                    cache_hit: false,
                };
                
                if let Some(callback) = request.callback {
                    let _ = callback.send(result).await;
                }
            }
            Err(e) => {
                error!("Single embedding failed: {}", e);
            }
        }
        
        Ok(())
    }
    
    /// Start stream processor task
    async fn start_stream_processor(&self) -> Result<()> {
        let processor = self.stream_processor.clone();
        let models = self.models.clone();
        let config = self.config.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(10));
            
            loop {
                interval.tick().await;
                
                // Check for pending requests
                let buffer_len = processor.batch_buffer.lock().await.len();
                if buffer_len > 0 {
                    // Process batch
                    debug!("Processing batch of {} requests", buffer_len);
                }
            }
        });
        
        Ok(())
    }
    
    /// Start cache cleanup task
    async fn start_cache_cleanup(&self) -> Result<()> {
        let cache = self.cache.clone();
        let ttl = self.config.cache_ttl;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                cache.cleanup(ttl).await;
            }
        });
        
        Ok(())
    }
    
    /// Start performance monitoring
    async fn start_monitoring(&self) -> Result<()> {
        let monitor = self.monitor.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                monitor.take_snapshot().await;
            }
        });
        
        Ok(())
    }
    
    /// Compute cache key for text
    fn compute_cache_key(&self, text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }
    
    /// Get pipeline metrics
    pub async fn get_metrics(&self) -> PipelineMetrics {
        self.monitor.metrics.read().await.clone()
    }
}

impl ModelManager {
    /// Create a new model manager
    pub fn new() -> Self {
        let manager = Self {
            models: DashMap::new(),
            stats: Arc::new(RwLock::new(ModelStats::default())),
            active_model: Arc::new(RwLock::new(String::new())),
        };
        
        // Register default models
        manager.register_default_models();
        
        manager
    }
    
    /// Register default models
    fn register_default_models(&self) {
        // Register lightweight model
        self.models.insert(
            "lightweight".to_string(),
            Arc::new(LightweightModel::new()),
        );
        
        // Set as active
        *self.active_model.blocking_write() = "lightweight".to_string();
    }
    
    /// Get a model by name
    fn get_model(&self, name: &str) -> Result<Arc<dyn EmbeddingModel>> {
        self.models
            .get(name)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AIAgentError::ProcessingError(format!("Model {} not found", name)))
    }
    
    /// Update model statistics
    async fn update_stats(&self, model: &str, count: usize, success: bool) {
        let mut stats = self.stats.write().await;
        let model_stats = stats.model_stats.entry(model.to_string())
            .or_insert_with(ModelPerformance::default);
        
        model_stats.total_embeddings += count as u64;
        if !success {
            model_stats.error_rate = 
                (model_stats.error_rate * model_stats.total_embeddings as f64 + count as f64) 
                / (model_stats.total_embeddings + count) as f64;
        }
    }
}

impl EmbeddingCache {
    /// Create a new embedding cache
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
            ttl_tracker: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get cached embedding
    pub fn get(&self, key: &u64) -> Option<CachedEmbedding> {
        self.cache.get_mut(key).map(|mut entry| {
            entry.hits += 1;
            entry.value().clone()
        })
    }
    
    /// Insert embedding into cache
    pub fn insert(&self, key: u64, embedding: CachedEmbedding) {
        self.cache.insert(key, embedding);
    }
    
    /// Clean up expired entries
    pub async fn cleanup(&self, ttl: Duration) {
        let now = Instant::now();
        let expired: Vec<u64> = self.cache
            .iter()
            .filter(|entry| now.duration_since(entry.value().created_at) > ttl)
            .map(|entry| *entry.key())
            .collect();
        
        for key in expired {
            self.cache.remove(&key);
        }
    }
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PipelineMetrics::default())),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
        }
    }
    
    /// Record a request
    pub async fn record_request(&self, latency: Duration) {
        let mut metrics = self.metrics.write().await;
        metrics.total_requests += 1;
        
        // Update average latency
        let n = metrics.total_requests as f64;
        metrics.avg_latency_ms = 
            (metrics.avg_latency_ms * (n - 1.0) + latency.as_millis() as f64) / n;
    }
    
    /// Record cache hit
    pub async fn record_cache_hit(&self) {
        let mut metrics = self.metrics.write().await;
        let total = metrics.total_requests as f64;
        let hits = metrics.cache_hit_rate * total + 1.0;
        metrics.cache_hit_rate = hits / (total + 1.0);
        metrics.total_requests += 1;
    }
    
    /// Take metrics snapshot
    pub async fn take_snapshot(&self) {
        let snapshot = MetricSnapshot {
            timestamp: Instant::now(),
            metrics: self.metrics.read().await.clone(),
        };
        
        let mut history = self.history.lock().await;
        history.push_back(snapshot);
        
        // Keep only recent history
        while history.len() > 100 {
            history.pop_front();
        }
    }
}

/// Lightweight embedding model implementation
struct LightweightModel {
    dimension: usize,
}

impl LightweightModel {
    fn new() -> Self {
        Self { dimension: 384 }
    }
}

#[async_trait]
impl EmbeddingModel for LightweightModel {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        // Simulate embedding generation
        let mut embedding = vec![0.0; self.dimension];
        for (i, byte) in text.bytes().enumerate() {
            embedding[i % self.dimension] += byte as f32 / 255.0;
        }
        
        // Normalize
        let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            for x in &mut embedding {
                *x /= norm;
            }
        }
        
        Ok(embedding)
    }
    
    async fn embed_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::new();
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }
    
    fn info(&self) -> ModelInfo {
        ModelInfo {
            name: "lightweight".to_string(),
            version: "1.0.0".to_string(),
            dimension: self.dimension,
            max_length: 512,
            model_type: ModelType::Lightweight,
        }
    }
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 100,
            timeout_ms: 1000,
            streaming_enabled: true,
            micro_batch_size: 32,
            cache_ttl: Duration::from_secs(3600),
            model_strategy: ModelStrategy::Balanced,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_realtime_embedding() {
        let config = PipelineConfig::default();
        let pipeline = RealtimeEmbeddingPipeline::new(config);
        
        let result = pipeline.embed_realtime(
            "Test transaction data".to_string(),
            Priority::Normal
        ).await;
        
        assert!(result.is_ok());
        let embedding = result.unwrap();
        assert!(!embedding.embedding.is_empty());
        assert_eq!(embedding.embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_embedding_cache() {
        let cache = EmbeddingCache::new();
        
        let embedding = CachedEmbedding {
            embedding: vec![0.1; 384],
            model: "test".to_string(),
            created_at: Instant::now(),
            hits: 0,
        };
        
        cache.insert(12345, embedding);
        assert!(cache.get(&12345).is_some());
    }
}