//! Context Cache for RAG ExEx
//! 
//! High-performance caching layer for preprocessed contexts and embeddings,
//! optimized for real-time SVM transaction processing.

use crate::errors::*;
use crate::ai::context::preprocessing::{PreprocessedContext, ContextChunk};
use async_trait::async_trait;
use dashmap::DashMap;
use lru::LruCache;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{debug, info, warn};

/// Multi-layer context cache with LRU eviction and TTL support
pub struct ContextCache {
    /// L1 cache - hot contexts (in-memory, fast access)
    l1_cache: Arc<RwLock<LruCache<CacheKey, CachedContext>>>,
    /// L2 cache - warm contexts (concurrent access)
    l2_cache: Arc<DashMap<CacheKey, CachedContext>>,
    /// L3 cache - cold contexts (prepared for disk storage)
    l3_cache: Arc<Mutex<HashMap<CacheKey, SerializedContext>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Cache statistics
    stats: Arc<Mutex<CacheStats>>,
    /// Eviction policy
    eviction_policy: Arc<dyn EvictionPolicy>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// L1 cache size (number of contexts)
    pub l1_size: usize,
    /// L2 cache size (number of contexts)
    pub l2_size: usize,
    /// L3 cache size (number of contexts)
    pub l3_size: usize,
    /// TTL for cached contexts
    pub ttl: Duration,
    /// Enable compression for L3 cache
    pub l3_compression: bool,
    /// Background cleanup interval
    pub cleanup_interval: Duration,
    /// Prefetch related contexts
    pub enable_prefetch: bool,
}

/// Cache key for context lookup
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct CacheKey {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Context version (for updates)
    pub version: u32,
    /// Optional agent ID for agent-specific contexts
    pub agent_id: Option<String>,
}

/// Cached context with metadata
#[derive(Debug, Clone)]
pub struct CachedContext {
    /// The preprocessed context
    pub context: Arc<PreprocessedContext>,
    /// Cache metadata
    pub metadata: CacheMetadata,
    /// Access pattern for prefetching
    pub access_pattern: AccessPattern,
}

/// Cache metadata
#[derive(Debug, Clone)]
pub struct CacheMetadata {
    /// Insertion timestamp
    pub inserted_at: Instant,
    /// Last access timestamp
    pub last_accessed: Instant,
    /// Access count
    pub access_count: u32,
    /// Size in bytes
    pub size_bytes: usize,
    /// Priority score
    pub priority: f64,
}

/// Access pattern tracking
#[derive(Debug, Clone)]
pub struct AccessPattern {
    /// Related transaction hashes
    pub related_txs: Vec<[u8; 32]>,
    /// Access frequency
    pub frequency: f64,
    /// Typical access sequence
    pub sequence: Vec<String>,
}

/// Serialized context for L3 cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedContext {
    /// Serialized data
    pub data: Vec<u8>,
    /// Compression type
    pub compression: CompressionType,
    /// Metadata
    pub metadata: SerializedMetadata,
}

/// Compression types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Serialized metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMetadata {
    pub inserted_at_ms: u64,
    pub size_bytes: usize,
    pub priority: f64,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total hits across all layers
    pub total_hits: u64,
    /// Total misses
    pub total_misses: u64,
    /// L1 hits
    pub l1_hits: u64,
    /// L2 hits
    pub l2_hits: u64,
    /// L3 hits
    pub l3_hits: u64,
    /// Total evictions
    pub evictions: u64,
    /// Average retrieval time
    pub avg_retrieval_time_us: f64,
}

/// Trait for cache eviction policies
#[async_trait]
pub trait EvictionPolicy: Send + Sync {
    /// Determine if an item should be evicted
    async fn should_evict(&self, metadata: &CacheMetadata, config: &CacheConfig) -> bool;
    
    /// Compare two items for eviction priority (lower = more likely to evict)
    fn eviction_priority(&self, metadata: &CacheMetadata) -> f64;
}

/// Cache operations trait
#[async_trait]
pub trait CacheOperations {
    /// Get a context from cache
    async fn get(&self, key: &CacheKey) -> Option<Arc<PreprocessedContext>>;
    
    /// Put a context into cache
    async fn put(&self, key: CacheKey, context: PreprocessedContext) -> Result<()>;
    
    /// Invalidate a cache entry
    async fn invalidate(&self, key: &CacheKey) -> Result<()>;
    
    /// Prefetch related contexts
    async fn prefetch(&self, key: &CacheKey) -> Result<()>;
}

