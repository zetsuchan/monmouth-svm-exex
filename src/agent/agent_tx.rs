//! AgentTx decoder for the 7702/agent_tx format

use crate::errors::*;
use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent transaction structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTx {
    /// Agent identity and authorization
    pub agent: AgentIdentity,
    
    /// Intent specification
    pub intent: Intent,
    
    /// Memory context requirements
    pub memory_context: MemoryContext,
    
    /// Cross-chain execution plans
    pub cross_chain_plans: Vec<CrossChainPlan>,
    
    /// Resource requirements
    pub resources: ResourceRequirements,
    
    /// Transaction metadata
    pub metadata: AgentTxMetadata,
    
    /// Signature
    pub signature: Vec<u8>,
}

/// Agent identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentIdentity {
    pub agent_id: String,
    pub public_key: Vec<u8>,
    pub agent_type: AgentType,
    pub capabilities: Vec<String>,
    pub attributes: HashMap<String, String>,
}

/// Agent type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentType {
    Executor,      // Can execute transactions
    Analyzer,      // Can analyze and route
    Orchestrator,  // Can orchestrate multi-step operations
    Validator,     // Can validate results
}

/// Intent specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub intent_id: String,
    pub intent_type: IntentType,
    pub description: String,
    pub parameters: Vec<IntentParameter>,
    pub constraints: Vec<IntentConstraint>,
    pub expiration_timestamp: u64,
}

/// Intent type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentType {
    Transfer,
    Swap,
    Stake,
    Bridge,
    Compute,
    Storage,
    Custom,
}

/// Intent parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentParameter {
    pub name: String,
    pub value: Vec<u8>,
    pub param_type: ParameterType,
    pub required: bool,
}

/// Parameter type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterType {
    Address,
    Amount,
    Token,
    ChainId,
    Bytes,
    String,
}

/// Intent constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentConstraint {
    pub constraint_type: ConstraintType,
    pub value: Vec<u8>,
    pub description: String,
}

/// Constraint type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    MaxGas,
    MaxSlippage,
    MinOutput,
    Deadline,
    AllowedContracts,
}

/// Memory context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryContext {
    pub required_memories: Vec<String>,
    pub memory_root: B256,
    pub requirements: Vec<MemoryRequirement>,
    pub max_memory_age_seconds: u64,
}

/// Memory requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRequirement {
    pub memory_type: MemoryType,
    pub query: String,
    pub min_count: u32,
    pub min_similarity: f32,
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
    Episodic,
    Semantic,
}

/// Cross-chain execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossChainPlan {
    pub chain_id: String,
    pub tools: Vec<ToolExecution>,
    pub target_contract: Address,
    pub calldata: Vec<u8>,
    pub resources: ResourceAllocation,
}

/// Tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_id: String,
    pub inputs: HashMap<String, Vec<u8>>,
    pub required_outputs: Vec<String>,
    pub can_fail: bool,
    pub timeout_ms: u64,
}

/// Resource allocation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub gas_limit: u64,
    pub compute_units: u64,
    pub memory_mb: u64,
    pub storage_kb: u64,
}

/// Resource requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub total_gas_estimate: u64,
    pub total_compute_units: u64,
    pub per_chain_allocation: HashMap<String, ResourceAllocation>,
    pub requires_gpu: bool,
    pub requires_tee: bool,
}

/// Agent transaction metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTxMetadata {
    pub nonce: u64,
    pub timestamp: u64,
    pub correlation_id: String,
    pub tags: Vec<String>,
    pub labels: HashMap<String, String>,
    pub priority: u32,
}

/// AgentTx decoder
pub struct AgentTxDecoder;

impl AgentTxDecoder {
    /// Decode an agent transaction from bytes
    pub fn decode(data: &[u8]) -> Result<AgentTx> {
        // Check minimum size
        if data.len() < 100 {
            return Err(SvmExExError::ProcessingError(
                "Invalid AgentTx data: too short".to_string()
            ));
        }
        
        // Try to decode as bincode first (for compatibility)
        if let Ok(tx) = bincode::deserialize::<AgentTx>(data) {
            return Ok(tx);
        }
        
        // Try JSON decoding
        if let Ok(tx) = serde_json::from_slice::<AgentTx>(data) {
            return Ok(tx);
        }
        
        // Try EIP-7702 format decoding
        Self::decode_eip7702_format(data)
    }
    
