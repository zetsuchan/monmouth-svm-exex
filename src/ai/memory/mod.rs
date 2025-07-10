//! Memory management for AI agents
//! 
//! This module provides hierarchical memory systems for agent learning and context retention.

pub mod agent_integration;
pub mod reorg_handling;
pub mod rollback;

pub use agent_integration::{
    AgentMemory, MemoryConfig, MemoryItem, MemoryEntry,
    Episode, EpisodeOutcome, Concept, Goal,
    MemoryType, MemoryStats,
};
pub use reorg_handling::{
    MemoryReorgHandler, ReorgConfig, MemoryCheckpoint, MemorySnapshot,
    ReorgMetrics, BlockInfo, CheckpointManager, RollbackEngine,
};
pub use rollback::{
    MemoryRollbackManager, RollbackConfig, RollbackState, RollbackPhase,
    ValidationResults, RollbackValidator, StateRestorer,
};

use std::sync::Arc;

/// Memory manager for multiple agents
pub struct MemoryManager {
    /// Agent memories by ID
    agents: dashmap::DashMap<String, Arc<AgentMemory>>,
    /// Default configuration
    default_config: MemoryConfig,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new(default_config: MemoryConfig) -> Self {
        Self {
            agents: dashmap::DashMap::new(),
            default_config,
        }
    }
    
    /// Get or create memory for an agent
    pub fn get_or_create_agent_memory(&self, agent_id: String) -> Arc<AgentMemory> {
        self.agents
            .entry(agent_id.clone())
            .or_insert_with(|| Arc::new(AgentMemory::new(agent_id, self.default_config.clone())))
            .clone()
    }
    
    /// Remove agent memory
    pub fn remove_agent(&self, agent_id: &str) -> Option<Arc<AgentMemory>> {
        self.agents.remove(agent_id).map(|(_, memory)| memory)
    }
    
    /// Get all agent IDs
    pub fn get_agent_ids(&self) -> Vec<String> {
        self.agents.iter().map(|entry| entry.key().clone()).collect()
    }
    
    /// Get total memory statistics across all agents
    pub async fn get_total_stats(&self) -> MemoryStats {
        let mut total = MemoryStats {
            short_term_count: 0,
            long_term_count: 0,
            episode_count: 0,
            concept_count: 0,
            total_memory_mb: 0,
        };
        
        for entry in self.agents.iter() {
            let stats = entry.value().get_stats().await;
            total.short_term_count += stats.short_term_count;
            total.long_term_count += stats.long_term_count;
            total.episode_count += stats.episode_count;
            total.concept_count += stats.concept_count;
            total.total_memory_mb += stats.total_memory_mb;
        }
        
        total
    }
    
    /// Consolidate all agent memories
    pub async fn consolidate_all(&self) -> Result<(), crate::errors::AIAgentError> {
        for entry in self.agents.iter() {
            entry.value().consolidate().await
                .map_err(|e| crate::errors::AIAgentError::ProcessingError(e.to_string()))?;
        }
        Ok(())
    }
}