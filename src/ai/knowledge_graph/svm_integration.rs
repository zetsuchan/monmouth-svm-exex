//! Knowledge Graph Integration for SVM Transactions
//! 
//! This module updates agent relationships and knowledge graphs based on
//! SVM transaction interactions and patterns.

use crate::errors::*;
use crate::ai::context::preprocessing::{PreprocessedContext, TransactionType};
use crate::ai::memory::agent_integration::{Concept, Relationship, RelationshipType};
use async_trait::async_trait;
use dashmap::DashMap;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock};
use tracing::{debug, info, warn};

/// Knowledge graph for SVM transaction relationships
pub struct SvmKnowledgeGraph {
    /// Core graph structure
    graph: Arc<RwLock<DiGraph<Entity, Edge>>>,
    /// Entity index for fast lookup
    entity_index: Arc<DashMap<String, NodeIndex>>,
    /// Pattern detector
    pattern_detector: Arc<PatternDetector>,
    /// Relationship analyzer
    relationship_analyzer: Arc<RelationshipAnalyzer>,
    /// Graph updater
    updater: Arc<GraphUpdater>,
    /// Configuration
    config: GraphConfig,
}

/// Graph configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    /// Maximum nodes in graph
    pub max_nodes: usize,
    /// Maximum edges per node
    pub max_edges_per_node: usize,
    /// Enable automatic pruning
    pub auto_prune: bool,
    /// Prune threshold (node age)
    pub prune_threshold: Duration,
    /// Pattern detection sensitivity
    pub pattern_sensitivity: f64,
    /// Update batch size
    pub update_batch_size: usize,
}

/// Entity in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Entity ID
    pub id: String,
    /// Entity type
    pub entity_type: EntityType,
    /// Entity properties
    pub properties: EntityProperties,
    /// Creation timestamp
    pub created_at: Instant,
    /// Last updated
    pub updated_at: Instant,
    /// Importance score
    pub importance: f64,
}

/// Entity types in the graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    /// Smart contract
    Contract,
    /// User account
    Account,
    /// Token
    Token,
    /// Transaction pattern
    Pattern,
    /// Protocol
    Protocol,
    /// Agent
    Agent,
}

/// Entity properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityProperties {
    /// Address (if applicable)
    pub address: Option<[u8; 32]>,
    /// Name or label
    pub name: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Activity metrics
    pub activity: ActivityMetrics,
}

/// Activity metrics for entities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivityMetrics {
    /// Transaction count
    pub tx_count: u64,
    /// Total volume
    pub volume: u128,
    /// Success rate
    pub success_rate: f64,
    /// Last active
    pub last_active: Option<u64>,
}

/// Edge in the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    /// Edge type
    pub edge_type: EdgeType,
    /// Edge weight
    pub weight: f64,
    /// Edge properties
    pub properties: EdgeProperties,
    /// Creation timestamp
    pub created_at: Instant,
}

/// Edge types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    /// Calls or interacts with
    Calls,
    /// Transfers to/from
    Transfers,
    /// Deploys
    Deploys,
    /// Owns
    Owns,
    /// Similar to
    Similar,
    /// Part of pattern
    PartOfPattern,
}

/// Edge properties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeProperties {
    /// Interaction count
    pub interaction_count: u64,
    /// Average value
    pub avg_value: f64,
    /// Temporal pattern
    pub temporal_pattern: Option<TemporalPattern>,
    /// Custom properties
    pub custom: HashMap<String, serde_json::Value>,
}

/// Temporal interaction pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalPattern {
    /// Pattern type
    pub pattern_type: String,
    /// Frequency (interactions per hour)
    pub frequency: f64,
    /// Periodicity (if detected)
    pub period: Option<Duration>,
}

/// Pattern detector for transaction patterns
pub struct PatternDetector {
    /// Detected patterns
    patterns: Arc<RwLock<HashMap<String, DetectedPattern>>>,
    /// Pattern templates
    templates: Vec<PatternTemplate>,
    /// Detection history
    history: Arc<Mutex<VecDeque<DetectionEvent>>>,
}

