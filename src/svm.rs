//! Solana Virtual Machine (SVM) Integration Module
//!
//! This module provides the core SVM processor that executes Solana transactions
//! within the reth execution environment.

use crate::{errors::*, utils};
use alloy_primitives::{Address, Bytes, B256};
use async_trait::async_trait;
use blake3::Hasher as Blake3Hasher;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "solana")]
use std::collections::HashMap;
use tokio::sync::RwLock;

#[cfg(feature = "solana")]
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
#[cfg(feature = "solana")]
use base64::Engine;

#[cfg(feature = "solana")]
use solana_sdk::{
    account::AccountSharedData, pubkey::Pubkey, transaction::Transaction as SolanaTransaction,
    transaction::VersionedTransaction,
};

#[cfg(feature = "solana")]
use solana_runtime::{bank::Bank, genesis_utils::create_genesis_config};

/// Trait for SVM processor implementations
#[async_trait]
pub trait SvmProcessor: Send + Sync {
    /// Process a transaction through the SVM
    async fn process_transaction(&self, tx_data: &[u8]) -> Result<SvmExecutionResult>;

    /// Get the current state root
    async fn get_state_root(&self) -> Result<B256>;

    /// Commit a batch of transactions
    async fn commit_batch(&self, batch: Vec<ProcessedTransaction>) -> Result<()>;

    /// Handle reorg by reverting to a checkpoint
    async fn revert_to_checkpoint(&self, checkpoint_id: u64) -> Result<()>;
}

/// Result of SVM transaction execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmExecutionResult {
    /// Transaction hash
    pub tx_hash: B256,
    /// Success status
    pub success: bool,
    /// Gas used (converted from Solana compute units)
    pub gas_used: u64,
    /// Return data
    pub return_data: Vec<u8>,
    /// Logs generated
    pub logs: Vec<String>,
    /// State changes
    pub state_changes: Vec<StateChange>,
}

/// Represents a state change in the SVM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChange {
    /// Account address
    pub address: Address,
    /// Previous balance
    pub prev_balance: u64,
    /// New balance
    pub new_balance: u64,
    /// Data changes
    pub data_changes: Option<DataChange>,
}

/// Data changes for an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataChange {
    /// Previous data hash
    pub prev_hash: B256,
    /// New data hash
    pub new_hash: B256,
    /// Size delta
    pub size_delta: i64,
}

/// A processed transaction ready for commitment
#[derive(Debug, Clone)]
pub struct ProcessedTransaction {
    /// Original EVM transaction hash
    pub evm_tx_hash: B256,
    /// SVM execution result
    pub svm_result: SvmExecutionResult,
    /// Block number
    pub block_number: u64,
}

/// Concrete SVM processor implementation
pub struct SvmProcessorImpl {
    #[cfg(feature = "solana")]
    bank: Arc<RwLock<Bank>>,
    /// State checkpoints for reorg handling
    checkpoints: Arc<RwLock<Vec<StateCheckpoint>>>,
    /// Current state root snapshot
    state_root: Arc<RwLock<B256>>,
    /// Configuration
    config: SvmConfig,
}

/// SVM processor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmConfig {
    /// Maximum compute units per transaction
    pub max_compute_units: u64,
    /// Enable parallel execution
    pub parallel_execution: bool,
    /// Checkpoint interval (blocks)
    pub checkpoint_interval: u64,
}

impl Default for SvmConfig {
    fn default() -> Self {
        Self {
            max_compute_units: 1_400_000,
            parallel_execution: true,
            checkpoint_interval: 100,
        }
    }
}

/// State checkpoint for reorg handling
#[derive(Debug, Clone)]
struct StateCheckpoint {
    /// Checkpoint ID (block number)
    pub id: u64,
    /// State root at this checkpoint
    pub state_root: B256,
    /// Timestamp
    pub timestamp: u64,
}

impl SvmProcessorImpl {
    /// Create a new SVM processor
    pub fn new(config: SvmConfig) -> Self {
        #[cfg(feature = "solana")]
        {
            let (genesis_config, _) = create_genesis_config(1_000_000);
            let bank = Bank::new_for_tests(&genesis_config);
            let initial_hash = {
                let mut hasher = Blake3Hasher::new();
                hasher.update(bank.hash().as_ref());
                B256::from_slice(hasher.finalize().as_bytes())
            };

            Self {
                bank: Arc::new(RwLock::new(bank)),
                checkpoints: Arc::new(RwLock::new(Vec::new())),
                state_root: Arc::new(RwLock::new(initial_hash)),
                config,
            }
        }

        #[cfg(not(feature = "solana"))]
        {
            Self {
                checkpoints: Arc::new(RwLock::new(Vec::new())),
                state_root: Arc::new(RwLock::new(B256::ZERO)),
                config,
            }
        }
    }

