//! Agent-specific Context for RAG
//! 
//! This module provides agent-aware context retrieval and ranking for RAG operations.

use crate::errors::*;
use crate::ai::context::preprocessing::{PreprocessedContext, ContextChunk};
use crate::ai::memory::agent_integration::{AgentMemory, MemoryItem, Episode};
use crate::ai::traits::{ContextData, EmbeddingData};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Agent-aware RAG context provider
pub struct AgentContextProvider {
    /// Agent memories by ID
    agent_memories: Arc<RwLock<HashMap<String, Arc<AgentMemory>>>>,
    /// Context ranking engine
    ranker: Arc<ContextRanker>,
    /// Context fusion engine
    fusion: Arc<ContextFusion>,
    /// Configuration
    config: AgentContextConfig,
}

/// Configuration for agent context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContextConfig {
    /// Maximum contexts to retrieve
    pub max_contexts: usize,
    /// Minimum relevance score
    pub min_relevance: f64,
    /// Enable memory-based ranking
    pub use_memory_ranking: bool,
    /// Enable temporal weighting
    pub temporal_weighting: bool,
    /// Context fusion strategy
    pub fusion_strategy: FusionStrategy,
}

/// Context fusion strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FusionStrategy {
    /// Simple concatenation
    Concatenate,
    /// Weighted averaging
    WeightedAverage,
    /// Hierarchical merging
    Hierarchical,
    /// Attention-based fusion
    Attention,
}

/// Agent-specific context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Agent ID
    pub agent_id: String,
    /// Retrieved contexts
    pub contexts: Vec<RankedContext>,
    /// Agent memories
    pub memories: Vec<MemoryItem>,
    /// Recent episodes
    pub episodes: Vec<Episode>,
    /// Context metadata
    pub metadata: ContextMetadata,
}

/// Ranked context with relevance score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedContext {
    /// The context data
    pub context: ContextData,
    /// Relevance score
    pub relevance: f64,
    /// Source information
    pub source: ContextSource,
    /// Ranking factors
    pub factors: RankingFactors,
}

/// Context source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSource {
    /// Source type
    pub source_type: SourceType,
    /// Source ID
    pub source_id: String,
    /// Retrieval timestamp
    pub retrieved_at: u64,
}

/// Source types for context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    /// From vector database
    VectorDB,
    /// From agent memory
    Memory,
    /// From transaction history
    History,
    /// From knowledge graph
    KnowledgeGraph,
    /// From external API
    External,
}

/// Factors used in ranking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingFactors {
    /// Semantic similarity
    pub semantic_similarity: f64,
    /// Temporal relevance
    pub temporal_relevance: f64,
    /// Agent preference
    pub agent_preference: f64,
    /// Historical success
    pub historical_success: f64,
    /// Context quality
    pub quality_score: f64,
}

/// Context metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// Total contexts retrieved
    pub total_retrieved: usize,
    /// Contexts after filtering
    pub filtered_count: usize,
    /// Average relevance score
    pub avg_relevance: f64,
    /// Processing time
    pub processing_time_ms: u64,
}

/// Context ranking engine
pub struct ContextRanker {
    /// Ranking weights
    weights: RankingWeights,
    /// Historical performance
    performance_history: Arc<RwLock<HashMap<String, PerformanceMetrics>>>,
}

/// Ranking weights configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingWeights {
    pub semantic_weight: f64,
    pub temporal_weight: f64,
    pub preference_weight: f64,
    pub success_weight: f64,
    pub quality_weight: f64,
}

/// Performance metrics for ranking
#[derive(Debug, Clone, Default)]
pub struct PerformanceMetrics {
    pub total_uses: u64,
    pub successful_uses: u64,
    pub avg_impact: f64,
}

/// Context fusion engine
pub struct ContextFusion {
    /// Fusion configuration
    config: FusionConfig,
    /// Fusion models
    models: HashMap<FusionStrategy, Box<dyn FusionModel>>,
}

/// Fusion configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionConfig {
    /// Maximum fused context size
    pub max_size: usize,
    /// Deduplication threshold
    pub dedup_threshold: f64,
    /// Enable semantic compression
    pub semantic_compression: bool,
}

/// Trait for fusion models
#[async_trait]
pub trait FusionModel: Send + Sync {
    /// Fuse multiple contexts
    async fn fuse(&self, contexts: Vec<RankedContext>) -> Result<FusedContext>;
}

