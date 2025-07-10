//! Context Pre-processing for RAG ExEx
//! 
//! This module provides real-time context preparation for SVM transactions,
//! including document chunking, metadata extraction, and indexing.

use crate::errors::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Context pre-processor for preparing transaction data for RAG
pub struct ContextPreprocessor {
    /// Chunking strategy
    chunker: Arc<dyn ChunkingStrategy>,
    /// Metadata extractors
    extractors: Vec<Arc<dyn MetadataExtractor>>,
    /// Context enrichers
    enrichers: Vec<Arc<dyn ContextEnricher>>,
    /// Processing configuration
    config: PreprocessingConfig,
    /// Metrics collector
    metrics: Arc<Mutex<PreprocessingMetrics>>,
}

/// Configuration for context preprocessing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessingConfig {
    /// Maximum chunk size in tokens
    pub max_chunk_size: usize,
    /// Chunk overlap in tokens
    pub chunk_overlap: usize,
    /// Enable parallel processing
    pub parallel_processing: bool,
    /// Maximum processing time per document (ms)
    pub timeout_ms: u64,
    /// Enable caching
    pub enable_cache: bool,
    /// Metadata extraction depth
    pub metadata_depth: MetadataDepth,
}

/// Depth of metadata extraction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MetadataDepth {
    /// Basic metadata only
    Basic,
    /// Include transaction details
    Detailed,
    /// Full analysis including patterns
    Comprehensive,
}

/// Preprocessed context ready for embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreprocessedContext {
    /// Original transaction hash
    pub tx_hash: [u8; 32],
    /// Processed chunks
    pub chunks: Vec<ContextChunk>,
    /// Extracted metadata
    pub metadata: ContextMetadata,
    /// Processing timestamp
    pub timestamp: u64,
    /// Processing duration
    pub processing_time_ms: u64,
}

/// Individual context chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextChunk {
    /// Unique chunk identifier
    pub id: String,
    /// Chunk content
    pub content: String,
    /// Token count
    pub token_count: usize,
    /// Position in original document
    pub position: ChunkPosition,
    /// Chunk-specific metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Relevance score (if pre-scored)
    pub relevance_score: Option<f64>,
}

/// Position information for a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkPosition {
    /// Start offset in original document
    pub start: usize,
    /// End offset in original document
    pub end: usize,
    /// Chunk index
    pub index: usize,
    /// Total chunks in document
    pub total_chunks: usize,
}

/// Metadata extracted from context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// Transaction type
    pub tx_type: TransactionType,
    /// Involved contracts
    pub contracts: Vec<ContractInfo>,
    /// Detected patterns
    pub patterns: Vec<PatternMatch>,
    /// Entity mentions
    pub entities: Vec<EntityMention>,
    /// Temporal references
    pub temporal_refs: Vec<TemporalReference>,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Transaction type classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Token transfer
    Transfer,
    /// Smart contract deployment
    Deployment,
    /// Contract interaction
    ContractCall,
    /// Cross-program invocation
    CrossProgram,
    /// System transaction
    System,
    /// Unknown type
    Unknown,
}

/// Contract information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    /// Contract address
    pub address: [u8; 32],
    /// Contract name (if known)
    pub name: Option<String>,
    /// Method called
    pub method: Option<String>,
    /// Interaction type
    pub interaction_type: String,
}

/// Pattern match in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMatch {
    /// Pattern type
    pub pattern_type: String,
    /// Pattern value
    pub value: String,
    /// Confidence score
    pub confidence: f64,
    /// Location in context
    pub location: (usize, usize),
}

/// Entity mention in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMention {
    /// Entity type (address, token, etc.)
    pub entity_type: String,
    /// Entity value
    pub value: String,
    /// Mention context
    pub context: String,
    /// Sentiment (if applicable)
    pub sentiment: Option<f32>,
}

/// Temporal reference in context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalReference {
    /// Reference type (absolute, relative)
    pub ref_type: String,
    /// Parsed value
    pub value: String,
    /// Unix timestamp (if resolved)
    pub timestamp: Option<u64>,
}

