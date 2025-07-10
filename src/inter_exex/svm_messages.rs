//! SVM-specific inter-ExEx message types
//!
//! This module defines messages for communication between the SVM ExEx
//! and the RAG Memory ExEx.

use crate::errors::*;
use crate::svm::{SvmExecutionResult, StateChange};
use crate::ai::traits::{RoutingDecision, DecisionType, ContextData};
use alloy_primitives::{Address, B256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// SVM-specific messages for inter-ExEx communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SvmExExMessage {
    /// Transaction forwarded for SVM processing
    TransactionForProcessing {
        /// EVM transaction hash
        tx_hash: B256,
        /// Transaction data (calldata)
        tx_data: Vec<u8>,
        /// AI routing decision with confidence
        routing_hint: RoutingDecision,
        /// Sender address
        sender: Address,
        /// Additional metadata
        metadata: TransactionMetadata,
    },
    
    /// Request RAG context for a transaction
    RequestRagContext {
        /// Transaction hash
        tx_hash: B256,
        /// Agent ID for agent-specific context
        agent_id: String,
        /// Query for context retrieval
        query: String,
        /// Context type hint
        context_type: ContextType,
    },
    
    /// RAG context response
    RagContextResponse {
        /// Original transaction hash
        tx_hash: B256,
        /// Retrieved contexts
        contexts: Vec<ContextData>,
        /// Confidence score
        confidence: f64,
        /// Processing time
        latency_ms: u64,
    },
    
    /// SVM execution completed
    SvmExecutionCompleted {
        /// Transaction hash
        tx_hash: B256,
        /// Execution result
        result: SvmExecutionResult,
        /// State changes
        state_changes: Vec<StateChange>,
        /// Gas used (converted from compute units)
        gas_used: u64,
        /// Execution time
        execution_time_ms: u64,
    },
    
    /// State synchronization message
    StateSync {
        /// Block height
        block_height: u64,
        /// Accounts Lattice Hash
        alh_hash: [u8; 32],
        /// Account updates
        account_updates: Vec<AccountUpdate>,
        /// Timestamp
        timestamp: u64,
    },
    
    /// Request memory storage
    StoreMemory {
        /// Agent ID
        agent_id: String,
        /// Memory items to store
        memories: Vec<MemoryItem>,
        /// Priority
        priority: MemoryPriority,
    },
    
    /// Request memory retrieval
    RetrieveMemory {
        /// Agent ID
        agent_id: String,
        /// Query for memory retrieval
        query: MemoryQuery,
        /// Maximum results
        limit: usize,
    },
    
    /// Memory operation result
    MemoryOperationResult {
        /// Operation ID
        operation_id: String,
        /// Success status
        success: bool,
        /// Retrieved memories (if applicable)
        memories: Option<Vec<MemoryItem>>,
        /// Error message (if failed)
        error: Option<String>,
    },
    
    /// Health check request
    HealthCheck {
        /// Requester ID
        requester: String,
        /// Component to check
        component: HealthCheckComponent,
    },
    
    /// Health check response
    HealthCheckResponse {
        /// Component checked
        component: HealthCheckComponent,
        /// Health status
        healthy: bool,
        /// Latency
        latency_ms: u64,
        /// Additional details
        details: HashMap<String, String>,
    },
}

/// Transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetadata {
    /// Block number
    pub block_number: u64,
    /// Transaction index
    pub tx_index: u64,
    /// Gas price
    pub gas_price: u64,
    /// Nonce
    pub nonce: u64,
    /// Custom tags
    pub tags: Vec<String>,
}

/// Context type for RAG queries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    /// Historical transactions
    Historical,
    /// Similar patterns
    Similar,
    /// Agent-specific context
    AgentSpecific,
    /// Security analysis
    Security,
    /// Performance optimization
    Performance,
}

/// Account update for state sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    /// Account address
    pub address: Address,
    /// New balance
    pub balance: u64,
    /// Data hash (if changed)
    pub data_hash: Option<B256>,
    /// Owner (if changed)
    pub owner: Option<Address>,
    /// Update type
    pub update_type: AccountUpdateType,
}

/// Type of account update
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountUpdateType {
    /// New account created
    Created,
    /// Balance changed
    BalanceUpdate,
    /// Data modified
    DataUpdate,
    /// Account deleted
    Deleted,
}

/// Memory item for storage/retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique ID
    pub id: String,
    /// Content
    pub content: String,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: u64,
    /// Memory type
    pub memory_type: MemoryType,
}

/// Memory type classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    /// Short-term working memory
    ShortTerm,
    /// Long-term storage
    LongTerm,
    /// Episode/event sequence
    Episodic,
    /// Concept/knowledge
    Semantic,
}

/// Memory priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPriority {
    /// Immediate storage
    High,
    /// Normal priority
    Medium,
    /// Background storage
    Low,
}

/// Memory query parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryQuery {
    /// Query text
    pub query: String,
    /// Memory types to search
    pub memory_types: Vec<MemoryType>,
    /// Time range (optional)
    pub time_range: Option<(u64, u64)>,
    /// Minimum similarity score
    pub min_similarity: f32,
}

/// Health check component
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthCheckComponent {
    /// SVM processor
    SvmProcessor,
    /// RAG system
    RagSystem,
    /// Memory store
    MemoryStore,
    /// Message bus
    MessageBus,
    /// Overall system
    System,
}

/// Message handlers for SVM ExEx
#[async_trait::async_trait]
pub trait SvmMessageHandler: Send + Sync {
    /// Handle incoming SVM message
    async fn handle_svm_message(&self, message: SvmExExMessage) -> Result<()>;
    
    /// Send message to RAG Memory ExEx
    async fn send_to_rag(&self, message: SvmExExMessage) -> Result<()>;
}

/// Convert from generic ExEx message to SVM-specific
impl TryFrom<super::ExExMessage> for SvmExExMessage {
    type Error = SvmExExError;
    
    fn try_from(msg: super::ExExMessage) -> Result<Self> {
        match &msg.payload {
            super::MessagePayload::Data(data) => {
                serde_json::from_slice(data)
                    .map_err(|e| SvmExExError::Serialization(e.into()))
            }
            _ => Err(SvmExExError::ProcessingError(
                "Not a data message".to_string()
            )),
        }
    }
}

/// Convert SVM-specific message to generic ExEx message
impl From<SvmExExMessage> for super::ExExMessage {
    fn from(svm_msg: SvmExExMessage) -> Self {
        let data = serde_json::to_vec(&svm_msg)
            .expect("Failed to serialize SVM message");
        
        let msg_type = match &svm_msg {
            SvmExExMessage::TransactionForProcessing { .. } => super::MessageType::TransactionProposal,
            SvmExExMessage::StateSync { .. } => super::MessageType::StateSync,
            _ => super::MessageType::Data,
        };
        
        super::ExExMessage {
            id: uuid::Uuid::new_v4().to_string(),
            message_type: msg_type,
            source: "svm-exex".to_string(),
            target: Some("rag-memory-exex".to_string()),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: 1,
            payload: super::MessagePayload::Data(data),
        }
    }
}