//! State Synchronization for Cross-ExEx Consistency
//!
//! This module handles state synchronization between SVM ExEx and RAG Memory ExEx
//! using Accounts Lattice Hash (ALH) for efficient verification.

use crate::errors::*;
use crate::enhanced_exex::AccountsLatticeHash;
use crate::inter_exex::{
    InterExExCoordinator, SvmExExMessage, AccountUpdate, AccountUpdateType,
};
use alloy_primitives::{Address, B256};
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tracing::{info, debug, warn, error};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// State synchronization service
pub struct StateSyncService {
    /// Inter-ExEx coordinator
    coordinator: Arc<InterExExCoordinator>,
    /// Current ALH state
    current_alh: Arc<RwLock<AccountsLatticeHash>>,
    /// State history for reorg handling
    state_history: Arc<RwLock<VecDeque<StateSnapshot>>>,
    /// Pending state updates
    pending_updates: Arc<RwLock<HashMap<u64, Vec<AccountUpdate>>>>,
    /// Sync configuration
    config: StateSyncConfig,
    /// Metrics
    metrics: Arc<SyncMetrics>,
}

/// State synchronization configuration
#[derive(Debug, Clone)]
pub struct StateSyncConfig {
    /// Maximum history size
    pub max_history_size: usize,
    /// Sync interval (seconds)
    pub sync_interval_secs: u64,
    /// Enable automatic sync
    pub auto_sync_enabled: bool,
    /// State divergence threshold
    pub divergence_threshold: f64,
    /// Maximum pending updates
    pub max_pending_updates: usize,
}

impl Default for StateSyncConfig {
    fn default() -> Self {
        Self {
            max_history_size: 1000,
            sync_interval_secs: 10,
            auto_sync_enabled: true,
            divergence_threshold: 0.001,
            max_pending_updates: 10000,
        }
    }
}

/// Snapshot of state at a specific block
#[derive(Debug, Clone)]
struct StateSnapshot {
    /// Block height
    block_height: u64,
    /// ALH at this block
    alh_hash: [u8; 32],
    /// Account count
    account_count: u64,
    /// Total lamports
    total_lamports: u128,
    /// Timestamp
    timestamp: Instant,
}

/// Synchronization metrics
#[derive(Default)]
struct SyncMetrics {
    /// Total syncs performed
    total_syncs: std::sync::atomic::AtomicU64,
    /// Successful syncs
    successful_syncs: std::sync::atomic::AtomicU64,
    /// Failed syncs
    failed_syncs: std::sync::atomic::AtomicU64,
    /// State divergences detected
    divergences_detected: std::sync::atomic::AtomicU64,
    /// Last sync time
    last_sync_time: RwLock<Option<Instant>>,
}

impl StateSyncService {
    /// Create a new state sync service
    pub fn new(
        coordinator: Arc<InterExExCoordinator>,
        initial_alh: AccountsLatticeHash,
        config: StateSyncConfig,
    ) -> Self {
        Self {
            coordinator,
            current_alh: Arc::new(RwLock::new(initial_alh)),
            state_history: Arc::new(RwLock::new(VecDeque::with_capacity(config.max_history_size))),
            pending_updates: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(SyncMetrics::default()),
        }
    }
    
    /// Start the state sync service
    pub async fn start(&self) -> Result<()> {
        if self.config.auto_sync_enabled {
            let service = self.clone();
            tokio::spawn(async move {
                service.sync_loop().await;
            });
        }
        
        info!("State sync service started");
        Ok(())
    }
    