/// Trait for chunking strategies
#[async_trait]
pub trait ChunkingStrategy: Send + Sync {
    /// Chunk the input content
    async fn chunk(&self, content: &str, config: &PreprocessingConfig) -> Result<Vec<ContextChunk>>;
    
    /// Get strategy name
    fn name(&self) -> &str;
}

/// Trait for metadata extraction
#[async_trait]
pub trait MetadataExtractor: Send + Sync {
    /// Extract metadata from content
    async fn extract(&self, content: &str, tx_data: &[u8]) -> Result<HashMap<String, serde_json::Value>>;
    
    /// Get extractor name
    fn name(&self) -> &str;
}

/// Trait for context enrichment
#[async_trait]
pub trait ContextEnricher: Send + Sync {
    /// Enrich context with additional information
    async fn enrich(&self, context: &mut PreprocessedContext) -> Result<()>;
    
    /// Get enricher name
    fn name(&self) -> &str;
}

/// Processing metrics
#[derive(Debug, Default)]
pub struct PreprocessingMetrics {
    /// Total contexts processed
    pub total_processed: u64,
    /// Average processing time
    pub avg_processing_time_ms: f64,
    /// Chunks per context
    pub avg_chunks_per_context: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

impl ContextPreprocessor {
    /// Create a new context preprocessor
    pub fn new(config: PreprocessingConfig) -> Self {
        Self {
            chunker: Arc::new(TokenBasedChunker::new()),
            extractors: vec![
                Arc::new(TransactionMetadataExtractor),
                Arc::new(ContractMetadataExtractor),
                Arc::new(PatternMetadataExtractor),
            ],
            enrichers: vec![
                Arc::new(EntityEnricher),
                Arc::new(TemporalEnricher),
            ],
            config,
            metrics: Arc::new(Mutex::new(PreprocessingMetrics::default())),
        }
    }
    
    /// Process transaction data into preprocessed context
    pub async fn process_transaction(
        &self,
        tx_hash: [u8; 32],
        tx_data: &[u8],
        additional_context: Option<&str>,
    ) -> Result<PreprocessedContext> {
        let start = std::time::Instant::now();
        
        // Convert transaction data to string representation
        let content = self.transaction_to_string(tx_data)?;
        
        // Add additional context if provided
        let full_content = if let Some(additional) = additional_context {
            format!("{}\n\n{}", content, additional)
        } else {
            content
        };
        
        // Chunk the content
        let chunks = self.chunker.chunk(&full_content, &self.config).await?;
        
        // Extract metadata
        let mut metadata = ContextMetadata {
            tx_type: TransactionType::Unknown,
            contracts: vec![],
            patterns: vec![],
            entities: vec![],
            temporal_refs: vec![],
            custom: HashMap::new(),
        };
        
        // Run metadata extractors
        if self.config.parallel_processing {
            let extraction_futures: Vec<_> = self.extractors.iter()
                .map(|extractor| extractor.extract(&full_content, tx_data))
                .collect();
            
            let results = futures::future::join_all(extraction_futures).await;
            
            for (i, result) in results.into_iter().enumerate() {
                if let Ok(extracted) = result {
                    debug!("Metadata from {}: {:?}", self.extractors[i].name(), extracted);
                    metadata.custom.extend(extracted);
                }
            }
        } else {
            for extractor in &self.extractors {
                if let Ok(extracted) = extractor.extract(&full_content, tx_data).await {
                    metadata.custom.extend(extracted);
                }
            }
        }
        
        // Create preprocessed context
        let mut context = PreprocessedContext {
            tx_hash,
            chunks,
            metadata,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            processing_time_ms: 0,
        };
        
        // Run enrichers
        for enricher in &self.enrichers {
            enricher.enrich(&mut context).await?;
        }
        
        // Update processing time
        context.processing_time_ms = start.elapsed().as_millis() as u64;
        
        // Update metrics
        self.update_metrics(&context).await;
        
        Ok(context)
    }
    