    /// Decode EIP-7702 format
    fn decode_eip7702_format(data: &[u8]) -> Result<AgentTx> {
        // EIP-7702 format:
        // [version: 1 byte][agent_id: 32 bytes][intent_type: 1 byte][data_length: 4 bytes][data: variable]
        
        if data.len() < 38 {
            return Err(SvmExExError::ProcessingError(
                "Invalid EIP-7702 format: too short".to_string()
            ));
        }
        
        let version = data[0];
        if version != 1 {
            return Err(SvmExExError::ProcessingError(
                format!("Unsupported AgentTx version: {}", version)
            ));
        }
        
        let agent_id_bytes = &data[1..33];
        let intent_type_byte = data[33];
        let data_length = u32::from_be_bytes([data[34], data[35], data[36], data[37]]) as usize;
        
        if data.len() < 38 + data_length {
            return Err(SvmExExError::ProcessingError(
                "Invalid EIP-7702 format: data length mismatch".to_string()
            ));
        }
        
        let intent_data = &data[38..38 + data_length];
        
        // Parse intent type
        let intent_type = match intent_type_byte {
            0 => IntentType::Transfer,
            1 => IntentType::Swap,
            2 => IntentType::Stake,
            3 => IntentType::Bridge,
            4 => IntentType::Compute,
            5 => IntentType::Storage,
            _ => IntentType::Custom,
        };
        
        // Create basic AgentTx structure
        Ok(AgentTx {
            agent: AgentIdentity {
                agent_id: hex::encode(agent_id_bytes),
                public_key: agent_id_bytes.to_vec(),
                agent_type: AgentType::Executor,
                capabilities: vec!["execute".to_string()],
                attributes: HashMap::new(),
            },
            intent: Intent {
                intent_id: uuid::Uuid::new_v4().to_string(),
                intent_type,
                description: "EIP-7702 agent transaction".to_string(),
                parameters: vec![],
                constraints: vec![],
                expiration_timestamp: 0,
            },
            memory_context: MemoryContext {
                required_memories: vec![],
                memory_root: B256::ZERO,
                requirements: vec![],
                max_memory_age_seconds: 3600,
            },
            cross_chain_plans: vec![],
            resources: ResourceRequirements {
                total_gas_estimate: 100_000,
                total_compute_units: 50_000,
                per_chain_allocation: HashMap::new(),
                requires_gpu: false,
                requires_tee: false,
            },
            metadata: AgentTxMetadata {
                nonce: 0,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                correlation_id: uuid::Uuid::new_v4().to_string(),
                tags: vec!["eip-7702".to_string()],
                labels: HashMap::new(),
                priority: 1,
            },
            signature: data[38 + data_length..].to_vec(),
        })
    }
    
    /// Validate an agent transaction
    pub fn validate(tx: &AgentTx) -> Result<()> {
        // Validate agent identity
        if tx.agent.agent_id.is_empty() {
            return Err(SvmExExError::ValidationError(
                "Invalid agent ID".to_string()
            ));
        }
        
        // Validate intent
        if tx.intent.intent_id.is_empty() {
            return Err(SvmExExError::ValidationError(
                "Invalid intent ID".to_string()
            ));
        }
        
        // Check expiration
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if tx.intent.expiration_timestamp > 0 && tx.intent.expiration_timestamp < now {
            return Err(SvmExExError::ValidationError(
                "Intent has expired".to_string()
            ));
        }
        
        // Validate signature
        if tx.signature.is_empty() {
            return Err(SvmExExError::ValidationError(
                "Missing signature".to_string()
            ));
        }
        
        // TODO: Implement actual signature verification
        
        Ok(())
    }
}

