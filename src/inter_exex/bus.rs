//! Message bus implementation for inter-ExEx communication

use super::messages::{ExExMessage, MessageType, NodeInfo};
use eyre::{eyre, Result};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, Mutex};
use tracing::{debug, error, info, warn};

/// Configuration for the message bus
#[derive(Debug, Clone)]
pub struct MessageBusConfig {
    /// Maximum number of queued messages per channel
    pub channel_capacity: usize,
    /// Network binding address
    pub bind_address: String,
    /// Discovery service endpoints
    pub discovery_endpoints: Vec<String>,
    /// Enable encryption for messages
    pub enable_encryption: bool,
    /// Message retention time in seconds
    pub message_retention_secs: u64,
    /// Maximum message size in bytes
    pub max_message_size: usize,
}

impl Default for MessageBusConfig {
    fn default() -> Self {
        Self {
            channel_capacity: 1000,
            bind_address: "0.0.0.0:9876".to_string(),
            discovery_endpoints: vec![],
            enable_encryption: true,
            message_retention_secs: 300, // 5 minutes
            max_message_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Message bus for inter-ExEx communication
pub struct MessageBus {
    /// Configuration
    config: MessageBusConfig,
    /// Subscriptions by message type
    subscriptions: Arc<RwLock<HashMap<MessageType, Vec<mpsc::Sender<ExExMessage>>>>>,
    /// Connected nodes
    connected_nodes: Arc<RwLock<HashMap<String, NodeConnection>>>,
    /// Outbound message queue
    outbound_queue: Arc<Mutex<mpsc::Sender<OutboundMessage>>>,
    /// Shutdown signal
    shutdown: Arc<RwLock<Option<mpsc::Sender<()>>>>,
}

/// Connection information for a node
#[derive(Debug, Clone)]
struct NodeConnection {
    /// Node information
    info: NodeInfo,
    /// Last heartbeat timestamp
    last_heartbeat: std::time::Instant,
    /// Message sender for this node
    sender: mpsc::Sender<ExExMessage>,
}

/// Outbound message with routing information
#[derive(Debug)]
struct OutboundMessage {
    /// Target node ID (None for broadcast)
    target: Option<String>,
    /// Message to send
    message: ExExMessage,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new(config: MessageBusConfig) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<OutboundMessage>(config.channel_capacity);
        
        // Spawn outbound message processor
        let connected_nodes = Arc::new(RwLock::new(HashMap::new()));
        let nodes_clone = connected_nodes.clone();
        
        tokio::spawn(async move {
            while let Some(outbound) = rx.recv().await {
                Self::process_outbound_message(outbound, &nodes_clone).await;
            }
        });

        Ok(Self {
            config,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            connected_nodes,
            outbound_queue: Arc::new(Mutex::new(tx)),
            shutdown: Arc::new(RwLock::new(None)),
        })
    }

    /// Start the message bus
    pub async fn start(&self) -> Result<()> {
        info!("Starting message bus on {}", self.config.bind_address);
        
        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        *self.shutdown.write().await = Some(shutdown_tx);

        // Start network listener
        self.start_network_listener().await?;
        
        // Start discovery service
        if !self.config.discovery_endpoints.is_empty() {
            self.start_discovery_service().await?;
        }

        // Start heartbeat monitor
        self.start_heartbeat_monitor().await?;

        Ok(())
    }

    /// Start network listener for incoming connections
    async fn start_network_listener(&self) -> Result<()> {
        // This would implement the actual network listening logic
        // For now, we'll create a placeholder
        debug!("Network listener started on {}", self.config.bind_address);
        Ok(())
    }

    /// Start discovery service for finding other nodes
    async fn start_discovery_service(&self) -> Result<()> {
        let discovery_endpoints = self.config.discovery_endpoints.clone();
        let connected_nodes = self.connected_nodes.clone();
        
        tokio::spawn(async move {
            for endpoint in discovery_endpoints {
                debug!("Connecting to discovery endpoint: {}", endpoint);
                // Implement actual discovery logic here
            }
        });
        
        Ok(())
    }

    /// Start heartbeat monitor to check node health
    async fn start_heartbeat_monitor(&self) -> Result<()> {
        let connected_nodes = self.connected_nodes.clone();
        let outbound_queue = self.outbound_queue.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            
            loop {
                interval.tick().await;
                
                // Check for stale connections
                let mut nodes = connected_nodes.write().await;
                let now = std::time::Instant::now();
                let stale_threshold = std::time::Duration::from_secs(90);
                
                nodes.retain(|node_id, conn| {
                    if now.duration_since(conn.last_heartbeat) > stale_threshold {
                        warn!("Node {} heartbeat timeout, removing", node_id);
                        false
                    } else {
                        true
                    }
                });
                
                // Send heartbeat to all connected nodes
                let heartbeat = ExExMessage::new(
                    MessageType::Heartbeat,
                    "self".to_string(), // Will be replaced with actual node ID
                    super::messages::MessagePayload::Empty,
                );
                
                if let Ok(queue) = outbound_queue.lock().await.send(OutboundMessage {
                    target: None,
                    message: heartbeat,
                }).await {
                    debug!("Heartbeat sent to all nodes");
                }
            }
        });
        
        Ok(())
    }

