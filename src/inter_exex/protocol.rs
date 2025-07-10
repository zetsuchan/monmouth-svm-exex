//! Protocol implementation for inter-ExEx communication

use super::messages::{ExExMessage, MessageType};
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Protocol version for inter-ExEx communication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProtocolVersion {
    /// Initial protocol version
    V1,
    /// Future versions can be added here
}

impl ProtocolVersion {
    /// Get the numeric version
    pub fn as_u8(&self) -> u8 {
        match self {
            ProtocolVersion::V1 => 1,
        }
    }

    /// Create from numeric version
    pub fn from_u8(version: u8) -> Option<Self> {
        match version {
            1 => Some(ProtocolVersion::V1),
            _ => None,
        }
    }
}

/// Protocol handler for inter-ExEx communication
pub struct Protocol {
    /// Current protocol version
    version: ProtocolVersion,
    /// Message handlers by type
    handlers: HashMap<MessageType, Box<dyn MessageHandler>>,
    /// Protocol extensions
    extensions: HashMap<String, Box<dyn ProtocolExtension>>,
}

/// Trait for message handlers
pub trait MessageHandler: Send + Sync {
    /// Handle a message
    fn handle(&self, message: &ExExMessage) -> Result<Option<ExExMessage>>;
    
    /// Validate a message
    fn validate(&self, message: &ExExMessage) -> Result<()>;
}

/// Trait for protocol extensions
pub trait ProtocolExtension: Send + Sync {
    /// Extension name
    fn name(&self) -> &str;
    
    /// Extension version
    fn version(&self) -> &str;
    
    /// Initialize the extension
    fn initialize(&mut self) -> Result<()>;
    
    /// Process a message through the extension
    fn process(&self, message: &mut ExExMessage) -> Result<()>;
}

impl Protocol {
    /// Create a new protocol handler
    pub fn new(version: ProtocolVersion) -> Self {
        let mut protocol = Self {
            version,
            handlers: HashMap::new(),
            extensions: HashMap::new(),
        };
        
        // Register default handlers
        protocol.register_default_handlers();
        
        protocol
    }

    /// Register default message handlers
    fn register_default_handlers(&mut self) {
        // Node announcement handler
        self.handlers.insert(
            MessageType::NodeAnnouncement,
            Box::new(NodeAnnouncementHandler),
        );
        
        // Heartbeat handler
        self.handlers.insert(
            MessageType::Heartbeat,
            Box::new(HeartbeatHandler),
        );
        
        // State sync handler
        self.handlers.insert(
            MessageType::StateSync,
            Box::new(StateSyncHandler),
        );
        
        // Transaction proposal handler
        self.handlers.insert(
            MessageType::TransactionProposal,
            Box::new(TransactionProposalHandler),
        );
    }

    /// Get current protocol version
    pub fn version(&self) -> ProtocolVersion {
        self.version
    }

    /// Register a custom message handler
    pub fn register_handler(&mut self, message_type: MessageType, handler: Box<dyn MessageHandler>) {
        self.handlers.insert(message_type, handler);
    }

    /// Register a protocol extension
    pub fn register_extension(&mut self, extension: Box<dyn ProtocolExtension>) -> Result<()> {
        let name = extension.name().to_string();
        self.extensions.insert(name, extension);
        Ok(())
    }

    /// Process an incoming message
    pub async fn process_message(&self, mut message: ExExMessage) -> Result<Option<ExExMessage>> {
        // Check protocol version compatibility
        if ProtocolVersion::from_u8(message.version).is_none() {
            return Err(eyre!("Unsupported protocol version: {}", message.version));
        }

        // Apply protocol extensions
        for extension in self.extensions.values() {
            extension.process(&mut message)?;
        }

        // Find and execute handler
        if let Some(handler) = self.handlers.get(&message.message_type) {
            handler.validate(&message)?;
            handler.handle(&message)
        } else {
            // No specific handler, use default processing
            Ok(None)
        }
    }

    /// Encode a message for transmission
    pub fn encode_message(&self, message: &ExExMessage) -> Result<Vec<u8>> {
        // Add protocol-specific encoding here
        bincode::serialize(message).map_err(|e| eyre!("Failed to encode message: {}", e))
    }

    /// Decode a message from transmission format
    pub fn decode_message(&self, data: &[u8]) -> Result<ExExMessage> {
        // Add protocol-specific decoding here
        bincode::deserialize(data).map_err(|e| eyre!("Failed to decode message: {}", e))
    }
}

/// Handler for node announcement messages
struct NodeAnnouncementHandler;

impl MessageHandler for NodeAnnouncementHandler {
    fn handle(&self, message: &ExExMessage) -> Result<Option<ExExMessage>> {
        // Process node announcement
        match &message.payload {
            super::messages::MessagePayload::NodeInfo(info) => {
                tracing::info!("Node announcement from {}: {:?}", message.source, info);
                // Could return an acknowledgment message
                Ok(None)
            }
            _ => Err(eyre!("Invalid payload for node announcement")),
        }
    }

