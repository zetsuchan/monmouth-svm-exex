//! Batch Embedding Processor for Large-Scale Operations
//! 
//! This module provides efficient batch processing for embedding generation,
//! optimized for bulk operations and offline processing.

use crate::errors::*;
use crate::ai::context::preprocessing::PreprocessedContext;
use crate::ai::embeddings::realtime::{EmbeddingModel, ModelInfo, Priority};
use async_trait::async_trait;
use dashmap::DashMap;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tokio::fs;
use tracing::{debug, info, warn};

/// Batch embedding processor for large-scale operations
pub struct BatchEmbeddingProcessor {
    /// Batch queue
    queue: Arc<BatchQueue>,
    /// Processing engine
    engine: Arc<ProcessingEngine>,
    /// Storage manager
    storage: Arc<StorageManager>,
    /// Configuration
    config: BatchConfig,
    /// Progress tracker
    progress: Arc<ProgressTracker>,
}

/// Batch processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Number of parallel workers
    pub num_workers: usize,
    /// Processing timeout per batch
    pub batch_timeout: Duration,
    /// Enable checkpointing
    pub enable_checkpoints: bool,
    /// Checkpoint interval
    pub checkpoint_interval: usize,
    /// Output format
    pub output_format: OutputFormat,
    /// Compression for storage
    pub enable_compression: bool,
}

/// Output formats for batch results
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Binary format (efficient)
    Binary,
    /// JSON format (readable)
    Json,
    /// Parquet format (analytics)
    Parquet,
    /// HDF5 format (scientific)
    HDF5,
}

/// Batch queue for processing
pub struct BatchQueue {
    /// Pending batches
    pending: Arc<Mutex<VecDeque<Batch>>>,
    /// Active batches
    active: DashMap<String, BatchStatus>,
    /// Completed batches
    completed: Arc<Mutex<Vec<CompletedBatch>>>,
}

/// Batch of items to process
#[derive(Debug, Clone)]
pub struct Batch {
    /// Batch ID
    pub id: String,
    /// Items in batch
    pub items: Vec<BatchItem>,
    /// Batch metadata
    pub metadata: BatchMetadata,
    /// Creation timestamp
    pub created_at: Instant,
}

/// Individual batch item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchItem {
    /// Item ID
    pub id: String,
    /// Text content
    pub text: String,
    /// Item metadata
    pub metadata: serde_json::Value,
    /// Processing priority
    pub priority: Priority,
}

/// Batch metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Source identifier
    pub source: String,
    /// Batch type
    pub batch_type: BatchType,
    /// Total items
    pub total_items: usize,
    /// Custom metadata
    pub custom: serde_json::Value,
}

/// Batch types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BatchType {
    /// Transaction contexts
    TransactionContext,
    /// Historical data
    Historical,
    /// Real-time overflow
    RealtimeOverflow,
    /// Reprocessing
    Reprocessing,
}

/// Batch processing status
#[derive(Debug, Clone)]
pub struct BatchStatus {
    /// Current state
    pub state: BatchState,
    /// Progress percentage
    pub progress: f32,
    /// Items processed
    pub items_processed: usize,
    /// Errors encountered
    pub errors: Vec<ProcessingError>,
    /// Start time
    pub started_at: Option<Instant>,
}

/// Batch processing states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchState {
    Queued,
    Processing,
    Checkpointing,
    Completed,
    Failed,
    Cancelled,
}

/// Processing error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingError {
    /// Item ID
    pub item_id: String,
    /// Error message
    pub message: String,
    /// Recoverable flag
    pub recoverable: bool,
}

/// Completed batch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedBatch {
    /// Batch ID
    pub id: String,
    /// Completion time
    pub completed_at: u64,
    /// Processing duration
    pub duration_ms: u64,
    /// Success count
    pub success_count: usize,
    /// Error count
    pub error_count: usize,
    /// Output location
    pub output_path: String,
}

