//! Message types for inter-ExEx communication

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use alloy_primitives::{Address, B256, U256};

/// Message types for inter-ExEx communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    /// Node announcement/discovery
    NodeAnnouncement,
    /// Heartbeat/health check
    Heartbeat,
    /// Transaction proposal
    TransactionProposal,
    /// State synchronization
    StateSync,
    /// Load balancing information
    LoadInfo,
    /// Consensus voting
    ConsensusVote,
    /// ALH (Accounts Lattice Hash) update
    ALHUpdate,
    /// Configuration sync
    ConfigSync,
    /// Emergency shutdown
    Shutdown,
    /// Generic data message
    Data,
    /// Agent transaction
    AgentTransaction,
}

/// Main message structure for inter-ExEx communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExExMessage {
    /// Unique message ID
    pub id: String,
    /// Message type
    pub message_type: MessageType,
    /// Source node ID
    pub source: String,
    /// Optional target node ID (None for broadcast)
    pub target: Option<String>,
    /// Message timestamp
    pub timestamp: u64,
    /// Protocol version
    pub version: u8,
    /// Message payload
    pub payload: MessagePayload,
}

/// Different payload types for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagePayload {
    /// Node information for discovery
    NodeInfo(NodeInfo),
    /// Transaction data for proposals
    Transaction(TransactionData),
    /// State synchronization data
    StateData(StateData),
    /// Load balancing metrics
    LoadMetrics(LoadMetrics),
    /// Consensus voting data
    ConsensusData(ConsensusData),
    /// ALH update data
    ALHData(ALHData),
    /// Configuration data
    Config(ConfigData),
    /// Generic data payload
    Data(Vec<u8>),
    /// Empty payload
    Empty,
}

/// Node information for discovery and identification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub node_id: String,
    /// Node type (SVM, RAG, etc.)
    pub node_type: NodeType,
    /// Network address for communication
    pub address: String,
    /// Node capabilities
    pub capabilities: Vec<String>,
    /// Current status
    pub status: NodeStatus,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl NodeInfo {
    pub fn new(node_id: String) -> Self {
        Self {
            node_id,
            node_type: NodeType::SVM,
            address: String::new(),
            capabilities: vec![],
            status: NodeStatus::Starting,
            metadata: HashMap::new(),
        }
    }
}

/// Node types in the ExEx network
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// SVM transaction processor
    SVM,
    /// RAG context provider
    RAG,
    /// Hybrid node with multiple capabilities
    Hybrid,
    /// Observer/monitoring node
    Observer,
}

/// Node operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is starting up
    Starting,
    /// Node is ready and operational
    Ready,
    /// Node is busy/overloaded
    Busy,
    /// Node is in maintenance mode
    Maintenance,
    /// Node is shutting down
    ShuttingDown,
    /// Node has encountered an error
    Error,
}

/// Transaction data for cross-ExEx proposals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    /// Transaction hash
    pub hash: B256,
    /// Sender address
    pub from: Address,
    /// Recipient address (if applicable)
    pub to: Option<Address>,
    /// Transaction value
    pub value: U256,
    /// Gas price/priority fee
    pub gas_price: U256,
    /// Transaction data/input
    pub data: Vec<u8>,
    /// SVM-specific metadata
    pub svm_metadata: Option<SvmMetadata>,
}

/// SVM-specific transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmMetadata {
    /// Compute units required
    pub compute_units: u64,
    /// Number of accounts accessed
    pub account_count: usize,
    /// Program IDs involved
    pub programs: Vec<String>,
    /// Priority level
    pub priority: u8,
}

/// State synchronization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateData {
    /// Block number
    pub block_number: u64,
    /// State root hash
    pub state_root: B256,
    /// ALH (Accounts Lattice Hash)
    pub alh: B256,
    /// Number of transactions processed
    pub tx_count: u64,
    /// Processing metrics
    pub metrics: ProcessingMetrics,
}