    /// Process outbound messages
    async fn process_outbound_message(
        msg: OutboundMessage,
        connected_nodes: &Arc<RwLock<HashMap<String, NodeConnection>>>,
    ) {
        let nodes = connected_nodes.read().await;
        
        match msg.target {
            Some(target_id) => {
                // Send to specific node
                if let Some(conn) = nodes.get(&target_id) {
                    if let Err(e) = conn.sender.send(msg.message).await {
                        error!("Failed to send message to {}: {}", target_id, e);
                    }
                } else {
                    warn!("Target node {} not found", target_id);
                }
            }
            None => {
                // Broadcast to all nodes
                for (node_id, conn) in nodes.iter() {
                    if let Err(e) = conn.sender.send(msg.message.clone()).await {
                        error!("Failed to broadcast to {}: {}", node_id, e);
                    }
                }
            }
        }
    }

    /// Announce node to the network
    pub async fn announce_node(&self, info: NodeInfo) -> Result<()> {
        let announcement = ExExMessage::new(
            MessageType::NodeAnnouncement,
            info.node_id.clone(),
            super::messages::MessagePayload::NodeInfo(info),
        );
        
        self.broadcast(announcement).await
    }

    /// Broadcast a message to all connected nodes
    pub async fn broadcast(&self, message: ExExMessage) -> Result<()> {
        let queue = self.outbound_queue.lock().await;
        queue.send(OutboundMessage {
            target: None,
            message,
        }).await.map_err(|e| eyre!("Failed to queue broadcast: {}", e))
    }

    /// Send a message to a specific node
    pub async fn send_to(&self, target: &str, message: ExExMessage) -> Result<()> {
        let queue = self.outbound_queue.lock().await;
        queue.send(OutboundMessage {
            target: Some(target.to_string()),
            message,
        }).await.map_err(|e| eyre!("Failed to queue message: {}", e))
    }

    /// Subscribe to messages of a specific type
    pub async fn subscribe(&self, message_type: MessageType) -> Result<mpsc::Receiver<ExExMessage>> {
        let (tx, rx) = mpsc::channel(self.config.channel_capacity);
        
        let mut subs = self.subscriptions.write().await;
        subs.entry(message_type)
            .or_insert_with(Vec::new)
            .push(tx);
        
        info!("New subscription for message type: {:?}", message_type);
        Ok(rx)
    }

    /// Handle incoming message
    pub async fn handle_incoming(&self, message: ExExMessage) -> Result<()> {
        // Validate message
        if !message.validate() {
            return Err(eyre!("Invalid message received"));
        }

        // Check message size
        let msg_size = bincode::serialize(&message)
            .map_err(|e| eyre!("Failed to serialize message: {}", e))?
            .len();
        
        if msg_size > self.config.max_message_size {
            return Err(eyre!("Message size {} exceeds limit", msg_size));
        }

        // Update heartbeat if applicable
        if message.message_type == MessageType::Heartbeat {
            let mut nodes = self.connected_nodes.write().await;
            if let Some(conn) = nodes.get_mut(&message.source) {
                conn.last_heartbeat = std::time::Instant::now();
            }
        }

        // Distribute to subscribers
        let subs = self.subscriptions.read().await;
        if let Some(subscribers) = subs.get(&message.message_type) {
            for sub in subscribers {
                if let Err(e) = sub.send(message.clone()).await {
                    debug!("Failed to deliver message to subscriber: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Register a new node connection
    pub async fn register_node(&self, info: NodeInfo, sender: mpsc::Sender<ExExMessage>) -> Result<()> {
        let mut nodes = self.connected_nodes.write().await;
        
        let conn = NodeConnection {
            info: info.clone(),
            last_heartbeat: std::time::Instant::now(),
            sender,
        };
        
        nodes.insert(info.node_id.clone(), conn);
        info!("Registered new node: {}", info.node_id);
        
        Ok(())
    }

    /// Get list of connected nodes
    pub async fn get_connected_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.connected_nodes.read().await;
        nodes.values().map(|conn| conn.info.clone()).collect()
    }

    /// Shutdown the message bus
    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down message bus");
        
        // Send shutdown signal
        if let Some(shutdown_tx) = self.shutdown.write().await.take() {
            let _ = shutdown_tx.send(()).await;
        }
        
        // Clear subscriptions
        self.subscriptions.write().await.clear();
        
        // Clear connected nodes
        self.connected_nodes.write().await.clear();
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_bus_creation() {
        let config = MessageBusConfig::default();
        let bus = MessageBus::new(config);
        assert!(bus.is_ok());
    }

    #[tokio::test]
    async fn test_subscription() {
        let config = MessageBusConfig::default();
        let bus = MessageBus::new(config).unwrap();
        
        let rx = bus.subscribe(MessageType::Data).await;
        assert!(rx.is_ok());
    }

    #[tokio::test]
    async fn test_node_registration() {
        let config = MessageBusConfig::default();
        let bus = MessageBus::new(config).unwrap();
        
        let (tx, _rx) = mpsc::channel(10);
        let info = NodeInfo::new("test-node".to_string());
        
        let result = bus.register_node(info, tx).await;
        assert!(result.is_ok());
        
        let nodes = bus.get_connected_nodes().await;
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "test-node");
    }
}