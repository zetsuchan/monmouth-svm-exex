//! Synchronization Protocol Implementation
//! 
//! This module defines the protocol for state synchronization between
//! RAG ExEx and SVM ExEx instances.

use crate::errors::*;
use crate::sync::state_sync::{StateSnapshot, StateDelta, SyncDirection};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, Mutex, RwLock};
use tracing::{debug, info, warn, error};

/// Synchronization protocol handler
pub struct SyncProtocol {
    /// Protocol version
    version: ProtocolVersion,
    
    /// Message handlers
    handlers: Arc<MessageHandlers>,
    
    /// Protocol extensions
    extensions: Arc<ProtocolExtensions>,
    
    /// Active connections
    connections: Arc<ConnectionManager>,
    
    /// Protocol metrics
    metrics: Arc<RwLock<ProtocolMetrics>>,
}

/// Protocol version information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    /// Major version
    pub major: u16,
    /// Minor version
    pub minor: u16,
    /// Patch version
    pub patch: u16,
    /// Protocol name
    pub name: String,
}

/// Sync protocol messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Handshake initiation
    Handshake {
        node_id: String,
        version: ProtocolVersion,
        capabilities: NodeCapabilities,
    },
    /// Handshake response
    HandshakeResponse {
        node_id: String,
        accepted: bool,
        version: ProtocolVersion,
    },
    /// Sync request
    SyncRequest(SyncRequest),
    /// Sync response
    SyncResponse(SyncResponse),
    /// Heartbeat
    Heartbeat {
        node_id: String,
        timestamp: SystemTime,
    },
    /// Error message
    Error(SyncError),
    /// Control message
    Control(ControlMessage),
}

/// Message types for routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MessageType {
    Handshake,
    HandshakeResponse,
    SyncRequest,
    SyncResponse,
    Heartbeat,
    Error,
    Control,
}

/// Node capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Supported message types
    pub message_types: Vec<MessageType>,
    /// Maximum message size
    pub max_message_size: usize,
    /// Compression support
    pub compression: Vec<CompressionType>,
    /// Encryption support
    pub encryption: Vec<EncryptionType>,
    /// Custom capabilities
    pub custom: HashMap<String, serde_json::Value>,
}

/// Compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Encryption types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncryptionType {
    None,
    Aes256Gcm,
    ChaCha20Poly1305,
}

/// Sync request types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    /// Request full state snapshot
    FullSnapshot {
        requester_id: String,
        include_metadata: bool,
    },
    /// Request incremental snapshot
    IncrementalSnapshot {
        requester_id: String,
        base_snapshot_id: String,
        after_timestamp: SystemTime,
    },
    /// Request state delta
    Delta {
        requester_id: String,
        from_snapshot: String,
        to_snapshot: Option<String>,
    },
    /// Validate state
    Validate {
        snapshot_id: String,
        checksum: String,
    },
    /// Query specific state
    Query {
        query_type: QueryType,
        parameters: HashMap<String, serde_json::Value>,
    },
}

/// Query types for state queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryType {
    /// Query memory state
    Memory {
        agent_id: Option<String>,
        memory_type: Option<String>,
    },
    /// Query graph state
    Graph {
        entity_id: Option<String>,
        depth: Option<usize>,
    },
    /// Query embeddings
    Embeddings {
        keys: Vec<String>,
    },
    /// Custom query
    Custom(String),
}

/// Sync response types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    /// Snapshot response
    Snapshot {
        snapshot: CompressedSnapshot,
        metadata: ResponseMetadata,
    },
    /// Delta response
    Delta {
        delta: CompressedDelta,
        metadata: ResponseMetadata,
    },
    /// Validation response
    Validation {
        valid: bool,
        details: Option<ValidationDetails>,
    },
    /// Query response
    QueryResult {
        data: Vec<u8>,
        format: DataFormat,
        metadata: ResponseMetadata,
    },
    /// Acknowledgment
    Ack {
        request_id: String,
        status: AckStatus,
    },
}

/// Compressed snapshot wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedSnapshot {
    /// Compression type used
    pub compression: CompressionType,
    /// Compressed data
    pub data: Vec<u8>,
    /// Uncompressed size
    pub uncompressed_size: usize,
    /// Checksum of uncompressed data
    pub checksum: String,
}

/// Compressed delta wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedDelta {
    /// Compression type used
    pub compression: CompressionType,
    /// Compressed data
    pub data: Vec<u8>,
    /// Uncompressed size
    pub uncompressed_size: usize,
    /// Number of changes
    pub change_count: usize,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Response timestamp
    pub timestamp: SystemTime,
    /// Processing time (ms)
    pub processing_time_ms: u64,
    /// Node that generated response
    pub node_id: String,
    /// Additional metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Data formats for query responses
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Bincode,
    MessagePack,
    Protobuf,
}

