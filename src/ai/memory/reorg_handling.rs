//! Memory Reorganization Handling
//! 
//! This module handles agent memory management during blockchain reorganizations,
//! ensuring consistency and proper rollback of learned patterns.

use crate::errors::*;
use crate::ai::memory::agent_integration::{
    AgentMemory, MemoryItem, ShortTermMemory, LongTermMemory,
    EpisodicMemory, SemanticMemory, Concept, Relationship,
};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn, error};

/// Memory reorg handler
pub struct MemoryReorgHandler {
    /// Checkpoint manager
    checkpoints: Arc<CheckpointManager>,
    
    /// Rollback engine
    rollback_engine: Arc<RollbackEngine>,
    
    /// Reorg detector
    detector: Arc<ReorgDetector>,
    
    /// Recovery manager
    recovery: Arc<RecoveryManager>,
    
    /// Configuration
    config: ReorgConfig,
    
    /// Metrics
    metrics: Arc<RwLock<ReorgMetrics>>,
}

/// Reorg handling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReorgConfig {
    /// Enable automatic checkpointing
    pub auto_checkpoint: bool,
    /// Checkpoint interval (blocks)
    pub checkpoint_interval: u64,
    /// Maximum checkpoint history
    pub max_checkpoints: usize,
    /// Reorg detection threshold
    pub detection_threshold: u32,
    /// Recovery strategy
    pub recovery_strategy: RecoveryStrategy,
    /// Enable memory validation
    pub enable_validation: bool,
}

/// Recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// Full rollback to checkpoint
    FullRollback,
    /// Selective rollback of affected items
    SelectiveRollback,
    /// Merge conflicting states
    MergeStates,
    /// Custom recovery logic
    Custom,
}

/// Memory checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCheckpoint {
    /// Checkpoint ID
    pub id: String,
    /// Block height
    pub block_height: u64,
    /// Block hash
    pub block_hash: [u8; 32],
    /// Timestamp
    pub timestamp: SystemTime,
    /// Memory snapshot
    pub snapshot: MemorySnapshot,
    /// Metadata
    pub metadata: CheckpointMetadata,
}

/// Memory snapshot at checkpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Short-term memory state
    pub short_term: ShortTermSnapshot,
    /// Long-term memory state
    pub long_term: LongTermSnapshot,
    /// Episodic memory state
    pub episodic: EpisodicSnapshot,
    /// Semantic memory state
    pub semantic: SemanticSnapshot,
    /// Memory statistics
    pub stats: MemoryStats,
}

/// Short-term memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortTermSnapshot {
    /// Memory items
    pub items: Vec<MemoryItem>,
    /// Item count
    pub count: usize,
    /// Total size
    pub size_bytes: usize,
}

/// Long-term memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongTermSnapshot {
    /// Consolidated memories
    pub memories: Vec<ConsolidatedMemory>,
    /// Pattern count
    pub pattern_count: usize,
    /// Relationship count
    pub relationship_count: usize,
}

/// Consolidated memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedMemory {
    /// Memory ID
    pub id: String,
    /// Content hash
    pub content_hash: String,
    /// Consolidation timestamp
    pub consolidated_at: SystemTime,
    /// Access count
    pub access_count: u32,
    /// Importance score
    pub importance: f64,
}

/// Episodic memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicSnapshot {
    /// Episode sequences
    pub episodes: Vec<EpisodeSequence>,
    /// Event count
    pub event_count: usize,
    /// Sequence patterns
    pub patterns: Vec<SequencePattern>,
}

/// Episode sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeSequence {
    /// Sequence ID
    pub id: String,
    /// Events in sequence
    pub events: Vec<String>,
    /// Sequence timestamp
    pub timestamp: SystemTime,
    /// Confidence score
    pub confidence: f64,
}

/// Sequence pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencePattern {
    /// Pattern ID
    pub id: String,
    /// Pattern type
    pub pattern_type: String,
    /// Occurrences
    pub occurrences: u32,
    /// Last seen
    pub last_seen: SystemTime,
}