/// Convert AgentTx to transaction data
impl AgentTx {
    /// Convert to standard transaction format
    pub fn to_transaction_data(&self) -> crate::inter_exex::messages::TransactionData {
        let agent_address = Address::from_slice(&self.agent.public_key[..20]);
        
        crate::inter_exex::messages::TransactionData {
            hash: B256::from_slice(&sha2::Sha256::digest(&bincode::serialize(self).unwrap())),
            from: agent_address,
            to: self.cross_chain_plans.first().map(|p| p.target_contract),
            value: U256::ZERO,
            gas_price: U256::from(self.resources.total_gas_estimate),
            data: self.encode_calldata(),
            svm_metadata: Some(crate::inter_exex::messages::SvmMetadata {
                compute_units: self.resources.total_compute_units,
                account_count: self.cross_chain_plans.len(),
                programs: self.cross_chain_plans.iter()
                    .flat_map(|p| p.tools.iter().map(|t| t.tool_id.clone()))
                    .collect(),
                priority: self.metadata.priority as u8,
            }),
        }
    }
    
    /// Encode calldata for execution
    fn encode_calldata(&self) -> Vec<u8> {
        // Simple encoding: just serialize the intent
        bincode::serialize(&self.intent).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_agent_tx_creation() {
        let agent_tx = AgentTx {
            agent: AgentIdentity {
                agent_id: "test-agent".to_string(),
                public_key: vec![0u8; 32],
                agent_type: AgentType::Executor,
                capabilities: vec!["execute".to_string()],
                attributes: HashMap::new(),
            },
            intent: Intent {
                intent_id: "test-intent".to_string(),
                intent_type: IntentType::Transfer,
                description: "Test transfer".to_string(),
                parameters: vec![],
                constraints: vec![],
                expiration_timestamp: 0,
            },
            memory_context: MemoryContext {
                required_memories: vec![],
                memory_root: B256::ZERO,
                requirements: vec![],
                max_memory_age_seconds: 3600,
            },
            cross_chain_plans: vec![],
            resources: ResourceRequirements {
                total_gas_estimate: 100_000,
                total_compute_units: 50_000,
                per_chain_allocation: HashMap::new(),
                requires_gpu: false,
                requires_tee: false,
            },
            metadata: AgentTxMetadata {
                nonce: 0,
                timestamp: 0,
                correlation_id: "test".to_string(),
                tags: vec![],
                labels: HashMap::new(),
                priority: 1,
            },
            signature: vec![0u8; 64],
        };
        
        assert_eq!(agent_tx.agent.agent_id, "test-agent");
        assert_eq!(agent_tx.intent.intent_type, IntentType::Transfer);
    }
    
    #[test]
    fn test_agent_tx_validation() {
        let mut agent_tx = AgentTx {
            agent: AgentIdentity {
                agent_id: "".to_string(), // Invalid
                public_key: vec![0u8; 32],
                agent_type: AgentType::Executor,
                capabilities: vec![],
                attributes: HashMap::new(),
            },
            intent: Intent {
                intent_id: "test".to_string(),
                intent_type: IntentType::Transfer,
                description: "Test".to_string(),
                parameters: vec![],
                constraints: vec![],
                expiration_timestamp: 0,
            },
            memory_context: MemoryContext {
                required_memories: vec![],
                memory_root: B256::ZERO,
                requirements: vec![],
                max_memory_age_seconds: 3600,
            },
            cross_chain_plans: vec![],
            resources: ResourceRequirements {
                total_gas_estimate: 100_000,
                total_compute_units: 50_000,
                per_chain_allocation: HashMap::new(),
                requires_gpu: false,
                requires_tee: false,
            },
            metadata: AgentTxMetadata {
                nonce: 0,
                timestamp: 0,
                correlation_id: "test".to_string(),
                tags: vec![],
                labels: HashMap::new(),
                priority: 1,
            },
            signature: vec![0u8; 64],
        };
        
        // Should fail with empty agent ID
        assert!(AgentTxDecoder::validate(&agent_tx).is_err());
        
        // Fix agent ID
        agent_tx.agent.agent_id = "test-agent".to_string();
        
        // Should pass now
        assert!(AgentTxDecoder::validate(&agent_tx).is_ok());
    }
}