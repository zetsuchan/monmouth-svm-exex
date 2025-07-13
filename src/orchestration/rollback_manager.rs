//! Rollback mechanisms for failed multi-step transactions

use crate::errors::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Rollback manager for handling transaction failures
pub struct RollbackManager {
    /// Rollback states
    states: Arc<RwLock<HashMap<String, RollbackState>>>,
    
    /// Rollback handlers
    handlers: Arc<RwLock<HashMap<String, Box<dyn RollbackHandler>>>>,
    
    /// Configuration
    config: RollbackConfig,
}

/// Rollback state for a transaction
#[derive(Debug, Clone)]
pub struct RollbackState {
    /// Transaction ID
    pub transaction_id: String,
    
    /// Execution steps completed
    pub completed_steps: Vec<CompletedStep>,
    
    /// Current status
    pub status: RollbackStatus,
    
    /// Created timestamp
    pub created_at: std::time::Instant,
    
    /// Error that triggered rollback
    pub error: Option<String>,
}

/// Completed execution step
#[derive(Debug, Clone)]
pub struct CompletedStep {
    /// Step ID
    pub step_id: String,
    
    /// Tool ID that was executed
    pub tool_id: String,
    
    /// State snapshot before execution
    pub pre_state: StateSnapshot,
    
    /// State changes made
    pub changes: Vec<StateChange>,
    
    /// Execution result
    pub result: StepResult,
    
    /// Timestamp
    pub timestamp: std::time::Instant,
}

/// State snapshot
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    /// Snapshot ID
    pub id: String,
    
    /// State data
    pub data: HashMap<String, Vec<u8>>,
    
    /// Metadata
    pub metadata: HashMap<String, String>,
}

/// State change
#[derive(Debug, Clone)]
pub struct StateChange {
    /// Change type
    pub change_type: ChangeType,
    
    /// Key affected
    pub key: String,
    
    /// Old value
    pub old_value: Option<Vec<u8>>,
    
    /// New value
    pub new_value: Option<Vec<u8>>,
}

/// Change type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
}

/// Step execution result
#[derive(Debug, Clone)]
pub struct StepResult {
    /// Success status
    pub success: bool,
    
    /// Gas used
    pub gas_used: u64,
    
    /// Output data
    pub output: Option<Vec<u8>>,
}

/// Rollback status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackStatus {
    /// Rollback pending
    Pending,
    
    /// Rollback in progress
    InProgress,
    
    /// Rollback completed successfully
    Completed,
    
    /// Rollback failed
    Failed,
    
    /// Manual intervention required
    ManualRequired,
}

/// Rollback handler trait
#[async_trait::async_trait]
pub trait RollbackHandler: Send + Sync {
    /// Check if this handler can rollback the given step
    fn can_handle(&self, step: &CompletedStep) -> bool;
    
    /// Perform rollback for a step
    async fn rollback(&self, step: &CompletedStep) -> Result<RollbackResult>;
    
    /// Verify rollback was successful
    async fn verify(&self, step: &CompletedStep) -> Result<bool>;
}

/// Rollback result
#[derive(Debug, Clone)]
pub struct RollbackResult {
    /// Success status
    pub success: bool,
    
    /// Reverted changes
    pub reverted_changes: Vec<StateChange>,
    
    /// Gas used for rollback
    pub gas_used: u64,
    
    /// Error message if failed
    pub error: Option<String>,
}

/// Rollback configuration
#[derive(Debug, Clone)]
pub struct RollbackConfig {
    /// Maximum rollback attempts
    pub max_attempts: u32,
    
    /// Rollback timeout
    pub timeout_ms: u64,
    
    /// Enable automatic rollback
    pub auto_rollback: bool,
    
    /// Preserve rollback history
    pub preserve_history: bool,
    
    /// History retention duration
    pub history_retention_hours: u64,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            timeout_ms: 300_000, // 5 minutes
            auto_rollback: true,
            preserve_history: true,
            history_retention_hours: 24,
        }
    }
}