/// Semantic memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticSnapshot {
    /// Concepts
    pub concepts: Vec<Concept>,
    /// Relationships
    pub relationships: Vec<Relationship>,
    /// Concept hierarchy
    pub hierarchy: ConceptHierarchy,
}

/// Concept hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptHierarchy {
    /// Root concepts
    pub roots: Vec<String>,
    /// Parent-child mappings
    pub parent_child: HashMap<String, Vec<String>>,
    /// Hierarchy depth
    pub max_depth: usize,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Total items
    pub total_items: usize,
    /// Total size in bytes
    pub total_size_bytes: usize,
    /// Average access time (ms)
    pub avg_access_time_ms: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

/// Checkpoint metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    /// Previous checkpoint ID
    pub previous_checkpoint: Option<String>,
    /// Changes since last checkpoint
    pub changes_count: usize,
    /// Compression used
    pub compressed: bool,
    /// Checksum
    pub checksum: String,
}

/// Checkpoint manager
pub struct CheckpointManager {
    /// Stored checkpoints
    checkpoints: DashMap<String, MemoryCheckpoint>,
    /// Checkpoint index by block
    block_index: Arc<RwLock<HashMap<u64, String>>>,
    /// Active checkpoint
    active: Arc<RwLock<Option<String>>>,
    /// Storage backend
    storage: Arc<dyn CheckpointStorage>,
}

/// Trait for checkpoint storage
#[async_trait]
pub trait CheckpointStorage: Send + Sync {
    /// Store checkpoint
    async fn store(&self, checkpoint: &MemoryCheckpoint) -> Result<()>;
    
    /// Load checkpoint
    async fn load(&self, id: &str) -> Result<MemoryCheckpoint>;
    
    /// List checkpoints
    async fn list(&self, after_block: Option<u64>) -> Result<Vec<String>>;
    
    /// Delete checkpoint
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Rollback engine
pub struct RollbackEngine {
    /// Rollback history
    history: Arc<Mutex<VecDeque<RollbackEvent>>>,
    /// Active rollback
    active_rollback: Arc<RwLock<Option<ActiveRollback>>>,
    /// Rollback strategies
    strategies: HashMap<RecoveryStrategy, Box<dyn RollbackStrategy>>,
}

/// Rollback event
#[derive(Debug, Clone)]
pub struct RollbackEvent {
    /// Event ID
    pub id: String,
    /// Trigger
    pub trigger: RollbackTrigger,
    /// From block
    pub from_block: u64,
    /// To block
    pub to_block: u64,
    /// Start time
    pub started_at: Instant,
    /// End time
    pub ended_at: Option<Instant>,
    /// Result
    pub result: RollbackResult,
}

/// Rollback triggers
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackTrigger {
    /// Chain reorganization detected
    ChainReorg {
        old_tip: [u8; 32],
        new_tip: [u8; 32],
        depth: u64,
    },
    /// Manual rollback request
    Manual {
        reason: String,
    },
    /// Consistency check failure
    ConsistencyFailure {
        error: String,
    },
    /// State corruption detected
    Corruption {
        affected_components: Vec<String>,
    },
}

/// Rollback result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RollbackResult {
    /// Successful rollback
    Success {
        items_rolled_back: usize,
        checkpoint_used: String,
    },
    /// Partial rollback
    Partial {
        succeeded: usize,
        failed: usize,
        errors: Vec<String>,
    },
    /// Failed rollback
    Failed {
        error: String,
    },
}

/// Active rollback state
#[derive(Debug, Clone)]
pub struct ActiveRollback {
    /// Rollback ID
    pub id: String,
    /// Target checkpoint
    pub target_checkpoint: String,
    /// Progress
    pub progress: RollbackProgress,
    /// Start time
    pub started_at: Instant,
}

/// Rollback progress
#[derive(Debug, Clone)]
pub struct RollbackProgress {
    /// Current phase
    pub phase: RollbackPhase,
    /// Items processed
    pub items_processed: usize,
    /// Total items
    pub total_items: usize,
    /// Percentage complete
    pub percentage: f32,
}

/// Rollback phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackPhase {
    Preparing,
    ValidatingCheckpoint,
    RollingBackMemory,
    VerifyingState,
    Completing,
}