/// Detected pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    /// Pattern ID
    pub id: String,
    /// Pattern type
    pub pattern_type: String,
    /// Entities involved
    pub entities: Vec<String>,
    /// Pattern strength
    pub strength: f64,
    /// Occurrences
    pub occurrences: u32,
    /// First detected
    pub first_detected: Instant,
    /// Last detected
    pub last_detected: Instant,
}

/// Pattern template for detection
#[derive(Debug, Clone)]
pub struct PatternTemplate {
    /// Template name
    pub name: String,
    /// Required entity types
    pub entity_types: Vec<EntityType>,
    /// Required edge types
    pub edge_types: Vec<EdgeType>,
    /// Minimum occurrences
    pub min_occurrences: u32,
    /// Detection function
    pub detector: Arc<dyn PatternDetectorFn>,
}

/// Trait for pattern detection functions
pub trait PatternDetectorFn: Send + Sync {
    /// Detect pattern in subgraph
    fn detect(&self, entities: &[Entity], edges: &[Edge]) -> Option<f64>;
}

/// Detection event
#[derive(Debug, Clone)]
pub struct DetectionEvent {
    /// Pattern detected
    pub pattern_id: String,
    /// Timestamp
    pub timestamp: Instant,
    /// Transaction context
    pub tx_hash: [u8; 32],
}

/// Relationship analyzer
pub struct RelationshipAnalyzer {
    /// Analysis algorithms
    algorithms: HashMap<String, Box<dyn AnalysisAlgorithm>>,
    /// Analysis cache
    cache: Arc<DashMap<String, AnalysisResult>>,
}

/// Trait for analysis algorithms
#[async_trait]
pub trait AnalysisAlgorithm: Send + Sync {
    /// Analyze relationships
    async fn analyze(&self, graph: &DiGraph<Entity, Edge>) -> Result<AnalysisResult>;
    
    /// Algorithm name
    fn name(&self) -> &str;
}

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Algorithm used
    pub algorithm: String,
    /// Key findings
    pub findings: Vec<Finding>,
    /// Metrics
    pub metrics: HashMap<String, f64>,
    /// Timestamp
    pub timestamp: Instant,
}

/// Analysis finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Finding type
    pub finding_type: FindingType,
    /// Entities involved
    pub entities: Vec<String>,
    /// Confidence score
    pub confidence: f64,
    /// Description
    pub description: String,
}

/// Finding types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FindingType {
    /// Central entity identified
    CentralEntity,
    /// Cluster detected
    Cluster,
    /// Anomaly found
    Anomaly,
    /// Trend identified
    Trend,
    /// Risk detected
    Risk,
}

/// Graph updater for real-time updates
pub struct GraphUpdater {
    /// Update queue
    update_queue: Arc<Mutex<VecDeque<GraphUpdate>>>,
    /// Batch processor
    batch_processor: Arc<BatchProcessor>,
}

/// Graph update operation
#[derive(Debug, Clone)]
pub struct GraphUpdate {
    /// Update type
    pub update_type: UpdateType,
    /// Update data
    pub data: UpdateData,
    /// Priority
    pub priority: UpdatePriority,
    /// Timestamp
    pub timestamp: Instant,
}

/// Update types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateType {
    AddEntity,
    UpdateEntity,
    AddEdge,
    UpdateEdge,
    RemoveEntity,
    RemoveEdge,
}

/// Update data
#[derive(Debug, Clone)]
pub enum UpdateData {
    Entity(Entity),
    Edge {
        source: String,
        target: String,
        edge: Edge,
    },
    EntityUpdate {
        id: String,
        updates: HashMap<String, serde_json::Value>,
    },
    EdgeUpdate {
        source: String,
        target: String,
        updates: HashMap<String, serde_json::Value>,
    },
    Remove {
        id: String,
    },
}