/// Processing engine for batch operations
pub struct ProcessingEngine {
    /// Embedding models
    models: Arc<RwLock<HashMap<String, Arc<dyn EmbeddingModel>>>>,
    /// Worker pool
    workers: Arc<WorkerPool>,
    /// Processing strategies
    strategies: HashMap<BatchType, Box<dyn ProcessingStrategy>>,
}

/// Worker pool for parallel processing
pub struct WorkerPool {
    /// Worker semaphore
    semaphore: Arc<Semaphore>,
    /// Worker stats
    stats: Arc<RwLock<WorkerStats>>,
}

/// Worker statistics
#[derive(Debug, Clone, Default)]
pub struct WorkerStats {
    /// Total tasks processed
    pub tasks_processed: u64,
    /// Average processing time
    pub avg_processing_time_ms: f64,
    /// Current active workers
    pub active_workers: usize,
}

/// Trait for processing strategies
#[async_trait]
pub trait ProcessingStrategy: Send + Sync {
    /// Process a batch
    async fn process(&self, batch: &Batch, engine: &ProcessingEngine) -> Result<BatchResult>;
    
    /// Validate batch before processing
    fn validate(&self, batch: &Batch) -> Result<()>;
}

/// Batch processing result
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Batch ID
    pub batch_id: String,
    /// Embeddings generated
    pub embeddings: Vec<ItemEmbedding>,
    /// Processing metadata
    pub metadata: ProcessingMetadata,
}

/// Item embedding result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemEmbedding {
    /// Item ID
    pub item_id: String,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Model used
    pub model: String,
    /// Processing time
    pub processing_time_ms: u64,
}

/// Processing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetadata {
    /// Total processing time
    pub total_time_ms: u64,
    /// Model performance
    pub model_performance: HashMap<String, ModelPerformance>,
    /// Resource usage
    pub resource_usage: ResourceUsage,
}

/// Model performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPerformance {
    /// Items processed
    pub items_processed: usize,
    /// Average time per item
    pub avg_time_per_item_ms: f64,
    /// Error rate
    pub error_rate: f64,
}

/// Resource usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU usage percentage
    pub cpu_usage: f32,
    /// Memory usage MB
    pub memory_usage_mb: usize,
    /// GPU usage percentage (if applicable)
    pub gpu_usage: Option<f32>,
}

/// Storage manager for batch results
pub struct StorageManager {
    /// Base output directory
    base_dir: String,
    /// Storage backends
    backends: HashMap<OutputFormat, Box<dyn StorageBackend>>,
    /// Compression settings
    compression: CompressionSettings,
}

/// Compression settings
#[derive(Debug, Clone)]
pub struct CompressionSettings {
    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,
    /// Compression level
    pub level: u32,
}

/// Compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Trait for storage backends
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store batch result
    async fn store(&self, result: &BatchResult, path: &Path) -> Result<()>;
    
    /// Load batch result
    async fn load(&self, path: &Path) -> Result<BatchResult>;
    
    /// Get format identifier
    fn format(&self) -> OutputFormat;
}

/// Progress tracking
pub struct ProgressTracker {
    /// Progress by batch
    progress: DashMap<String, BatchProgress>,
    /// Global statistics
    global_stats: Arc<RwLock<GlobalStats>>,
}

/// Batch progress information
#[derive(Debug, Clone)]
pub struct BatchProgress {
    /// Total items
    pub total_items: usize,
    /// Processed items
    pub processed_items: usize,
    /// Failed items
    pub failed_items: usize,
    /// Start time
    pub start_time: Instant,
    /// Estimated completion
    pub estimated_completion: Option<Instant>,
}

/// Global processing statistics
#[derive(Debug, Clone, Default)]
pub struct GlobalStats {
    /// Total batches processed
    pub total_batches: u64,
    /// Total items processed
    pub total_items: u64,
    /// Average batch processing time
    pub avg_batch_time_ms: f64,
    /// Success rate
    pub success_rate: f64,
}

use std::collections::HashMap;