/// Validation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationDetails {
    /// Expected checksum
    pub expected_checksum: String,
    /// Actual checksum
    pub actual_checksum: String,
    /// Mismatched fields
    pub mismatches: Vec<FieldMismatch>,
}

/// Field mismatch information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMismatch {
    /// Field path
    pub path: String,
    /// Expected value
    pub expected: String,
    /// Actual value
    pub actual: String,
}

/// Acknowledgment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AckStatus {
    Received,
    Processing,
    Completed,
    Failed,
}

/// Control messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ControlMessage {
    /// Pause synchronization
    Pause {
        reason: String,
        duration: Option<Duration>,
    },
    /// Resume synchronization
    Resume,
    /// Reset connection
    Reset {
        reason: String,
    },
    /// Negotiate parameters
    Negotiate {
        parameters: NegotiationParameters,
    },
}

/// Negotiation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationParameters {
    /// Preferred compression
    pub compression: Option<CompressionType>,
    /// Preferred encryption
    pub encryption: Option<EncryptionType>,
    /// Maximum batch size
    pub max_batch_size: Option<usize>,
    /// Sync interval
    pub sync_interval: Option<Duration>,
}

/// Sync protocol errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncError {
    /// Error code
    pub code: ErrorCode,
    /// Error message
    pub message: String,
    /// Error details
    pub details: Option<HashMap<String, serde_json::Value>>,
    /// Retry after (if applicable)
    pub retry_after: Option<Duration>,
}

/// Error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    /// Protocol version mismatch
    VersionMismatch,
    /// Invalid message format
    InvalidMessage,
    /// Authentication failed
    AuthenticationFailed,
    /// Resource not found
    NotFound,
    /// Operation timeout
    Timeout,
    /// Rate limit exceeded
    RateLimited,
    /// Internal error
    InternalError,
    /// Invalid state
    InvalidState,
}

/// Message handlers
pub struct MessageHandlers {
    /// Registered handlers
    handlers: DashMap<MessageType, Arc<dyn MessageHandler>>,
}

/// Trait for message handlers
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Handle a message
    async fn handle(&self, msg: SyncMessage, ctx: &HandlerContext) -> Result<Option<SyncMessage>>;
    
    /// Message type this handler processes
    fn message_type(&self) -> MessageType;
}

/// Handler context
pub struct HandlerContext {
    /// Node ID
    pub node_id: String,
    /// Connection info
    pub connection: ConnectionInfo,
    /// Protocol version
    pub version: ProtocolVersion,
}

/// Connection information
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Remote node ID
    pub remote_id: String,
    /// Connection established at
    pub established_at: Instant,
    /// Last activity
    pub last_activity: Instant,
    /// Connection state
    pub state: ConnectionState,
}

/// Connection states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Authenticated,
    Syncing,
    Paused,
    Disconnected,
}

/// Protocol extensions
pub struct ProtocolExtensions {
    /// Compression handlers
    compressors: HashMap<CompressionType, Arc<dyn Compressor>>,
    /// Encryption handlers
    encryptors: HashMap<EncryptionType, Arc<dyn Encryptor>>,
    /// Custom extensions
    custom: HashMap<String, Arc<dyn ProtocolExtension>>,
}

/// Trait for compression
#[async_trait]
pub trait Compressor: Send + Sync {
    /// Compress data
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Decompress data
    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
    
    /// Compression type
    fn compression_type(&self) -> CompressionType;
}

/// Trait for encryption
#[async_trait]
pub trait Encryptor: Send + Sync {
    /// Encrypt data
    async fn encrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>>;
    
    /// Decrypt data
    async fn decrypt(&self, data: &[u8], key: &[u8]) -> Result<Vec<u8>>;
    
    /// Encryption type
    fn encryption_type(&self) -> EncryptionType;
}

/// Trait for protocol extensions
#[async_trait]
pub trait ProtocolExtension: Send + Sync {
    /// Process outgoing message
    async fn process_outgoing(&self, msg: &mut SyncMessage) -> Result<()>;
    
    /// Process incoming message
    async fn process_incoming(&self, msg: &mut SyncMessage) -> Result<()>;
    
    /// Extension name
    fn name(&self) -> &str;
}

/// Connection manager
pub struct ConnectionManager {
    /// Active connections
    connections: DashMap<String, ManagedConnection>,
    /// Connection pool
    pool: Arc<ConnectionPool>,
}