    /// Parse EVM calldata into Solana transaction format
    #[cfg(feature = "solana")]
    async fn parse_svm_transaction(&self, tx_data: &[u8]) -> Result<SolanaTransaction> {
        if tx_data.is_empty() {
            return Err(SvmExExError::SvmExecutionError(
                "Empty transaction payload".to_string(),
            ));
        }

        // Try direct binary decoding first (borsh/bincode)
        if let Ok(tx) = Self::decode_transaction_bytes(tx_data) {
            return Ok(tx);
        }

        // Fallback for textual encodings: base64 or JSON wrappers
        if let Ok(text) = std::str::from_utf8(tx_data) {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Err(SvmExExError::SvmExecutionError(
                    "Transaction payload only contained whitespace".to_string(),
                ));
            }

            // JSON wrapper { "transaction": "<base64>" }
            if trimmed.starts_with('{') {
                #[derive(Deserialize)]
                struct Payload<'a> {
                    #[serde(borrow)]
                    transaction: &'a str,
                }

                let payload: Payload = serde_json::from_str(trimmed).map_err(|e| {
                    SvmExExError::SvmExecutionError(format!(
                        "Failed to parse transaction JSON wrapper: {}",
                        e
                    ))
                })?;

                let raw = BASE64_STANDARD
                    .decode(payload.transaction.trim().as_bytes())
                    .map_err(|e| {
                        SvmExExError::SvmExecutionError(format!(
                            "Failed to decode base64 transaction: {}",
                            e
                        ))
                    })?;

                return Self::decode_transaction_bytes(&raw);
            }