impl BatchEmbeddingProcessor {
    /// Create a new batch processor
    pub fn new(config: BatchConfig) -> Self {
        Self {
            queue: Arc::new(BatchQueue::new()),
            engine: Arc::new(ProcessingEngine::new(config.num_workers)),
            storage: Arc::new(StorageManager::new(&config)),
            config,
            progress: Arc::new(ProgressTracker::new()),
        }
    }
    
    /// Submit a batch for processing
    pub async fn submit_batch(&self, batch: Batch) -> Result<String> {
        let batch_id = batch.id.clone();
        
        // Validate batch
        self.validate_batch(&batch)?;
        
        // Add to queue
        self.queue.enqueue(batch).await?;
        
        // Initialize progress tracking
        self.progress.init_batch(&batch_id, batch.items.len());
        
        info!("Batch {} submitted with {} items", batch_id, batch.items.len());
        Ok(batch_id)
    }
    
    /// Process batches from queue
    pub async fn process_queue(&self) -> Result<()> {
        while let Some(batch) = self.queue.dequeue().await? {
            let batch_id = batch.id.clone();
            
            // Update status
            self.queue.update_status(&batch_id, BatchState::Processing);
            
            // Process batch
            match self.process_batch(batch).await {
                Ok(result) => {
                    // Store result
                    let output_path = self.storage.store_result(&result).await?;
                    
                    // Mark as completed
                    self.queue.complete_batch(batch_id.clone(), output_path).await?;
                    
                    info!("Batch {} completed successfully", batch_id);
                }
                Err(e) => {
                    error!("Batch {} failed: {}", batch_id, e);
                    self.queue.update_status(&batch_id, BatchState::Failed);
                }
            }
        }
        
        Ok(())
    }
    
    /// Process a single batch
    async fn process_batch(&self, batch: Batch) -> Result<BatchResult> {
        let start = Instant::now();
        let batch_type = batch.metadata.batch_type;
        
        // Get processing strategy
        let strategy = self.engine.get_strategy(batch_type)?;
        
        // Process with strategy
        let result = strategy.process(&batch, &self.engine).await?;
        
        // Update progress
        self.progress.complete_batch(&batch.id, start.elapsed());
        
        Ok(result)
    }
    
    /// Create checkpoint
    pub async fn checkpoint(&self, batch_id: &str) -> Result<()> {
        if !self.config.enable_checkpoints {
            return Ok(());
        }
        
        info!("Creating checkpoint for batch {}", batch_id);
        
        // Get current progress
        let progress = self.progress.get_progress(batch_id)?;
        
        // Save checkpoint
        let checkpoint = Checkpoint {
            batch_id: batch_id.to_string(),
            progress: progress.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.storage.save_checkpoint(&checkpoint).await?;
        
        Ok(())
    }
    
    /// Resume from checkpoint
    pub async fn resume_from_checkpoint(&self, batch_id: &str) -> Result<()> {
        let checkpoint = self.storage.load_checkpoint(batch_id).await?;
        
        info!("Resuming batch {} from checkpoint", batch_id);
        
        // Restore progress
        self.progress.restore_progress(batch_id, checkpoint.progress);
        
        Ok(())
    }
    
    /// Get batch status
    pub fn get_status(&self, batch_id: &str) -> Option<BatchStatus> {
        self.queue.get_status(batch_id)
    }
    
    /// Get global statistics
    pub async fn get_global_stats(&self) -> GlobalStats {
        self.progress.global_stats.read().await.clone()
    }
    
    /// Validate batch before processing
    fn validate_batch(&self, batch: &Batch) -> Result<()> {
        if batch.items.is_empty() {
            return Err(AIAgentError::ProcessingError("Empty batch".to_string()));
        }
        
        if batch.items.len() > self.config.max_batch_size {
            return Err(AIAgentError::ProcessingError(
                format!("Batch size {} exceeds maximum {}", 
                    batch.items.len(), 
                    self.config.max_batch_size)
            ));
        }
        
        Ok(())
    }
}

impl BatchQueue {
    /// Create a new batch queue
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            active: DashMap::new(),
            completed: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Enqueue a batch
    pub async fn enqueue(&self, batch: Batch) -> Result<()> {
        let batch_id = batch.id.clone();
        
        // Add to pending queue
        self.pending.lock().await.push_back(batch);
        
        // Initialize status
        self.active.insert(batch_id, BatchStatus {
            state: BatchState::Queued,
            progress: 0.0,
            items_processed: 0,
            errors: vec![],
            started_at: None,
        });
        
        Ok(())
    }
    
