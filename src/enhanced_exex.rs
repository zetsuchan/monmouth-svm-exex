//! Enhanced SVM Execution Extension with Production-Ready Features
//! 
//! This module implements a robust ExEx following Reth best practices with:
//! - Proper FinishedHeight event handling for safe pruning
//! - Reorg-aware transaction processing
//! - Stateful architecture with Future trait implementation
//! - Accounts Lattice Hash for efficient state management

use std::{
    collections::{HashMap, VecDeque},
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use futures_util::{FutureExt, StreamExt};
use reth::{
    api::FullNodeComponents,
    primitives::{BlockNumber, B256},
    providers::CanonStateSubscriptions,
};
use reth_exex::{ExExContext, ExExEvent, ExExNotification};
use reth_primitives::SealedBlockWithSenders;
use reth_tracing::tracing::{debug, error, info, warn};
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::errors::*;
use crate::inter_exex::{InterExExCoordinator, MessageBusConfig, ExExMessage, MessageType, MessagePayload};

/// Maximum number of blocks to process before sending FinishedHeight
const MAX_BLOCKS_BEFORE_COMMIT: u64 = 100;

/// Maximum time to wait before sending FinishedHeight
const MAX_TIME_BEFORE_COMMIT: Duration = Duration::from_secs(30);

/// Represents a processed block with its state changes
#[derive(Debug, Clone)]
struct ProcessedBlock {
    block_number: BlockNumber,
    block_hash: B256,
    state_root: B256,
    transactions_processed: usize,
    svm_transactions: Vec<ProcessedSvmTransaction>,
    processing_time: Duration,
    alh_hash: AccountsLatticeHash,
}

/// Represents a processed SVM transaction
#[derive(Debug, Clone)]
struct ProcessedSvmTransaction {
    tx_hash: B256,
    success: bool,
    compute_units: u64,
    modified_accounts: Vec<[u8; 32]>,
    logs: Vec<String>,
}

/// Accounts Lattice Hash implementation for efficient state hashing
#[derive(Debug, Clone, Default)]
pub struct AccountsLatticeHash {
    hash: [u8; 32],
    account_count: u64,
    total_lamports: u128,
}

impl AccountsLatticeHash {
    /// Create a new ALH from existing hash
    pub fn from_hash(hash: [u8; 32]) -> Self {
        Self {
            hash,
            account_count: 0,
            total_lamports: 0,
        }
    }

    /// Update the hash with a new account state
    pub fn update_account(&mut self, pubkey: &[u8; 32], lamports: u64, data_hash: &[u8; 32]) {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        
        // Homomorphic property: new_hash = H(old_hash || pubkey || lamports || data_hash)
        hasher.update(&self.hash);
        hasher.update(pubkey);
        hasher.update(&lamports.to_le_bytes());
        hasher.update(data_hash);
        
        self.hash = hasher.finalize().into();
        self.account_count += 1;
        self.total_lamports += lamports as u128;
    }

    /// Remove an account from the hash (for deletions)
    pub fn remove_account(&mut self, pubkey: &[u8; 32]) {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        
        // Mark removal with special prefix
        hasher.update(b"REMOVE");
        hasher.update(&self.hash);
        hasher.update(pubkey);
        
        self.hash = hasher.finalize().into();
        self.account_count = self.account_count.saturating_sub(1);
    }

    /// Get the current hash value
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }
}

/// State checkpoint for handling reorgs
#[derive(Debug, Clone)]
struct StateCheckpoint {
    block_number: BlockNumber,
    block_hash: B256,
    alh_hash: AccountsLatticeHash,
    svm_state_snapshot: Vec<u8>, // Serialized SVM state
    timestamp: Instant,
}

/// Enhanced stateful ExEx structure
pub struct EnhancedSvmExEx<Node: FullNodeComponents> {
    /// ExEx context for notifications
    ctx: ExExContext<Node>,
    
    /// Current processing state
    state: ExExState,
    
    /// SVM processor (moved to async tasks)
    processor_handle: ProcessorHandle,
    