    fn validate(&self, message: &ExExMessage) -> Result<()> {
        match &message.payload {
            super::messages::MessagePayload::NodeInfo(info) => {
                if info.node_id.is_empty() {
                    return Err(eyre!("Empty node ID"));
                }
                Ok(())
            }
            _ => Err(eyre!("Invalid payload type for node announcement")),
        }
    }
}

/// Handler for heartbeat messages
struct HeartbeatHandler;

impl MessageHandler for HeartbeatHandler {
    fn handle(&self, message: &ExExMessage) -> Result<Option<ExExMessage>> {
        tracing::debug!("Heartbeat from {}", message.source);
        // Heartbeats don't require a response
        Ok(None)
    }

    fn validate(&self, message: &ExExMessage) -> Result<()> {
        match &message.payload {
            super::messages::MessagePayload::Empty => Ok(()),
            _ => Err(eyre!("Heartbeat should have empty payload")),
        }
    }
}

/// Handler for state synchronization messages
struct StateSyncHandler;

impl MessageHandler for StateSyncHandler {
    fn handle(&self, message: &ExExMessage) -> Result<Option<ExExMessage>> {
        match &message.payload {
            super::messages::MessagePayload::StateData(state) => {
                tracing::debug!(
                    "State sync from {} at block {}",
                    message.source,
                    state.block_number
                );
                // Process state synchronization
                Ok(None)
            }
            _ => Err(eyre!("Invalid payload for state sync")),
        }
    }

    fn validate(&self, message: &ExExMessage) -> Result<()> {
        match &message.payload {
            super::messages::MessagePayload::StateData(state) => {
                if state.block_number == 0 {
                    return Err(eyre!("Invalid block number"));
                }
                Ok(())
            }
            _ => Err(eyre!("Invalid payload type for state sync")),
        }
    }
}

/// Handler for transaction proposal messages
struct TransactionProposalHandler;

impl MessageHandler for TransactionProposalHandler {
    fn handle(&self, message: &ExExMessage) -> Result<Option<ExExMessage>> {
        match &message.payload {
            super::messages::MessagePayload::Transaction(tx_data) => {
                tracing::debug!(
                    "Transaction proposal from {}: {:?}",
                    message.source,
                    tx_data.hash
                );
                // Process transaction proposal
                // Could return an acceptance/rejection message
                Ok(None)
            }
            _ => Err(eyre!("Invalid payload for transaction proposal")),
        }
    }

    fn validate(&self, message: &ExExMessage) -> Result<()> {
        match &message.payload {
            super::messages::MessagePayload::Transaction(tx_data) => {
                if tx_data.data.is_empty() && tx_data.value == alloy_primitives::U256::ZERO {
                    return Err(eyre!("Empty transaction"));
                }
                Ok(())
            }
            _ => Err(eyre!("Invalid payload type for transaction proposal")),
        }
    }
}

/// Example protocol extension for encryption
pub struct EncryptionExtension {
    enabled: bool,
    key: Option<Vec<u8>>,
}

impl EncryptionExtension {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            key: None,
        }
    }
}

impl ProtocolExtension for EncryptionExtension {
    fn name(&self) -> &str {
        "encryption"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        if self.enabled {
            // Initialize encryption key
            self.key = Some(vec![0u8; 32]); // Placeholder
        }
        Ok(())
    }

    fn process(&self, _message: &mut ExExMessage) -> Result<()> {
        if self.enabled {
            // Apply encryption/decryption
            tracing::debug!("Processing message through encryption extension");
        }
        Ok(())
    }
}

/// Example protocol extension for compression
pub struct CompressionExtension {
    compression_level: u32,
}

impl CompressionExtension {
    pub fn new(compression_level: u32) -> Self {
        Self { compression_level }
    }
}

impl ProtocolExtension for CompressionExtension {
    fn name(&self) -> &str {
        "compression"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn initialize(&mut self) -> Result<()> {
        if self.compression_level > 9 {
            return Err(eyre!("Invalid compression level"));
        }
        Ok(())
    }

    fn process(&self, _message: &mut ExExMessage) -> Result<()> {
        // Apply compression/decompression
        tracing::debug!("Processing message through compression extension");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_version() {
        assert_eq!(ProtocolVersion::V1.as_u8(), 1);
        assert_eq!(ProtocolVersion::from_u8(1), Some(ProtocolVersion::V1));
        assert_eq!(ProtocolVersion::from_u8(99), None);
    }

    #[test]
    fn test_protocol_creation() {
        let protocol = Protocol::new(ProtocolVersion::V1);
        assert_eq!(protocol.version(), ProtocolVersion::V1);
        assert!(!protocol.handlers.is_empty());
    }

    #[test]
    fn test_extension_registration() {
        let mut protocol = Protocol::new(ProtocolVersion::V1);
        let extension = Box::new(EncryptionExtension::new(true));
        
        let result = protocol.register_extension(extension);
        assert!(result.is_ok());
        assert!(protocol.extensions.contains_key("encryption"));
    }
}