/// Managed connection
pub struct ManagedConnection {
    /// Connection info
    pub info: ConnectionInfo,
    /// Message sender
    pub sender: mpsc::Sender<SyncMessage>,
    /// Message receiver
    pub receiver: Arc<Mutex<mpsc::Receiver<SyncMessage>>>,
    /// Connection metrics
    pub metrics: ConnectionMetrics,
}

/// Connection metrics
#[derive(Debug, Clone, Default)]
pub struct ConnectionMetrics {
    /// Messages sent
    pub messages_sent: u64,
    /// Messages received
    pub messages_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Errors
    pub error_count: u32,
}

/// Connection pool
pub struct ConnectionPool {
    /// Maximum connections
    max_connections: usize,
    /// Connection timeout
    connection_timeout: Duration,
    /// Idle timeout
    idle_timeout: Duration,
}

/// Protocol metrics
#[derive(Debug, Clone, Default)]
pub struct ProtocolMetrics {
    /// Total messages processed
    pub messages_processed: u64,
    /// Messages by type
    pub messages_by_type: HashMap<MessageType, u64>,
    /// Average processing time (ms)
    pub avg_processing_time_ms: f64,
    /// Error rate
    pub error_rate: f64,
    /// Active connections
    pub active_connections: usize,
}

impl SyncProtocol {
    /// Create a new sync protocol
    pub fn new() -> Self {
        Self {
            version: ProtocolVersion {
                major: 1,
                minor: 0,
                patch: 0,
                name: "rag-svm-sync".to_string(),
            },
            handlers: Arc::new(MessageHandlers::new()),
            extensions: Arc::new(ProtocolExtensions::new()),
            connections: Arc::new(ConnectionManager::new()),
            metrics: Arc::new(RwLock::new(ProtocolMetrics::default())),
        }
    }
    
    /// Initialize the protocol
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing sync protocol v{}", self.version_string());
        
        // Register default handlers
        self.register_default_handlers().await?;
        
        // Initialize extensions
        self.extensions.initialize().await?;
        
        Ok(())
    }
    
    /// Process incoming message
    pub async fn process_message(
        &self,
        msg: SyncMessage,
        connection_id: &str,
    ) -> Result<Option<SyncMessage>> {
        let start = Instant::now();
        
        // Get connection
        let connection = self.connections
            .get_connection(connection_id)
            .ok_or_else(|| AIAgentError::ProcessingError("Connection not found".to_string()))?;
        
        // Create handler context
        let ctx = HandlerContext {
            node_id: connection.info.remote_id.clone(),
            connection: connection.info.clone(),
            version: self.version.clone(),
        };
        
        // Process through extensions
        let mut msg = msg;
        self.extensions.process_incoming(&mut msg).await?;
        
        // Get message type
        let msg_type = self.get_message_type(&msg);
        
        // Handle message
        let response = self.handlers.handle(msg_type, msg, &ctx).await?;
        
        // Update metrics
        self.update_metrics(msg_type, start.elapsed()).await;
        
        Ok(response)
    }
    
    /// Send message
    pub async fn send_message(
        &self,
        msg: SyncMessage,
        connection_id: &str,
    ) -> Result<()> {
        // Process through extensions
        let mut msg = msg;
        self.extensions.process_outgoing(&mut msg).await?;
        
        // Get connection
        let connection = self.connections
            .get_connection(connection_id)
            .ok_or_else(|| AIAgentError::ProcessingError("Connection not found".to_string()))?;
        
        // Send message
        connection.sender.send(msg).await?;
        
        Ok(())
    }
    
    /// Establish connection
    pub async fn connect(&self, remote_id: &str, endpoint: &str) -> Result<String> {
        info!("Establishing connection to {} at {}", remote_id, endpoint);
        
        // Create connection
        let connection_id = self.connections.create_connection(remote_id).await?;
        
        // Send handshake
        let handshake = SyncMessage::Handshake {
            node_id: "local_node".to_string(), // Would use actual node ID
            version: self.version.clone(),
            capabilities: self.get_capabilities(),
        };
        
        self.send_message(handshake, &connection_id).await?;
        
        Ok(connection_id)
    }
    
    /// Get protocol version string
    fn version_string(&self) -> String {
        format!("{}.{}.{}", self.version.major, self.version.minor, self.version.patch)
    }
    
    /// Register default message handlers
    async fn register_default_handlers(&self) -> Result<()> {
        // Register handshake handler
        self.handlers.register(Arc::new(HandshakeHandler::new()));
        
        // Register sync request handler
        self.handlers.register(Arc::new(SyncRequestHandler::new()));
        
        // Register heartbeat handler
        self.handlers.register(Arc::new(HeartbeatHandler::new()));
        
        Ok(())
    }
    
    /// Get message type
    fn get_message_type(&self, msg: &SyncMessage) -> MessageType {
        match msg {
            SyncMessage::Handshake { .. } => MessageType::Handshake,
            SyncMessage::HandshakeResponse { .. } => MessageType::HandshakeResponse,
            SyncMessage::SyncRequest(_) => MessageType::SyncRequest,
            SyncMessage::SyncResponse(_) => MessageType::SyncResponse,
            SyncMessage::Heartbeat { .. } => MessageType::Heartbeat,
            SyncMessage::Error(_) => MessageType::Error,
            SyncMessage::Control(_) => MessageType::Control,
        }
    }
    
    /// Get node capabilities
    fn get_capabilities(&self) -> NodeCapabilities {
        NodeCapabilities {
            message_types: vec![
                MessageType::Handshake,
                MessageType::HandshakeResponse,
                MessageType::SyncRequest,
                MessageType::SyncResponse,
                MessageType::Heartbeat,
                MessageType::Error,
                MessageType::Control,
            ],
            max_message_size: 10 * 1024 * 1024, // 10MB
            compression: vec![CompressionType::None, CompressionType::Gzip],
            encryption: vec![EncryptionType::None],
            custom: HashMap::new(),
        }
    }
    
    /// Update protocol metrics
    async fn update_metrics(&self, msg_type: MessageType, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        
        metrics.messages_processed += 1;
        *metrics.messages_by_type.entry(msg_type).or_insert(0) += 1;
        
        // Update average processing time
        let n = metrics.messages_processed as f64;
        let duration_ms = duration.as_millis() as f64;
        metrics.avg_processing_time_ms = 
            (metrics.avg_processing_time_ms * (n - 1.0) + duration_ms) / n;
        
        metrics.active_connections = self.connections.active_count();
    }
}

