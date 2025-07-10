//! State Synchronization Module
//! 
//! This module provides bidirectional state synchronization between
//! RAG ExEx and SVM ExEx instances.

pub mod state_sync;
pub mod protocol;

pub use state_sync::{
    StateSyncManager, SyncConfig, SyncState, SyncStatus,
    StateSnapshot, StateDelta, SyncDirection,
};
pub use protocol::{
    SyncProtocol, ProtocolVersion, SyncMessage, MessageType,
    SyncRequest, SyncResponse, SyncError as ProtocolError,
};

use crate::errors::*;
use std::sync::Arc;
use tokio::sync::RwLock;

/// State synchronization service
pub struct SyncService {
    /// State sync manager
    pub manager: Arc<StateSyncManager>,
    
    /// Sync protocol handler
    pub protocol: Arc<SyncProtocol>,
    
    /// Service configuration
    config: ServiceConfig,
}

/// Service configuration
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Enable automatic sync
    pub auto_sync: bool,
    /// Sync interval (seconds)
    pub sync_interval: u64,
    /// Maximum sync batch size
    pub max_batch_size: usize,
    /// Enable compression
    pub enable_compression: bool,
}

impl SyncService {
    /// Create a new sync service
    pub fn new(config: ServiceConfig, sync_config: SyncConfig) -> Self {
        Self {
            manager: Arc::new(StateSyncManager::new(sync_config)),
            protocol: Arc::new(SyncProtocol::new()),
            config,
        }
    }
    
    /// Start the sync service
    pub async fn start(&self) -> Result<()> {
        // Initialize protocol
        self.protocol.initialize().await?;
        
        // Start sync manager
        self.manager.start().await?;
        
        // Start auto-sync if enabled
        if self.config.auto_sync {
            self.start_auto_sync().await?;
        }
        
        Ok(())
    }
    
    /// Start automatic synchronization
    async fn start_auto_sync(&self) -> Result<()> {
        let manager = self.manager.clone();
        let interval = self.config.sync_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval)
            );
            
            loop {
                interval.tick().await;
                
                if let Err(e) = manager.sync_all().await {
                    tracing::error!("Auto-sync failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Manual sync trigger
    pub async fn sync_now(&self) -> Result<SyncStatus> {
        self.manager.sync_all().await
    }
    
    /// Get sync status
    pub async fn get_status(&self) -> SyncStatus {
        self.manager.get_status().await
    }
}

/// Sync event types
#[derive(Debug, Clone)]
pub enum SyncEvent {
    /// Sync started
    Started {
        direction: SyncDirection,
        peer: String,
    },
    /// Sync progress
    Progress {
        items_synced: usize,
        total_items: usize,
    },
    /// Sync completed
    Completed {
        duration_ms: u64,
        items_synced: usize,
    },
    /// Sync failed
    Failed {
        error: String,
    },
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            auto_sync: true,
            sync_interval: 30,
            max_batch_size: 1000,
            enable_compression: true,
        }
    }
}