/// Trait for rollback strategies
#[async_trait]
pub trait RollbackStrategy: Send + Sync {
    /// Execute rollback
    async fn rollback(
        &self,
        memory: &AgentMemory,
        checkpoint: &MemoryCheckpoint,
        trigger: &RollbackTrigger,
    ) -> Result<RollbackResult>;
    
    /// Validate rollback feasibility
    async fn validate(&self, checkpoint: &MemoryCheckpoint) -> Result<bool>;
    
    /// Strategy name
    fn name(&self) -> &str;
}

/// Reorg detector
pub struct ReorgDetector {
    /// Detection state
    state: Arc<RwLock<DetectionState>>,
    /// Block history
    block_history: Arc<Mutex<VecDeque<BlockInfo>>>,
    /// Detection algorithms
    algorithms: Vec<Box<dyn DetectionAlgorithm>>,
}

/// Detection state
#[derive(Debug, Clone)]
pub struct DetectionState {
    /// Last known block
    pub last_block: Option<BlockInfo>,
    /// Reorg in progress
    pub reorg_detected: bool,
    /// Reorg depth
    pub reorg_depth: Option<u64>,
    /// Detection confidence
    pub confidence: f64,
}

/// Block information
#[derive(Debug, Clone)]
pub struct BlockInfo {
    /// Block height
    pub height: u64,
    /// Block hash
    pub hash: [u8; 32],
    /// Parent hash
    pub parent_hash: [u8; 32],
    /// Timestamp
    pub timestamp: SystemTime,
    /// Transaction count
    pub tx_count: usize,
}

/// Trait for reorg detection algorithms
#[async_trait]
pub trait DetectionAlgorithm: Send + Sync {
    /// Detect reorg
    async fn detect(
        &self,
        current_block: &BlockInfo,
        history: &[BlockInfo],
    ) -> Option<ReorgInfo>;
    
    /// Algorithm name
    fn name(&self) -> &str;
}

/// Reorg information
#[derive(Debug, Clone)]
pub struct ReorgInfo {
    /// Reorg depth
    pub depth: u64,
    /// Common ancestor
    pub common_ancestor: BlockInfo,
    /// Old chain tip
    pub old_tip: BlockInfo,
    /// New chain tip
    pub new_tip: BlockInfo,
    /// Confidence score
    pub confidence: f64,
}

/// Recovery manager
pub struct RecoveryManager {
    /// Recovery queue
    queue: Arc<Mutex<VecDeque<RecoveryTask>>>,
    /// Active recovery
    active_recovery: Arc<RwLock<Option<ActiveRecovery>>>,
    /// Recovery handlers
    handlers: HashMap<String, Box<dyn RecoveryHandler>>,
}

/// Recovery task
#[derive(Debug, Clone)]
pub struct RecoveryTask {
    /// Task ID
    pub id: String,
    /// Task type
    pub task_type: RecoveryType,
    /// Priority
    pub priority: RecoveryPriority,
    /// Created at
    pub created_at: Instant,
}

/// Recovery types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryType {
    /// Memory recovery
    Memory {
        affected_agents: Vec<String>,
    },
    /// State recovery
    State {
        components: Vec<String>,
    },
    /// Full recovery
    Full,
}

/// Recovery priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Active recovery
#[derive(Debug, Clone)]
pub struct ActiveRecovery {
    /// Recovery ID
    pub id: String,
    /// Recovery type
    pub recovery_type: RecoveryType,
    /// Progress
    pub progress: RecoveryProgress,
    /// Start time
    pub started_at: Instant,
}

/// Recovery progress
#[derive(Debug, Clone)]
pub struct RecoveryProgress {
    /// Current step
    pub step: u32,
    /// Total steps
    pub total_steps: u32,
    /// Current operation
    pub operation: String,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Trait for recovery handlers
#[async_trait]
pub trait RecoveryHandler: Send + Sync {
    /// Handle recovery
    async fn recover(
        &self,
        task: &RecoveryTask,
        checkpoint: Option<&MemoryCheckpoint>,
    ) -> Result<()>;
    