    /// Dequeue next batch
    pub async fn dequeue(&self) -> Result<Option<Batch>> {
        Ok(self.pending.lock().await.pop_front())
    }
    
    /// Update batch status
    pub fn update_status(&self, batch_id: &str, state: BatchState) {
        if let Some(mut status) = self.active.get_mut(batch_id) {
            status.state = state;
            if state == BatchState::Processing && status.started_at.is_none() {
                status.started_at = Some(Instant::now());
            }
        }
    }
    
    /// Complete a batch
    pub async fn complete_batch(&self, batch_id: String, output_path: String) -> Result<()> {
        if let Some((_, status)) = self.active.remove(&batch_id) {
            let completed = CompletedBatch {
                id: batch_id,
                completed_at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                duration_ms: status.started_at
                    .map(|s| s.elapsed().as_millis() as u64)
                    .unwrap_or(0),
                success_count: status.items_processed,
                error_count: status.errors.len(),
                output_path,
            };
            
            self.completed.lock().await.push(completed);
        }
        
        Ok(())
    }
    
    /// Get batch status
    pub fn get_status(&self, batch_id: &str) -> Option<BatchStatus> {
        self.active.get(batch_id).map(|entry| entry.value().clone())
    }
}

impl ProcessingEngine {
    /// Create a new processing engine
    pub fn new(num_workers: usize) -> Self {
        let mut strategies: HashMap<BatchType, Box<dyn ProcessingStrategy>> = HashMap::new();
        strategies.insert(BatchType::TransactionContext, Box::new(TransactionContextStrategy));
        strategies.insert(BatchType::Historical, Box::new(HistoricalDataStrategy));
        
        Self {
            models: Arc::new(RwLock::new(HashMap::new())),
            workers: Arc::new(WorkerPool::new(num_workers)),
            strategies,
        }
    }
    
