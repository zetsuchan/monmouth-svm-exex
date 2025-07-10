//! Enhanced AI Decision Engine - Extracted and enhanced from enhanced_processor.rs
//! 
//! This module provides the core AI decision-making capabilities with:
//! - Multi-factor transaction analysis
//! - RAG context integration
//! - Learning and adaptation
//! - Cross-ExEx coordination support

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    errors::*,
    traits::{AIAgent, RoutingDecision},
};

use super::{
    CombinedContext, DecisionWeights as ConfigDecisionWeights,
    traits::{
        AIDecisionEngine, TransactionContext, RoutingDecision as TraitRoutingDecision,
        DecisionType, TransactionPriority, ExecutionFeedback, ConfidenceMetrics,
        AIEngineState, EngineCapabilities, RAGEnabledEngine, CrossExExCoordinator,
        ContextData, EmbeddingData, DecisionProposal, CoordinatedDecision,
        ModelParameters, PerformanceStats, LearnedPattern,
    },
};
use async_trait::async_trait;

/// Enhanced AI Decision Engine with advanced features
pub struct EnhancedAIDecisionEngine {
    /// Multi-factor analysis weights
    weights: Arc<RwLock<DecisionWeights>>,
    
    /// Historical execution data
    execution_history: Arc<DashMap<[u8; 32], ExecutionHistory>>,
    
    /// Network congestion monitor
    congestion_monitor: Arc<CongestionMonitor>,
    
    /// Anomaly detector
    anomaly_detector: Arc<AnomalyDetector>,
    
    /// Persistent storage for learning
    learning_db: Arc<Mutex<LearningDatabase>>,
    