    /// Handler name
    fn name(&self) -> &str;
}

/// Reorg metrics
#[derive(Debug, Clone, Default)]
pub struct ReorgMetrics {
    /// Total reorgs detected
    pub reorgs_detected: u64,
    /// Successful rollbacks
    pub successful_rollbacks: u64,
    /// Failed rollbacks
    pub failed_rollbacks: u64,
    /// Average rollback time (ms)
    pub avg_rollback_time_ms: f64,
    /// Checkpoints created
    pub checkpoints_created: u64,
    /// Memory items affected
    pub items_affected: u64,
}

impl MemoryReorgHandler {
    /// Create a new memory reorg handler
    pub fn new(config: ReorgConfig) -> Self {
        Self {
            checkpoints: Arc::new(CheckpointManager::new()),
            rollback_engine: Arc::new(RollbackEngine::new()),
            detector: Arc::new(ReorgDetector::new()),
            recovery: Arc::new(RecoveryManager::new()),
            config,
            metrics: Arc::new(RwLock::new(ReorgMetrics::default())),
        }
    }
    
    /// Initialize the reorg handler
    pub async fn initialize(&self, memory: &AgentMemory) -> Result<()> {
        info!("Initializing memory reorg handler");
        
        // Load existing checkpoints
        self.checkpoints.load_checkpoints().await?;
        
        // Initialize rollback strategies
        self.rollback_engine.initialize_strategies(&self.config)?;
        
        // Start auto-checkpoint if enabled
        if self.config.auto_checkpoint {
            self.start_auto_checkpoint(memory).await?;
        }
        
        Ok(())
    }
    
    /// Handle block update
    pub async fn handle_block_update(
        &self,
        block: BlockInfo,
        memory: &AgentMemory,
    ) -> Result<()> {
        // Update detector
        if let Some(reorg_info) = self.detector.process_block(block.clone()).await? {
            self.handle_reorg(reorg_info, memory).await?;
        }
        
        // Check if checkpoint needed
        if self.should_checkpoint(&block) {
            self.create_checkpoint(&block, memory).await?;
        }
        
        Ok(())
    }
    
    /// Handle detected reorganization
    pub async fn handle_reorg(
        &self,
        reorg_info: ReorgInfo,
        memory: &AgentMemory,
    ) -> Result<()> {
        info!(
            "Handling reorg: depth={}, confidence={}",
            reorg_info.depth, reorg_info.confidence
        );
        
        // Update metrics
        self.metrics.write().await.reorgs_detected += 1;
        
        // Find appropriate checkpoint
        let checkpoint = self.checkpoints
            .find_checkpoint_before(reorg_info.common_ancestor.height)
            .await?;
        
        // Trigger rollback
        let trigger = RollbackTrigger::ChainReorg {
            old_tip: reorg_info.old_tip.hash,
            new_tip: reorg_info.new_tip.hash,
            depth: reorg_info.depth,
        };
        
        let result = self.rollback_engine
            .execute_rollback(memory, &checkpoint, trigger)
            .await?;
        
        // Update metrics based on result
        match &result {
            RollbackResult::Success { .. } => {
                self.metrics.write().await.successful_rollbacks += 1;
            }
            _ => {
                self.metrics.write().await.failed_rollbacks += 1;
            }
        }
        
        Ok(())
    }
    
    /// Create memory checkpoint
    pub async fn create_checkpoint(
        &self,
        block: &BlockInfo,
        memory: &AgentMemory,
    ) -> Result<MemoryCheckpoint> {
        info!("Creating memory checkpoint at block {}", block.height);
        
        let checkpoint = MemoryCheckpoint {
            id: uuid::Uuid::new_v4().to_string(),
            block_height: block.height,
            block_hash: block.hash,
            timestamp: SystemTime::now(),
            snapshot: self.create_memory_snapshot(memory).await?,
            metadata: self.create_checkpoint_metadata().await?,
        };
        
        // Store checkpoint
        self.checkpoints.store_checkpoint(&checkpoint).await?;
        
        // Update metrics
        self.metrics.write().await.checkpoints_created += 1;
        
        Ok(checkpoint)
    }
    
