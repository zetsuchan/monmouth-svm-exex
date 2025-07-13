//! Connection pool management for high throughput

use crate::errors::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use tonic::transport::{Channel, Endpoint};
use tracing::{debug, error, info, warn};

/// Connection pool for managing gRPC client connections
pub struct ConnectionPool {
    /// Pool of active connections
    connections: Arc<Mutex<HashMap<String, PooledConnection>>>,
    
    /// Maximum connections per endpoint
    max_connections_per_endpoint: usize,
    
    /// Connection timeout
    connection_timeout: std::time::Duration,
    
    /// Idle timeout for connections
    idle_timeout: std::time::Duration,
    
    /// Semaphore for limiting total connections
    connection_semaphore: Arc<Semaphore>,
}

/// A pooled connection
struct PooledConnection {
    /// The actual gRPC channel
    channel: Channel,
    
    /// Last used timestamp
    last_used: std::time::Instant,
    
    /// Number of active requests
    active_requests: usize,
    
    /// Connection health status
    healthy: bool,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(max_total_connections: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            max_connections_per_endpoint: 10,
            connection_timeout: std::time::Duration::from_secs(5),
            idle_timeout: std::time::Duration::from_secs(300), // 5 minutes
            connection_semaphore: Arc::new(Semaphore::new(max_total_connections)),
        }
    }
    
    /// Get or create a connection to an endpoint
    pub async fn get_connection(&self, endpoint: &str) -> Result<Channel> {
        // First, try to get an existing healthy connection
        {
            let mut connections = self.connections.lock().await;
            if let Some(conn) = connections.get_mut(endpoint) {
                if conn.healthy {
                    conn.last_used = std::time::Instant::now();
                    conn.active_requests += 1;
                    debug!("Reusing existing connection to {}", endpoint);
                    return Ok(conn.channel.clone());
                }
            }
        }
        
        // No healthy connection found, create a new one
        self.create_connection(endpoint).await
    }
    
    /// Create a new connection
    async fn create_connection(&self, endpoint: &str) -> Result<Channel> {
        // Acquire permit from semaphore
        let _permit = self.connection_semaphore
            .acquire()
            .await
            .map_err(|_| SvmExExError::NetworkError("Connection limit reached".to_string()))?;
        
        info!("Creating new connection to {}", endpoint);
        
        let channel = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| SvmExExError::NetworkError(e.to_string()))?
            .timeout(self.connection_timeout)
            .connect_lazy();
        
        // Store the connection
        let pooled_conn = PooledConnection {
            channel: channel.clone(),
            last_used: std::time::Instant::now(),
            active_requests: 1,
            healthy: true,
        };
        
        let mut connections = self.connections.lock().await;
        connections.insert(endpoint.to_string(), pooled_conn);
        
        Ok(channel)
    }
    
    /// Release a connection back to the pool
    pub async fn release_connection(&self, endpoint: &str, success: bool) {
        let mut connections = self.connections.lock().await;
        if let Some(conn) = connections.get_mut(endpoint) {
            conn.active_requests = conn.active_requests.saturating_sub(1);
            if !success {
                conn.healthy = false;
                warn!("Marking connection to {} as unhealthy", endpoint);
            }
        }
    }
    
    /// Clean up idle connections
    pub async fn cleanup_idle_connections(&self) {
        let mut connections = self.connections.lock().await;
        let now = std::time::Instant::now();
        
        connections.retain(|endpoint, conn| {
            let idle_duration = now.duration_since(conn.last_used);
            let should_keep = conn.active_requests > 0 || idle_duration < self.idle_timeout;
            
            if !should_keep {
                info!("Removing idle connection to {}", endpoint);
            }
            
            should_keep
        });
    }
    
    /// Start background cleanup task
    pub fn start_cleanup_task(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                self.cleanup_idle_connections().await;
            }
        });
    }
    
    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        let connections = self.connections.lock().await;
        
        let total_connections = connections.len();
        let healthy_connections = connections.values().filter(|c| c.healthy).count();
        let active_requests: usize = connections.values().map(|c| c.active_requests).sum();
        
        PoolStats {
            total_connections,
            healthy_connections,
            active_requests,
        }
    }
}

/// Connection pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub healthy_connections: usize,
    pub active_requests: usize,
}

/// Connection pool builder for advanced configuration
pub struct ConnectionPoolBuilder {
    max_total_connections: usize,
    max_connections_per_endpoint: usize,
    connection_timeout: std::time::Duration,
    idle_timeout: std::time::Duration,
}

impl Default for ConnectionPoolBuilder {
    fn default() -> Self {
        Self {
            max_total_connections: 1000,
            max_connections_per_endpoint: 10,
            connection_timeout: std::time::Duration::from_secs(5),
            idle_timeout: std::time::Duration::from_secs(300),
        }
    }
}

impl ConnectionPoolBuilder {
    /// Set maximum total connections
    pub fn max_total_connections(mut self, max: usize) -> Self {
        self.max_total_connections = max;
        self
    }
    
    /// Set maximum connections per endpoint
    pub fn max_connections_per_endpoint(mut self, max: usize) -> Self {
        self.max_connections_per_endpoint = max;
        self
    }
    
    /// Set connection timeout
    pub fn connection_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.connection_timeout = timeout;
        self
    }
    
    /// Set idle timeout
    pub fn idle_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }
    
    /// Build the connection pool
    pub fn build(self) -> ConnectionPool {
        let mut pool = ConnectionPool::new(self.max_total_connections);
        pool.max_connections_per_endpoint = self.max_connections_per_endpoint;
        pool.connection_timeout = self.connection_timeout;
        pool.idle_timeout = self.idle_timeout;
        pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_connection_pool_creation() {
        let pool = ConnectionPool::new(100);
        let stats = pool.get_stats().await;
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.healthy_connections, 0);
        assert_eq!(stats.active_requests, 0);
    }
    
    #[tokio::test]
    async fn test_connection_pool_builder() {
        let pool = ConnectionPoolBuilder::default()
            .max_total_connections(50)
            .connection_timeout(std::time::Duration::from_secs(10))
            .build();
        
        assert_eq!(pool.connection_timeout, std::time::Duration::from_secs(10));
    }
}