    /// Main synchronization loop
    async fn sync_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.sync_interval_secs));
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.perform_sync().await {
                error!("State sync failed: {}", e);
                self.metrics.failed_syncs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }
    
    /// Perform state synchronization
    async fn perform_sync(&self) -> Result<()> {
        debug!("Performing state synchronization");
        
        // Get current state
        let alh = self.current_alh.read().await;
        let current_hash = alh.hash();
        let account_count = alh.account_count;
        let total_lamports = alh.total_lamports;
        drop(alh);
        
        // Get pending updates
        let pending = self.pending_updates.read().await;
        let latest_block = pending.keys().max().copied().unwrap_or(0);
        drop(pending);
        
        // Create sync message
        let sync_msg = SvmExExMessage::StateSync {
            block_height: latest_block,
            alh_hash: current_hash,
            account_updates: vec![], // Would include actual updates
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        // Broadcast state
        let msg = sync_msg.into();
        self.coordinator.broadcast(msg).await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to broadcast state: {}", e)))?;
        
        // Update metrics
        self.metrics.total_syncs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.metrics.successful_syncs.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        *self.metrics.last_sync_time.write().await = Some(Instant::now());
        
        // Store snapshot
        self.store_snapshot(latest_block, current_hash, account_count, total_lamports).await?;
        
        info!("State sync completed for block {}", latest_block);
        Ok(())
    }
    
    /// Store a state snapshot
    async fn store_snapshot(
        &self,
        block_height: u64,
        alh_hash: [u8; 32],
        account_count: u64,
        total_lamports: u128,
    ) -> Result<()> {
        let snapshot = StateSnapshot {
            block_height,
            alh_hash,
            account_count,
            total_lamports,
            timestamp: Instant::now(),
        };
        
        let mut history = self.state_history.write().await;
        history.push_back(snapshot);
        
        // Evict old snapshots
        while history.len() > self.config.max_history_size {
            history.pop_front();
        }
        
        Ok(())
    }
    
    /// Handle incoming state sync from another ExEx
    pub async fn handle_state_sync(
        &self,
        block_height: u64,
        remote_alh: [u8; 32],
        account_updates: Vec<AccountUpdate>,
    ) -> Result<()> {
        debug!("Received state sync for block {} with {} updates", block_height, account_updates.len());
        
        // Compare ALH
        let local_alh = self.current_alh.read().await;
        let local_hash = local_alh.hash();
        drop(local_alh);
        
        if local_hash != remote_alh {
            warn!("State divergence detected at block {}", block_height);
            self.metrics.divergences_detected.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            
            // Calculate divergence
            let divergence = self.calculate_divergence(&local_hash, &remote_alh);
            
            if divergence > self.config.divergence_threshold {
                error!("Significant state divergence: {:.2}%", divergence * 100.0);
                // Would trigger reconciliation process
            }
        } else {
            debug!("State is synchronized at block {}", block_height);
        }
        
        // Store updates
        let mut pending = self.pending_updates.write().await;
        pending.insert(block_height, account_updates);
        
        // Clean old updates
        if pending.len() > self.config.max_pending_updates {
            let min_block = pending.keys().min().copied().unwrap_or(0);
            pending.remove(&min_block);
        }
        
        Ok(())
    }
    
    /// Update local ALH with account changes
    pub async fn update_account(
        &self,
        address: Address,
        balance: u64,
        data_hash: Option<B256>,
        update_type: AccountUpdateType,
    ) -> Result<()> {
        let mut alh = self.current_alh.write().await;
        
        match update_type {
            AccountUpdateType::Created | AccountUpdateType::BalanceUpdate | AccountUpdateType::DataUpdate => {
                let data_hash_bytes = data_hash.map(|h| h.0).unwrap_or([0u8; 32]);
                alh.update_account(&address.0 .0, balance, &data_hash_bytes);
            }
            AccountUpdateType::Deleted => {
                alh.remove_account(&address.0 .0);
            }
        }
        
        Ok(())
    }
    
    /// Get current state info
    pub async fn get_state_info(&self) -> StateInfo {
        let alh = self.current_alh.read().await;
        let history = self.state_history.read().await;
        let pending = self.pending_updates.read().await;
        
        StateInfo {
            current_alh: alh.hash(),
            account_count: alh.account_count,
            total_lamports: alh.total_lamports,
            history_size: history.len(),
            pending_updates: pending.values().map(|v| v.len()).sum(),
            last_sync: *self.metrics.last_sync_time.read().await,
        }
    }
    
    /// Calculate divergence between two ALH values
    fn calculate_divergence(&self, local: &[u8; 32], remote: &[u8; 32]) -> f64 {
        // Simple bit difference calculation
        let mut diff_bits = 0;
        for i in 0..32 {
            diff_bits += (local[i] ^ remote[i]).count_ones();
        }
        
        diff_bits as f64 / 256.0
    }
    
    /// Revert to a previous state
    pub async fn revert_to_block(&self, block_height: u64) -> Result<()> {
        let history = self.state_history.read().await;
        
        // Find snapshot
        let snapshot = history.iter()
            .find(|s| s.block_height == block_height)
            .ok_or_else(|| SvmExExError::ProcessingError(
                format!("No snapshot found for block {}", block_height)
            ))?;
        
        // Create new ALH from snapshot
        let mut new_alh = AccountsLatticeHash::new();
        // Note: In real implementation, would restore full state
        new_alh.account_count = snapshot.account_count;
        new_alh.total_lamports = snapshot.total_lamports;
        
        // Update current ALH
        let mut current = self.current_alh.write().await;
        *current = new_alh;
        
        info!("Reverted state to block {}", block_height);
        Ok(())
    }
}

/// Clone implementation for service
impl Clone for StateSyncService {
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            current_alh: self.current_alh.clone(),
            state_history: self.state_history.clone(),
            pending_updates: self.pending_updates.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

/// State information
#[derive(Debug, Clone)]
pub struct StateInfo {
    /// Current ALH
    pub current_alh: [u8; 32],
    /// Account count
    pub account_count: u64,
    /// Total lamports
    pub total_lamports: u128,
    /// History size
    pub history_size: usize,
    /// Pending updates count
    pub pending_updates: usize,
    /// Last sync time
    pub last_sync: Option<Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_state_sync_creation() {
        let config = crate::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            InterExExCoordinator::new(config, "test-node".to_string())
                .expect("Failed to create coordinator")
        );
        
        let alh = AccountsLatticeHash::new();
        let sync_config = StateSyncConfig::default();
        let service = StateSyncService::new(coordinator, alh, sync_config);
        
        let info = service.get_state_info().await;
        assert_eq!(info.account_count, 0);
        assert_eq!(info.total_lamports, 0);
    }
    
    #[tokio::test]
    async fn test_account_update() {
        let config = crate::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            InterExExCoordinator::new(config, "test-node".to_string())
                .expect("Failed to create coordinator")
        );
        
        let alh = AccountsLatticeHash::new();
        let sync_config = StateSyncConfig::default();
        let service = StateSyncService::new(coordinator, alh, sync_config);
        
        // Update account
        let address = Address::default();
        service.update_account(
            address,
            1000,
            None,
            AccountUpdateType::Created,
        ).await.expect("Failed to update account");
        
        let info = service.get_state_info().await;
        assert_eq!(info.account_count, 1);
        assert_eq!(info.total_lamports, 1000);
    }
}