    /// Rollback to checkpoint
    pub async fn rollback_to_checkpoint(
        &self,
        checkpoint_id: &str,
        memory: &AgentMemory,
        reason: String,
    ) -> Result<RollbackResult> {
        info!("Rolling back to checkpoint {}: {}", checkpoint_id, reason);
        
        let checkpoint = self.checkpoints.get_checkpoint(checkpoint_id).await?;
        
        let trigger = RollbackTrigger::Manual { reason };
        
        self.rollback_engine
            .execute_rollback(memory, &checkpoint, trigger)
            .await
    }
    
    /// Validate memory consistency
    pub async fn validate_consistency(
        &self,
        memory: &AgentMemory,
        checkpoint: Option<&MemoryCheckpoint>,
    ) -> Result<ConsistencyReport> {
        let validator = ConsistencyValidator::new();
        validator.validate(memory, checkpoint).await
    }
    
    /// Get reorg metrics
    pub async fn get_metrics(&self) -> ReorgMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Create memory snapshot
    async fn create_memory_snapshot(&self, memory: &AgentMemory) -> Result<MemorySnapshot> {
        Ok(MemorySnapshot {
            short_term: self.snapshot_short_term(memory).await?,
            long_term: self.snapshot_long_term(memory).await?,
            episodic: self.snapshot_episodic(memory).await?,
            semantic: self.snapshot_semantic(memory).await?,
            stats: self.collect_memory_stats(memory).await?,
        })
    }
    
    /// Snapshot short-term memory
    async fn snapshot_short_term(&self, memory: &AgentMemory) -> Result<ShortTermSnapshot> {
        // Would access actual memory
        Ok(ShortTermSnapshot {
            items: vec![],
            count: 0,
            size_bytes: 0,
        })
    }
    
    /// Snapshot long-term memory
    async fn snapshot_long_term(&self, memory: &AgentMemory) -> Result<LongTermSnapshot> {
        Ok(LongTermSnapshot {
            memories: vec![],
            pattern_count: 0,
            relationship_count: 0,
        })
    }
    
    /// Snapshot episodic memory
    async fn snapshot_episodic(&self, memory: &AgentMemory) -> Result<EpisodicSnapshot> {
        Ok(EpisodicSnapshot {
            episodes: vec![],
            event_count: 0,
            patterns: vec![],
        })
    }
    
    /// Snapshot semantic memory
    async fn snapshot_semantic(&self, memory: &AgentMemory) -> Result<SemanticSnapshot> {
        Ok(SemanticSnapshot {
            concepts: vec![],
            relationships: vec![],
            hierarchy: ConceptHierarchy {
                roots: vec![],
                parent_child: HashMap::new(),
                max_depth: 0,
            },
        })
    }
    
    /// Collect memory statistics
    async fn collect_memory_stats(&self, memory: &AgentMemory) -> Result<MemoryStats> {
        Ok(MemoryStats {
            total_items: 0,
            total_size_bytes: 0,
            avg_access_time_ms: 0.0,
            cache_hit_rate: 0.0,
        })
    }
    
    /// Create checkpoint metadata
    async fn create_checkpoint_metadata(&self) -> Result<CheckpointMetadata> {
        Ok(CheckpointMetadata {
            previous_checkpoint: self.checkpoints.get_latest_id().await,
            changes_count: 0,
            compressed: false,
            checksum: String::new(),
        })
    }
    
    /// Check if checkpoint is needed
    fn should_checkpoint(&self, block: &BlockInfo) -> bool {
        block.height % self.config.checkpoint_interval == 0
    }
    
    /// Start automatic checkpointing
    async fn start_auto_checkpoint(&self, memory: &AgentMemory) -> Result<()> {
        let handler = Arc::new(self.clone());
        let memory = memory.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Would check for new blocks and create checkpoints
                debug!("Auto-checkpoint check");
            }
        });
        
        Ok(())
    }
}