            // Plain base64 string
            if let Ok(raw) = BASE64_STANDARD.decode(trimmed.as_bytes()) {
                return Self::decode_transaction_bytes(&raw);
            }
        }

        Err(SvmExExError::SvmExecutionError(
            "Unsupported SVM transaction encoding".to_string(),
        ))
    }

    /// Execute a Solana transaction
    #[cfg(feature = "solana")]
    async fn execute_solana_transaction(
        &self,
        tx: &SolanaTransaction,
    ) -> Result<SvmExecutionResult> {
        let mut bank = self.bank.write().await;

        let mut pre_accounts: HashMap<Pubkey, AccountSharedData> = HashMap::new();
        for pubkey in tx.message.account_keys.iter() {
            if let Some(account) = bank.get_account(pubkey) {
                pre_accounts.insert(*pubkey, account);
            }
        }

        let tx_hash = {
            let mut hasher = Blake3Hasher::new();
            if let Some(signature) = tx.signatures.get(0) {
                hasher.update(signature.as_ref());
            }
            hasher.update(&tx.message_data());
            B256::from_slice(hasher.finalize().as_bytes())
        };

        let estimated_units = tx
            .message
            .instructions
            .iter()
            .fold(0u64, |acc, ix| {
                acc.saturating_add((ix.accounts.len() as u64).saturating_mul(25))
                    .saturating_add(ix.data.len() as u64 * 10)
            })
            .min(self.config.max_compute_units);

        bank.process_transaction(tx).map_err(|e| {
            SvmExExError::SvmExecutionError(format!("Solana execution error: {}", e))
        })?;

        let mut state_changes = Vec::new();

        for pubkey in tx.message.account_keys.iter() {
            let before = pre_accounts.get(pubkey);
            let after = bank.get_account(pubkey);

            let prev_balance = before.map(|a| a.lamports()).unwrap_or(0);
            let new_balance = after.as_ref().map(|a| a.lamports()).unwrap_or(0);

            let data_delta = match (before, after) {
                (Some(prev), Some(next)) => {
                    let prev_hash = Self::hash_account_data(prev);
                    let new_hash = Self::hash_account_data(next);
                    if prev_hash != new_hash {
                        Some(DataChange {
                            prev_hash,
                            new_hash,
                            size_delta: next.data().len() as i64 - prev.data().len() as i64,
                        })
                    } else {
                        None
                    }
                }
                (None, Some(next)) => Some(DataChange {
                    prev_hash: B256::ZERO,
                    new_hash: Self::hash_account_data(next),
                    size_delta: next.data().len() as i64,
                }),
                (Some(prev), None) => Some(DataChange {
                    prev_hash: Self::hash_account_data(prev),
                    new_hash: B256::ZERO,
                    size_delta: -(prev.data().len() as i64),
                }),
                (None, None) => None,
            };

            if prev_balance != new_balance || data_delta.is_some() {
                state_changes.push(StateChange {
                    address: Self::pubkey_to_address(pubkey),
                    prev_balance,
                    new_balance,
                    data_changes: data_delta,
                });
            }
        }

        let logs = vec![format!(
            "Processed {} instructions across {} accounts",
            tx.message.instructions.len(),
            tx.message.account_keys.len()
        )];

        Ok(SvmExecutionResult {
            tx_hash,
            success: true,
            gas_used: estimated_units,
            return_data: vec![],
            logs,
            state_changes,
        })
    }

    #[cfg(feature = "solana")]
    fn decode_transaction_bytes(bytes: &[u8]) -> Result<SolanaTransaction> {
        use std::convert::TryFrom;

        if let Ok(tx) = SolanaTransaction::try_from_slice(bytes) {
            return Ok(tx);
        }

        if let Ok(versioned) = bincode::deserialize::<VersionedTransaction>(bytes) {
            return SolanaTransaction::try_from(versioned).map_err(|e| {
                SvmExExError::SvmExecutionError(format!(
                    "Failed to convert versioned transaction: {}",
                    e
                ))
            });
        }

        Err(SvmExExError::SvmExecutionError(
            "Unsupported Solana transaction encoding".to_string(),
        ))
    }

    #[cfg(feature = "solana")]
    fn pubkey_to_address(pubkey: &Pubkey) -> Address {
        let bytes = pubkey.to_bytes();
        Address::from_slice(&bytes[12..32])
    }

    #[cfg(feature = "solana")]
    fn hash_account_data(account: &AccountSharedData) -> B256 {
        let mut hasher = Blake3Hasher::new();
        hasher.update(account.data());
        hasher.update(account.owner().as_ref());
        hasher.update(&account.lamports().to_le_bytes());
        hasher.update(&[account.executable() as u8]);
        B256::from_slice(hasher.finalize().as_bytes())
    }

    async fn record_checkpoint_if_needed(&self, block_number: u64, state_root: B256) {
        if block_number == 0 {
            return;
        }

        let mut checkpoints = self.checkpoints.write().await;

        if let Some(last) = checkpoints.last_mut() {
            if last.id == block_number {
                last.state_root = state_root;
                last.timestamp = utils::current_timestamp_ms();
                return;
            }
        }

        let should_record = checkpoints.is_empty()
            || block_number.saturating_sub(checkpoints.last().map(|cp| cp.id).unwrap_or(0))
                >= self.config.checkpoint_interval;

        if should_record {
            checkpoints.push(StateCheckpoint {
                id: block_number,
                state_root,
                timestamp: utils::current_timestamp_ms(),
            });

            while checkpoints.len() > 64 {
                checkpoints.remove(0);
            }
        }
    }
}

#[async_trait]
impl SvmProcessor for SvmProcessorImpl {
    async fn process_transaction(&self, tx_data: &[u8]) -> Result<SvmExecutionResult> {
        #[cfg(feature = "solana")]
        {
            // Parse transaction
            let tx = self.parse_svm_transaction(tx_data).await?;

            // Execute
            self.execute_solana_transaction(&tx).await
        }

        #[cfg(not(feature = "solana"))]
        {
            // Return mock result when Solana is disabled
            Ok(SvmExecutionResult {
                tx_hash: B256::default(),
                success: true,
                gas_used: 5000,
                return_data: vec![],
                logs: vec!["Mock SVM execution (Solana feature disabled)".to_string()],
                state_changes: vec![],
            })
        }
    }

    async fn get_state_root(&self) -> Result<B256> {
        Ok(*self.state_root.read().await)
    }