impl ContextCache {
    /// Create a new context cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(LruCache::new(config.l1_size))),
            l2_cache: Arc::new(DashMap::with_capacity(config.l2_size)),
            l3_cache: Arc::new(Mutex::new(HashMap::with_capacity(config.l3_size))),
            config: config.clone(),
            stats: Arc::new(Mutex::new(CacheStats::default())),
            eviction_policy: Arc::new(LRUWithTTL::new()),
        }
    }
    
    /// Start background cleanup task
    pub fn start_cleanup_task(&self) {
        let cache = self.clone();
        let interval = self.config.cleanup_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                if let Err(e) = cache.cleanup().await {
                    warn!("Cache cleanup error: {}", e);
                }
            }
        });
    }
    
    /// Perform cache cleanup
    async fn cleanup(&self) -> Result<()> {
        let now = Instant::now();
        let ttl = self.config.ttl;
        
        // Clean L2 cache
        let mut expired_keys = Vec::new();
        for entry in self.l2_cache.iter() {
            if now.duration_since(entry.value().metadata.inserted_at) > ttl {
                expired_keys.push(entry.key().clone());
            }
        }
        
        for key in expired_keys {
            self.l2_cache.remove(&key);
            self.stats.lock().await.evictions += 1;
        }
        
        // Clean L3 cache
        let mut l3_cache = self.l3_cache.lock().await;
        l3_cache.retain(|_, v| {
            let age_ms = now.elapsed().as_millis() as u64 - v.metadata.inserted_at_ms;
            age_ms < ttl.as_millis() as u64
        });
        
        Ok(())
    }
    
    /// Update cache statistics
    async fn update_stats(&self, hit_level: Option<u8>, retrieval_time: Duration) {
        let mut stats = self.stats.lock().await;
        
        match hit_level {
            Some(1) => {
                stats.l1_hits += 1;
                stats.total_hits += 1;
            }
            Some(2) => {
                stats.l2_hits += 1;
                stats.total_hits += 1;
            }
            Some(3) => {
                stats.l3_hits += 1;
                stats.total_hits += 1;
            }
            None => {
                stats.total_misses += 1;
            }
            _ => {}
        }
        
        // Update average retrieval time
        let total_requests = stats.total_hits + stats.total_misses;
        let retrieval_us = retrieval_time.as_micros() as f64;
        stats.avg_retrieval_time_us = 
            (stats.avg_retrieval_time_us * (total_requests - 1) as f64 + retrieval_us) 
            / total_requests as f64;
    }
    
    /// Promote context between cache layers
    async fn promote_context(&self, key: &CacheKey, context: Arc<PreprocessedContext>) {
        // Try to promote to L1
        let mut l1 = self.l1_cache.write();
        if l1.len() >= self.config.l1_size {
            // Evict least recently used
            if let Some((evicted_key, evicted_context)) = l1.pop_lru() {
                drop(l1);
                // Demote to L2
                self.l2_cache.insert(evicted_key, evicted_context);
            }
        } else {
            drop(l1);
        }
        
        // Insert into L1
        let cached = CachedContext {
            context,
            metadata: CacheMetadata {
                inserted_at: Instant::now(),
                last_accessed: Instant::now(),
                access_count: 1,
                size_bytes: 0, // Would calculate actual size
                priority: 1.0,
            },
            access_pattern: AccessPattern {
                related_txs: vec![],
                frequency: 1.0,
                sequence: vec![],
            },
        };
        
        self.l1_cache.write().put(key.clone(), cached);
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.lock().await.clone()
    }
    
    /// Clear all cache layers
    pub async fn clear(&self) -> Result<()> {
        self.l1_cache.write().clear();
        self.l2_cache.clear();
        self.l3_cache.lock().await.clear();
        *self.stats.lock().await = CacheStats::default();
        Ok(())
    }
}

#[async_trait]
impl CacheOperations for ContextCache {
    async fn get(&self, key: &CacheKey) -> Option<Arc<PreprocessedContext>> {
        let start = Instant::now();
        
        // Check L1 cache
        {
            let mut l1 = self.l1_cache.write();
            if let Some(cached) = l1.get_mut(key) {
                cached.metadata.last_accessed = Instant::now();
                cached.metadata.access_count += 1;
                let context = cached.context.clone();
                drop(l1);
                self.update_stats(Some(1), start.elapsed()).await;
                return Some(context);
            }
        }
        
        // Check L2 cache
        if let Some(mut entry) = self.l2_cache.get_mut(key) {
            entry.metadata.last_accessed = Instant::now();
            entry.metadata.access_count += 1;
            let context = entry.context.clone();
            drop(entry);
            
            // Promote to L1 if frequently accessed
            if self.l2_cache.get(key).unwrap().metadata.access_count > 3 {
                self.promote_context(key, context.clone()).await;
            }
            
            self.update_stats(Some(2), start.elapsed()).await;
            return Some(context);
        }
        
        // Check L3 cache
        let l3_cache = self.l3_cache.lock().await;
        if let Some(serialized) = l3_cache.get(key) {
            // Deserialize context
            let data = match serialized.compression {
                CompressionType::None => &serialized.data,
                CompressionType::Gzip => {
                    // Would decompress here
                    &serialized.data
                }
                _ => &serialized.data,
            };
            
            if let Ok(context) = bincode::deserialize::<PreprocessedContext>(data) {
                let arc_context = Arc::new(context);
                drop(l3_cache);
                
                // Promote to L2
                let cached = CachedContext {
                    context: arc_context.clone(),
                    metadata: CacheMetadata {
                        inserted_at: Instant::now(),
                        last_accessed: Instant::now(),
                        access_count: 1,
                        size_bytes: serialized.metadata.size_bytes,
                        priority: serialized.metadata.priority,
                    },
                    access_pattern: AccessPattern {
                        related_txs: vec![],
                        frequency: 1.0,
                        sequence: vec![],
                    },
                };
                self.l2_cache.insert(key.clone(), cached);
                
                self.update_stats(Some(3), start.elapsed()).await;
                return Some(arc_context);
            }
        }
        
        self.update_stats(None, start.elapsed()).await;
        None
    }
    
