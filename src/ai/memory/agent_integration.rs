//! Agent Memory Integration for RAG ExEx
//! 
//! This module connects agent memories to RAG context retrieval,
//! enabling persistent learning and context-aware decision making.

use crate::errors::*;
use crate::ai::context::preprocessing::{PreprocessedContext, ContextMetadata};
use crate::ai::traits::{ContextData, EmbeddingData};
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Query engine for memory operations
pub trait MemoryQueryEngine: Send + Sync {
    /// Query memories based on criteria
    fn query(&self, query: &str) -> Vec<MemoryItem>;
}

/// Status of a memory item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStatus {
    /// Active and available
    Active,
    /// Archived but accessible
    Archived,
    /// Pending consolidation
    Pending,
    /// Marked for deletion
    Deleted,
}

/// Agent memory system with hierarchical storage
pub struct AgentMemory {
    /// Short-term memory (working memory)
    short_term: Arc<RwLock<ShortTermMemory>>,
    /// Long-term memory (persistent storage)
    long_term: Arc<LongTermMemory>,
    /// Episodic memory (transaction sequences)
    episodic: Arc<Mutex<EpisodicMemory>>,
    /// Semantic memory (learned concepts)
    semantic: Arc<SemanticMemory>,
    /// Memory configuration
    config: MemoryConfig,
    /// Memory consolidation engine
    consolidator: Arc<MemoryConsolidator>,
}

/// Configuration for agent memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Short-term memory capacity
    pub short_term_capacity: usize,
    /// Long-term memory size limit (MB)
    pub long_term_size_mb: usize,
    /// Episode retention count
    pub episode_retention: usize,
    /// Consolidation interval (seconds)
    pub consolidation_interval: u64,
    /// Enable memory compression
    pub enable_compression: bool,
    /// Memory decay rate
    pub decay_rate: f64,
}

/// Short-term memory for immediate context
#[derive(Debug)]
pub struct ShortTermMemory {
    /// Active memories
    memories: VecDeque<MemoryItem>,
    /// Attention weights
    attention: HashMap<String, f64>,
    /// Working context
    working_context: Option<WorkingContext>,
}

/// Long-term memory with vector storage
pub struct LongTermMemory {
    /// Memory index by ID
    index: DashMap<String, MemoryEntry>,
    /// Vector embeddings
    embeddings: Arc<RwLock<HashMap<String, Vec<f32>>>>,
    /// Memory categories
    categories: DashMap<String, Vec<String>>,
    /// Access patterns
    access_patterns: Arc<Mutex<AccessPatterns>>,
}

/// Episodic memory for transaction sequences
#[derive(Debug)]
pub struct EpisodicMemory {
    /// Transaction episodes
    episodes: VecDeque<Episode>,
    /// Episode index by context
    context_index: HashMap<String, Vec<usize>>,
    /// Pattern recognition
    patterns: Vec<EpisodePattern>,
}

/// Semantic memory for learned concepts
pub struct SemanticMemory {
    /// Concept graph
    concepts: DashMap<String, Concept>,
    /// Relationships between concepts
    relationships: DashMap<(String, String), Relationship>,
    /// Inference rules
    rules: Vec<InferenceRule>,
}

/// Individual memory item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryItem {
    /// Unique identifier
    pub id: String,
    /// Memory content
    pub content: String,
    /// Associated transaction
    pub tx_hash: Option<[u8; 32]>,
    /// Importance score
    pub importance: f64,
    /// Creation timestamp
    pub created_at: u64,
    /// Last accessed
    pub last_accessed: u64,
    /// Access count
    pub access_count: u32,
}

/// Long-term memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Memory item
    pub item: MemoryItem,
    /// Embedding vector
    pub embedding_id: String,
    /// Memory strength
    pub strength: f64,
    /// Associations
    pub associations: Vec<String>,
    /// Context tags
    pub tags: Vec<String>,
}

/// Working context for active processing
#[derive(Debug, Clone)]
pub struct WorkingContext {
    /// Current focus
    pub focus: Vec<String>,
    /// Active goals
    pub goals: Vec<Goal>,
    /// Context state
    pub state: HashMap<String, serde_json::Value>,
}

/// Goal representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    /// Goal ID
    pub id: String,
    /// Goal description
    pub description: String,
    /// Priority
    pub priority: f64,
    /// Progress
    pub progress: f64,
}

/// Transaction episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Episode ID
    pub id: String,
    /// Transaction sequence
    pub transactions: Vec<TransactionMemory>,
    /// Episode outcome
    pub outcome: EpisodeOutcome,
    /// Duration
    pub duration_ms: u64,
    /// Learned insights
    pub insights: Vec<String>,
}

