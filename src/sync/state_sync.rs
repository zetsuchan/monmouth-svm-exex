//! State Synchronization Manager
//! 
//! This module handles the core state synchronization logic between
//! RAG ExEx and SVM ExEx instances.

use crate::errors::*;
#[cfg(feature = "ai-agents")]
use crate::ai::knowledge_graph::{SvmKnowledgeGraph, Entity, Edge};
#[cfg(feature = "ai-agents")]
use crate::ai::memory::AgentMemory;
#[cfg(feature = "ai-agents")]
use crate::ai::embeddings::EmbeddingService;

#[cfg(not(feature = "ai-agents"))]
use crate::stubs::{SvmKnowledgeGraph, Entity, Edge, AgentMemory, EmbeddingService};
use crate::inter_exex::{ExExMessage, MessageType as InterMessageType};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tracing::{debug, info, warn, error};

/// State synchronization manager
pub struct StateSyncManager {
    /// Current sync state
    state: Arc<RwLock<SyncState>>,
    
    /// State snapshots
    snapshots: Arc<SnapshotManager>,
    
    /// Delta tracker
    delta_tracker: Arc<DeltaTracker>,
    
    /// Sync coordinator
    coordinator: Arc<SyncCoordinator>,
    
    /// Configuration
    config: SyncConfig,
    
    /// Metrics
    metrics: Arc<RwLock<SyncMetrics>>,
}

/// Synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Sync mode
    pub mode: SyncMode,
    /// Snapshot interval (seconds)
    pub snapshot_interval: u64,
    /// Delta threshold for triggering sync
    pub delta_threshold: usize,
    /// Conflict resolution strategy
    pub conflict_resolution: ConflictResolution,
    /// Maximum retries
    pub max_retries: u32,
    /// Retry delay (ms)
    pub retry_delay_ms: u64,
    /// Enable validation
    pub enable_validation: bool,
}

/// Synchronization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncMode {
    /// Real-time synchronization
    RealTime,
    /// Periodic batch synchronization
    Periodic,
    /// Manual synchronization only
    Manual,
    /// Adaptive based on load
    Adaptive,
}

/// Conflict resolution strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Latest write wins
    LatestWins,
    /// Source system wins
    SourceWins,
    /// Manual resolution required
    Manual,
    /// Custom resolution function
    Custom,
}

/// Current synchronization state
#[derive(Debug, Clone)]
pub struct SyncState {
    /// Current status
    pub status: SyncStatus,
    /// Last sync timestamp
    pub last_sync: Option<SystemTime>,
    /// Active syncs
    pub active_syncs: HashMap<String, ActiveSync>,
    /// Pending changes
    pub pending_changes: usize,
    /// Error count
    pub error_count: u32,
}

/// Sync status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    /// Idle, no sync in progress
    Idle,
    /// Sync in progress
    Syncing,
    /// Sync failed, retry pending
    Failed,
    /// Sync paused
    Paused,
}

/// Active sync information
#[derive(Debug, Clone)]
pub struct ActiveSync {
    /// Sync ID
    pub id: String,
    /// Direction
    pub direction: SyncDirection,
    /// Start time
    pub started_at: Instant,
    /// Items processed
    pub items_processed: usize,
    /// Total items
    pub total_items: usize,
}

/// Sync direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    /// Push to remote
    Push,
    /// Pull from remote
    Pull,
    /// Bidirectional sync
    Bidirectional,
}

/// State snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Snapshot type
    pub snapshot_type: SnapshotType,
    /// State data
    pub data: SnapshotData,
    /// Checksum
    pub checksum: String,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Snapshot types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotType {
    /// Full state snapshot
    Full,
    /// Incremental snapshot
    Incremental,
    /// Checkpoint snapshot
    Checkpoint,
}

/// Snapshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    /// Memory state
    pub memory_state: MemorySnapshot,
    /// Knowledge graph state
    pub graph_state: GraphSnapshot,
    /// Embedding state
    pub embedding_state: EmbeddingSnapshot,
    /// Custom state
    pub custom_state: HashMap<String, serde_json::Value>,
}