    /// Convert transaction data to string representation
    fn transaction_to_string(&self, tx_data: &[u8]) -> Result<String> {
        // For now, use hex representation with structure
        let hex_str = hex::encode(tx_data);
        
        // Try to parse as Solana transaction
        if let Ok(tx) = bincode::deserialize::<SvmTransactionInfo>(tx_data) {
            Ok(format!(
                "Transaction: {}\nFrom: {}\nTo: {:?}\nValue: {}\nData: {}",
                hex::encode(&tx.signature),
                hex::encode(&tx.from),
                tx.to.map(|t| hex::encode(&t)),
                tx.value,
                hex_str
            ))
        } else {
            // Fallback to hex representation
            Ok(format!("Raw transaction data: {}", hex_str))
        }
    }
    
    /// Update processing metrics
    async fn update_metrics(&self, context: &PreprocessedContext) {
        let mut metrics = self.metrics.lock().await;
        metrics.total_processed += 1;
        
        // Update rolling averages
        let n = metrics.total_processed as f64;
        metrics.avg_processing_time_ms = 
            (metrics.avg_processing_time_ms * (n - 1.0) + context.processing_time_ms as f64) / n;
        metrics.avg_chunks_per_context = 
            (metrics.avg_chunks_per_context * (n - 1.0) + context.chunks.len() as f64) / n;
    }
    
    /// Get current metrics
    pub async fn get_metrics(&self) -> PreprocessingMetrics {
        self.metrics.lock().await.clone()
    }
}

/// Token-based chunking strategy
struct TokenBasedChunker {
    /// Tokenizer (simplified for now)
    max_tokens: usize,
}

impl TokenBasedChunker {
    fn new() -> Self {
        Self {
            max_tokens: 512, // Default max tokens
        }
    }
}

#[async_trait]
impl ChunkingStrategy for TokenBasedChunker {
    async fn chunk(&self, content: &str, config: &PreprocessingConfig) -> Result<Vec<ContextChunk>> {
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut chunks = Vec::new();
        
        let chunk_size = config.max_chunk_size.min(self.max_tokens);
        let overlap = config.chunk_overlap;
        
        let mut start = 0;
        let mut chunk_index = 0;
        
        while start < words.len() {
            let end = (start + chunk_size).min(words.len());
            let chunk_content = words[start..end].join(" ");
            
            chunks.push(ContextChunk {
                id: format!("chunk_{}", chunk_index),
                content: chunk_content,
                token_count: end - start,
                position: ChunkPosition {
                    start,
                    end,
                    index: chunk_index,
                    total_chunks: 0, // Will be updated
                },
                metadata: HashMap::new(),
                relevance_score: None,
            });
            
            chunk_index += 1;
            start = if start + chunk_size >= words.len() {
                words.len()
            } else {
                start + chunk_size - overlap
            };
        }
        
        // Update total chunks
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.position.total_chunks = total;
        }
        
        Ok(chunks)
    }
    
    fn name(&self) -> &str {
        "TokenBasedChunker"
    }
}

/// Transaction metadata extractor
struct TransactionMetadataExtractor;

#[async_trait]
impl MetadataExtractor for TransactionMetadataExtractor {
    async fn extract(&self, _content: &str, tx_data: &[u8]) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();
        
        // Extract basic transaction info
        metadata.insert(
            "data_size".to_string(),
            serde_json::json!(tx_data.len())
        );
        
        // Try to detect transaction type
        if tx_data.starts_with(b"transfer") {
            metadata.insert(
                "detected_type".to_string(),
                serde_json::json!("transfer")
            );
        }
        
        Ok(metadata)
    }
    
    fn name(&self) -> &str {
        "TransactionMetadataExtractor"
    }
}

/// Contract metadata extractor
struct ContractMetadataExtractor;

#[async_trait]
impl MetadataExtractor for ContractMetadataExtractor {
    async fn extract(&self, content: &str, _tx_data: &[u8]) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();
        
        // Simple contract detection (would be more sophisticated in production)
        if content.contains("contract") || content.contains("program") {
            metadata.insert(
                "has_contract_interaction".to_string(),
                serde_json::json!(true)
            );
        }
        
        Ok(metadata)
    }
    
    fn name(&self) -> &str {
        "ContractMetadataExtractor"
    }
}

/// Pattern metadata extractor
struct PatternMetadataExtractor;