/// Processing metrics for state data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingMetrics {
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Number of successful transactions
    pub successful_txs: u64,
    /// Number of failed transactions
    pub failed_txs: u64,
    /// Gas used
    pub gas_used: U256,
}

/// Load balancing metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetrics {
    /// Current load percentage (0-100)
    pub load_percentage: u8,
    /// Available compute units
    pub available_compute: u64,
    /// Queue depth
    pub queue_depth: usize,
    /// Average processing time (ms)
    pub avg_processing_time: u64,
    /// Memory usage percentage
    pub memory_usage: u8,
    /// Network bandwidth usage (bytes/sec)
    pub bandwidth_usage: u64,
}

/// Consensus voting data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusData {
    /// Proposal ID being voted on
    pub proposal_id: String,
    /// Vote type
    pub vote_type: VoteType,
    /// Vote value
    pub vote: bool,
    /// Optional justification
    pub justification: Option<String>,
    /// Signature for vote verification
    pub signature: Vec<u8>,
}

/// Types of consensus votes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    /// Vote on transaction inclusion
    TransactionInclusion,
    /// Vote on state transition
    StateTransition,
    /// Vote on configuration change
    ConfigurationChange,
    /// Vote on node admission/removal
    NodeMembership,
}

/// ALH (Accounts Lattice Hash) update data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ALHData {
    /// Block number
    pub block_number: u64,
    /// Previous ALH
    pub prev_alh: B256,
    /// New ALH
    pub new_alh: B256,
    /// Account updates in this block
    pub account_updates: Vec<AccountUpdate>,
    /// Proof data for verification
    pub proof: Vec<u8>,
}

/// Individual account update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    /// Account address
    pub address: Address,
    /// Previous state hash
    pub prev_hash: B256,
    /// New state hash
    pub new_hash: B256,
    /// Update type
    pub update_type: UpdateType,
}

/// Types of account updates
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateType {
    /// Balance change
    Balance,
    /// Code deployment
    CodeDeploy,
    /// Storage update
    Storage,
    /// Account creation
    Creation,
    /// Account deletion
    Deletion,
}

/// Configuration synchronization data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigData {
    /// Configuration version
    pub version: u32,
    /// Configuration type
    pub config_type: ConfigType,
    /// Configuration values
    pub values: HashMap<String, serde_json::Value>,
    /// Timestamp of configuration
    pub timestamp: u64,
}

/// Types of configuration data
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigType {
    /// AI model parameters
    AIParameters,
    /// Network settings
    NetworkSettings,
    /// Processing limits
    ProcessingLimits,
    /// Security policies
    SecurityPolicies,
    /// General settings
    General,
}

impl ExExMessage {
    /// Create a new message
    pub fn new(
        message_type: MessageType,
        source: String,
        payload: MessagePayload,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            message_type,
            source,
            target: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            version: 1,
            payload,
        }
    }

    /// Create a targeted message
    pub fn new_targeted(
        message_type: MessageType,
        source: String,
        target: String,
        payload: MessagePayload,
    ) -> Self {
        let mut msg = Self::new(message_type, source, payload);
        msg.target = Some(target);
        msg
    }

    /// Check if message is a broadcast
    pub fn is_broadcast(&self) -> bool {
        self.target.is_none()
    }

    /// Validate message integrity
    pub fn validate(&self) -> bool {
        // Basic validation
        !self.id.is_empty() && !self.source.is_empty() && self.version > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = ExExMessage::new(
            MessageType::Heartbeat,
            "node1".to_string(),
            MessagePayload::Empty,
        );
        
        assert!(msg.validate());
        assert!(msg.is_broadcast());
        assert_eq!(msg.message_type, MessageType::Heartbeat);
    }

    #[test]
    fn test_targeted_message() {
        let msg = ExExMessage::new_targeted(
            MessageType::Data,
            "node1".to_string(),
            "node2".to_string(),
            MessagePayload::Data(vec![1, 2, 3]),
        );
        
        assert!(!msg.is_broadcast());
        assert_eq!(msg.target, Some("node2".to_string()));
    }
}