/// Transaction memory within episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMemory {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Decision made
    pub decision: String,
    /// Context snapshot
    pub context: ContextSnapshot,
    /// Result
    pub result: TransactionResult,
}

/// Context snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSnapshot {
    /// Key context elements
    pub elements: HashMap<String, serde_json::Value>,
    /// Active memories at time
    pub active_memories: Vec<String>,
}

/// Transaction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionResult {
    /// Success status
    pub success: bool,
    /// Performance metrics
    pub metrics: HashMap<String, f64>,
    /// Side effects
    pub side_effects: Vec<String>,
}

/// Episode outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EpisodeOutcome {
    Success,
    PartialSuccess,
    Failure,
    Unknown,
}

/// Episode pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodePattern {
    /// Pattern ID
    pub id: String,
    /// Pattern template
    pub template: Vec<PatternElement>,
    /// Success rate
    pub success_rate: f64,
    /// Occurrence count
    pub occurrences: u32,
}

/// Pattern element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternElement {
    /// Element type
    pub element_type: String,
    /// Constraints
    pub constraints: HashMap<String, serde_json::Value>,
}

/// Concept in semantic memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Concept {
    /// Concept ID
    pub id: String,
    /// Concept name
    pub name: String,
    /// Description
    pub description: String,
    /// Properties
    pub properties: HashMap<String, serde_json::Value>,
    /// Activation level
    pub activation: f64,
}

/// Relationship between concepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    /// Relationship type
    pub rel_type: RelationshipType,
    /// Strength
    pub strength: f64,
    /// Bidirectional
    pub bidirectional: bool,
}

/// Relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    IsA,
    HasA,
    Causes,
    Prevents,
    Requires,
    Similar,
    Opposite,
}

/// Inference rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRule {
    /// Rule ID
    pub id: String,
    /// Conditions
    pub conditions: Vec<Condition>,
    /// Conclusions
    pub conclusions: Vec<Conclusion>,
    /// Confidence
    pub confidence: f64,
}

/// Rule condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// Condition type
    pub cond_type: String,
    /// Parameters
    pub params: HashMap<String, serde_json::Value>,
}

/// Rule conclusion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conclusion {
    /// Action type
    pub action: String,
    /// Parameters
    pub params: HashMap<String, serde_json::Value>,
}

/// Access patterns for optimization
#[derive(Debug, Default)]
pub struct AccessPatterns {
    /// Frequency map
    pub frequency: HashMap<String, u32>,
    /// Co-access patterns
    pub co_access: HashMap<(String, String), u32>,
    /// Temporal patterns
    pub temporal: Vec<TemporalPattern>,
}

/// Temporal access pattern
#[derive(Debug, Clone)]
pub struct TemporalPattern {
    /// Pattern ID
    pub id: String,
    /// Time series
    pub series: Vec<(u64, String)>,
    /// Periodicity
    pub period: Option<u64>,
}

/// Memory consolidator
pub struct MemoryConsolidator {
    /// Consolidation rules
    rules: Vec<ConsolidationRule>,
    /// Compression engine
    compressor: Arc<dyn MemoryCompressor>,
}

/// Consolidation rule
#[derive(Debug, Clone)]
pub struct ConsolidationRule {
    /// Rule ID
    pub id: String,
    /// Source memory type
    pub source: MemoryType,
    /// Target memory type
    pub target: MemoryType,
    /// Conditions
    pub conditions: Vec<Condition>,
}

/// Memory types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    ShortTerm,
    LongTerm,
    Episodic,
    Semantic,
}

/// Trait for memory compression
#[async_trait]
pub trait MemoryCompressor: Send + Sync {
    /// Compress memories
    async fn compress(&self, memories: Vec<MemoryItem>) -> Result<CompressedMemory>;
    
    /// Decompress memory
    async fn decompress(&self, compressed: &CompressedMemory) -> Result<Vec<MemoryItem>>;
}

/// Compressed memory representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedMemory {
    /// Compression type
    pub compression_type: String,
    /// Compressed data
    pub data: Vec<u8>,
    /// Original count
    pub original_count: usize,
    /// Compression ratio
    pub ratio: f64,
}

impl AgentMemory {
    /// Create new agent memory system
    pub fn new(agent_id: String, config: MemoryConfig) -> Self {
        Self {
            short_term: Arc::new(RwLock::new(ShortTermMemory {
                memories: VecDeque::with_capacity(config.short_term_capacity),
                attention: HashMap::new(),
                working_context: None,
            })),
            long_term: Arc::new(LongTermMemory {
                index: DashMap::new(),
                embeddings: Arc::new(RwLock::new(HashMap::new())),
                categories: DashMap::new(),
                access_patterns: Arc::new(Mutex::new(AccessPatterns::default())),
            }),
            episodic: Arc::new(Mutex::new(EpisodicMemory {
                episodes: VecDeque::with_capacity(config.episode_retention),
                context_index: HashMap::new(),
                patterns: Vec::new(),
            })),
            semantic: Arc::new(SemanticMemory {
                concepts: DashMap::new(),
                relationships: DashMap::new(),
                rules: Vec::new(),
            }),
            config,
            consolidator: Arc::new(MemoryConsolidator {
                rules: vec![],
                compressor: Arc::new(SimpleCompressor),
            }),
        }
    }
    