/// Update priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum UpdatePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Batch processor for updates
pub struct BatchProcessor {
    /// Batch size
    batch_size: usize,
    /// Processing interval
    interval: Duration,
}

impl SvmKnowledgeGraph {
    /// Create a new SVM knowledge graph
    pub fn new(config: GraphConfig) -> Self {
        Self {
            graph: Arc::new(RwLock::new(DiGraph::new())),
            entity_index: Arc::new(DashMap::new()),
            pattern_detector: Arc::new(PatternDetector::new()),
            relationship_analyzer: Arc::new(RelationshipAnalyzer::new()),
            updater: Arc::new(GraphUpdater::new(config.update_batch_size)),
            config,
        }
    }
    
    /// Process SVM transaction context
    pub async fn process_transaction(
        &self,
        context: &PreprocessedContext,
        tx_result: TransactionResult,
    ) -> Result<()> {
        info!("Processing transaction {} for knowledge graph", hex::encode(&context.tx_hash));
        
        // Extract entities from transaction
        let entities = self.extract_entities(context).await?;
        
        // Update or create entities
        for entity in entities {
            self.add_or_update_entity(entity).await?;
        }
        
        // Extract relationships
        let relationships = self.extract_relationships(context, &tx_result).await?;
        
        // Update edges
        for (source, target, edge) in relationships {
            self.add_or_update_edge(source, target, edge).await?;
        }
        
        // Detect patterns
        self.detect_patterns(context).await?;
        
        // Analyze updated graph
        self.analyze_relationships().await?;
        
        Ok(())
    }
    
    /// Add or update entity
    async fn add_or_update_entity(&self, entity: Entity) -> Result<()> {
        let update = GraphUpdate {
            update_type: if self.entity_index.contains_key(&entity.id) {
                UpdateType::UpdateEntity
            } else {
                UpdateType::AddEntity
            },
            data: UpdateData::Entity(entity),
            priority: UpdatePriority::Normal,
            timestamp: Instant::now(),
        };
        
        self.updater.queue_update(update).await
    }
    
    /// Add or update edge
    async fn add_or_update_edge(
        &self,
        source: String,
        target: String,
        edge: Edge,
    ) -> Result<()> {
        let update = GraphUpdate {
            update_type: UpdateType::AddEdge,
            data: UpdateData::Edge { source, target, edge },
            priority: UpdatePriority::Normal,
            timestamp: Instant::now(),
        };
        
        self.updater.queue_update(update).await
    }
    
    /// Extract entities from transaction context
    async fn extract_entities(&self, context: &PreprocessedContext) -> Result<Vec<Entity>> {
        let mut entities = Vec::new();
        
        // Extract contract entities
        for contract in &context.metadata.contracts {
            entities.push(Entity {
                id: hex::encode(&contract.address),
                entity_type: EntityType::Contract,
                properties: EntityProperties {
                    address: Some(contract.address),
                    name: contract.name.clone(),
                    metadata: HashMap::new(),
                    activity: ActivityMetrics::default(),
                },
                created_at: Instant::now(),
                updated_at: Instant::now(),
                importance: 0.5,
            });
        }
        
        // Extract account entities
        for entity_mention in &context.metadata.entities {
            if entity_mention.entity_type == "address" {
                entities.push(Entity {
                    id: entity_mention.value.clone(),
                    entity_type: EntityType::Account,
                    properties: EntityProperties {
                        address: None, // Would parse from value
                        name: None,
                        metadata: HashMap::new(),
                        activity: ActivityMetrics::default(),
                    },
                    created_at: Instant::now(),
                    updated_at: Instant::now(),
                    importance: 0.3,
                });
            }
        }
        
        Ok(entities)
    }
    