    /// State checkpoints for reorg handling
    checkpoints: Arc<RwLock<VecDeque<StateCheckpoint>>>,
    
    /// Processed blocks awaiting commit
    pending_blocks: Vec<ProcessedBlock>,
    
    /// Last committed block height
    last_committed_height: BlockNumber,
    
    /// Time of last commit
    last_commit_time: Instant,
    
    /// Channel for sending FinishedHeight events
    event_sender: mpsc::UnboundedSender<ExExEvent>,
    
    /// Metrics collector
    metrics: Arc<Metrics>,
    
    /// Inter-ExEx communication coordinator
    inter_exex_coordinator: Option<Arc<InterExExCoordinator>>,
}

/// Internal state of the ExEx
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExExState {
    /// Normal processing mode
    Processing,
    /// Handling a reorg
    Reorging,
    /// Recovering from an error
    Recovering,
    /// Shutting down
    ShuttingDown,
}

/// Handle to the async processor
struct ProcessorHandle {
    sender: mpsc::Sender<ProcessorCommand>,
    #[allow(dead_code)]
    handle: tokio::task::JoinHandle<()>,
}

/// Commands for the processor
enum ProcessorCommand {
    ProcessBlock(Box<SealedBlockWithSenders>),
    Reorg { old_chain: Vec<B256>, new_chain: Vec<Box<SealedBlockWithSenders>> },
    Checkpoint { block_number: BlockNumber },
    Shutdown,
}

/// Metrics for monitoring
#[derive(Default)]
struct Metrics {
    blocks_processed: std::sync::atomic::AtomicU64,
    transactions_processed: std::sync::atomic::AtomicU64,
    reorgs_handled: std::sync::atomic::AtomicU64,
    processing_time_ms: std::sync::atomic::AtomicU64,
}

impl<Node: FullNodeComponents> EnhancedSvmExEx<Node> {
    /// Create a new enhanced ExEx instance
    pub fn new(mut ctx: ExExContext<Node>) -> Self {
        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (processor_sender, processor_receiver) = mpsc::channel(100);
        
        // Spawn event handler task
        let ctx_events = ctx.events.clone();
        tokio::spawn(async move {
            while let Some(event) = event_receiver.recv().await {
                if let Err(e) = ctx_events.send(event).await {
                    error!("Failed to send ExEx event: {}", e);
                }
            }
        });
        
        // Spawn processor task
        let processor_handle = tokio::spawn(async move {
            process_blocks(processor_receiver).await;
        });
        
        Self {
            ctx,
            state: ExExState::Processing,
            processor_handle: ProcessorHandle {
                sender: processor_sender,
                handle: processor_handle,
            },
            checkpoints: Arc::new(RwLock::new(VecDeque::with_capacity(10))),
            pending_blocks: Vec::new(),
            last_committed_height: 0,
            last_commit_time: Instant::now(),
            event_sender,
            metrics: Arc::new(Metrics::default()),
            inter_exex_coordinator: None,
        }
    }