    /// Store memory from preprocessed context
    pub async fn store_from_context(
        &self,
        context: &PreprocessedContext,
        importance: f64,
    ) -> Result<String> {
        let memory_id = uuid::Uuid::new_v4().to_string();
        
        // Create memory item
        let memory = MemoryItem {
            id: memory_id.clone(),
            content: self.summarize_context(context),
            tx_hash: Some(context.tx_hash),
            importance,
            created_at: context.timestamp,
            last_accessed: context.timestamp,
            access_count: 0,
        };
        
        // Store in short-term memory
        {
            let mut stm = self.short_term.write().await;
            stm.memories.push_back(memory.clone());
            
            // Evict old memories if over capacity
            while stm.memories.len() > self.config.short_term_capacity {
                if let Some(old_memory) = stm.memories.pop_front() {
                    // Move to long-term if important enough
                    if old_memory.importance > 0.5 {
                        self.transfer_to_long_term(old_memory).await?;
                    }
                }
            }
        }
        
        Ok(memory_id)
    }
    
    /// Retrieve memories relevant to a query
    pub async fn retrieve(
        &self,
        query: &str,
        limit: usize,
        include_types: Vec<MemoryType>,
    ) -> Result<Vec<MemoryItem>> {
        let mut results = Vec::new();
        
        // Search short-term memory
        if include_types.contains(&MemoryType::ShortTerm) {
            let stm = self.short_term.read().await;
            for memory in &stm.memories {
                if memory.content.contains(query) {
                    results.push(memory.clone());
                }
            }
        }
        
        // Search long-term memory
        if include_types.contains(&MemoryType::LongTerm) {
            for entry in self.long_term.index.iter() {
                if entry.value().item.content.contains(query) {
                    results.push(entry.value().item.clone());
                }
            }
        }
        
        // Sort by relevance and recency
        results.sort_by(|a, b| {
            let score_a = a.importance * (a.access_count as f64 + 1.0);
            let score_b = b.importance * (b.access_count as f64 + 1.0);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        Ok(results.into_iter().take(limit).collect())
    }
    
    /// Update working context
    pub async fn update_working_context(&self, focus: Vec<String>, goals: Vec<Goal>) -> Result<()> {
        let mut stm = self.short_term.write().await;
        stm.working_context = Some(WorkingContext {
            focus,
            goals,
            state: HashMap::new(),
        });
        Ok(())
    }
    
    /// Record transaction episode
    pub async fn record_episode(&self, episode: Episode) -> Result<()> {
        let mut episodic = self.episodic.lock().await;
        
        // Add to episodes
        episodic.episodes.push_back(episode.clone());
        
        // Maintain size limit
        while episodic.episodes.len() > self.config.episode_retention {
            episodic.episodes.pop_front();
        }
        
        // Update pattern recognition
        self.extract_patterns(&episode, &mut episodic.patterns).await?;
        
        Ok(())
    }
    
    /// Learn concept
    pub async fn learn_concept(&self, concept: Concept) -> Result<()> {
        self.semantic.concepts.insert(concept.id.clone(), concept);
        Ok(())
    }
    
    /// Add relationship between concepts
    pub async fn add_relationship(
        &self,
        concept1: String,
        concept2: String,
        relationship: Relationship,
    ) -> Result<()> {
        self.semantic.relationships.insert((concept1, concept2), relationship);
        Ok(())
    }
    
    /// Get memory statistics
    pub async fn get_stats(&self) -> MemoryStats {
        let stm = self.short_term.read().await;
        
        MemoryStats {
            short_term_count: stm.memories.len(),
            long_term_count: self.long_term.index.len(),
            episode_count: self.episodic.lock().await.episodes.len(),
            concept_count: self.semantic.concepts.len(),
            total_memory_mb: 0, // Would calculate actual usage
        }
    }
    
    /// Consolidate memories
    pub async fn consolidate(&self) -> Result<()> {
        info!("Starting memory consolidation");
        
        // Move important short-term memories to long-term
        let stm_memories: Vec<MemoryItem> = {
            let stm = self.short_term.read().await;
            stm.memories.iter()
                .filter(|m| m.importance > 0.7)
                .cloned()
                .collect()
        };
        
        for memory in stm_memories {
            self.transfer_to_long_term(memory).await?;
        }
        
        // Extract semantic knowledge from episodes
        let episodes = self.episodic.lock().await.episodes.clone();
        for episode in episodes.iter().rev().take(10) {
            self.extract_semantic_knowledge(episode).await?;
        }
        
        Ok(())
    }
    
    /// Summarize context for memory storage
    fn summarize_context(&self, context: &PreprocessedContext) -> String {
        format!(
            "Transaction {} with {} chunks, type: {:?}",
            hex::encode(&context.tx_hash),
            context.chunks.len(),
            context.metadata.tx_type
        )
    }
    
    /// Transfer memory to long-term storage
    async fn transfer_to_long_term(&self, memory: MemoryItem) -> Result<()> {
        let embedding_id = format!("emb_{}", memory.id);
        
        let entry = MemoryEntry {
            item: memory,
            embedding_id: embedding_id.clone(),
            strength: 1.0,
            associations: vec![],
            tags: vec![],
        };
        
        self.long_term.index.insert(entry.item.id.clone(), entry);
        
        // Would generate actual embedding here
        self.long_term.embeddings.write().await.insert(
            embedding_id,
            vec![0.0; 384], // Placeholder embedding
        );
        
        Ok(())
    }
    
    /// Extract patterns from episode
    async fn extract_patterns(&self, episode: &Episode, patterns: &mut Vec<EpisodePattern>) -> Result<()> {
        // Simple pattern extraction - would be more sophisticated
        if episode.outcome == EpisodeOutcome::Success && episode.transactions.len() > 1 {
            let pattern = EpisodePattern {
                id: uuid::Uuid::new_v4().to_string(),
                template: vec![],
                success_rate: 1.0,
                occurrences: 1,
            };
            patterns.push(pattern);
        }
        Ok(())
    }
    
    /// Extract semantic knowledge from episode
    async fn extract_semantic_knowledge(&self, episode: &Episode) -> Result<()> {
        // Extract concepts from successful episodes
        if episode.outcome == EpisodeOutcome::Success {
            for insight in &episode.insights {
                let concept = Concept {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: format!("Insight_{}", &insight[..20.min(insight.len())]),
                    description: insight.clone(),
                    properties: HashMap::new(),
                    activation: 0.5,
                };
                self.semantic.concepts.insert(concept.id.clone(), concept);
            }
        }
        Ok(())
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub short_term_count: usize,
    pub long_term_count: usize,
    pub episode_count: usize,
    pub concept_count: usize,
    pub total_memory_mb: usize,
}

/// Simple memory compressor
struct SimpleCompressor;

#[async_trait]
impl MemoryCompressor for SimpleCompressor {
    async fn compress(&self, memories: Vec<MemoryItem>) -> Result<CompressedMemory> {
        let data = bincode::serialize(&memories)?;
        Ok(CompressedMemory {
            compression_type: "simple".to_string(),
            data,
            original_count: memories.len(),
            ratio: 1.0,
        })
    }
    
    async fn decompress(&self, compressed: &CompressedMemory) -> Result<Vec<MemoryItem>> {
        Ok(bincode::deserialize(&compressed.data)?)
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_capacity: 100,
            long_term_size_mb: 1024,
            episode_retention: 1000,
            consolidation_interval: 300,
            enable_compression: true,
            decay_rate: 0.95,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::context::preprocessing::*;

    #[tokio::test]
    async fn test_memory_storage() {
        let config = MemoryConfig::default();
        let memory = AgentMemory::new("test-agent".to_string(), config);
        
        let context = PreprocessedContext {
            tx_hash: [1u8; 32],
            chunks: vec![],
            metadata: ContextMetadata {
                tx_type: TransactionType::Transfer,
                contracts: vec![],
                patterns: vec![],
                entities: vec![],
                temporal_refs: vec![],
                custom: HashMap::new(),
            },
            timestamp: 1000,
            processing_time_ms: 10,
        };
        
        let memory_id = memory.store_from_context(&context, 0.8).await.unwrap();
        assert!(!memory_id.is_empty());
        
        // Test retrieval
        let results = memory.retrieve("Transaction", 10, vec![MemoryType::ShortTerm]).await.unwrap();
        assert!(!results.is_empty());
    }

    #[tokio::test]
    async fn test_episode_recording() {
        let config = MemoryConfig::default();
        let memory = AgentMemory::new("test-agent".to_string(), config);
        
        let episode = Episode {
            id: "ep1".to_string(),
            transactions: vec![],
            outcome: EpisodeOutcome::Success,
            duration_ms: 100,
            insights: vec!["Test insight".to_string()],
        };
        
        memory.record_episode(episode).await.unwrap();
        
        let stats = memory.get_stats().await;
        assert_eq!(stats.episode_count, 1);
    }
}