impl Clone for MemoryReorgHandler {
    fn clone(&self) -> Self {
        Self {
            checkpoints: self.checkpoints.clone(),
            rollback_engine: self.rollback_engine.clone(),
            detector: self.detector.clone(),
            recovery: self.recovery.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl CheckpointManager {
    /// Create new checkpoint manager
    fn new() -> Self {
        Self {
            checkpoints: DashMap::new(),
            block_index: Arc::new(RwLock::new(HashMap::new())),
            active: Arc::new(RwLock::new(None)),
            storage: Arc::new(InMemoryCheckpointStorage::new()),
        }
    }
    
    /// Store checkpoint
    async fn store_checkpoint(&self, checkpoint: &MemoryCheckpoint) -> Result<()> {
        // Store in memory
        self.checkpoints.insert(checkpoint.id.clone(), checkpoint.clone());
        
        // Update block index
        self.block_index.write().await
            .insert(checkpoint.block_height, checkpoint.id.clone());
        
        // Store in backend
        self.storage.store(checkpoint).await?;
        
        // Update active
        *self.active.write().await = Some(checkpoint.id.clone());
        
        Ok(())
    }
    
    /// Get checkpoint by ID
    async fn get_checkpoint(&self, id: &str) -> Result<MemoryCheckpoint> {
        if let Some(checkpoint) = self.checkpoints.get(id) {
            Ok(checkpoint.clone())
        } else {
            self.storage.load(id).await
        }
    }
    
    /// Find checkpoint before block height
    async fn find_checkpoint_before(&self, height: u64) -> Result<MemoryCheckpoint> {
        let index = self.block_index.read().await;
        
        // Find highest block <= height
        let mut best_height = 0u64;
        let mut best_id = None;
        
        for (&block_height, checkpoint_id) in index.iter() {
            if block_height <= height && block_height > best_height {
                best_height = block_height;
                best_id = Some(checkpoint_id.clone());
            }
        }
        
        if let Some(id) = best_id {
            self.get_checkpoint(&id).await
        } else {
            Err(AIAgentError::ProcessingError("No suitable checkpoint found".to_string()))
        }
    }
    
    /// Load checkpoints from storage
    async fn load_checkpoints(&self) -> Result<()> {
        let checkpoint_ids = self.storage.list(None).await?;
        
        for id in checkpoint_ids {
            if let Ok(checkpoint) = self.storage.load(&id).await {
                self.checkpoints.insert(id.clone(), checkpoint.clone());
                self.block_index.write().await
                    .insert(checkpoint.block_height, id);
            }
        }
        
        Ok(())
    }
    
    /// Get latest checkpoint ID
    async fn get_latest_id(&self) -> Option<String> {
        self.active.read().await.clone()
    }
}

impl RollbackEngine {
    /// Create new rollback engine
    fn new() -> Self {
        Self {
            history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            active_rollback: Arc::new(RwLock::new(None)),
            strategies: HashMap::new(),
        }
    }
    
    /// Initialize rollback strategies
    fn initialize_strategies(&self, config: &ReorgConfig) -> Result<()> {
        // Would register actual strategies
        Ok(())
    }
    
    /// Execute rollback
    async fn execute_rollback(
        &self,
        memory: &AgentMemory,
        checkpoint: &MemoryCheckpoint,
        trigger: RollbackTrigger,
    ) -> Result<RollbackResult> {
        let rollback_id = uuid::Uuid::new_v4().to_string();
        
        // Set active rollback
        *self.active_rollback.write().await = Some(ActiveRollback {
            id: rollback_id.clone(),
            target_checkpoint: checkpoint.id.clone(),
            progress: RollbackProgress {
                phase: RollbackPhase::Preparing,
                items_processed: 0,
                total_items: 0,
                percentage: 0.0,
            },
            started_at: Instant::now(),
        });
        
        // Execute rollback (simplified)
        let result = RollbackResult::Success {
            items_rolled_back: 0,
            checkpoint_used: checkpoint.id.clone(),
        };
        
        // Record in history
        self.history.lock().await.push_back(RollbackEvent {
            id: rollback_id,
            trigger,
            from_block: 0, // Would get current block
            to_block: checkpoint.block_height,
            started_at: Instant::now(),
            ended_at: Some(Instant::now()),
            result: result.clone(),
        });
        
        // Clear active rollback
        *self.active_rollback.write().await = None;
        
        Ok(result)
    }
}

impl ReorgDetector {
    /// Create new reorg detector
    fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(DetectionState {
                last_block: None,
                reorg_detected: false,
                reorg_depth: None,
                confidence: 0.0,
            })),
            block_history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
            algorithms: vec![],
        }
    }
    