    /// Get processing strategy
    pub fn get_strategy(&self, batch_type: BatchType) -> Result<&Box<dyn ProcessingStrategy>> {
        self.strategies.get(&batch_type)
            .ok_or_else(|| AIAgentError::ProcessingError(
                format!("No strategy for batch type {:?}", batch_type)
            ))
    }
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(size: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(size)),
            stats: Arc::new(RwLock::new(WorkerStats::default())),
        }
    }
    
    /// Acquire worker permit
    pub async fn acquire(&self) -> Result<tokio::sync::SemaphorePermit> {
        Ok(self.semaphore.acquire().await?)
    }
    
    /// Update worker statistics
    pub async fn update_stats(&self, processing_time: Duration) {
        let mut stats = self.stats.write().await;
        stats.tasks_processed += 1;
        
        let n = stats.tasks_processed as f64;
        stats.avg_processing_time_ms = 
            (stats.avg_processing_time_ms * (n - 1.0) + processing_time.as_millis() as f64) / n;
    }
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new(config: &BatchConfig) -> Self {
        let mut backends: HashMap<OutputFormat, Box<dyn StorageBackend>> = HashMap::new();
        backends.insert(OutputFormat::Json, Box::new(JsonStorageBackend));
        backends.insert(OutputFormat::Binary, Box::new(BinaryStorageBackend));
        
        Self {
            base_dir: "./batch_output".to_string(),
            backends,
            compression: CompressionSettings {
                algorithm: if config.enable_compression {
                    CompressionAlgorithm::Zstd
                } else {
                    CompressionAlgorithm::None
                },
                level: 3,
            },
        }
    }
    
    /// Store batch result
    pub async fn store_result(&self, result: &BatchResult) -> Result<String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let output_path = format!("{}/batch_{}_{}",
            self.base_dir,
            result.batch_id,
            timestamp
        );
        
        // Create directory
        fs::create_dir_all(&self.base_dir).await?;
        
        // Store with appropriate backend
        let backend = self.backends.get(&OutputFormat::Json)
            .ok_or_else(|| AIAgentError::ProcessingError("No storage backend".to_string()))?;
        
        backend.store(result, Path::new(&output_path)).await?;
        
        Ok(output_path)
    }
    
    /// Save checkpoint
    pub async fn save_checkpoint(&self, checkpoint: &Checkpoint) -> Result<()> {
        let path = format!("{}/checkpoints/{}.json", self.base_dir, checkpoint.batch_id);
        fs::create_dir_all(format!("{}/checkpoints", self.base_dir)).await?;
        
        let data = serde_json::to_string(checkpoint)?;
        fs::write(path, data).await?;
        
        Ok(())
    }
    
    /// Load checkpoint
    pub async fn load_checkpoint(&self, batch_id: &str) -> Result<Checkpoint> {
        let path = format!("{}/checkpoints/{}.json", self.base_dir, batch_id);
        let data = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&data)?)
    }
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        Self {
            progress: DashMap::new(),
            global_stats: Arc::new(RwLock::new(GlobalStats::default())),
        }
    }
    
    /// Initialize batch progress
    pub fn init_batch(&self, batch_id: &str, total_items: usize) {
        self.progress.insert(batch_id.to_string(), BatchProgress {
            total_items,
            processed_items: 0,
            failed_items: 0,
            start_time: Instant::now(),
            estimated_completion: None,
        });
    }
    
    /// Update batch progress
    pub fn update_progress(&self, batch_id: &str, processed: usize, failed: usize) {
        if let Some(mut progress) = self.progress.get_mut(batch_id) {
            progress.processed_items = processed;
            progress.failed_items = failed;
            
            // Estimate completion
            let elapsed = progress.start_time.elapsed();
            let rate = processed as f64 / elapsed.as_secs_f64();
            let remaining = progress.total_items - processed;
            
            if rate > 0.0 {
                let eta_secs = remaining as f64 / rate;
                progress.estimated_completion = Some(
                    Instant::now() + Duration::from_secs_f64(eta_secs)
                );
            }
        }
    }
    
    /// Complete batch
    pub fn complete_batch(&self, batch_id: &str, duration: Duration) {
        if let Some((_, progress)) = self.progress.remove(batch_id) {
            // Update global stats
            tokio::spawn(async move {
                let mut stats = self.global_stats.write().await;
                stats.total_batches += 1;
                stats.total_items += progress.total_items as u64;
                
                let n = stats.total_batches as f64;
                stats.avg_batch_time_ms = 
                    (stats.avg_batch_time_ms * (n - 1.0) + duration.as_millis() as f64) / n;
                
                let success_items = progress.processed_items - progress.failed_items;
                stats.success_rate = 
                    (stats.success_rate * (stats.total_items - progress.total_items) as f64 
                    + success_items as f64) / stats.total_items as f64;
            });
        }
    }
    
    /// Get batch progress
    pub fn get_progress(&self, batch_id: &str) -> Result<BatchProgress> {
        self.progress.get(batch_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AIAgentError::ProcessingError("Batch not found".to_string()))
    }
    
    /// Restore progress from checkpoint
    pub fn restore_progress(&self, batch_id: &str, progress: BatchProgress) {
        self.progress.insert(batch_id.to_string(), progress);
    }
}

/// Transaction context processing strategy
struct TransactionContextStrategy;