/// Memory snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    /// Short-term memory items
    pub short_term_count: usize,
    /// Long-term memory items
    pub long_term_count: usize,
    /// Memory checksum
    pub checksum: String,
    /// Serialized memory data
    pub data: Vec<u8>,
}

/// Graph snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    /// Node count
    pub node_count: usize,
    /// Edge count
    pub edge_count: usize,
    /// Graph checksum
    pub checksum: String,
    /// Serialized graph data
    pub data: Vec<u8>,
}

/// Embedding snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingSnapshot {
    /// Total embeddings
    pub embedding_count: usize,
    /// Cache size
    pub cache_size: usize,
    /// Embedding checksum
    pub checksum: String,
}

/// State delta
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDelta {
    /// Delta ID
    pub id: String,
    /// Base snapshot ID
    pub base_snapshot_id: String,
    /// Timestamp
    pub timestamp: SystemTime,
    /// Changes
    pub changes: Vec<StateChange>,
    /// Size in bytes
    pub size_bytes: usize,
}

/// State change types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StateChange {
    /// Memory update
    MemoryUpdate {
        agent_id: String,
        operation: MemoryOperation,
        data: Vec<u8>,
    },
    /// Graph update
    GraphUpdate {
        operation: GraphOperation,
        data: Vec<u8>,
    },
    /// Embedding update
    EmbeddingUpdate {
        key: String,
        operation: EmbeddingOperation,
        data: Vec<u8>,
    },
}

/// Memory operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryOperation {
    Add,
    Update,
    Delete,
    Consolidate,
}

/// Graph operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GraphOperation {
    AddNode,
    UpdateNode,
    DeleteNode,
    AddEdge,
    UpdateEdge,
    DeleteEdge,
}

/// Embedding operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmbeddingOperation {
    Add,
    Update,
    Delete,
    Invalidate,
}

/// Snapshot manager
pub struct SnapshotManager {
    /// Stored snapshots
    snapshots: DashMap<String, StateSnapshot>,
    /// Snapshot index by timestamp
    timeline: Arc<RwLock<Vec<(SystemTime, String)>>>,
    /// Storage backend
    storage: Arc<dyn SnapshotStorage>,
}

/// Trait for snapshot storage
#[async_trait]
pub trait SnapshotStorage: Send + Sync {
    /// Store snapshot
    async fn store(&self, snapshot: &StateSnapshot) -> Result<()>;
    
    /// Load snapshot
    async fn load(&self, id: &str) -> Result<StateSnapshot>;
    
    /// List snapshots
    async fn list(&self, after: Option<SystemTime>) -> Result<Vec<String>>;
    
    /// Delete snapshot
    async fn delete(&self, id: &str) -> Result<()>;
}

/// Delta tracker for incremental sync
pub struct DeltaTracker {
    /// Pending deltas
    pending: Arc<Mutex<VecDeque<StateDelta>>>,
    /// Applied deltas
    applied: DashMap<String, AppliedDelta>,
    /// Delta buffer
    buffer: Arc<Mutex<DeltaBuffer>>,
}

/// Applied delta information
#[derive(Debug, Clone)]
pub struct AppliedDelta {
    /// Delta ID
    pub id: String,
    /// Applied timestamp
    pub applied_at: SystemTime,
    /// Result
    pub result: DeltaResult,
}

/// Delta application result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaResult {
    Success,
    PartialSuccess { applied: usize, failed: usize },
    Failed { error: String },
}

/// Delta buffer for batching
pub struct DeltaBuffer {
    /// Current changes
    changes: Vec<StateChange>,
    /// Buffer size
    size_bytes: usize,
    /// Last flush
    last_flush: Instant,
}

