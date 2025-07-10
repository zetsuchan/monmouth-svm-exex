//! Knowledge Graph Module for Entity Relationship Management
//! 
//! This module provides graph-based knowledge representation and analysis
//! for blockchain entities and their relationships.

pub mod svm_integration;

pub use svm_integration::{
    SvmKnowledgeGraph, GraphConfig, Entity, EntityType, Edge, EdgeType,
    GraphQuery, GraphQueryResult, GraphStats, TransactionResult,
};

use std::sync::Arc;

/// Knowledge graph service managing multiple graphs
pub struct KnowledgeGraphService {
    /// Main transaction graph
    pub transaction_graph: Arc<SvmKnowledgeGraph>,
    /// Agent relationship graph
    pub agent_graph: Arc<SvmKnowledgeGraph>,
    /// Protocol interaction graph
    pub protocol_graph: Arc<SvmKnowledgeGraph>,
}

impl KnowledgeGraphService {
    /// Create a new knowledge graph service
    pub fn new(config: GraphConfig) -> Self {
        Self {
            transaction_graph: Arc::new(SvmKnowledgeGraph::new(config.clone())),
            agent_graph: Arc::new(SvmKnowledgeGraph::new(config.clone())),
            protocol_graph: Arc::new(SvmKnowledgeGraph::new(config)),
        }
    }
    
    /// Get combined statistics from all graphs
    pub async fn get_combined_stats(&self) -> CombinedGraphStats {
        CombinedGraphStats {
            transaction_stats: self.transaction_graph.get_stats().await,
            agent_stats: self.agent_graph.get_stats().await,
            protocol_stats: self.protocol_graph.get_stats().await,
        }
    }
    
    /// Cross-graph query
    pub async fn cross_graph_query(&self, entity_id: &str) -> CrossGraphResult {
        let mut result = CrossGraphResult {
            entity_id: entity_id.to_string(),
            transaction_presence: false,
            agent_presence: false,
            protocol_presence: false,
            connections: vec![],
        };
        
        // Check presence in each graph
        if let Ok(GraphQueryResult::Entity(_)) = self.transaction_graph
            .query(GraphQuery::GetEntity { id: entity_id.to_string() })
            .await
        {
            result.transaction_presence = true;
        }
        
        if let Ok(GraphQueryResult::Entity(_)) = self.agent_graph
            .query(GraphQuery::GetEntity { id: entity_id.to_string() })
            .await
        {
            result.agent_presence = true;
        }
        
        if let Ok(GraphQueryResult::Entity(_)) = self.protocol_graph
            .query(GraphQuery::GetEntity { id: entity_id.to_string() })
            .await
        {
            result.protocol_presence = true;
        }
        
        result
    }
}

/// Combined statistics from all graphs
#[derive(Debug, Clone)]
pub struct CombinedGraphStats {
    pub transaction_stats: GraphStats,
    pub agent_stats: GraphStats,
    pub protocol_stats: GraphStats,
}

/// Cross-graph query result
#[derive(Debug, Clone)]
pub struct CrossGraphResult {
    pub entity_id: String,
    pub transaction_presence: bool,
    pub agent_presence: bool,
    pub protocol_presence: bool,
    pub connections: Vec<CrossGraphConnection>,
}

/// Connection across graphs
#[derive(Debug, Clone)]
pub struct CrossGraphConnection {
    pub graph_type: GraphType,
    pub connected_entity: String,
    pub connection_type: String,
}

/// Graph types in the service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphType {
    Transaction,
    Agent,
    Protocol,
}