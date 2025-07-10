//! Stub implementations for AI and optional features
//! 
//! This module provides minimal stub implementations of AI-related types and traits
//! when the corresponding features are disabled, allowing the code to compile
//! without full functionality.

use crate::errors::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stub SearchResult for when AI features are disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub metadata: HashMap<String, String>,
}

/// Stub VectorStore trait for when AI features are disabled
#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    async fn search(&self, _query: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        Ok(vec![])
    }
}

/// Stub EmbeddingResult for when AI features are disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResult {
    pub embedding: Vec<f32>,
    pub input: String,
}

/// Stub EmbeddingService for when AI features are disabled
#[derive(Debug, Clone)]
pub struct EmbeddingService;

impl EmbeddingService {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn start(&self) -> Result<()> {
        Ok(())
    }
}

/// Stub SvmKnowledgeGraph for when AI features are disabled
#[derive(Debug, Clone)]
pub struct SvmKnowledgeGraph;

/// Stub Entity for when AI features are disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
}

/// Stub Edge for when AI features are disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub edge_type: String,
}

/// Stub AgentMemory for when AI features are disabled
#[derive(Debug, Clone)]
pub struct AgentMemory;

impl AgentMemory {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn initialize(&self) -> Result<()> {
        Ok(())
    }
}

/// Stub ContextData for when AI features are disabled
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextData {
    pub data: HashMap<String, String>,
}