    /// Extract relationships from transaction
    async fn extract_relationships(
        &self,
        context: &PreprocessedContext,
        tx_result: &TransactionResult,
    ) -> Result<Vec<(String, String, Edge)>> {
        let mut relationships = Vec::new();
        
        // Simple from->to relationship
        if let (Some(from), Some(to)) = (self.extract_from(context), self.extract_to(context)) {
            let edge = Edge {
                edge_type: match context.metadata.tx_type {
                    TransactionType::Transfer => EdgeType::Transfers,
                    TransactionType::ContractCall => EdgeType::Calls,
                    TransactionType::Deployment => EdgeType::Deploys,
                    _ => EdgeType::Calls,
                },
                weight: 1.0,
                properties: EdgeProperties {
                    interaction_count: 1,
                    avg_value: 0.0, // Would extract from transaction
                    temporal_pattern: None,
                    custom: HashMap::new(),
                },
                created_at: Instant::now(),
            };
            
            relationships.push((from, to, edge));
        }
        
        Ok(relationships)
    }
    
    /// Detect patterns in the graph
    async fn detect_patterns(&self, context: &PreprocessedContext) -> Result<()> {
        self.pattern_detector.detect_from_context(context, &*self.graph.read().await).await
    }
    
    /// Analyze relationships in the graph
    async fn analyze_relationships(&self) -> Result<()> {
        let graph = self.graph.read().await;
        self.relationship_analyzer.analyze_graph(&*graph).await
    }
    
    /// Query the knowledge graph
    pub async fn query(&self, query: GraphQuery) -> Result<GraphQueryResult> {
        let graph = self.graph.read().await;
        
        match query {
            GraphQuery::GetEntity { id } => {
                if let Some(&node_idx) = self.entity_index.get(&id) {
                    if let Some(entity) = graph.node_weight(node_idx) {
                        Ok(GraphQueryResult::Entity(entity.clone()))
                    } else {
                        Err(AIAgentError::ProcessingError("Entity not found".to_string()))
                    }
                } else {
                    Err(AIAgentError::ProcessingError("Entity not found".to_string()))
                }
            }
            GraphQuery::GetNeighbors { id, depth } => {
                let neighbors = self.get_neighbors(&id, depth, &*graph)?;
                Ok(GraphQueryResult::Entities(neighbors))
            }
            GraphQuery::FindPath { source, target } => {
                let path = self.find_path(&source, &target, &*graph)?;
                Ok(GraphQueryResult::Path(path))
            }
            GraphQuery::GetPatterns => {
                let patterns = self.pattern_detector.get_all_patterns().await?;
                Ok(GraphQueryResult::Patterns(patterns))
            }
        }
    }
    
    /// Get graph statistics
    pub async fn get_stats(&self) -> GraphStats {
        let graph = self.graph.read().await;
        
        GraphStats {
            node_count: graph.node_count(),
            edge_count: graph.edge_count(),
            entity_types: self.count_entity_types(&*graph),
            edge_types: self.count_edge_types(&*graph),
            avg_degree: self.calculate_avg_degree(&*graph),
        }
    }
    
    /// Extract 'from' address
    fn extract_from(&self, context: &PreprocessedContext) -> Option<String> {
        // Simple extraction - would be more sophisticated
        context.metadata.entities.first()
            .map(|e| e.value.clone())
    }
    
    /// Extract 'to' address
    fn extract_to(&self, context: &PreprocessedContext) -> Option<String> {
        // Simple extraction - would be more sophisticated
        context.metadata.entities.get(1)
            .map(|e| e.value.clone())
    }
    
    /// Get neighbors of an entity
    fn get_neighbors(
        &self,
        id: &str,
        depth: usize,
        graph: &DiGraph<Entity, Edge>,
    ) -> Result<Vec<Entity>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut results = Vec::new();
        