/// Sync coordinator
pub struct SyncCoordinator {
    /// Remote endpoints
    remotes: DashMap<String, RemoteEndpoint>,
    /// Sync sessions
    sessions: DashMap<String, SyncSession>,
    /// Message channel
    message_tx: mpsc::Sender<SyncMessage>,
    message_rx: Arc<Mutex<mpsc::Receiver<SyncMessage>>>,
}

/// Remote endpoint information
#[derive(Debug, Clone)]
pub struct RemoteEndpoint {
    /// Endpoint ID
    pub id: String,
    /// Endpoint URL
    pub url: String,
    /// Connection state
    pub state: ConnectionState,
    /// Last seen
    pub last_seen: Instant,
    /// Capabilities
    pub capabilities: EndpointCapabilities,
}

/// Connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Connecting,
    Error,
}

/// Endpoint capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointCapabilities {
    /// Supported sync modes
    pub sync_modes: Vec<SyncMode>,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Supports compression
    pub compression: bool,
    /// Protocol version
    pub protocol_version: String,
}

/// Sync session
#[derive(Debug, Clone)]
pub struct SyncSession {
    /// Session ID
    pub id: String,
    /// Remote endpoint
    pub remote_id: String,
    /// Direction
    pub direction: SyncDirection,
    /// State
    pub state: SessionState,
    /// Progress
    pub progress: SyncProgress,
}

/// Session states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Initializing,
    Negotiating,
    Transferring,
    Validating,
    Completing,
    Completed,
    Failed,
}

/// Sync progress tracking
#[derive(Debug, Clone)]
pub struct SyncProgress {
    /// Total items
    pub total_items: usize,
    /// Processed items
    pub processed_items: usize,
    /// Bytes transferred
    pub bytes_transferred: usize,
    /// Start time
    pub started_at: Instant,
    /// Estimated completion
    pub estimated_completion: Option<Instant>,
}

/// Sync messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request snapshot
    RequestSnapshot {
        requester_id: String,
        after_timestamp: Option<SystemTime>,
    },
    /// Send snapshot
    SendSnapshot {
        snapshot: StateSnapshot,
    },
    /// Request delta
    RequestDelta {
        base_snapshot_id: String,
        after_timestamp: SystemTime,
    },
    /// Send delta
    SendDelta {
        delta: StateDelta,
    },
    /// Acknowledge receipt
    Acknowledge {
        message_id: String,
        result: ProcessingResult,
    },
}

/// Processing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProcessingResult {
    Success,
    Failed { error: String },
    Retry { reason: String },
}

/// Sync metrics
#[derive(Debug, Clone, Default)]
pub struct SyncMetrics {
    /// Total syncs
    pub total_syncs: u64,
    /// Successful syncs
    pub successful_syncs: u64,
    /// Failed syncs
    pub failed_syncs: u64,
    /// Bytes synced
    pub bytes_synced: u64,
    /// Average sync time (ms)
    pub avg_sync_time_ms: f64,
    /// Last sync duration
    pub last_sync_duration_ms: u64,
}

impl StateSyncManager {
    /// Create a new state sync manager
    pub fn new(config: SyncConfig) -> Self {
        let (tx, rx) = mpsc::channel(1000);
        
        Self {
            state: Arc::new(RwLock::new(SyncState {
                status: SyncStatus::Idle,
                last_sync: None,
                active_syncs: HashMap::new(),
                pending_changes: 0,
                error_count: 0,
            })),
            snapshots: Arc::new(SnapshotManager::new()),
            delta_tracker: Arc::new(DeltaTracker::new()),
            coordinator: Arc::new(SyncCoordinator::new(tx, rx)),
            config,
            metrics: Arc::new(RwLock::new(SyncMetrics::default())),
        }
    }
    
    /// Start the sync manager
    pub async fn start(&self) -> Result<()> {
        info!("Starting state sync manager");
        
        // Start snapshot scheduler
        if self.config.snapshot_interval > 0 {
            self.start_snapshot_scheduler().await?;
        }
        
        // Start delta processor
        self.start_delta_processor().await?;
        
        // Start message handler
        self.coordinator.start_message_handler().await?;
        
        Ok(())
    }
    
