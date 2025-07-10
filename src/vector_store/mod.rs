//! Vector Store Module
//! 
//! This module provides vector database operations with consistency
//! guarantees during blockchain reorganizations.

pub mod consistency;
pub mod recovery;

pub use consistency::{
    VectorStoreConsistencyManager, ConsistencyConfig, ConsistencyState,
    VectorSnapshot, VectorCheckpoint, ConsistencyValidator,
};
pub use recovery::{
    VectorStoreRecoveryManager, RecoveryConfig, RecoveryStrategy,
    RecoveryResult, VectorRecoveryHandler,
};

use crate::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Vector store service with consistency guarantees
pub struct VectorStoreService {
    /// Consistency manager
    pub consistency: Arc<VectorStoreConsistencyManager>,
    
    /// Recovery manager
    pub recovery: Arc<VectorStoreRecoveryManager>,
    
    /// Service configuration
    config: ServiceConfig,
}

/// Service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Enable automatic consistency checks
    pub auto_consistency: bool,
    /// Consistency check interval (seconds)
    pub check_interval: u64,
    /// Enable recovery on startup
    pub auto_recovery: bool,
    /// Vector store endpoint
    pub endpoint: String,
    /// Maximum vector dimensions
    pub max_dimensions: usize,
}

impl VectorStoreService {
    /// Create a new vector store service
    pub fn new(config: ServiceConfig) -> Self {
        let consistency_config = ConsistencyConfig {
            check_interval: config.check_interval,
            auto_repair: true,
            validation_level: consistency::ValidationLevel::Full,
            snapshot_interval: 3600,
            max_snapshots: 10,
        };
        
        let recovery_config = RecoveryConfig {
            strategy: recovery::RecoveryStrategy::IncrementalReplay,
            max_retries: 3,
            retry_delay: std::time::Duration::from_secs(5),
            enable_validation: true,
            parallel_recovery: true,
        };
        
        Self {
            consistency: Arc::new(VectorStoreConsistencyManager::new(consistency_config)),
            recovery: Arc::new(VectorStoreRecoveryManager::new(recovery_config)),
            config,
        }
    }
    
    /// Start the vector store service
    pub async fn start(&self) -> Result<()> {
        // Initialize consistency manager
        self.consistency.initialize().await?;
        
        // Initialize recovery manager
        self.recovery.initialize().await?;
        
        // Start auto-consistency checks if enabled
        if self.config.auto_consistency {
            self.start_consistency_monitoring().await?;
        }
        
        // Perform recovery check if enabled
        if self.config.auto_recovery {
            self.check_and_recover().await?;
        }
        
        Ok(())
    }
    
    /// Handle blockchain reorganization
    pub async fn handle_reorg(
        &self,
        old_tip: [u8; 32],
        new_tip: [u8; 32],
        depth: u64,
    ) -> Result<()> {
        // Check for consistency issues
        let consistency_issues = self.consistency
            .check_reorg_impact(old_tip, new_tip, depth)
            .await?;
        
        if !consistency_issues.is_empty() {
            // Trigger recovery
            self.recovery
                .recover_from_reorg(old_tip, new_tip, depth)
                .await?;
        }
        
        Ok(())
    }
    
    /// Validate vector store consistency
    pub async fn validate_consistency(&self) -> Result<bool> {
        self.consistency.validate().await
    }
    
    /// Create vector store snapshot
    pub async fn create_snapshot(&self, block_height: u64) -> Result<String> {
        self.consistency.create_snapshot(block_height).await
    }
    
    /// Start consistency monitoring
    async fn start_consistency_monitoring(&self) -> Result<()> {
        let consistency = self.consistency.clone();
        let interval = self.config.check_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(interval)
            );
            
            loop {
                interval.tick().await;
                
                if let Err(e) = consistency.perform_consistency_check().await {
                    tracing::error!("Consistency check failed: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Check and recover if needed
    async fn check_and_recover(&self) -> Result<()> {
        if !self.validate_consistency().await? {
            tracing::warn!("Vector store inconsistency detected, starting recovery");
            self.recovery.recover().await?;
        }
        
        Ok(())
    }
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            auto_consistency: true,
            check_interval: 300, // 5 minutes
            auto_recovery: true,
            endpoint: "http://localhost:6333".to_string(),
            max_dimensions: 1536,
        }
    }
}