        if let Some(&start_idx) = self.entity_index.get(id) {
            queue.push_back((start_idx, 0));
            visited.insert(start_idx);
            
            while let Some((node_idx, current_depth)) = queue.pop_front() {
                if current_depth > depth {
                    break;
                }
                
                if let Some(entity) = graph.node_weight(node_idx) {
                    if current_depth > 0 {
                        results.push(entity.clone());
                    }
                }
                
                if current_depth < depth {
                    for neighbor in graph.neighbors(node_idx) {
                        if visited.insert(neighbor) {
                            queue.push_back((neighbor, current_depth + 1));
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find path between entities
    fn find_path(
        &self,
        source: &str,
        target: &str,
        graph: &DiGraph<Entity, Edge>,
    ) -> Result<Vec<String>> {
        use petgraph::algo::dijkstra;
        
        let source_idx = self.entity_index.get(source)
            .ok_or_else(|| AIAgentError::ProcessingError("Source not found".to_string()))?;
        let target_idx = self.entity_index.get(target)
            .ok_or_else(|| AIAgentError::ProcessingError("Target not found".to_string()))?;
        
        let path_map = dijkstra(
            graph,
            *source_idx.value(),
            Some(*target_idx.value()),
            |_| 1,
        );
        
        if path_map.contains_key(target_idx.value()) {
            // Reconstruct path - simplified
            Ok(vec![source.to_string(), target.to_string()])
        } else {
            Ok(vec![])
        }
    }
    
    /// Count entity types
    fn count_entity_types(&self, graph: &DiGraph<Entity, Edge>) -> HashMap<EntityType, usize> {
        let mut counts = HashMap::new();
        
        for node in graph.node_weights() {
            *counts.entry(node.entity_type).or_insert(0) += 1;
        }
        
        counts
    }
    
    /// Count edge types
    fn count_edge_types(&self, graph: &DiGraph<Entity, Edge>) -> HashMap<EdgeType, usize> {
        let mut counts = HashMap::new();
        
        for edge in graph.edge_weights() {
            *counts.entry(edge.edge_type).or_insert(0) += 1;
        }
        
        counts
    }
    
    /// Calculate average degree
    fn calculate_avg_degree(&self, graph: &DiGraph<Entity, Edge>) -> f64 {
        if graph.node_count() == 0 {
            0.0
        } else {
            graph.edge_count() as f64 / graph.node_count() as f64
        }
    }
}

/// Transaction result for graph updates
#[derive(Debug, Clone)]
pub struct TransactionResult {
    /// Success status
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Value transferred
    pub value: u128,
}

/// Graph query types
#[derive(Debug, Clone)]
pub enum GraphQuery {
    /// Get a specific entity
    GetEntity { id: String },
    /// Get neighbors of an entity
    GetNeighbors { id: String, depth: usize },
    /// Find path between entities
    FindPath { source: String, target: String },
    /// Get all patterns
    GetPatterns,
}

/// Graph query results
#[derive(Debug, Clone)]
pub enum GraphQueryResult {
    Entity(Entity),
    Entities(Vec<Entity>),
    Path(Vec<String>),
    Patterns(Vec<DetectedPattern>),
}

/// Graph statistics
#[derive(Debug, Clone)]
pub struct GraphStats {
    pub node_count: usize,
    pub edge_count: usize,
    pub entity_types: HashMap<EntityType, usize>,
    pub edge_types: HashMap<EdgeType, usize>,
    pub avg_degree: f64,
}

impl PatternDetector {
    /// Create a new pattern detector
    pub fn new() -> Self {
        Self {
            patterns: Arc::new(RwLock::new(HashMap::new())),
            templates: Self::default_templates(),
            history: Arc::new(Mutex::new(VecDeque::with_capacity(1000))),
        }
    }
    
    /// Get default pattern templates
    fn default_templates() -> Vec<PatternTemplate> {
        vec![
            // Add default templates here
        ]
    }
    
    /// Detect patterns from context
    pub async fn detect_from_context(
        &self,
        context: &PreprocessedContext,
        graph: &DiGraph<Entity, Edge>,
    ) -> Result<()> {
        // Pattern detection logic
        Ok(())
    }
    
    /// Get all detected patterns
    pub async fn get_all_patterns(&self) -> Result<Vec<DetectedPattern>> {
        Ok(self.patterns.read().await.values().cloned().collect())
    }
}

impl RelationshipAnalyzer {
    /// Create a new relationship analyzer
    pub fn new() -> Self {
        let mut algorithms: HashMap<String, Box<dyn AnalysisAlgorithm>> = HashMap::new();
        algorithms.insert("centrality".to_string(), Box::new(CentralityAnalysis));
        algorithms.insert("clustering".to_string(), Box::new(ClusteringAnalysis));
        
        Self {
            algorithms,
            cache: Arc::new(DashMap::new()),
        }
    }
    
    /// Analyze graph with all algorithms
    pub async fn analyze_graph(&self, graph: &DiGraph<Entity, Edge>) -> Result<()> {
        for (name, algorithm) in &self.algorithms {
            match algorithm.analyze(graph).await {
                Ok(result) => {
                    self.cache.insert(name.clone(), result);
                }
                Err(e) => {
                    warn!("Analysis {} failed: {}", name, e);
                }
            }
        }
        Ok(())
    }
}

impl GraphUpdater {
    /// Create a new graph updater
    pub fn new(batch_size: usize) -> Self {
        Self {
            update_queue: Arc::new(Mutex::new(VecDeque::new())),
            batch_processor: Arc::new(BatchProcessor {
                batch_size,
                interval: Duration::from_millis(100),
            }),
        }
    }
    
    /// Queue an update
    pub async fn queue_update(&self, update: GraphUpdate) -> Result<()> {
        self.update_queue.lock().await.push_back(update);
        Ok(())
    }
}

/// Centrality analysis algorithm
struct CentralityAnalysis;

#[async_trait]
impl AnalysisAlgorithm for CentralityAnalysis {
    async fn analyze(&self, graph: &DiGraph<Entity, Edge>) -> Result<AnalysisResult> {
        // Simplified centrality analysis
        Ok(AnalysisResult {
            algorithm: self.name().to_string(),
            findings: vec![],
            metrics: HashMap::new(),
            timestamp: Instant::now(),
        })
    }
    
    fn name(&self) -> &str {
        "centrality"
    }
}

/// Clustering analysis algorithm
struct ClusteringAnalysis;

#[async_trait]
impl AnalysisAlgorithm for ClusteringAnalysis {
    async fn analyze(&self, graph: &DiGraph<Entity, Edge>) -> Result<AnalysisResult> {
        // Simplified clustering analysis
        Ok(AnalysisResult {
            algorithm: self.name().to_string(),
            findings: vec![],
            metrics: HashMap::new(),
            timestamp: Instant::now(),
        })
    }
    
    fn name(&self) -> &str {
        "clustering"
    }
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            max_nodes: 100000,
            max_edges_per_node: 1000,
            auto_prune: true,
            prune_threshold: Duration::from_secs(86400 * 30), // 30 days
            pattern_sensitivity: 0.7,
            update_batch_size: 100,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::context::preprocessing::{ContextMetadata, ContractInfo};

    #[tokio::test]
    async fn test_graph_creation() {
        let config = GraphConfig::default();
        let graph = SvmKnowledgeGraph::new(config);
        
        let stats = graph.get_stats().await;
        assert_eq!(stats.node_count, 0);
        assert_eq!(stats.edge_count, 0);
    }

    #[tokio::test]
    async fn test_entity_extraction() {
        let graph = SvmKnowledgeGraph::new(GraphConfig::default());
        
        let context = PreprocessedContext {
            tx_hash: [1u8; 32],
            chunks: vec![],
            metadata: ContextMetadata {
                tx_type: TransactionType::Transfer,
                contracts: vec![
                    ContractInfo {
                        address: [2u8; 32],
                        name: Some("TestContract".to_string()),
                        method: Some("transfer".to_string()),
                        interaction_type: "call".to_string(),
                    }
                ],
                patterns: vec![],
                entities: vec![],
                temporal_refs: vec![],
                custom: HashMap::new(),
            },
            timestamp: 0,
            processing_time_ms: 0,
        };
        
        let entities = graph.extract_entities(&context).await.unwrap();
        assert_eq!(entities.len(), 1);
        assert_eq!(entities[0].entity_type, EntityType::Contract);
    }
}