    async fn put(&self, key: CacheKey, context: PreprocessedContext) -> Result<()> {
        let arc_context = Arc::new(context);
        let size_bytes = bincode::serialize(&*arc_context)?.len();
        
        let cached = CachedContext {
            context: arc_context,
            metadata: CacheMetadata {
                inserted_at: Instant::now(),
                last_accessed: Instant::now(),
                access_count: 0,
                size_bytes,
                priority: 1.0,
            },
            access_pattern: AccessPattern {
                related_txs: vec![],
                frequency: 1.0,
                sequence: vec![],
            },
        };
        
        // Insert into L1 if space available
        let mut l1 = self.l1_cache.write();
        if l1.len() < self.config.l1_size {
            l1.put(key, cached);
        } else {
            drop(l1);
            // Insert into L2
            self.l2_cache.insert(key, cached);
        }
        
        Ok(())
    }
    
    async fn invalidate(&self, key: &CacheKey) -> Result<()> {
        // Remove from all cache layers
        self.l1_cache.write().pop(key);
        self.l2_cache.remove(key);
        self.l3_cache.lock().await.remove(key);
        Ok(())
    }
    
    async fn prefetch(&self, key: &CacheKey) -> Result<()> {
        if !self.config.enable_prefetch {
            return Ok(());
        }
        
        // Get access pattern
        let pattern = if let Some(entry) = self.l2_cache.get(key) {
            entry.access_pattern.clone()
        } else if let Some(cached) = self.l1_cache.read().peek(key) {
            cached.access_pattern.clone()
        } else {
            return Ok(());
        };
        
        // Prefetch related contexts
        for tx_hash in pattern.related_txs.iter().take(3) {
            let related_key = CacheKey {
                tx_hash: *tx_hash,
                version: 0,
                agent_id: key.agent_id.clone(),
            };
            
            // Check if already in cache
            if self.get(&related_key).await.is_none() {
                // Would trigger loading from storage here
                debug!("Prefetching related context: {:?}", related_key);
            }
        }
        
        Ok(())
    }
}

/// LRU with TTL eviction policy
struct LRUWithTTL {
    ttl_weight: f64,
    access_weight: f64,
}

impl LRUWithTTL {
    fn new() -> Self {
        Self {
            ttl_weight: 0.6,
            access_weight: 0.4,
        }
    }
}

#[async_trait]
impl EvictionPolicy for LRUWithTTL {
    async fn should_evict(&self, metadata: &CacheMetadata, config: &CacheConfig) -> bool {
        let age = Instant::now().duration_since(metadata.inserted_at);
        age > config.ttl
    }
    
    fn eviction_priority(&self, metadata: &CacheMetadata) -> f64 {
        let age_score = Instant::now().duration_since(metadata.last_accessed).as_secs_f64();
        let access_score = 1.0 / (metadata.access_count as f64 + 1.0);
        
        age_score * self.ttl_weight + access_score * self.access_weight
    }
}

impl Clone for ContextCache {
    fn clone(&self) -> Self {
        Self {
            l1_cache: self.l1_cache.clone(),
            l2_cache: self.l2_cache.clone(),
            l3_cache: self.l3_cache.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
            eviction_policy: self.eviction_policy.clone(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 100,
            l2_size: 1000,
            l3_size: 10000,
            ttl: Duration::from_secs(3600), // 1 hour
            l3_compression: true,
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            enable_prefetch: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_operations() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);
        
        let key = CacheKey {
            tx_hash: [1u8; 32],
            version: 0,
            agent_id: None,
        };
        
        // Test miss
        assert!(cache.get(&key).await.is_none());
        
        // Test put and get
        let context = PreprocessedContext {
            tx_hash: [1u8; 32],
            chunks: vec![],
            metadata: Default::default(),
            timestamp: 0,
            processing_time_ms: 0,
        };
        
        cache.put(key.clone(), context.clone()).await.unwrap();
        assert!(cache.get(&key).await.is_some());
        
        // Test invalidate
        cache.invalidate(&key).await.unwrap();
        assert!(cache.get(&key).await.is_none());
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let config = CacheConfig::default();
        let cache = ContextCache::new(config);
        
        let stats = cache.get_stats().await;
        assert_eq!(stats.total_hits, 0);
        assert_eq!(stats.total_misses, 0);
    }
}