impl MessageHandlers {
    /// Create new message handlers
    fn new() -> Self {
        Self {
            handlers: DashMap::new(),
        }
    }
    
    /// Register a handler
    fn register(&self, handler: Arc<dyn MessageHandler>) {
        let msg_type = handler.message_type();
        self.handlers.insert(msg_type, handler);
    }
    
    /// Handle a message
    async fn handle(
        &self,
        msg_type: MessageType,
        msg: SyncMessage,
        ctx: &HandlerContext,
    ) -> Result<Option<SyncMessage>> {
        if let Some(handler) = self.handlers.get(&msg_type) {
            handler.handle(msg, ctx).await
        } else {
            Err(AIAgentError::ProcessingError(
                format!("No handler for message type {:?}", msg_type)
            ))
        }
    }
}

impl ProtocolExtensions {
    /// Create new protocol extensions
    fn new() -> Self {
        let mut extensions = Self {
            compressors: HashMap::new(),
            encryptors: HashMap::new(),
            custom: HashMap::new(),
        };
        
        // Register default compressor
        extensions.compressors.insert(
            CompressionType::None,
            Arc::new(NoOpCompressor),
        );
        
        // Register default encryptor
        extensions.encryptors.insert(
            EncryptionType::None,
            Arc::new(NoOpEncryptor),
        );
        
        extensions
    }
    
    /// Initialize extensions
    async fn initialize(&self) -> Result<()> {
        Ok(())
    }
    
    /// Process incoming message through extensions
    async fn process_incoming(&self, msg: &mut SyncMessage) -> Result<()> {
        for extension in self.custom.values() {
            extension.process_incoming(msg).await?;
        }
        Ok(())
    }
    
    /// Process outgoing message through extensions
    async fn process_outgoing(&self, msg: &mut SyncMessage) -> Result<()> {
        for extension in self.custom.values() {
            extension.process_outgoing(msg).await?;
        }
        Ok(())
    }
}

impl ConnectionManager {
    /// Create new connection manager
    fn new() -> Self {
        Self {
            connections: DashMap::new(),
            pool: Arc::new(ConnectionPool {
                max_connections: 100,
                connection_timeout: Duration::from_secs(30),
                idle_timeout: Duration::from_secs(300),
            }),
        }
    }
    
    /// Create a new connection
    async fn create_connection(&self, remote_id: &str) -> Result<String> {
        let connection_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(1000);
        
        let connection = ManagedConnection {
            info: ConnectionInfo {
                remote_id: remote_id.to_string(),
                established_at: Instant::now(),
                last_activity: Instant::now(),
                state: ConnectionState::Connecting,
            },
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
            metrics: ConnectionMetrics::default(),
        };
        
        self.connections.insert(connection_id.clone(), connection);
        
        Ok(connection_id)
    }
    