    /// Sync all state
    pub async fn sync_all(&self) -> Result<SyncStatus> {
        let mut state = self.state.write().await;
        
        if state.status == SyncStatus::Syncing {
            return Ok(state.status.clone());
        }
        
        state.status = SyncStatus::Syncing;
        drop(state);
        
        let start = Instant::now();
        
        // Create snapshot
        let snapshot = self.create_snapshot(SnapshotType::Full).await?;
        
        // Get remote endpoints
        let remotes = self.coordinator.get_active_remotes().await;
        
        // Sync with each remote
        for remote in remotes {
            match self.sync_with_remote(&remote.id, SyncDirection::Bidirectional).await {
                Ok(_) => {
                    debug!("Synced with remote {}", remote.id);
                }
                Err(e) => {
                    error!("Failed to sync with remote {}: {}", remote.id, e);
                }
            }
        }
        
        // Update metrics
        let duration = start.elapsed();
        self.update_metrics(duration, true).await;
        
        // Update state
        let mut state = self.state.write().await;
        state.status = SyncStatus::Idle;
        state.last_sync = Some(SystemTime::now());
        
        Ok(state.status.clone())
    }
    
    /// Create a state snapshot
    pub async fn create_snapshot(&self, snapshot_type: SnapshotType) -> Result<StateSnapshot> {
        info!("Creating {:?} snapshot", snapshot_type);
        
        let snapshot = StateSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: SystemTime::now(),
            snapshot_type,
            data: self.collect_snapshot_data().await?,
            checksum: String::new(), // Will be calculated
            metadata: HashMap::new(),
        };
        
        // Calculate checksum
        let checksum = self.calculate_checksum(&snapshot)?;
        let mut snapshot = snapshot;
        snapshot.checksum = checksum;
        
        // Store snapshot
        self.snapshots.store(&snapshot).await?;
        