impl RollbackManager {
    /// Create a new rollback manager
    pub fn new(config: RollbackConfig) -> Self {
        Self {
            states: Arc::new(RwLock::new(HashMap::new())),
            handlers: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Register a rollback handler
    pub async fn register_handler(
        &self,
        name: String,
        handler: Box<dyn RollbackHandler>
    ) -> Result<()> {
        let mut handlers = self.handlers.write().await;
        
        if handlers.contains_key(&name) {
            return Err(SvmExExError::ProcessingError(
                format!("Handler {} already registered", name)
            ));
        }
        
        info!("Registered rollback handler: {}", name);
        handlers.insert(name, handler);
        Ok(())
    }
    
    /// Create a new rollback state
    pub async fn create_rollback_state(&self, transaction_id: String) -> Result<()> {
        let mut states = self.states.write().await;
        
        if states.contains_key(&transaction_id) {
            return Err(SvmExExError::ProcessingError(
                format!("Rollback state already exists for {}", transaction_id)
            ));
        }
        
        let state = RollbackState {
            transaction_id: transaction_id.clone(),
            completed_steps: Vec::new(),
            status: RollbackStatus::Pending,
            created_at: std::time::Instant::now(),
            error: None,
        };
        
        states.insert(transaction_id, state);
        Ok(())
    }
    
    /// Add a completed step
    pub async fn add_completed_step(
        &self,
        transaction_id: &str,
        step: CompletedStep
    ) -> Result<()> {
        let mut states = self.states.write().await;
        
        let state = states.get_mut(transaction_id)
            .ok_or_else(|| SvmExExError::ProcessingError(
                format!("Rollback state not found for {}", transaction_id)
            ))?;
        
        state.completed_steps.push(step);
        Ok(())
    }
    
    /// Trigger rollback for a transaction
    pub async fn trigger_rollback(
        &self,
        transaction_id: &str,
        error: String
    ) -> Result<()> {
        info!("Triggering rollback for transaction {}: {}", transaction_id, error);
        
        // Update state
        {
            let mut states = self.states.write().await;
            let state = states.get_mut(transaction_id)
                .ok_or_else(|| SvmExExError::ProcessingError(
                    format!("Rollback state not found for {}", transaction_id)
                ))?;
            
            state.status = RollbackStatus::InProgress;
            state.error = Some(error);
        }
        
        // Perform rollback if auto-rollback is enabled
        if self.config.auto_rollback {
            self.perform_rollback(transaction_id).await?;
        }
        
        Ok(())
    }
    
    /// Perform rollback
    async fn perform_rollback(&self, transaction_id: &str) -> Result<()> {
        let steps = {
            let states = self.states.read().await;
            states.get(transaction_id)
                .map(|s| s.completed_steps.clone())
                .unwrap_or_default()
        };
        
        // Rollback in reverse order
        let mut success = true;
        let mut rollback_count = 0;
        
        for step in steps.iter().rev() {
            match self.rollback_step(step).await {
                Ok(result) => {
                    if result.success {
                        rollback_count += 1;
                        info!("Successfully rolled back step {}", step.step_id);
                    } else {
                        error!("Failed to rollback step {}: {:?}", 
                            step.step_id, result.error);
                        success = false;
                        break;
                    }
                }
                Err(e) => {
                    error!("Error rolling back step {}: {}", step.step_id, e);
                    success = false;
                    break;
                }
            }
        }
        
        // Update status
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(transaction_id) {
            state.status = if success {
                RollbackStatus::Completed
            } else if rollback_count == 0 {
                RollbackStatus::Failed
            } else {
                RollbackStatus::ManualRequired
            };
        }
        
        if success {
            info!("Rollback completed successfully for {}", transaction_id);
        } else {
            error!("Rollback failed for {}, manual intervention may be required", 
                transaction_id);
        }
        
        Ok(())
    }
    
    /// Rollback a single step
    async fn rollback_step(&self, step: &CompletedStep) -> Result<RollbackResult> {
        let handlers = self.handlers.read().await;
        
        // Find appropriate handler
        for (name, handler) in handlers.iter() {
            if handler.can_handle(step) {
                debug!("Using handler {} for step {}", name, step.step_id);
                
                // Attempt rollback with timeout
                let result = tokio::time::timeout(
                    std::time::Duration::from_millis(self.config.timeout_ms),
                    handler.rollback(step)
                ).await;
                
                match result {
                    Ok(Ok(rollback_result)) => {
                        // Verify if requested
                        if rollback_result.success {
                            let verified = handler.verify(step).await?;
                            if !verified {
                                warn!("Rollback verification failed for step {}", 
                                    step.step_id);
                            }
                        }
                        return Ok(rollback_result);
                    }
                    Ok(Err(e)) => {
                        return Err(e);
                    }
                    Err(_) => {
                        return Err(SvmExExError::ProcessingError(
                            "Rollback timeout".to_string()
                        ));
                    }
                }
            }
        }
        
        // No handler found
        Err(SvmExExError::ProcessingError(
            format!("No rollback handler found for step {}", step.step_id)
        ))
    }
    
    /// Get rollback status
    pub async fn get_status(&self, transaction_id: &str) -> Option<RollbackStatus> {
        let states = self.states.read().await;
        states.get(transaction_id).map(|s| s.status)
    }
    
    /// Clean up old rollback states
    pub async fn cleanup_old_states(&self) {
        if !self.config.preserve_history {
            return;
        }
        
        let retention_duration = std::time::Duration::from_secs(
            self.config.history_retention_hours * 3600
        );
        
        let mut states = self.states.write().await;
        let now = std::time::Instant::now();
        
        states.retain(|_, state| {
            let age = now.duration_since(state.created_at);
            age < retention_duration || 
            state.status == RollbackStatus::InProgress ||
            state.status == RollbackStatus::ManualRequired
        });
    }
    
    /// Start cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                std::time::Duration::from_secs(3600)
            ); // Every hour
            
            loop {
                interval.tick().await;
                self.cleanup_old_states().await;
            }
        });
    }
}