    /// Get connection
    fn get_connection(&self, connection_id: &str) -> Option<ManagedConnection> {
        self.connections.get(connection_id).map(|entry| ManagedConnection {
            info: entry.info.clone(),
            sender: entry.sender.clone(),
            receiver: entry.receiver.clone(),
            metrics: entry.metrics.clone(),
        })
    }
    
    /// Get active connection count
    fn active_count(&self) -> usize {
        self.connections
            .iter()
            .filter(|entry| entry.info.state == ConnectionState::Connected)
            .count()
    }
}

// Example handler implementations

/// Handshake handler
struct HandshakeHandler;

impl HandshakeHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for HandshakeHandler {
    async fn handle(&self, msg: SyncMessage, ctx: &HandlerContext) -> Result<Option<SyncMessage>> {
        if let SyncMessage::Handshake { node_id, version, capabilities } = msg {
            info!("Received handshake from {} with version {:?}", node_id, version);
            
            // Check version compatibility
            let accepted = version.major == 1; // Simple check
            
            let response = SyncMessage::HandshakeResponse {
                node_id: "local_node".to_string(),
                accepted,
                version: ctx.version.clone(),
            };
            
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }
    
    fn message_type(&self) -> MessageType {
        MessageType::Handshake
    }
}

/// Sync request handler
struct SyncRequestHandler;

impl SyncRequestHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for SyncRequestHandler {
    async fn handle(&self, msg: SyncMessage, ctx: &HandlerContext) -> Result<Option<SyncMessage>> {
        if let SyncMessage::SyncRequest(request) = msg {
            debug!("Handling sync request from {}", ctx.node_id);
            
            // Handle based on request type
            let response = match request {
                SyncRequest::FullSnapshot { .. } => {
                    // Would fetch actual snapshot
                    SyncMessage::SyncResponse(SyncResponse::Ack {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        status: AckStatus::Processing,
                    })
                }
                _ => {
                    SyncMessage::SyncResponse(SyncResponse::Ack {
                        request_id: uuid::Uuid::new_v4().to_string(),
                        status: AckStatus::Received,
                    })
                }
            };
            
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }
    
    fn message_type(&self) -> MessageType {
        MessageType::SyncRequest
    }
}

/// Heartbeat handler
struct HeartbeatHandler;

impl HeartbeatHandler {
    fn new() -> Self {
        Self
    }
}

#[async_trait]
impl MessageHandler for HeartbeatHandler {
    async fn handle(&self, msg: SyncMessage, ctx: &HandlerContext) -> Result<Option<SyncMessage>> {
        if let SyncMessage::Heartbeat { node_id, timestamp } = msg {
            debug!("Heartbeat from {} at {:?}", node_id, timestamp);
            
            // Echo heartbeat
            Ok(Some(SyncMessage::Heartbeat {
                node_id: "local_node".to_string(),
                timestamp: SystemTime::now(),
            }))
        } else {
            Ok(None)
        }
    }
    
    fn message_type(&self) -> MessageType {
        MessageType::Heartbeat
    }
}

// No-op implementations for testing

struct NoOpCompressor;

#[async_trait]
impl Compressor for NoOpCompressor {
    async fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    async fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    fn compression_type(&self) -> CompressionType {
        CompressionType::None
    }
}

struct NoOpEncryptor;

#[async_trait]
impl Encryptor for NoOpEncryptor {
    async fn encrypt(&self, data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    async fn decrypt(&self, data: &[u8], _key: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }
    
    fn encryption_type(&self) -> EncryptionType {
        EncryptionType::None
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self {
            major: 1,
            minor: 0,
            patch: 0,
            name: "rag-svm-sync".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_protocol_initialization() {
        let protocol = SyncProtocol::new();
        assert!(protocol.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_message_handling() {
        let protocol = SyncProtocol::new();
        protocol.initialize().await.unwrap();
        
        // Create test connection
        let connection_id = protocol.connect("test_remote", "test://endpoint").await.unwrap();
        
        // Test handshake message
        let msg = SyncMessage::Handshake {
            node_id: "test_node".to_string(),
            version: ProtocolVersion::default(),
            capabilities: NodeCapabilities {
                message_types: vec![MessageType::Handshake],
                max_message_size: 1024,
                compression: vec![CompressionType::None],
                encryption: vec![EncryptionType::None],
                custom: HashMap::new(),
            },
        };
        
        let response = protocol.process_message(msg, &connection_id).await;
        assert!(response.is_ok());
    }
}