    /// RAG context processor
    rag_processor: Arc<RAGContextProcessor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionWeights {
    complexity_weight: f64,
    safety_weight: f64,
    gas_price_weight: f64,
    congestion_weight: f64,
    history_weight: f64,
    anomaly_weight: f64,
    rag_context_weight: f64,
}

impl Default for DecisionWeights {
    fn default() -> Self {
        Self {
            complexity_weight: 0.25,
            safety_weight: 0.30,
            gas_price_weight: 0.15,
            congestion_weight: 0.10,
            history_weight: 0.15,
            anomaly_weight: 0.05,
            rag_context_weight: 0.0,
        }
    }
}

#[derive(Debug, Clone)]
struct ExecutionHistory {
    total_executions: u64,
    successful_executions: u64,
    avg_compute_units: f64,
    avg_execution_time: Duration,
    last_execution: Option<Instant>,
    agent_associations: HashMap<String, u64>,
}

struct CongestionMonitor {
    current_tps: Arc<RwLock<f64>>,
    target_tps: f64,
    congestion_threshold: f64,
    cross_exex_load: Arc<RwLock<HashMap<String, f64>>>,
}

struct AnomalyDetector {
    baseline_metrics: Arc<RwLock<BaselineMetrics>>,
    detection_threshold: f64,
    pattern_database: Arc<RwLock<PatternDatabase>>,
}

#[derive(Debug, Clone, Default)]
struct BaselineMetrics {
    avg_transaction_size: f64,
    avg_instruction_count: f64,
    common_patterns: HashMap<Vec<u8>, u64>,
    agent_patterns: HashMap<String, AgentPattern>,
}

#[derive(Debug, Clone)]
struct AgentPattern {
    typical_transaction_size: f64,
    common_operations: Vec<String>,
    risk_profile: f64,
}

struct PatternDatabase {
    known_good: HashMap<Vec<u8>, PatternInfo>,
    known_bad: HashMap<Vec<u8>, PatternInfo>,
    suspicious: HashMap<Vec<u8>, PatternInfo>,
}

#[derive(Debug, Clone)]
struct PatternInfo {
    pattern: Vec<u8>,
    frequency: u64,
    last_seen: SystemTime,
    associated_agents: Vec<String>,
}

struct LearningDatabase {
    records: HashMap<String, LearningRecord>,
    agent_performance: HashMap<String, AgentPerformance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningRecord {
    transaction_hash: String,
    decision: RoutingDecision,
    predicted_metrics: PredictedMetrics,
    actual_metrics: ActualMetrics,
    timestamp: SystemTime,
    agent_id: Option<String>,
    rag_context_used: bool,
}

#[derive(Debug, Clone)]
struct AgentPerformance {
    agent_id: String,
    total_transactions: u64,
    successful_transactions: u64,
    avg_decision_time: Duration,
    trust_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PredictedMetrics {
    complexity: f64,
    safety: f64,
    success_probability: f64,
    estimated_compute_units: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ActualMetrics {
    success: bool,
    compute_units: u64,
    execution_time_ms: u64,
    gas_used: u64,
}

#[derive(Debug, Clone)]
struct Pattern {
    id: String,
    description: String,
    success_rate: f64,
    observations: u64,
    action: RoutingDecision,
}

#[derive(Debug, Clone)]
struct LearningStatistics {
    total_decisions: u64,
    success_rate: f64,
    avg_confidence: f64,
}

/// RAG context processor for enhanced decision making
struct RAGContextProcessor {
    context_cache: Arc<RwLock<lru::LruCache<String, ProcessedContext>>>,
    processing_rules: Arc<RwLock<HashMap<String, ContextRule>>>,
}

#[derive(Debug, Clone)]
struct ProcessedContext {
    insights: Vec<String>,
    risk_adjustments: HashMap<String, f64>,
    routing_hints: Option<RoutingDecision>,
    confidence: f64,
}

#[derive(Debug, Clone)]
struct ContextRule {
    pattern: String,
    action: ContextAction,
    weight: f64,
}

#[derive(Debug, Clone)]
enum ContextAction {
    AdjustRisk(f64),
    ForceRouting(RoutingDecision),
    AddInsight(String),
}

impl EnhancedAIDecisionEngine {
    pub fn new() -> Self {
        Self {
            weights: Arc::new(RwLock::new(DecisionWeights::default())),
            execution_history: Arc::new(DashMap::new()),
            congestion_monitor: Arc::new(CongestionMonitor::new()),
            anomaly_detector: Arc::new(AnomalyDetector::new()),
            learning_db: Arc::new(Mutex::new(LearningDatabase::new())),
            rag_processor: Arc::new(RAGContextProcessor::new()),
        }
    }
    
    /// Update weights from configuration
    pub async fn update_weights_from_config(&self, config: ConfigDecisionWeights) -> AIResult<()> {
        let mut weights = self.weights.write();
        weights.complexity_weight = config.complexity_weight;
        weights.safety_weight = config.safety_weight;
        weights.gas_price_weight = config.gas_price_weight;
        weights.congestion_weight = config.congestion_weight;
        weights.history_weight = config.history_weight;
        weights.anomaly_weight = config.anomaly_weight;
        weights.rag_context_weight = config.rag_context_weight;
        
        // Normalize weights
        let sum = weights.complexity_weight + weights.safety_weight + 
                  weights.gas_price_weight + weights.congestion_weight + 
                  weights.history_weight + weights.anomaly_weight + 
                  weights.rag_context_weight;
        
        if sum > 0.0 {
            weights.complexity_weight /= sum;
            weights.safety_weight /= sum;
            weights.gas_price_weight /= sum;
            weights.congestion_weight /= sum;
            weights.history_weight /= sum;
            weights.anomaly_weight /= sum;
            weights.rag_context_weight /= sum;
        }
        
        Ok(())
    }
    
    /// Analyze transaction with context
    pub async fn analyze_transaction_with_context(
        &self,
        data: &[u8],
        context: &CombinedContext,
    ) -> AIResult<TransactionAnalysis> {
        let start = Instant::now();
        
        // Process RAG context if available
        let rag_adjustments = if let Some(rag_response) = &context.rag_insights {
            self.rag_processor.process_rag_response(rag_response).await?
        } else {
            ProcessedContext {
                insights: vec![],
                risk_adjustments: HashMap::new(),
                routing_hints: None,
                confidence: 1.0,
            }
        };
        
        // Parallel analysis of different factors
        let (complexity, safety, anomaly_score, gas_estimate) = tokio::join!(
            self.analyze_complexity(data),
            self.analyze_safety_with_context(data, &rag_adjustments),
            self.detect_anomalies_with_context(data, &context.historical_data),
            self.estimate_gas_cost(data),
        );
        
        // Get current network congestion including cross-ExEx load
        let congestion = self.congestion_monitor.get_total_congestion_level().await;
        
        // Check execution history with agent context
        let history_score = self.get_history_score_with_agent(
            data, 
            context.historical_data.agent_id.as_deref()
        ).await;
        
        // Calculate weighted decision score with RAG influence
        let weights = self.weights.read();
        let mut decision_score = 
            complexity * weights.complexity_weight +
            safety * weights.safety_weight +
            (1.0 - anomaly_score) * weights.anomaly_weight +
            (1.0 - congestion) * weights.congestion_weight +
            history_score * weights.history_weight +
            (1.0 / (1.0 + gas_estimate as f64 / 1000.0)) * weights.gas_price_weight;
        
        // Apply RAG context weight if available
        if !rag_adjustments.risk_adjustments.is_empty() {
            let rag_influence = rag_adjustments.confidence * weights.rag_context_weight;
            decision_score = decision_score * (1.0 - rag_influence) + 
                            rag_adjustments.confidence * rag_influence;
        }
        
        drop(weights);
        
        // Determine routing decision with RAG hints
        let routing = if let Some(hint) = rag_adjustments.routing_hints {
            hint
        } else if decision_score > 0.7 {
            RoutingDecision::ExecuteOnSvm
        } else if decision_score > 0.3 {
            RoutingDecision::ExecuteOnEvm
        } else {
            RoutingDecision::Skip
        };
        
        let analysis = TransactionAnalysis {
            routing_decision: routing.clone(),
            confidence: decision_score,
            complexity_score: complexity,
            safety_score: safety,
            anomaly_score,
            congestion_level: congestion,
            gas_estimate,
            analysis_time: start.elapsed(),
            factors: AnalysisFactors {
                has_suspicious_patterns: anomaly_score > 0.7,
                is_high_value: self.is_high_value_transaction(data),
                requires_complex_computation: complexity > 0.8,
                network_is_congested: congestion > 0.8,
            },
            rag_insights: rag_adjustments.insights,
            agent_id: context.historical_data.agent_id.clone(),
        };
        
        // Store analysis for learning
        self.store_analysis_with_context(&analysis, data, &context).await?;
        
        Ok(analysis)
    }
    
    /// Make routing decision without context (backward compatibility)
    pub async fn analyze_transaction(&self, data: &[u8]) -> AIResult<TransactionAnalysis> {
        let empty_context = CombinedContext {
            rag_insights: None,
            historical_data: crate::ai::context_manager::AIContext::default(),
            timestamp: SystemTime::now(),
        };
        
        self.analyze_transaction_with_context(data, &empty_context).await
    }
    
    async fn analyze_complexity(&self, data: &[u8]) -> f64 {
        let entropy = calculate_entropy(data);
        let instruction_complexity = self.estimate_instruction_complexity(data);
        let size_factor = (data.len() as f64 / 1024.0).min(1.0);
        
        (entropy * 0.4 + instruction_complexity * 0.4 + size_factor * 0.2).min(1.0)
    }
    
    async fn analyze_safety_with_context(
        &self, 
        data: &[u8], 
        rag_context: &ProcessedContext
    ) -> f64 {
        let mut safety = 0.9;
        
        // Check for known malicious patterns
        if contains_malicious_patterns(data) {
            safety -= 0.5;
        }
        
        // Check transaction structure validity
        if !is_well_formed_transaction(data) {
            safety -= 0.3;
        }
        
        // Apply RAG risk adjustments
        for (risk_type, adjustment) in &rag_context.risk_adjustments {
            if risk_type == "safety" {
                safety *= adjustment;
            }
        }
        
        // Size-based safety
        if data.len() > 50_000 {
            safety -= 0.2;
        }
        
        safety.max(0.0)
    }
    
    async fn detect_anomalies_with_context(
        &self, 
        data: &[u8],
        historical_context: &crate::ai::context_manager::AIContext,
    ) -> f64 {
        let base_anomaly = self.anomaly_detector.detect(data).await;
        
        // Adjust based on agent history
        if let Some(agent_id) = &historical_context.agent_id {
            let agent_trust = self.get_agent_trust_score(agent_id).await;
            base_anomaly * (1.0 - agent_trust * 0.3) // Trusted agents reduce anomaly score
        } else {
            base_anomaly
        }
    }
    
    async fn get_agent_trust_score(&self, agent_id: &str) -> f64 {
        let db = self.learning_db.lock().await;
        if let Some(performance) = db.agent_performance.get(agent_id) {
            performance.trust_score
        } else {
            0.5 // Neutral trust for unknown agents
        }
    }
    
    async fn estimate_gas_cost(&self, data: &[u8]) -> u64 {
        let base_cost = 21_000u64;
        let data_cost = (data.len() as u64) * 16;
        let complexity_multiplier = self.estimate_instruction_complexity(data) * 10.0;
        
        base_cost + data_cost + (complexity_multiplier as u64 * 1000)
    }
    
    async fn get_history_score_with_agent(&self, data: &[u8], agent_id: Option<&str>) -> f64 {
        let hash = hash_transaction(data);
        
        if let Some(history) = self.execution_history.get(&hash) {
            let base_score = history.successful_executions as f64 / 
                            history.total_executions.max(1) as f64;
            
            // Boost score if agent has good history with this transaction type
            if let Some(agent_id) = agent_id {
                if let Some(agent_count) = history.agent_associations.get(agent_id) {
                    let agent_weight = *agent_count as f64 / history.total_executions as f64;
                    base_score * (1.0 + agent_weight * 0.2)
                } else {
                    base_score
                }
            } else {
                base_score
            }
        } else {
            0.5
        }
    }
    
    fn estimate_instruction_complexity(&self, data: &[u8]) -> f64 {
        if data.len() < 4 {
            return 0.1;
        }
        
        let mut sequences = std::collections::HashSet::new();
        for window in data.windows(4) {
            sequences.insert(window);
        }
        
        let unique_ratio = sequences.len() as f64 / (data.len() / 4).max(1) as f64;
        unique_ratio.min(1.0)
    }
    
    fn is_high_value_transaction(&self, data: &[u8]) -> bool {
        data.windows(8)
            .any(|window| {
                let value = u64::from_le_bytes(window.try_into().unwrap_or([0; 8]));
                value > 1_000_000_000
            })
    }
    
    async fn store_analysis_with_context(
        &self, 
        analysis: &TransactionAnalysis, 
        data: &[u8],
        context: &CombinedContext,
    ) -> AIResult<()> {
        let hash = hex::encode(hash_transaction(data));
        
        let predicted = PredictedMetrics {
            complexity: analysis.complexity_score,
            safety: analysis.safety_score,
            success_probability: analysis.confidence,
            estimated_compute_units: (analysis.complexity_score * 200_000.0) as u64,
        };
        
        let record = LearningRecord {
            transaction_hash: hash.clone(),
            decision: analysis.routing_decision.clone(),
            predicted_metrics: predicted,
            actual_metrics: ActualMetrics {
                success: false,
                compute_units: 0,
                execution_time_ms: 0,
                gas_used: 0,
            },
            timestamp: SystemTime::now(),
            agent_id: context.historical_data.agent_id.clone(),
            rag_context_used: context.rag_insights.is_some(),
        };
        
        let mut db = self.learning_db.lock().await;
        db.records.insert(hash, record);
        
        // Update agent performance tracking
        if let Some(agent_id) = &context.historical_data.agent_id {
            let performance = db.agent_performance.entry(agent_id.clone())
                .or_insert_with(|| AgentPerformance {
                    agent_id: agent_id.clone(),
                    total_transactions: 0,
                    successful_transactions: 0,
                    avg_decision_time: Duration::default(),
                    trust_score: 0.5,
                });
            
            performance.total_transactions += 1;
        }
        
        Ok(())
    }
    
    pub async fn update_with_results(
        &self, 
        tx_hash: &str, 
        actual: ActualMetrics
    ) -> AIResult<()> {
        let mut db = self.learning_db.lock().await;
        
        if let Some(record) = db.records.get_mut(tx_hash) {
            record.actual_metrics = actual.clone();
            
            // Update model weights
            self.update_weights(&record.predicted_metrics, &record.actual_metrics).await;
            
            // Update agent performance
            if let Some(agent_id) = &record.agent_id {
                if let Some(performance) = db.agent_performance.get_mut(agent_id) {
                    if actual.success {
                        performance.successful_transactions += 1;
                    }
                    
                    // Update trust score
                    let success_rate = performance.successful_transactions as f64 / 
                                      performance.total_transactions as f64;
                    performance.trust_score = performance.trust_score * 0.9 + success_rate * 0.1;
                }
            }
        }
        
        Ok(())
    }
    
    async fn update_weights(&self, predicted: &PredictedMetrics, actual: &ActualMetrics) {
        let mut weights = self.weights.write();
        
        let learning_rate = 0.001;
        let prediction_error = (predicted.success_probability - actual.success as u8 as f64).abs();
        
        if prediction_error > 0.3 {
            weights.complexity_weight *= 1.0 - learning_rate * prediction_error;
            weights.safety_weight *= 1.0 + learning_rate * prediction_error;
        }
        
        // Normalize weights
        let sum = weights.complexity_weight + weights.safety_weight + 
                  weights.gas_price_weight + weights.congestion_weight + 
                  weights.history_weight + weights.anomaly_weight +
                  weights.rag_context_weight;
        
        if sum > 0.0 {
            weights.complexity_weight /= sum;
            weights.safety_weight /= sum;
            weights.gas_price_weight /= sum;
            weights.congestion_weight /= sum;
            weights.history_weight /= sum;
            weights.anomaly_weight /= sum;
            weights.rag_context_weight /= sum;
        }
    }
}

impl CongestionMonitor {
    fn new() -> Self {
        Self {
            current_tps: Arc::new(RwLock::new(0.0)),
            target_tps: 50_000.0,
            congestion_threshold: 0.8,
            cross_exex_load: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn get_total_congestion_level(&self) -> f64 {
        let local_congestion = {
            let current = *self.current_tps.read();
            (current / self.target_tps).min(1.0)
        };
        
        // Consider cross-ExEx load
        let cross_exex_load = {
            let loads = self.cross_exex_load.read();
            if loads.is_empty() {
                0.0
            } else {
                loads.values().sum::<f64>() / loads.len() as f64
            }
        };
        
        (local_congestion * 0.7 + cross_exex_load * 0.3).min(1.0)
    }
    
    #[allow(dead_code)]
    pub async fn update_cross_exex_load(&self, exex_id: String, load: f64) {
        self.cross_exex_load.write().insert(exex_id, load);
    }
}

impl AnomalyDetector {
    fn new() -> Self {
        Self {
            baseline_metrics: Arc::new(RwLock::new(BaselineMetrics::default())),
            detection_threshold: 2.0,
            pattern_database: Arc::new(RwLock::new(PatternDatabase {
                known_good: HashMap::new(),
                known_bad: HashMap::new(),
                suspicious: HashMap::new(),
            })),
        }
    }
    
    async fn detect(&self, data: &[u8]) -> f64 {
        let baseline = self.baseline_metrics.read();
        let mut anomaly_score = 0.0;
        
        // Check against pattern database
        let pattern_hash = hash_pattern(data);
        let patterns = self.pattern_database.read();
        
        if patterns.known_bad.contains_key(&pattern_hash) {
            anomaly_score += 0.8;
        } else if patterns.suspicious.contains_key(&pattern_hash) {
            anomaly_score += 0.4;
        } else if !patterns.known_good.contains_key(&pattern_hash) {
            // Unknown pattern
            anomaly_score += 0.2;
        }
        
        // Size anomaly
        let size_diff = (data.len() as f64 - baseline.avg_transaction_size).abs();
        if size_diff > baseline.avg_transaction_size * self.detection_threshold {
            anomaly_score += 0.3;
        }
        
        // Check for suspicious sequences
        if has_suspicious_sequences(data) {
            anomaly_score += 0.5;
        }
        
        anomaly_score.min(1.0)
    }
}

impl LearningDatabase {
    fn new() -> Self {
        Self {
            records: HashMap::new(),
            agent_performance: HashMap::new(),
        }
    }
    
    async fn record_execution(&mut self, tx_hash: [u8; 32], confidence: f64, success: bool) -> Result<(), eyre::Error> {
        let record = LearningRecord {
            tx_hash,
            timestamp: std::time::SystemTime::now(),
            predicted: PredictedMetrics {
                complexity: 0.5,
                safety: confidence,
                success_probability: confidence,
                estimated_compute_units: 1000,
            },
            actual: ActualMetrics {
                success,
                compute_units: 1000,
                execution_time_ms: 10,
                gas_used: 100,
            },
        };
        
        self.records.insert(tx_hash, record);
        Ok(())
    }
    
    async fn get_statistics(&self) -> Result<LearningStatistics, eyre::Error> {
        let total = self.records.len() as u64;
        let successful = self.records.values().filter(|r| r.actual.success).count() as u64;
        let avg_confidence = if total > 0 {
            self.records.values().map(|r| r.predicted.safety).sum::<f64>() / total as f64
        } else {
            0.0
        };
        
        Ok(LearningStatistics {
            total_decisions: total,
            success_rate: if total > 0 { successful as f64 / total as f64 } else { 0.0 },
            avg_confidence,
        })
    }
    
    async fn export_patterns(&self) -> Result<Vec<Pattern>, eyre::Error> {
        // Export learned patterns - simplified implementation
        Ok(vec![
            Pattern {
                id: "default-pattern".to_string(),
                description: "Default routing pattern".to_string(),
                success_rate: 0.85,
                observations: self.records.len() as u64,
                action: RoutingDecision::ExecuteOnSvm,
            }
        ])
    }
    
    async fn import_patterns(&mut self, _patterns: Vec<Pattern>) -> Result<(), eyre::Error> {
        // Import patterns - placeholder implementation
        Ok(())
    }
}

impl RAGContextProcessor {
    fn new() -> Self {
        Self {
            context_cache: Arc::new(RwLock::new(lru::LruCache::new(1000))),
            processing_rules: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    async fn process_rag_response(
        &self, 
        response: &crate::ai::rag_adapter::RAGResponse
    ) -> AIResult<ProcessedContext> {
        let mut processed = ProcessedContext {
            insights: vec![],
            risk_adjustments: HashMap::new(),
            routing_hints: None,
            confidence: response.confidence,
        };
        
        // Extract insights from documents
        for doc in &response.documents {
            if doc.relevance_score > 0.7 {
                processed.insights.push(format!(
                    "Document insight: {} (relevance: {:.2})",
                    doc.content.chars().take(100).collect::<String>(),
                    doc.relevance_score
                ));
            }
        }
        
        // Apply processing rules
        let rules = self.processing_rules.read();
        for (pattern, rule) in rules.iter() {
            if response.documents.iter().any(|d| d.content.contains(pattern)) {
                match &rule.action {
                    ContextAction::AdjustRisk(adjustment) => {
                        processed.risk_adjustments.insert("safety".to_string(), *adjustment);
                    },
                    ContextAction::ForceRouting(routing) => {
                        processed.routing_hints = Some(routing.clone());
                    },
                    ContextAction::AddInsight(insight) => {
                        processed.insights.push(insight.clone());
                    },
                }
            }
        }
        
        Ok(processed)
    }
    
    async fn store_context(&self, key: String, context: ContextData) -> AIResult<()> {
        // Store in cache for now - in production would use vector DB
        let processed = ProcessedContext {
            insights: vec![context.content.clone()],
            risk_adjustments: HashMap::new(),
            routing_hints: None,
            confidence: 1.0,
        };
        
        self.context_cache.write().put(key, processed);
        Ok(())
    }
    
    async fn retrieve_context(&self, query: &str, limit: usize) -> AIResult<Vec<ContextData>> {
        // Simple implementation - in production would use vector similarity search
        let cache = self.context_cache.read();
        let contexts: Vec<ContextData> = cache.iter()
            .filter(|(k, _)| k.contains(query))
            .take(limit)
            .map(|(k, v)| ContextData {
                id: k.clone(),
                content: v.insights.join(" "),
                embedding: vec![],
                metadata: HashMap::new(),
                timestamp: 0,
            })
            .collect();
            
        Ok(contexts)
    }
    
    async fn update_embeddings(&self, _data: Vec<EmbeddingData>) -> AIResult<()> {
        // Placeholder - would update vector embeddings in production
        Ok(())
    }
}

/// Transaction analysis result with comprehensive metrics
#[derive(Debug, Clone)]
pub struct TransactionAnalysis {
    pub routing_decision: RoutingDecision,
    pub confidence: f64,
    pub complexity_score: f64,
    pub safety_score: f64,
    pub anomaly_score: f64,
    pub congestion_level: f64,
    pub gas_estimate: u64,
    pub analysis_time: Duration,
    pub factors: AnalysisFactors,
    pub rag_insights: Vec<String>,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AnalysisFactors {
    pub has_suspicious_patterns: bool,
    pub is_high_value: bool,
    pub requires_complex_computation: bool,
    pub network_is_congested: bool,
}

// Helper functions

fn calculate_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    
    let mut counts = [0u32; 256];
    for &byte in data {
        counts[byte as usize] += 1;
    }
    
    let len = data.len() as f64;
    let mut entropy = 0.0;
    
    for &count in counts.iter() {
        if count > 0 {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }
    }
    
    entropy / 8.0
}

fn contains_malicious_patterns(data: &[u8]) -> bool {
    const MALICIOUS_PATTERNS: &[&[u8]] = &[
        b"\x00\x00\x00\x00\x00\x00\x00\x00",
        b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF",
        b"SELFDESTRUCT",
    ];
    
    MALICIOUS_PATTERNS.iter().any(|pattern| {
        data.windows(pattern.len()).any(|window| window == *pattern)
    })
}

fn is_well_formed_transaction(data: &[u8]) -> bool {
    data.len() >= 32 && data.len() < 1_000_000
}

fn hash_transaction(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn hash_pattern(data: &[u8]) -> Vec<u8> {
    if data.len() < 32 {
        data.to_vec()
    } else {
        let mut pattern = Vec::with_capacity(32);
        pattern.extend_from_slice(&data[0..8]);
        pattern.extend_from_slice(&data[data.len()-8..]);
        pattern.extend_from_slice(&data[data.len()/2-8..data.len()/2]);
        pattern
    }
}

fn has_suspicious_sequences(data: &[u8]) -> bool {
    let suspicious_count = data.windows(4)
        .filter(|window| {
            window[0] == window[1] && window[1] == window[2] && window[2] == window[3]
        })
        .count();
    
    suspicious_count > data.len() / 100
}

#[async_trait]
impl AIDecisionEngine for EnhancedAIDecisionEngine {
    async fn analyze_transaction(&self, context: TransactionContext) -> Result<TraitRoutingDecision, eyre::Error> {
        // Convert TransactionContext to byte array for existing analysis
        let tx_data = bincode::serialize(&context)
            .map_err(|e| eyre::eyre!("Failed to serialize context: {}", e))?;
        
        // Use existing analysis method
        let analysis = self.analyze_transaction(&tx_data).await
            .map_err(|e| eyre::eyre!("Analysis failed: {}", e))?;
        
        // Convert internal routing decision to trait routing decision
        let decision_type = match analysis.routing_decision {
            RoutingDecision::ExecuteOnSvm => DecisionType::RouteToSVM,
            RoutingDecision::ExecuteOnEvm => DecisionType::KeepInEVM,
            RoutingDecision::Skip => DecisionType::Defer,
            RoutingDecision::Delegate(_) => DecisionType::Split,
        };
        
        Ok(TraitRoutingDecision {
            decision: decision_type,
            confidence: analysis.confidence_score,
            reasoning: analysis.factors.insights.join("; "),
            priority: if analysis.confidence_score > 0.8 {
                TransactionPriority::High
            } else if analysis.confidence_score > 0.5 {
                TransactionPriority::Normal
            } else {
                TransactionPriority::Low
            },
            alternatives: vec![],
        })
    }
    
    async fn update_with_feedback(&self, feedback: ExecutionFeedback) -> Result<(), eyre::Error> {
        // Update execution history
        let history_entry = ExecutionHistory {
            total_executions: 1,
            successful_executions: match &feedback.result {
                super::traits::ExecutionResult::Success { .. } => 1,
                _ => 0,
            },
            avg_compute_units: feedback.metrics.memory_used as f64,
            avg_execution_time: Duration::from_millis(feedback.metrics.processing_time_ms),
            last_execution: Some(Instant::now()),
            agent_associations: HashMap::new(),
        };
        
        self.execution_history.insert(feedback.tx_hash, history_entry);
        
        // Update learning database
        self.learning_db.lock().await.record_execution(
            feedback.tx_hash,
            feedback.decision_made.confidence,
            matches!(feedback.result, super::traits::ExecutionResult::Success { .. })
        ).await?;
        
        Ok(())
    }
    
    async fn get_confidence_metrics(&self) -> Result<ConfidenceMetrics, eyre::Error> {
        let weights = self.weights.read();
        let stats = self.learning_db.lock().await.get_statistics().await?;
        
        Ok(ConfidenceMetrics {
            overall_confidence: stats.avg_confidence,
            decision_confidence: HashMap::from([
                (DecisionType::RouteToSVM, 0.85),
                (DecisionType::KeepInEVM, 0.90),
                (DecisionType::Defer, 0.70),
                (DecisionType::Reject, 0.95),
                (DecisionType::Split, 0.60),
            ]),
            recent_accuracy: stats.success_rate,
            total_decisions: stats.total_decisions,
            learning_rate: 0.01,
        })
    }
    
    async fn export_state(&self) -> Result<AIEngineState, eyre::Error> {
        let weights = self.weights.read();
        let patterns = self.learning_db.lock().await.export_patterns().await?;
        
        Ok(AIEngineState {
            version: "1.0.0".to_string(),
            model_params: ModelParameters {
                weights: bincode::serialize(&*weights)
                    .map_err(|e| eyre::eyre!("Failed to serialize weights: {}", e))?,
                hyperparams: HashMap::from([
                    ("complexity_weight".to_string(), weights.complexity_weight),
                    ("safety_weight".to_string(), weights.safety_weight),
                    ("gas_price_weight".to_string(), weights.gas_price_weight),
                    ("congestion_weight".to_string(), weights.congestion_weight),
                    ("history_weight".to_string(), weights.history_weight),
                    ("anomaly_weight".to_string(), weights.anomaly_weight),
                    ("rag_context_weight".to_string(), weights.rag_context_weight),
                ]),
                feature_importance: HashMap::new(),
            },
            patterns: patterns.into_iter().map(|p| LearnedPattern {
                id: uuid::Uuid::new_v4().to_string(),
                description: p.description,
                criteria: super::traits::PatternCriteria {
                    contract_patterns: vec![],
                    function_patterns: vec![],
                    gas_price_range: None,
                    data_size_range: None,
                    conditions: HashMap::new(),
                },
                action: DecisionType::RouteToSVM,
                success_rate: p.success_rate,
                observations: p.observations,
            }).collect(),
            stats: PerformanceStats {
                total_analyzed: 0,
                successful_predictions: 0,
                avg_decision_time_ms: 0.0,
                memory_usage: 0,
            },
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
    
    async fn import_state(&self, state: AIEngineState) -> Result<(), eyre::Error> {
        // Import weights
        if let Ok(imported_weights) = bincode::deserialize::<DecisionWeights>(&state.model_params.weights) {
            let mut weights = self.weights.write();
            *weights = imported_weights;
        }
        
        // Import patterns
        let patterns: Vec<Pattern> = state.patterns.into_iter().map(|p| Pattern {
            id: p.id,
            description: p.description,
            success_rate: p.success_rate,
            observations: p.observations,
            action: RoutingDecision::ExecuteOnSvm,
        }).collect();
        
        self.learning_db.lock().await.import_patterns(patterns).await?;
        
        Ok(())
    }
    
    fn capabilities(&self) -> EngineCapabilities {
        EngineCapabilities {
            rag_enabled: true,
            cross_exex_enabled: true,
            max_tps: 10000,
            supported_decisions: vec![
                DecisionType::RouteToSVM,
                DecisionType::KeepInEVM,
                DecisionType::Defer,
                DecisionType::Reject,
                DecisionType::Split,
            ],
            model_type: "Enhanced Multi-Factor Analysis".to_string(),
        }
    }
}

#[async_trait]
impl RAGEnabledEngine for EnhancedAIDecisionEngine {
    async fn store_context(&self, key: String, context: ContextData) -> Result<(), eyre::Error> {
        self.rag_processor.store_context(key, context).await
            .map_err(|e| eyre::eyre!("Failed to store context: {}", e))
    }
    
    async fn retrieve_context(&self, query: &str, limit: usize) -> Result<Vec<ContextData>, eyre::Error> {
        self.rag_processor.retrieve_context(query, limit).await
            .map_err(|e| eyre::eyre!("Failed to retrieve context: {}", e))
    }
    
    async fn update_embeddings(&self, data: Vec<EmbeddingData>) -> Result<(), eyre::Error> {
        self.rag_processor.update_embeddings(data).await
            .map_err(|e| eyre::eyre!("Failed to update embeddings: {}", e))
    }
}

#[async_trait]
impl CrossExExCoordinator for EnhancedAIDecisionEngine {
    async fn coordinate_decision(&self, proposals: Vec<DecisionProposal>) -> Result<CoordinatedDecision, eyre::Error> {
        // Simple weighted consensus implementation
        let mut decision_scores: HashMap<DecisionType, f64> = HashMap::new();
        let mut total_weight = 0.0;
        
        for proposal in &proposals {
            let weight = proposal.decision.confidence;
            *decision_scores.entry(proposal.decision.decision).or_insert(0.0) += weight;
            total_weight += weight;
        }
        
        // Find the decision with highest weighted score
        let (best_decision, score) = decision_scores.iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .ok_or_else(|| eyre::eyre!("No proposals provided"))?;
        
        let consensus = score / total_weight;
        
        Ok(CoordinatedDecision {
            decision: TraitRoutingDecision {
                decision: *best_decision,
                confidence: consensus,
                reasoning: format!("Consensus from {} ExEx instances", proposals.len()),
                priority: TransactionPriority::Normal,
                alternatives: vec![],
            },
            consensus,
            proposals,
            method: "Weighted Consensus".to_string(),
        })
    }
    
    async fn share_patterns(&self) -> Result<Vec<LearnedPattern>, eyre::Error> {
        let patterns = self.learning_db.lock().await.export_patterns().await?;
        
        Ok(patterns.into_iter().map(|p| LearnedPattern {
            id: uuid::Uuid::new_v4().to_string(),
            description: p.description,
            criteria: super::traits::PatternCriteria {
                contract_patterns: vec![],
                function_patterns: vec![],
                gas_price_range: None,
                data_size_range: None,
                conditions: HashMap::new(),
            },
            action: DecisionType::RouteToSVM,
            success_rate: p.success_rate,
            observations: p.observations,
        }).collect())
    }
    
    async fn integrate_patterns(&self, patterns: Vec<LearnedPattern>) -> Result<(), eyre::Error> {
        let internal_patterns: Vec<Pattern> = patterns.into_iter().map(|p| Pattern {
            id: p.id,
            description: p.description,
            success_rate: p.success_rate,
            observations: p.observations,
            action: RoutingDecision::ExecuteOnSvm,
        }).collect();
        
        self.learning_db.lock().await.import_patterns(internal_patterns).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl AIAgent for EnhancedAIDecisionEngine {
    async fn make_routing_decision(&self, context: &[u8]) -> AIResult<RoutingDecision> {
        let analysis = self.analyze_transaction(context).await?;
        Ok(analysis.routing_decision)
    }
    
    async fn store_context(&self, key: &str, context: &[u8]) -> AIResult<()> {
        let mut db = self.learning_db.lock().await;
        db.records.insert(key.to_string(), LearningRecord {
            transaction_hash: key.to_string(),
            decision: RoutingDecision::Skip,
            predicted_metrics: PredictedMetrics::default(),
            actual_metrics: ActualMetrics::default(),
            timestamp: SystemTime::now(),
            agent_id: None,
            rag_context_used: false,
        });
        Ok(())
    }
    
    async fn retrieve_context(&self, key: &str) -> AIResult<Option<Vec<u8>>> {
        let db = self.learning_db.lock().await;
        Ok(db.records.get(key).map(|r| r.transaction_hash.as_bytes().to_vec()))
    }
    
    async fn update_memory(&self, experience: &[u8]) -> AIResult<()> {
        let mut baseline = self.baseline_metrics.write();
        
        let old_avg = baseline.avg_transaction_size;
        let count = baseline.common_patterns.len() as f64 + 1.0;
        baseline.avg_transaction_size = (old_avg * (count - 1.0) + experience.len() as f64) / count;
        
        let pattern = hash_pattern(experience);
        *baseline.common_patterns.entry(pattern).or_insert(0) += 1;
        
        Ok(())
    }
}

impl Default for PredictedMetrics {
    fn default() -> Self {
        Self {
            complexity: 0.5,
            safety: 0.5,
            success_probability: 0.5,
            estimated_compute_units: 100_000,
        }
    }
}

impl Default for ActualMetrics {
    fn default() -> Self {
        Self {
            success: false,
            compute_units: 0,
            execution_time_ms: 0,
            gas_used: 0,
        }
    }
}