        Ok(snapshot)
    }
    
    /// Apply state delta
    pub async fn apply_delta(&self, delta: StateDelta) -> Result<()> {
        info!("Applying delta {} with {} changes", delta.id, delta.changes.len());
        
        let mut applied = 0;
        let mut failed = 0;
        
        for change in &delta.changes {
            match self.apply_change(change).await {
                Ok(_) => applied += 1,
                Err(e) => {
                    error!("Failed to apply change: {}", e);
                    failed += 1;
                }
            }
        }
        
        let result = if failed == 0 {
            DeltaResult::Success
        } else if applied > 0 {
            DeltaResult::PartialSuccess { applied, failed }
        } else {
            DeltaResult::Failed {
                error: "All changes failed".to_string(),
            }
        };
        
        self.delta_tracker.mark_applied(&delta.id, result).await;
        
        Ok(())
    }
    
    /// Sync with a specific remote
    async fn sync_with_remote(
        &self,
        remote_id: &str,
        direction: SyncDirection,
    ) -> Result<()> {
        // Create sync session
        let session = self.coordinator.create_session(remote_id, direction).await?;
        
        match direction {
            SyncDirection::Push => {
                self.push_to_remote(&session).await?;
            }
            SyncDirection::Pull => {
                self.pull_from_remote(&session).await?;
            }
            SyncDirection::Bidirectional => {
                self.push_to_remote(&session).await?;
                self.pull_from_remote(&session).await?;
            }
        }
        
        Ok(())
    }
    
    /// Push state to remote
    async fn push_to_remote(&self, session: &SyncSession) -> Result<()> {
        // Get latest snapshot
        let snapshot = self.snapshots.get_latest().await?;
        
        // Send snapshot
        self.coordinator.send_message(
            &session.remote_id,
            SyncMessage::SendSnapshot { snapshot },
        ).await?;
        
        Ok(())
    }
    
    /// Pull state from remote
    async fn pull_from_remote(&self, session: &SyncSession) -> Result<()> {
        // Request snapshot
        self.coordinator.send_message(
            &session.remote_id,
            SyncMessage::RequestSnapshot {
                requester_id: session.id.clone(),
                after_timestamp: self.state.read().await.last_sync,
            },
        ).await?;
        
        // Wait for response (simplified)
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        Ok(())
    }
    
    /// Collect snapshot data
    async fn collect_snapshot_data(&self) -> Result<SnapshotData> {
        Ok(SnapshotData {
            memory_state: MemorySnapshot {
                short_term_count: 0, // Would query actual memory
                long_term_count: 0,
                checksum: String::new(),
                data: vec![],
            },
            graph_state: GraphSnapshot {
                node_count: 0, // Would query actual graph
                edge_count: 0,
                checksum: String::new(),
                data: vec![],
            },
            embedding_state: EmbeddingSnapshot {
                embedding_count: 0, // Would query actual embeddings
                cache_size: 0,
                checksum: String::new(),
            },
            custom_state: HashMap::new(),
        })
    }
    
    /// Apply a state change
    async fn apply_change(&self, change: &StateChange) -> Result<()> {
        match change {
            StateChange::MemoryUpdate { agent_id, operation, data } => {
                // Apply memory update
                debug!("Applying memory update for agent {}", agent_id);
            }
            StateChange::GraphUpdate { operation, data } => {
                // Apply graph update
                debug!("Applying graph update");
            }
            StateChange::EmbeddingUpdate { key, operation, data } => {
                // Apply embedding update
                debug!("Applying embedding update for key {}", key);
            }
        }
        Ok(())
    }
    
    /// Calculate snapshot checksum
    fn calculate_checksum(&self, snapshot: &StateSnapshot) -> Result<String> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        
        let data = serde_json::to_vec(snapshot)?;
        hasher.update(&data);
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// Start snapshot scheduler
    async fn start_snapshot_scheduler(&self) -> Result<()> {
        let manager = Arc::new(self.clone());
        let interval = self.config.snapshot_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval));
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.create_snapshot(SnapshotType::Incremental).await {
                    error!("Scheduled snapshot failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Start delta processor
    async fn start_delta_processor(&self) -> Result<()> {
        let tracker = self.delta_tracker.clone();
        let manager = Arc::new(self.clone());
        
        tokio::spawn(async move {
            loop {
                if let Some(delta) = tracker.get_next_pending().await {
                    if let Err(e) = manager.apply_delta(delta).await {
                        error!("Delta processing failed: {}", e);
                    }
                }
                
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        Ok(())
    }
    
    /// Update sync metrics
    async fn update_metrics(&self, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        
        metrics.total_syncs += 1;
        if success {
            metrics.successful_syncs += 1;
        } else {
            metrics.failed_syncs += 1;
        }
        
        let duration_ms = duration.as_millis() as u64;
        metrics.last_sync_duration_ms = duration_ms;
        
        // Update average
        let n = metrics.total_syncs as f64;
        metrics.avg_sync_time_ms = 
            (metrics.avg_sync_time_ms * (n - 1.0) + duration_ms as f64) / n;
    }
    
    /// Get current sync status
    pub async fn get_status(&self) -> SyncStatus {
        self.state.read().await.status.clone()
    }
    
    /// Get sync metrics
    pub async fn get_metrics(&self) -> SyncMetrics {
        self.metrics.read().await.clone()
    }
}

impl Clone for StateSyncManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            snapshots: self.snapshots.clone(),
            delta_tracker: self.delta_tracker.clone(),
            coordinator: self.coordinator.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new() -> Self {
        Self {
            snapshots: DashMap::new(),
            timeline: Arc::new(RwLock::new(Vec::new())),
            storage: Arc::new(InMemorySnapshotStorage::new()),
        }
    }
    
    /// Store a snapshot
    pub async fn store(&self, snapshot: &StateSnapshot) -> Result<()> {
        // Store in memory
        self.snapshots.insert(snapshot.id.clone(), snapshot.clone());
        
        // Update timeline
        let mut timeline = self.timeline.write().await;
        timeline.push((snapshot.timestamp, snapshot.id.clone()));
        timeline.sort_by_key(|(ts, _)| *ts);
        
        // Store in backend
        self.storage.store(snapshot).await?;
        
        Ok(())
    }
    
    /// Get latest snapshot
    pub async fn get_latest(&self) -> Result<StateSnapshot> {
        let timeline = self.timeline.read().await;
        
        if let Some((_, id)) = timeline.last() {
            if let Some(snapshot) = self.snapshots.get(id) {
                return Ok(snapshot.clone());
            }
        }
        
        Err(AIAgentError::ProcessingError("No snapshots available".to_string()))
    }
}

impl DeltaTracker {
    /// Create a new delta tracker
    pub fn new() -> Self {
        Self {
            pending: Arc::new(Mutex::new(VecDeque::new())),
            applied: DashMap::new(),
            buffer: Arc::new(Mutex::new(DeltaBuffer {
                changes: Vec::new(),
                size_bytes: 0,
                last_flush: Instant::now(),
            })),
        }
    }
    
    /// Add change to buffer
    pub async fn add_change(&self, change: StateChange) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        
        let size = bincode::serialize(&change)?.len();
        buffer.changes.push(change);
        buffer.size_bytes += size;
        
        // Auto-flush if buffer is large
        if buffer.size_bytes > 1024 * 1024 { // 1MB
            self.flush_buffer().await?;
        }
        
        Ok(())
    }
    
    /// Flush buffer to delta
    pub async fn flush_buffer(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        
        if buffer.changes.is_empty() {
            return Ok(());
        }
        
        let delta = StateDelta {
            id: uuid::Uuid::new_v4().to_string(),
            base_snapshot_id: String::new(), // Would track actual base
            timestamp: SystemTime::now(),
            changes: std::mem::take(&mut buffer.changes),
            size_bytes: buffer.size_bytes,
        };
        
        buffer.size_bytes = 0;
        buffer.last_flush = Instant::now();
        
        self.pending.lock().await.push_back(delta);
        
        Ok(())
    }
    
    /// Get next pending delta
    pub async fn get_next_pending(&self) -> Option<StateDelta> {
        self.pending.lock().await.pop_front()
    }
    
    /// Mark delta as applied
    pub async fn mark_applied(&self, delta_id: &str, result: DeltaResult) {
        self.applied.insert(delta_id.to_string(), AppliedDelta {
            id: delta_id.to_string(),
            applied_at: SystemTime::now(),
            result,
        });
    }
}

impl SyncCoordinator {
    /// Create a new sync coordinator
    pub fn new(tx: mpsc::Sender<SyncMessage>, rx: mpsc::Receiver<SyncMessage>) -> Self {
        Self {
            remotes: DashMap::new(),
            sessions: DashMap::new(),
            message_tx: tx,
            message_rx: Arc::new(Mutex::new(rx)),
        }
    }
    
    /// Start message handler
    pub async fn start_message_handler(&self) -> Result<()> {
        let coordinator = Arc::new(self.clone());
        let rx = self.message_rx.clone();
        
        tokio::spawn(async move {
            let mut rx = rx.lock().await;
            
            while let Some(msg) = rx.recv().await {
                if let Err(e) = coordinator.handle_message(msg).await {
                    error!("Message handling failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle sync message
    async fn handle_message(&self, msg: SyncMessage) -> Result<()> {
        match msg {
            SyncMessage::RequestSnapshot { requester_id, after_timestamp } => {
                debug!("Received snapshot request from {}", requester_id);
            }
            SyncMessage::SendSnapshot { snapshot } => {
                debug!("Received snapshot {}", snapshot.id);
            }
            SyncMessage::RequestDelta { base_snapshot_id, after_timestamp } => {
                debug!("Received delta request for base {}", base_snapshot_id);
            }
            SyncMessage::SendDelta { delta } => {
                debug!("Received delta {}", delta.id);
            }
            SyncMessage::Acknowledge { message_id, result } => {
                debug!("Received acknowledgment for {}: {:?}", message_id, result);
            }
        }
        Ok(())
    }
    
    /// Get active remote endpoints
    pub async fn get_active_remotes(&self) -> Vec<RemoteEndpoint> {
        self.remotes
            .iter()
            .filter(|entry| entry.value().state == ConnectionState::Connected)
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    /// Create sync session
    pub async fn create_session(
        &self,
        remote_id: &str,
        direction: SyncDirection,
    ) -> Result<SyncSession> {
        let session = SyncSession {
            id: uuid::Uuid::new_v4().to_string(),
            remote_id: remote_id.to_string(),
            direction,
            state: SessionState::Initializing,
            progress: SyncProgress {
                total_items: 0,
                processed_items: 0,
                bytes_transferred: 0,
                started_at: Instant::now(),
                estimated_completion: None,
            },
        };
        
        self.sessions.insert(session.id.clone(), session.clone());
        
        Ok(session)
    }
    
    /// Send message to remote
    pub async fn send_message(&self, remote_id: &str, msg: SyncMessage) -> Result<()> {
        // In real implementation, would send over network
        self.message_tx.send(msg).await?;
        Ok(())
    }
}

impl Clone for SyncCoordinator {
    fn clone(&self) -> Self {
        Self {
            remotes: self.remotes.clone(),
            sessions: self.sessions.clone(),
            message_tx: self.message_tx.clone(),
            message_rx: self.message_rx.clone(),
        }
    }
}

/// In-memory snapshot storage (for testing)
struct InMemorySnapshotStorage {
    storage: DashMap<String, StateSnapshot>,
}

impl InMemorySnapshotStorage {
    fn new() -> Self {
        Self {
            storage: DashMap::new(),
        }
    }
}

#[async_trait]
impl SnapshotStorage for InMemorySnapshotStorage {
    async fn store(&self, snapshot: &StateSnapshot) -> Result<()> {
        self.storage.insert(snapshot.id.clone(), snapshot.clone());
        Ok(())
    }
    
    async fn load(&self, id: &str) -> Result<StateSnapshot> {
        self.storage
            .get(id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| AIAgentError::ProcessingError("Snapshot not found".to_string()))
    }
    
    async fn list(&self, after: Option<SystemTime>) -> Result<Vec<String>> {
        let snapshots: Vec<String> = if let Some(after) = after {
            self.storage
                .iter()
                .filter(|entry| entry.value().timestamp > after)
                .map(|entry| entry.key().clone())
                .collect()
        } else {
            self.storage.iter().map(|entry| entry.key().clone()).collect()
        };
        
        Ok(snapshots)
    }
    
    async fn delete(&self, id: &str) -> Result<()> {
        self.storage.remove(id);
        Ok(())
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            mode: SyncMode::Adaptive,
            snapshot_interval: 300, // 5 minutes
            delta_threshold: 1000,
            conflict_resolution: ConflictResolution::LatestWins,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_validation: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_state_sync_manager() {
        let config = SyncConfig::default();
        let manager = StateSyncManager::new(config);
        
        // Start manager
        assert!(manager.start().await.is_ok());
        
        // Check initial status
        assert_eq!(manager.get_status().await, SyncStatus::Idle);
    }

    #[tokio::test]
    async fn test_snapshot_creation() {
        let manager = StateSyncManager::new(SyncConfig::default());
        
        // Create snapshot
        let snapshot = manager.create_snapshot(SnapshotType::Full).await.unwrap();
        
        assert!(!snapshot.id.is_empty());
        assert!(!snapshot.checksum.is_empty());
    }

    #[tokio::test]
    async fn test_delta_tracking() {
        let tracker = DeltaTracker::new();
        
        // Add change
        let change = StateChange::MemoryUpdate {
            agent_id: "test_agent".to_string(),
            operation: MemoryOperation::Add,
            data: vec![1, 2, 3],
        };
        
        assert!(tracker.add_change(change).await.is_ok());
    }
}