#[async_trait]
impl MetadataExtractor for PatternMetadataExtractor {
    async fn extract(&self, content: &str, _tx_data: &[u8]) -> Result<HashMap<String, serde_json::Value>> {
        let mut metadata = HashMap::new();
        
        // Detect common patterns
        let patterns = vec![
            ("swap", "DeFi swap operation"),
            ("stake", "Staking operation"),
            ("mint", "Token minting"),
            ("burn", "Token burning"),
            ("vote", "Governance voting"),
        ];
        
        let mut detected_patterns = Vec::new();
        for (pattern, description) in patterns {
            if content.to_lowercase().contains(pattern) {
                detected_patterns.push(json!({
                    "pattern": pattern,
                    "description": description
                }));
            }
        }
        
        if !detected_patterns.is_empty() {
            metadata.insert(
                "detected_patterns".to_string(),
                serde_json::json!(detected_patterns)
            );
        }
        
        Ok(metadata)
    }
    
    fn name(&self) -> &str {
        "PatternMetadataExtractor"
    }
}

/// Entity enricher
struct EntityEnricher;

#[async_trait]
impl ContextEnricher for EntityEnricher {
    async fn enrich(&self, context: &mut PreprocessedContext) -> Result<()> {
        // Extract entities from chunks
        for chunk in &context.chunks {
            // Simple address detection (32-byte values)
            let hex_pattern = regex::Regex::new(r"[0-9a-fA-F]{64}").unwrap();
            for capture in hex_pattern.captures_iter(&chunk.content) {
                if let Some(matched) = capture.get(0) {
                    context.metadata.entities.push(EntityMention {
                        entity_type: "address".to_string(),
                        value: matched.as_str().to_string(),
                        context: chunk.content.clone(),
                        sentiment: None,
                    });
                }
            }
        }
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "EntityEnricher"
    }
}

/// Temporal enricher
struct TemporalEnricher;

#[async_trait]
impl ContextEnricher for TemporalEnricher {
    async fn enrich(&self, context: &mut PreprocessedContext) -> Result<()> {
        // Add current timestamp as a temporal reference
        context.metadata.temporal_refs.push(TemporalReference {
            ref_type: "processing_time".to_string(),
            value: "now".to_string(),
            timestamp: Some(context.timestamp),
        });
        
        Ok(())
    }
    
    fn name(&self) -> &str {
        "TemporalEnricher"
    }
}

/// Simplified SVM transaction info for parsing
#[derive(Debug, Serialize, Deserialize)]
struct SvmTransactionInfo {
    signature: [u8; 64],
    from: [u8; 32],
    to: Option<[u8; 32]>,
    value: u64,
}

impl Default for PreprocessingConfig {
    fn default() -> Self {
        Self {
            max_chunk_size: 512,
            chunk_overlap: 50,
            parallel_processing: true,
            timeout_ms: 5000,
            enable_cache: true,
            metadata_depth: MetadataDepth::Detailed,
        }
    }
}

impl Clone for PreprocessingMetrics {
    fn clone(&self) -> Self {
        Self {
            total_processed: self.total_processed,
            avg_processing_time_ms: self.avg_processing_time_ms,
            avg_chunks_per_context: self.avg_chunks_per_context,
            cache_hit_rate: self.cache_hit_rate,
        }
    }
}

use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_context_preprocessing() {
        let config = PreprocessingConfig::default();
        let preprocessor = ContextPreprocessor::new(config);
        
        let tx_hash = [0u8; 32];
        let tx_data = b"test transaction data";
        
        let result = preprocessor.process_transaction(
            tx_hash,
            tx_data,
            Some("Additional context about the transaction")
        ).await;
        
        assert!(result.is_ok());
        let context = result.unwrap();
        assert!(!context.chunks.is_empty());
        assert_eq!(context.tx_hash, tx_hash);
    }

    #[tokio::test]
    async fn test_token_chunking() {
        let chunker = TokenBasedChunker::new();
        let config = PreprocessingConfig {
            max_chunk_size: 10,
            chunk_overlap: 2,
            ..Default::default()
        };
        
        let content = "This is a test content for chunking with overlap to test the chunking strategy";
        let chunks = chunker.chunk(content, &config).await.unwrap();
        
        assert!(chunks.len() > 1);
        assert!(chunks[0].token_count <= 10);
    }
}