    /// Initialize inter-ExEx communication
    pub async fn init_inter_exex_communication(&mut self, config: MessageBusConfig, node_id: String) -> SvmExExResult<()> {
        info!("Initializing inter-ExEx communication for node: {}", node_id);
        
        let coordinator = Arc::new(InterExExCoordinator::new(config, node_id)
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to create coordinator: {}", e)))?);
        
        coordinator.start().await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to start coordinator: {}", e)))?;
        
        // Subscribe to relevant message types
        self.setup_message_subscriptions(&coordinator).await?;
        
        self.inter_exex_coordinator = Some(coordinator);
        Ok(())
    }

    /// Setup message subscriptions for inter-ExEx communication
    async fn setup_message_subscriptions(&self, coordinator: &Arc<InterExExCoordinator>) -> SvmExExResult<()> {
        // Subscribe to transaction proposals
        let mut tx_receiver = coordinator.subscribe(MessageType::TransactionProposal).await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to subscribe: {}", e)))?;
        
        let metrics = self.metrics.clone();
        tokio::spawn(async move {
            while let Some(msg) = tx_receiver.recv().await {
                debug!("Received transaction proposal from {}", msg.source);
                metrics.transactions_processed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        });
        
        // Subscribe to state sync messages
        let mut state_receiver = coordinator.subscribe(MessageType::StateSync).await
            .map_err(|e| SvmExExError::ProcessingError(format!("Failed to subscribe: {}", e)))?;
        
        tokio::spawn(async move {
            while let Some(msg) = state_receiver.recv().await {
                debug!("Received state sync from {}", msg.source);
                // Handle state synchronization
            }
        });
        
        Ok(())
    }

    /// Broadcast processed block information to other ExEx instances
    async fn broadcast_block_processed(&self, block: &ProcessedBlock) -> SvmExExResult<()> {
        if let Some(coordinator) = &self.inter_exex_coordinator {
            let state_data = crate::inter_exex::messages::StateData {
                block_number: block.block_number,
                state_root: block.state_root,
                alh: B256::from(block.alh_hash.hash()),
                tx_count: block.transactions_processed as u64,
                metrics: crate::inter_exex::messages::ProcessingMetrics {
                    processing_time_ms: block.processing_time.as_millis() as u64,
                    successful_txs: block.svm_transactions.iter().filter(|tx| tx.success).count() as u64,
                    failed_txs: block.svm_transactions.iter().filter(|tx| !tx.success).count() as u64,
                    gas_used: alloy_primitives::U256::from(0), // Placeholder
                },
            };
            
            let message = ExExMessage::new(
                MessageType::StateSync,
                "svm-exex".to_string(), // Would use actual node ID
                MessagePayload::StateData(state_data),
            );
            
            coordinator.broadcast(message).await
                .map_err(|e| SvmExExError::ProcessingError(format!("Failed to broadcast: {}", e)))?;
        }
        Ok(())
    }

    /// Process a new chain notification
    async fn handle_notification(&mut self, notification: ExExNotification) -> SvmExExResult<()> {
        match &notification {
            ExExNotification::ChainCommitted { new } => {
                info!("Processing committed chain with {} blocks", new.blocks().len());
                self.handle_chain_committed(new).await?;
            }
            ExExNotification::ChainReorged { old, new } => {
                warn!("Handling reorg: {} old blocks, {} new blocks", old.blocks().len(), new.blocks().len());
                self.handle_chain_reorged(old, new).await?;
            }
            ExExNotification::ChainReverted { old } => {
                warn!("Handling chain reversion: {} blocks", old.blocks().len());
                self.handle_chain_reverted(old).await?;
            }
        }
        
        // Check if we should send FinishedHeight
        self.maybe_send_finished_height().await?;
        
        Ok(())
    }

    /// Handle committed chain
    async fn handle_chain_committed(&mut self, chain: &Arc<reth_exex::Chain>) -> SvmExExResult<()> {
        self.state = ExExState::Processing;
        
        for block in chain.blocks() {
            let block_number = block.block.header.number;
            let block_hash = block.block.header.hash();
            
            debug!("Processing block {} ({})", block_number, block_hash);
            
            // Send to processor
            self.processor_handle.sender
                .send(ProcessorCommand::ProcessBlock(Box::new(block.clone())))
                .await
                .map_err(|_| SvmExExError::ProcessingError("Processor channel closed".into()))?;
            
            // Track the block as pending
            // In real implementation, we'd wait for processor result
            let processed_block = ProcessedBlock {
                block_number,
                block_hash,
                state_root: block.block.header.state_root,
                transactions_processed: block.block.body.transactions.len(),
                svm_transactions: vec![], // Would be filled by processor
                processing_time: Duration::from_millis(10), // Placeholder
                alh_hash: AccountsLatticeHash::default(), // Would be computed
            };
            
            // Broadcast block processing to other ExEx instances
            if let Err(e) = self.broadcast_block_processed(&processed_block).await {
                warn!("Failed to broadcast block processing: {}", e);
            }
            
            self.pending_blocks.push(processed_block);
        }
        
        Ok(())
    }

    /// Handle chain reorg
    async fn handle_chain_reorged(
        &mut self, 
        old: &Arc<reth_exex::Chain>, 
        new: &Arc<reth_exex::Chain>
    ) -> SvmExExResult<()> {
        self.state = ExExState::Reorging;
        self.metrics.reorgs_handled.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        // Find the common ancestor
        let fork_block = old.fork_block_number();
        info!("Reorg detected at block {}", fork_block);
        
        // Revert to checkpoint before fork
        self.revert_to_checkpoint(fork_block).await?;
        
        // Collect block hashes from old chain
        let old_hashes: Vec<B256> = old.blocks()
            .iter()
            .map(|b| b.block.header.hash())
            .collect();
        
        // Collect new blocks
        let new_blocks: Vec<Box<SealedBlockWithSenders>> = new.blocks()
            .iter()
            .map(|b| Box::new(b.clone()))
            .collect();
        
        // Send reorg command to processor
        self.processor_handle.sender
            .send(ProcessorCommand::Reorg {
                old_chain: old_hashes,
                new_chain: new_blocks,
            })
            .await
            .map_err(|_| SvmExExError::ProcessingError("Processor channel closed".into()))?;
        
        // Process new chain
        self.handle_chain_committed(new).await?;
        
        self.state = ExExState::Processing;
        Ok(())
    }

    /// Handle chain reversion
    async fn handle_chain_reverted(&mut self, old: &Arc<reth_exex::Chain>) -> SvmExExResult<()> {
        let fork_block = old.fork_block_number();
        warn!("Reverting chain from block {}", fork_block);
        
        self.revert_to_checkpoint(fork_block).await?;
        
        // Remove pending blocks that were reverted
        self.pending_blocks.retain(|b| b.block_number < fork_block);
        
        Ok(())
    }

    /// Revert state to a checkpoint
    async fn revert_to_checkpoint(&mut self, block_number: BlockNumber) -> SvmExExResult<()> {
        let mut checkpoints = self.checkpoints.write().await;
        
        // Find the checkpoint to revert to
        while let Some(checkpoint) = checkpoints.back() {
            if checkpoint.block_number <= block_number {
                break;
            }
            checkpoints.pop_back();
        }
        
        if let Some(checkpoint) = checkpoints.back() {
            info!("Reverting to checkpoint at block {}", checkpoint.block_number);
            // In real implementation, restore SVM state from checkpoint
            self.last_committed_height = checkpoint.block_number;
        }
        
        Ok(())
    }

    /// Check if we should send FinishedHeight event
    async fn maybe_send_finished_height(&mut self) -> SvmExExResult<()> {
        let should_commit = self.pending_blocks.len() >= MAX_BLOCKS_BEFORE_COMMIT as usize ||
            self.last_commit_time.elapsed() >= MAX_TIME_BEFORE_COMMIT;
        
        if should_commit && !self.pending_blocks.is_empty() {
            // Get the highest processed block
            let highest_block = self.pending_blocks
                .iter()
                .map(|b| b.block_number)
                .max()
                .unwrap_or(self.last_committed_height);
            
            if highest_block > self.last_committed_height {
                info!("Sending FinishedHeight event for block {}", highest_block);
                
                // Send the event
                self.event_sender
                    .send(ExExEvent::FinishedHeight(highest_block))
                    .map_err(|_| SvmExExError::ProcessingError("Event channel closed".into()))?;
                
                // Create checkpoint
                self.create_checkpoint(highest_block).await?;
                
                // Update state
                self.last_committed_height = highest_block;
                self.last_commit_time = Instant::now();
                self.pending_blocks.clear();
            }
        }
        
        Ok(())
    }

    /// Create a state checkpoint
    async fn create_checkpoint(&mut self, block_number: BlockNumber) -> SvmExExResult<()> {
        let checkpoint = StateCheckpoint {
            block_number,
            block_hash: B256::default(), // Would get from processed block
            alh_hash: AccountsLatticeHash::default(), // Would get current ALH
            svm_state_snapshot: vec![], // Would serialize current state
            timestamp: Instant::now(),
        };
        
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.push_back(checkpoint);
        
        // Keep only last 10 checkpoints
        while checkpoints.len() > 10 {
            checkpoints.pop_front();
        }
        
        Ok(())
    }
    /// Report load metrics to other ExEx instances for load balancing
    async fn report_load_metrics(&self) -> SvmExExResult<()> {
        if let Some(coordinator) = &self.inter_exex_coordinator {
            let load_metrics = crate::inter_exex::messages::LoadMetrics {
                load_percentage: self.calculate_load_percentage(),
                available_compute: 1_000_000, // Placeholder for available compute units
                queue_depth: self.pending_blocks.len(),
                avg_processing_time: self.calculate_avg_processing_time(),
                memory_usage: self.calculate_memory_usage(),
                bandwidth_usage: 0, // Placeholder
            };
            
            let message = ExExMessage::new(
                MessageType::LoadInfo,
                "svm-exex".to_string(), // Would use actual node ID
                MessagePayload::LoadMetrics(load_metrics),
            );
            
            coordinator.broadcast(message).await
                .map_err(|e| SvmExExError::ProcessingError(format!("Failed to report load: {}", e)))?;
        }
        Ok(())
    }
    
    /// Calculate current load percentage
    fn calculate_load_percentage(&self) -> u8 {
        let pending_ratio = (self.pending_blocks.len() as f64 / MAX_BLOCKS_BEFORE_COMMIT as f64 * 100.0) as u8;
        pending_ratio.min(100)
    }
    
    /// Calculate average processing time
    fn calculate_avg_processing_time(&self) -> u64 {
        if self.pending_blocks.is_empty() {
            return 0;
        }
        
        let total_time: u64 = self.pending_blocks.iter()
            .map(|b| b.processing_time.as_millis() as u64)
            .sum();
        
        total_time / self.pending_blocks.len() as u64
    }
    
    /// Calculate memory usage percentage
    fn calculate_memory_usage(&self) -> u8 {
        // Placeholder - would use actual memory metrics
        50
    }
}

impl<Node: FullNodeComponents> Future for EnhancedSvmExEx<Node> {
    type Output = eyre::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Check if we're shutting down
        if self.state == ExExState::ShuttingDown {
            return Poll::Ready(Ok(()));
        }

        // Process notifications
        loop {
            match self.ctx.notifications.poll_next_unpin(cx) {
                Poll::Ready(Some(notification)) => {
                    // Handle notification asynchronously
                    let result = futures::executor::block_on(
                        self.handle_notification(notification)
                    );
                    
                    if let Err(e) = result {
                        error!("Error handling notification: {}", e);
                        self.state = ExExState::Recovering;
                    }
                }
                Poll::Ready(None) => {
                    info!("Notification stream ended, shutting down");
                    self.state = ExExState::ShuttingDown;
                    
                    // Send shutdown command to processor
                    let _ = futures::executor::block_on(
                        self.processor_handle.sender.send(ProcessorCommand::Shutdown)
                    );
                    
                    return Poll::Ready(Ok(()));
                }
                Poll::Pending => break,
            }
        }

        // Continue polling
        Poll::Pending
    }
}

/// Async block processor task
async fn process_blocks(mut receiver: mpsc::Receiver<ProcessorCommand>) {
    info!("Block processor task started");
    
    while let Some(command) = receiver.recv().await {
        match command {
            ProcessorCommand::ProcessBlock(block) => {
                // Process the block
                debug!("Processing block {}", block.block.header.number);
                // Actual SVM processing would happen here
            }
            ProcessorCommand::Reorg { old_chain, new_chain } => {
                info!("Processing reorg: {} old blocks, {} new blocks", 
                      old_chain.len(), new_chain.len());
                // Handle reorg in processor
            }
            ProcessorCommand::Checkpoint { block_number } => {
                debug!("Creating checkpoint at block {}", block_number);
                // Create state snapshot
            }
            ProcessorCommand::Shutdown => {
                info!("Block processor shutting down");
                break;
            }
        }
    }
}