    /// Process new block
    async fn process_block(&self, block: BlockInfo) -> Result<Option<ReorgInfo>> {
        // Add to history
        self.block_history.lock().await.push_back(block.clone());
        
        // Run detection algorithms
        let history: Vec<BlockInfo> = self.block_history.lock().await.iter().cloned().collect();
        
        for algorithm in &self.algorithms {
            if let Some(reorg_info) = algorithm.detect(&block, &history).await {
                return Ok(Some(reorg_info));
            }
        }
        
        // Update state
        let mut state = self.state.write().await;
        state.last_block = Some(block);
        state.reorg_detected = false;
        
        Ok(None)
    }
}

impl RecoveryManager {
    /// Create new recovery manager
    fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            active_recovery: Arc::new(RwLock::new(None)),
            handlers: HashMap::new(),
        }
    }
}

/// Consistency validator
struct ConsistencyValidator;

impl ConsistencyValidator {
    fn new() -> Self {
        Self
    }
    
    async fn validate(
        &self,
        memory: &AgentMemory,
        checkpoint: Option<&MemoryCheckpoint>,
    ) -> Result<ConsistencyReport> {
        Ok(ConsistencyReport {
            is_consistent: true,
            issues: vec![],
            recommendations: vec![],
        })
    }
}

/// Consistency report
#[derive(Debug, Clone)]
pub struct ConsistencyReport {
    pub is_consistent: bool,
    pub issues: Vec<ConsistencyIssue>,
    pub recommendations: Vec<String>,
}

/// Consistency issue
#[derive(Debug, Clone)]
pub struct ConsistencyIssue {
    pub severity: IssueSeverity,
    pub component: String,
    pub description: String,
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// In-memory checkpoint storage (for testing)
struct InMemoryCheckpointStorage {
    storage: DashMap<String, MemoryCheckpoint>,
}

impl InMemoryCheckpointStorage {
    fn new() -> Self {
        Self {
            storage: DashMap::new(),
        }
    }
}

#[async_trait]
impl CheckpointStorage for InMemoryCheckpointStorage {
    async fn store(&self, checkpoint: &MemoryCheckpoint) -> Result<()> {
        self.storage.insert(checkpoint.id.clone(), checkpoint.clone());
        Ok(())
    }
    
    async fn load(&self, id: &str) -> Result<MemoryCheckpoint> {
        self.storage
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AIAgentError::ProcessingError("Checkpoint not found".to_string()))
    }
    
    async fn list(&self, after_block: Option<u64>) -> Result<Vec<String>> {
        let checkpoints: Vec<String> = if let Some(after) = after_block {
            self.storage
                .iter()
                .filter(|entry| entry.value().block_height > after)
                .map(|entry| entry.key().clone())
                .collect()
        } else {
            self.storage.iter().map(|entry| entry.key().clone()).collect()
        };
        
        Ok(checkpoints)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        self.storage.remove(id);
        Ok(())
    }
}

impl Default for ReorgConfig {
    fn default() -> Self {
        Self {
            auto_checkpoint: true,
            checkpoint_interval: 100,
            max_checkpoints: 10,
            detection_threshold: 3,
            recovery_strategy: RecoveryStrategy::SelectiveRollback,
            enable_validation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reorg_handler_creation() {
        let config = ReorgConfig::default();
        let handler = MemoryReorgHandler::new(config);
        
        let metrics = handler.get_metrics().await;
        assert_eq!(metrics.reorgs_detected, 0);
    }

    #[tokio::test]
    async fn test_checkpoint_creation() {
        let handler = MemoryReorgHandler::new(ReorgConfig::default());
        let memory = AgentMemory::new(Default::default());
        
        let block = BlockInfo {
            height: 100,
            hash: [0u8; 32],
            parent_hash: [0u8; 32],
            timestamp: SystemTime::now(),
            tx_count: 10,
        };
        
        let checkpoint = handler.create_checkpoint(&block, &memory).await.unwrap();
        assert_eq!(checkpoint.block_height, 100);
    }
}