/// Fused context result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusedContext {
    /// Fused content
    pub content: String,
    /// Source contexts
    pub sources: Vec<String>,
    /// Fusion metadata
    pub metadata: FusionMetadata,
}

/// Fusion metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FusionMetadata {
    /// Fusion strategy used
    pub strategy: FusionStrategy,
    /// Information preserved ratio
    pub preservation_ratio: f64,
    /// Fusion confidence
    pub confidence: f64,
}

impl AgentContextProvider {
    /// Create a new agent context provider
    pub fn new(config: AgentContextConfig) -> Self {
        Self {
            agent_memories: Arc::new(RwLock::new(HashMap::new())),
            ranker: Arc::new(ContextRanker::new()),
            fusion: Arc::new(ContextFusion::new(FusionConfig::default())),
            config,
        }
    }
    
    /// Register agent memory
    pub async fn register_agent(&self, agent_id: String, memory: Arc<AgentMemory>) {
        self.agent_memories.write().await.insert(agent_id, memory);
    }
    
    /// Get agent-specific context
    pub async fn get_agent_context(
        &self,
        agent_id: &str,
        query: &str,
        base_contexts: Vec<ContextData>,
    ) -> Result<AgentContext> {
        let start = std::time::Instant::now();
        
        // Get agent memory if available
        let agent_memory = self.agent_memories.read().await.get(agent_id).cloned();
        
        // Retrieve agent memories
        let memories = if let Some(memory) = &agent_memory {
            memory.retrieve(
                query,
                self.config.max_contexts / 2,
                vec![crate::ai::memory::MemoryType::ShortTerm, crate::ai::memory::MemoryType::LongTerm],
            ).await?
        } else {
            vec![]
        };
        
        // Convert base contexts to ranked contexts
        let mut ranked_contexts = Vec::new();
        for (i, context) in base_contexts.into_iter().enumerate() {
            let ranking = self.ranker.rank_context(
                &context,
                query,
                agent_id,
                &memories,
            ).await?;
            
            ranked_contexts.push(RankedContext {
                context,
                relevance: ranking.calculate_score(),
                source: ContextSource {
                    source_type: SourceType::VectorDB,
                    source_id: format!("base_{}", i),
                    retrieved_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                },
                factors: ranking,
            });
        }
        
        // Add memory-based contexts
        for memory in &memories {
            let context = ContextData {
                id: memory.id.clone(),
                content: memory.content.clone(),
                embedding: vec![], // Would have actual embedding
                metadata: HashMap::from([
                    ("importance".to_string(), serde_json::json!(memory.importance)),
                    ("access_count".to_string(), serde_json::json!(memory.access_count)),
                ]),
                timestamp: memory.created_at,
            };
            
            let ranking = RankingFactors {
                semantic_similarity: 0.8, // Would calculate actual similarity
                temporal_relevance: self.calculate_temporal_relevance(memory.last_accessed),
                agent_preference: memory.importance,
                historical_success: memory.access_count as f64 / 100.0,
                quality_score: 0.9,
            };
            
            ranked_contexts.push(RankedContext {
                context,
                relevance: ranking.calculate_score(),
                source: ContextSource {
                    source_type: SourceType::Memory,
                    source_id: memory.id.clone(),
                    retrieved_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                },
                factors: ranking,
            });
        }
        
        // Sort by relevance
        ranked_contexts.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap());
        
        // Filter by minimum relevance
        let filtered: Vec<RankedContext> = ranked_contexts
            .into_iter()
            .filter(|rc| rc.relevance >= self.config.min_relevance)
            .take(self.config.max_contexts)
            .collect();
        
        let avg_relevance = if filtered.is_empty() {
            0.0
        } else {
            filtered.iter().map(|rc| rc.relevance).sum::<f64>() / filtered.len() as f64
        };
        
        Ok(AgentContext {
            agent_id: agent_id.to_string(),
            contexts: filtered,
            memories: memories.clone(),
            episodes: vec![], // Would fetch recent episodes
            metadata: ContextMetadata {
                total_retrieved: ranked_contexts.len(),
                filtered_count: filtered.len(),
                avg_relevance,
                processing_time_ms: start.elapsed().as_millis() as u64,
            },
        })
    }
    
    /// Fuse agent contexts
    pub async fn fuse_contexts(&self, contexts: Vec<RankedContext>) -> Result<FusedContext> {
        self.fusion.fuse(contexts, self.config.fusion_strategy).await
    }
    
    /// Update ranking based on feedback
    pub async fn update_ranking_feedback(
        &self,
        context_id: &str,
        agent_id: &str,
        success: bool,
        impact: f64,
    ) -> Result<()> {
        self.ranker.update_performance(context_id, agent_id, success, impact).await
    }
    
    /// Calculate temporal relevance
    fn calculate_temporal_relevance(&self, timestamp: u64) -> f64 {
        if !self.config.temporal_weighting {
            return 1.0;
        }
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let age_seconds = now.saturating_sub(timestamp);
        let age_hours = age_seconds as f64 / 3600.0;
        
        // Exponential decay with half-life of 24 hours
        0.5_f64.powf(age_hours / 24.0)
    }
}