#[async_trait]
impl ProcessingStrategy for TransactionContextStrategy {
    async fn process(&self, batch: &Batch, engine: &ProcessingEngine) -> Result<BatchResult> {
        let mut embeddings = Vec::new();
        let start = Instant::now();
        
        // Process items in parallel chunks
        let chunk_size = 100;
        for chunk in batch.items.chunks(chunk_size) {
            let _permit = engine.workers.acquire().await?;
            
            // Process chunk
            for item in chunk {
                // Simulate embedding generation
                let embedding = vec![0.1; 384]; // Would use actual model
                
                embeddings.push(ItemEmbedding {
                    item_id: item.id.clone(),
                    embedding,
                    model: "default".to_string(),
                    processing_time_ms: 10,
                });
            }
        }
        
        Ok(BatchResult {
            batch_id: batch.id.clone(),
            embeddings,
            metadata: ProcessingMetadata {
                total_time_ms: start.elapsed().as_millis() as u64,
                model_performance: HashMap::new(),
                resource_usage: ResourceUsage {
                    cpu_usage: 50.0,
                    memory_usage_mb: 512,
                    gpu_usage: None,
                },
            },
        })
    }
    
    fn validate(&self, _batch: &Batch) -> Result<()> {
        Ok(())
    }
}

/// Historical data processing strategy  
struct HistoricalDataStrategy;

#[async_trait]
impl ProcessingStrategy for HistoricalDataStrategy {
    async fn process(&self, batch: &Batch, engine: &ProcessingEngine) -> Result<BatchResult> {
        // Similar to transaction context but with different optimizations
        TransactionContextStrategy.process(batch, engine).await
    }
    
    fn validate(&self, _batch: &Batch) -> Result<()> {
        Ok(())
    }
}

/// JSON storage backend
struct JsonStorageBackend;

#[async_trait]
impl StorageBackend for JsonStorageBackend {
    async fn store(&self, result: &BatchResult, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(result)?;
        fs::write(path.with_extension("json"), json).await?;
        Ok(())
    }
    
    async fn load(&self, path: &Path) -> Result<BatchResult> {
        let data = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&data)?)
    }
    
    fn format(&self) -> OutputFormat {
        OutputFormat::Json
    }
}

/// Binary storage backend
struct BinaryStorageBackend;

#[async_trait]
impl StorageBackend for BinaryStorageBackend {
    async fn store(&self, result: &BatchResult, path: &Path) -> Result<()> {
        let data = bincode::serialize(result)?;
        fs::write(path.with_extension("bin"), data).await?;
        Ok(())
    }
    
    async fn load(&self, path: &Path) -> Result<BatchResult> {
        let data = fs::read(path).await?;
        Ok(bincode::deserialize(&data)?)
    }
    
    fn format(&self) -> OutputFormat {
        OutputFormat::Binary
    }
}

/// Checkpoint data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub batch_id: String,
    pub progress: BatchProgress,
    pub timestamp: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10000,
            num_workers: num_cpus::get(),
            batch_timeout: Duration::from_secs(3600),
            enable_checkpoints: true,
            checkpoint_interval: 1000,
            output_format: OutputFormat::Json,
            enable_compression: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_submission() {
        let config = BatchConfig::default();
        let processor = BatchEmbeddingProcessor::new(config);
        
        let batch = Batch {
            id: "test_batch".to_string(),
            items: vec![
                BatchItem {
                    id: "item1".to_string(),
                    text: "Test text 1".to_string(),
                    metadata: serde_json::json!({}),
                    priority: Priority::Normal,
                },
            ],
            metadata: BatchMetadata {
                source: "test".to_string(),
                batch_type: BatchType::TransactionContext,
                total_items: 1,
                custom: serde_json::json!({}),
            },
            created_at: Instant::now(),
        };
        
        let result = processor.submit_batch(batch).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_batch_validation() {
        let processor = BatchEmbeddingProcessor::new(BatchConfig::default());
        
        let empty_batch = Batch {
            id: "empty".to_string(),
            items: vec![],
            metadata: BatchMetadata {
                source: "test".to_string(),
                batch_type: BatchType::TransactionContext,
                total_items: 0,
                custom: serde_json::json!({}),
            },
            created_at: Instant::now(),
        };
        
        assert!(processor.validate_batch(&empty_batch).is_err());
    }
}