/// Example rollback handler for state changes
pub struct StateRollbackHandler;

#[async_trait::async_trait]
impl RollbackHandler for StateRollbackHandler {
    fn can_handle(&self, step: &CompletedStep) -> bool {
        // Can handle any step with state changes
        !step.changes.is_empty()
    }
    
    async fn rollback(&self, step: &CompletedStep) -> Result<RollbackResult> {
        let mut reverted_changes = Vec::new();
        
        // Revert each change
        for change in &step.changes {
            let reverted = match change.change_type {
                ChangeType::Create => {
                    // Delete the created item
                    StateChange {
                        change_type: ChangeType::Delete,
                        key: change.key.clone(),
                        old_value: change.new_value.clone(),
                        new_value: None,
                    }
                }
                ChangeType::Update => {
                    // Restore old value
                    StateChange {
                        change_type: ChangeType::Update,
                        key: change.key.clone(),
                        old_value: change.new_value.clone(),
                        new_value: change.old_value.clone(),
                    }
                }
                ChangeType::Delete => {
                    // Recreate the deleted item
                    StateChange {
                        change_type: ChangeType::Create,
                        key: change.key.clone(),
                        old_value: None,
                        new_value: change.old_value.clone(),
                    }
                }
            };
            
            reverted_changes.push(reverted);
        }
        
        Ok(RollbackResult {
            success: true,
            reverted_changes,
            gas_used: 50000, // Estimated
            error: None,
        })
    }
    
    async fn verify(&self, step: &CompletedStep) -> Result<bool> {
        // In a real implementation, this would check actual state
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rollback_manager_creation() {
        let manager = RollbackManager::new(RollbackConfig::default());
        
        let result = manager.create_rollback_state("tx-1".to_string()).await;
        assert!(result.is_ok());
        
        let status = manager.get_status("tx-1").await;
        assert_eq!(status, Some(RollbackStatus::Pending));
    }
    
    #[tokio::test]
    async fn test_rollback_handler_registration() {
        let manager = RollbackManager::new(RollbackConfig::default());
        
        let handler = Box::new(StateRollbackHandler);
        let result = manager.register_handler("state".to_string(), handler).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_completed_step_addition() {
        let manager = RollbackManager::new(RollbackConfig::default());
        
        manager.create_rollback_state("tx-1".to_string()).await.unwrap();
        
        let step = CompletedStep {
            step_id: "step-1".to_string(),
            tool_id: "tool-1".to_string(),
            pre_state: StateSnapshot {
                id: "snapshot-1".to_string(),
                data: HashMap::new(),
                metadata: HashMap::new(),
            },
            changes: vec![
                StateChange {
                    change_type: ChangeType::Create,
                    key: "key1".to_string(),
                    old_value: None,
                    new_value: Some(b"value1".to_vec()),
                }
            ],
            result: StepResult {
                success: true,
                gas_used: 100000,
                output: None,
            },
            timestamp: std::time::Instant::now(),
        };
        
        let result = manager.add_completed_step("tx-1", step).await;
        assert!(result.is_ok());
    }
}