impl ContextRanker {
    /// Create a new context ranker
    pub fn new() -> Self {
        Self {
            weights: RankingWeights {
                semantic_weight: 0.4,
                temporal_weight: 0.2,
                preference_weight: 0.2,
                success_weight: 0.1,
                quality_weight: 0.1,
            },
            performance_history: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Rank a context
    pub async fn rank_context(
        &self,
        context: &ContextData,
        query: &str,
        agent_id: &str,
        memories: &[MemoryItem],
    ) -> Result<RankingFactors> {
        // Calculate semantic similarity (simplified)
        let semantic_similarity = self.calculate_semantic_similarity(&context.content, query);
        
        // Calculate temporal relevance
        let temporal_relevance = 0.5_f64.powf(
            (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() - context.timestamp) as f64 / 86400.0
        );
        
        // Calculate agent preference based on similar memories
        let agent_preference = self.calculate_agent_preference(context, memories);
        
        // Get historical success rate
        let key = format!("{}:{}", agent_id, context.id);
        let historical_success = {
            let history = self.performance_history.read().await;
            history.get(&key)
                .map(|metrics| metrics.successful_uses as f64 / (metrics.total_uses as f64 + 1.0))
                .unwrap_or(0.5)
        };
        
        // Quality score (placeholder)
        let quality_score = 0.8;
        
        Ok(RankingFactors {
            semantic_similarity,
            temporal_relevance,
            agent_preference,
            historical_success,
            quality_score,
        })
    }
    
    /// Update performance metrics
    pub async fn update_performance(
        &self,
        context_id: &str,
        agent_id: &str,
        success: bool,
        impact: f64,
    ) -> Result<()> {
        let key = format!("{}:{}", agent_id, context_id);
        let mut history = self.performance_history.write().await;
        
        let metrics = history.entry(key).or_insert_with(PerformanceMetrics::default);
        metrics.total_uses += 1;
        if success {
            metrics.successful_uses += 1;
        }
        metrics.avg_impact = (metrics.avg_impact * (metrics.total_uses - 1) as f64 + impact) 
            / metrics.total_uses as f64;
        
        Ok(())
    }
    
    /// Calculate semantic similarity (simplified)
    fn calculate_semantic_similarity(&self, content: &str, query: &str) -> f64 {
        // Simple word overlap for now
        let content_words: std::collections::HashSet<&str> = content.split_whitespace().collect();
        let query_words: std::collections::HashSet<&str> = query.split_whitespace().collect();
        
        let intersection = content_words.intersection(&query_words).count();
        let union = content_words.union(&query_words).count();
        
        if union == 0 {
            0.0
        } else {
            intersection as f64 / union as f64
        }
    }
    
    /// Calculate agent preference
    fn calculate_agent_preference(&self, context: &ContextData, memories: &[MemoryItem]) -> f64 {
        // Check if similar content exists in memories
        let similar_count = memories.iter()
            .filter(|m| m.content.contains(&context.content[..context.content.len().min(50)]))
            .count();
        
        (similar_count as f64 / (memories.len() as f64 + 1.0)).min(1.0)
    }
}

impl RankingFactors {
    /// Calculate overall score
    pub fn calculate_score(&self) -> f64 {
        let weights = RankingWeights {
            semantic_weight: 0.4,
            temporal_weight: 0.2,
            preference_weight: 0.2,
            success_weight: 0.1,
            quality_weight: 0.1,
        };
        
        self.semantic_similarity * weights.semantic_weight +
        self.temporal_relevance * weights.temporal_weight +
        self.agent_preference * weights.preference_weight +
        self.historical_success * weights.success_weight +
        self.quality_score * weights.quality_weight
    }
}

impl ContextFusion {
    /// Create a new context fusion engine
    pub fn new(config: FusionConfig) -> Self {
        let mut models: HashMap<FusionStrategy, Box<dyn FusionModel>> = HashMap::new();
        models.insert(FusionStrategy::Concatenate, Box::new(ConcatenateFusion));
        models.insert(FusionStrategy::WeightedAverage, Box::new(WeightedAverageFusion));
        
        Self { config, models }
    }
    
    /// Fuse contexts using specified strategy
    pub async fn fuse(
        &self,
        contexts: Vec<RankedContext>,
        strategy: FusionStrategy,
    ) -> Result<FusedContext> {
        if let Some(model) = self.models.get(&strategy) {
            model.fuse(contexts).await
        } else {
            // Fallback to concatenation
            ConcatenateFusion.fuse(contexts).await
        }
    }
}

/// Simple concatenation fusion
struct ConcatenateFusion;

#[async_trait]
impl FusionModel for ConcatenateFusion {
    async fn fuse(&self, contexts: Vec<RankedContext>) -> Result<FusedContext> {
        let mut content = String::new();
        let mut sources = Vec::new();
        
        for (i, context) in contexts.iter().enumerate() {
            if i > 0 {
                content.push_str("\n\n");
            }
            content.push_str(&context.context.content);
            sources.push(context.context.id.clone());
        }
        
        Ok(FusedContext {
            content,
            sources,
            metadata: FusionMetadata {
                strategy: FusionStrategy::Concatenate,
                preservation_ratio: 1.0,
                confidence: 0.9,
            },
        })
    }
}

/// Weighted average fusion
struct WeightedAverageFusion;

#[async_trait]
impl FusionModel for WeightedAverageFusion {
    async fn fuse(&self, contexts: Vec<RankedContext>) -> Result<FusedContext> {
        // For text, we'll use relevance-weighted selection
        let total_relevance: f64 = contexts.iter().map(|c| c.relevance).sum();
        let mut content = String::new();
        let mut sources = Vec::new();
        
        for context in contexts {
            let weight = context.relevance / total_relevance;
            let words: Vec<&str> = context.context.content.split_whitespace().collect();
            let word_count = (words.len() as f64 * weight) as usize;
            
            content.push_str(&words[..word_count.min(words.len())].join(" "));
            content.push_str(" ");
            sources.push(context.context.id.clone());
        }
        
        Ok(FusedContext {
            content: content.trim().to_string(),
            sources,
            metadata: FusionMetadata {
                strategy: FusionStrategy::WeightedAverage,
                preservation_ratio: 0.8,
                confidence: 0.85,
            },
        })
    }
}

impl Default for AgentContextConfig {
    fn default() -> Self {
        Self {
            max_contexts: 10,
            min_relevance: 0.3,
            use_memory_ranking: true,
            temporal_weighting: true,
            fusion_strategy: FusionStrategy::WeightedAverage,
        }
    }
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            max_size: 4096,
            dedup_threshold: 0.9,
            semantic_compression: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_context_provider() {
        let config = AgentContextConfig::default();
        let provider = AgentContextProvider::new(config);
        
        let base_contexts = vec![
            ContextData {
                id: "ctx1".to_string(),
                content: "Test context about transaction processing".to_string(),
                embedding: vec![],
                metadata: HashMap::new(),
                timestamp: 1000,
            },
        ];
        
        let result = provider.get_agent_context(
            "agent1",
            "transaction",
            base_contexts,
        ).await;
        
        assert!(result.is_ok());
        let context = result.unwrap();
        assert_eq!(context.agent_id, "agent1");
        assert!(!context.contexts.is_empty());
    }

    #[tokio::test]
    async fn test_context_fusion() {
        let fusion = ContextFusion::new(FusionConfig::default());
        
        let contexts = vec![
            RankedContext {
                context: ContextData {
                    id: "1".to_string(),
                    content: "First context".to_string(),
                    embedding: vec![],
                    metadata: HashMap::new(),
                    timestamp: 0,
                },
                relevance: 0.9,
                source: ContextSource {
                    source_type: SourceType::VectorDB,
                    source_id: "1".to_string(),
                    retrieved_at: 0,
                },
                factors: RankingFactors {
                    semantic_similarity: 0.9,
                    temporal_relevance: 0.8,
                    agent_preference: 0.7,
                    historical_success: 0.6,
                    quality_score: 0.9,
                },
            },
        ];
        
        let result = fusion.fuse(contexts, FusionStrategy::Concatenate).await;
        assert!(result.is_ok());
    }
}