    async fn commit_batch(&self, batch: Vec<ProcessedTransaction>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        let current_root = *self.state_root.read().await;
        let mut hasher = Blake3Hasher::new();
        hasher.update(current_root.as_slice());

        let mut latest_block = 0u64;

        for tx in &batch {
            latest_block = latest_block.max(tx.block_number);
            hasher.update(tx.evm_tx_hash.as_slice());
            hasher.update(tx.svm_result.tx_hash.as_slice());
            hasher.update(&tx.svm_result.gas_used.to_le_bytes());
            hasher.update(&[tx.svm_result.success as u8]);

            for log in &tx.svm_result.logs {
                hasher.update(log.as_bytes());
            }

            for change in &tx.svm_result.state_changes {
                hasher.update(change.address.as_slice());
                hasher.update(&change.prev_balance.to_le_bytes());
                hasher.update(&change.new_balance.to_le_bytes());

                if let Some(data) = &change.data_changes {
                    hasher.update(data.prev_hash.as_slice());
                    hasher.update(data.new_hash.as_slice());
                    hasher.update(&data.size_delta.to_le_bytes());
                }
            }
        }

        hasher.update(&(batch.len() as u64).to_le_bytes());

        let new_root = B256::from_slice(hasher.finalize().as_bytes());
        *self.state_root.write().await = new_root;

        self.record_checkpoint_if_needed(latest_block, new_root)
            .await;

        Ok(())
    }

    async fn revert_to_checkpoint(&self, checkpoint_id: u64) -> Result<()> {
        let mut checkpoints = self.checkpoints.write().await;

        // Find and revert to checkpoint
        if let Some(pos) = checkpoints.iter().position(|cp| cp.id == checkpoint_id) {
            checkpoints.truncate(pos + 1);
            if let Some(checkpoint) = checkpoints.last() {
                *self.state_root.write().await = checkpoint.state_root;
            }
            tracing::info!("Reverted to checkpoint: {}", checkpoint_id);
            Ok(())
        } else {
            Err(SvmExExError::CheckpointNotFound(checkpoint_id))
        }
    }
}

/// Create a default SVM processor
pub fn create_svm_processor() -> Arc<dyn SvmProcessor> {
    Arc::new(SvmProcessorImpl::new(SvmConfig::default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_state_change(seed: u8) -> StateChange {
        StateChange {
            address: Address::from_slice(&[seed; 20]),
            prev_balance: seed as u64,
            new_balance: seed as u64 + 100,
            data_changes: Some(DataChange {
                prev_hash: B256::from([seed; 32]),
                new_hash: B256::from([seed.wrapping_add(1); 32]),
                size_delta: 16,
            }),
        }
    }

    fn processed_tx(block: u64, seed: u8) -> ProcessedTransaction {
        ProcessedTransaction {
            evm_tx_hash: B256::from_low_u64_be(block + 1),
            svm_result: SvmExecutionResult {
                tx_hash: B256::from_low_u64_be(block + 10),
                success: true,
                gas_used: 1_000 + seed as u64,
                return_data: vec![],
                logs: vec![format!("log-{}", seed)],
                state_changes: vec![sample_state_change(seed)],
            },
            block_number: block,
        }
    }

    #[tokio::test]
    async fn commit_batch_updates_root() {
        let processor = SvmProcessorImpl::new(SvmConfig {
            checkpoint_interval: 5,
            ..SvmConfig::default()
        });

        let initial = processor.get_state_root().await.unwrap();
        processor
            .commit_batch(vec![processed_tx(1, 1), processed_tx(2, 2)])
            .await
            .unwrap();
        let updated = processor.get_state_root().await.unwrap();
        assert_ne!(initial, updated);
    }

    #[tokio::test]
    async fn revert_restores_previous_root() {
        let processor = SvmProcessorImpl::new(SvmConfig {
            checkpoint_interval: 1,
            ..SvmConfig::default()
        });

        processor
            .commit_batch(vec![processed_tx(10, 3)])
            .await
            .unwrap();
        let checkpoint_root = processor.get_state_root().await.unwrap();

        processor
            .commit_batch(vec![processed_tx(11, 4)])
            .await
            .unwrap();
        let post_root = processor.get_state_root().await.unwrap();
        assert_ne!(checkpoint_root, post_root);

        processor.revert_to_checkpoint(10).await.unwrap();
        let reverted = processor.get_state_root().await.unwrap();
        assert_eq!(checkpoint_root, reverted);
    }
}
