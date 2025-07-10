//! Enhanced SVM Processor with ALH and Advanced Features
//! 
//! This module implements the core SVM processing logic with:
//! - Accounts Lattice Hash for efficient state management
//! - Multi-tier caching strategy
//! - Parallel instruction execution
//! - Advanced AI decision engine

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use dashmap::DashMap;
use lru::LruCache;
use parking_lot::RwLock;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Semaphore};

use crate::{
    enhanced_exex::AccountsLatticeHash,
    errors::*,
    traits::{AIAgent, RoutingDecision},
};

/// Multi-tier cache configuration
pub struct CacheConfig {
    pub l1_size: usize,        // Hot accounts (in-memory)
    pub l2_size: usize,        // Warm accounts (in-memory)
    pub l3_enabled: bool,      // Cold storage (disk)
    pub ttl_seconds: u64,      // Time-to-live for cached items
    pub prefetch_enabled: bool, // AI-driven prefetching
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 10_000,
            l2_size: 100_000,
            l3_enabled: true,
            ttl_seconds: 300,
            prefetch_enabled: true,
        }
    }
}

/// Enhanced account structure with ALH integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAccount {
    pub pubkey: [u8; 32],
    pub lamports: u64,
    pub data: Vec<u8>,
    pub owner: [u8; 32],
    pub executable: bool,
    pub rent_epoch: u64,
    pub data_hash: [u8; 32],
    pub last_modified_slot: u64,
    pub access_count: u64,
    pub last_accessed: SystemTime,
}

impl EnhancedAccount {
    /// Compute data hash for ALH
    pub fn compute_data_hash(&mut self) {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        hasher.update(&self.owner);
        hasher.update(&[self.executable as u8]);
        self.data_hash = hasher.finalize().into();
    }

    /// Update access statistics
    pub fn record_access(&mut self) {
        self.access_count += 1;
        self.last_accessed = SystemTime::now();
    }
}

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    data: T,
    inserted_at: Instant,
    access_count: u64,
    tier: CacheTier,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CacheTier {
    L1Hot,
    L2Warm,
    L3Cold,
}

/// Multi-tier account cache
pub struct AccountCache {
    config: CacheConfig,
    l1_cache: Arc<RwLock<LruCache<[u8; 32], CacheEntry<EnhancedAccount>>>>,
    l2_cache: Arc<DashMap<[u8; 32], CacheEntry<EnhancedAccount>>>,
    access_patterns: Arc<RwLock<HashMap<[u8; 32], AccessPattern>>>,
    prefetch_predictor: Arc<Mutex<PrefetchPredictor>>,
}

/// Access pattern tracking for intelligent caching
#[derive(Debug, Clone)]
struct AccessPattern {
    pub access_times: Vec<Instant>,
    pub access_intervals: Vec<Duration>,
    pub avg_interval: Duration,
    pub prediction_confidence: f64,
}

/// AI-driven prefetch predictor
struct PrefetchPredictor {
    model_weights: HashMap<String, f64>,
    learning_rate: f64,
    prediction_cache: LruCache<[u8; 32], PrefetchPrediction>,
}

#[derive(Debug, Clone)]
struct PrefetchPrediction {
    pub likely_next_accounts: Vec<[u8; 32]>,
    pub confidence: f64,
    pub predicted_at: Instant,
}

impl AccountCache {
    pub fn new(config: CacheConfig) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(LruCache::new(config.l1_size))),
            l2_cache: Arc::new(DashMap::new()),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            prefetch_predictor: Arc::new(Mutex::new(PrefetchPredictor::new())),
            config,
        }
    }

    /// Get account with intelligent caching
    pub async fn get_account(&self, pubkey: &[u8; 32]) -> Option<EnhancedAccount> {
        // Check L1 (hot) cache
        {
            let mut l1 = self.l1_cache.write();
            if let Some(entry) = l1.get_mut(pubkey) {
                entry.access_count += 1;
                self.record_access_pattern(pubkey).await;
                return Some(entry.data.clone());
            }
        }

        // Check L2 (warm) cache
        if let Some(mut entry) = self.l2_cache.get_mut(pubkey) {
            entry.access_count += 1;
            let account = entry.data.clone();
            
            // Promote to L1 if frequently accessed
            if entry.access_count > 10 {
                self.promote_to_l1(pubkey, account.clone()).await;
            }
            
            self.record_access_pattern(pubkey).await;
            return Some(account);
        }

        // L3 (cold storage) would be checked here
        // Not implemented in this example

        None
    }

    /// Insert account with appropriate tier placement
    pub async fn insert_account(&self, account: EnhancedAccount) {
        let pubkey = account.pubkey;
        
        // Determine initial tier based on AI prediction
        let tier = self.determine_initial_tier(&account).await;
        
        let entry = CacheEntry {
            data: account,
            inserted_at: Instant::now(),
            access_count: 1,
            tier,
        };

        match tier {
            CacheTier::L1Hot => {
                let mut l1 = self.l1_cache.write();
                l1.put(pubkey, entry);
            }
            CacheTier::L2Warm => {
                self.l2_cache.insert(pubkey, entry);
            }
            CacheTier::L3Cold => {
                // Would write to disk storage
            }
        }

        // Trigger prefetch for related accounts
        if self.config.prefetch_enabled {
            self.prefetch_related_accounts(&pubkey).await;
        }
    }

    /// Record access pattern for predictive caching
    async fn record_access_pattern(&self, pubkey: &[u8; 32]) {
        let mut patterns = self.access_patterns.write();
        let now = Instant::now();
        
        let pattern = patterns.entry(*pubkey).or_insert_with(|| AccessPattern {
            access_times: Vec::new(),
            access_intervals: Vec::new(),
            avg_interval: Duration::from_secs(0),
            prediction_confidence: 0.0,
        });

        if let Some(last_access) = pattern.access_times.last() {
            let interval = now - *last_access;
            pattern.access_intervals.push(interval);
            
            // Update average interval
            let sum: Duration = pattern.access_intervals.iter().sum();
            pattern.avg_interval = sum / pattern.access_intervals.len() as u32;
            
            // Update prediction confidence
            if pattern.access_intervals.len() > 3 {
                let variance = pattern.access_intervals.iter()
                    .map(|i| {
                        let diff = i.as_secs_f64() - pattern.avg_interval.as_secs_f64();
                        diff * diff
                    })
                    .sum::<f64>() / pattern.access_intervals.len() as f64;
                
                pattern.prediction_confidence = 1.0 / (1.0 + variance.sqrt());
            }
        }
        
        pattern.access_times.push(now);
        
        // Keep only recent history
        if pattern.access_times.len() > 100 {
            pattern.access_times.drain(0..50);
            pattern.access_intervals.drain(0..50);
        }
    }

    /// Promote account to L1 cache
    async fn promote_to_l1(&self, pubkey: &[u8; 32], account: EnhancedAccount) {
        let mut l1 = self.l1_cache.write();
        
        // If L1 is full, demote least recently used to L2
        if l1.len() >= self.config.l1_size {
            if let Some((demoted_key, demoted_entry)) = l1.pop_lru() {
                self.l2_cache.insert(demoted_key, CacheEntry {
                    data: demoted_entry.data,
                    inserted_at: demoted_entry.inserted_at,
                    access_count: demoted_entry.access_count,
                    tier: CacheTier::L2Warm,
                });
            }
        }
        
        l1.put(*pubkey, CacheEntry {
            data: account,
            inserted_at: Instant::now(),
            access_count: 1,
            tier: CacheTier::L1Hot,
        });
    }

    /// Determine initial cache tier using AI
    async fn determine_initial_tier(&self, account: &EnhancedAccount) -> CacheTier {
        // Simple heuristic for now, would use AI model in production
        if account.executable || account.lamports > 1_000_000 {
            CacheTier::L1Hot
        } else if account.data.len() > 1024 {
            CacheTier::L2Warm
        } else {
            CacheTier::L3Cold
        }
    }

    /// Prefetch related accounts based on access patterns
    async fn prefetch_related_accounts(&self, pubkey: &[u8; 32]) {
        let predictor = self.prefetch_predictor.lock().await;
        if let Some(prediction) = predictor.get_prediction(pubkey) {
            if prediction.confidence > 0.7 {
                for related_pubkey in &prediction.likely_next_accounts {
                    // Async prefetch (would load from storage)
                    tokio::spawn({
                        let pubkey = *related_pubkey;
                        async move {
                            // Load account from storage
                            // Insert into appropriate cache tier
                        }
                    });
                }
            }
        }
    }
}

impl PrefetchPredictor {
    fn new() -> Self {
        Self {
            model_weights: HashMap::new(),
            learning_rate: 0.01,
            prediction_cache: LruCache::new(1000),
        }
    }

    fn get_prediction(&self, pubkey: &[u8; 32]) -> Option<PrefetchPrediction> {
        self.prediction_cache.peek(pubkey).cloned()
    }

    #[allow(dead_code)]
    fn update_model(&mut self, actual_sequence: &[[u8; 32]], predicted: &[[u8; 32]]) {
        // Update model weights based on prediction accuracy
        // Simplified example - real implementation would use proper ML
        let accuracy = actual_sequence.iter()
            .zip(predicted.iter())
            .filter(|(a, p)| a == p)
            .count() as f64 / actual_sequence.len().max(1) as f64;
        
        let weight_key = "sequence_accuracy".to_string();
        let current_weight = self.model_weights.get(&weight_key).copied().unwrap_or(0.5);
        let new_weight = current_weight + self.learning_rate * (accuracy - current_weight);
        self.model_weights.insert(weight_key, new_weight);
    }
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DecisionWeights {
    complexity_weight: f64,
    safety_weight: f64,
    gas_price_weight: f64,
    congestion_weight: f64,
    history_weight: f64,
    anomaly_weight: f64,
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
}

struct CongestionMonitor {
    current_tps: Arc<RwLock<f64>>,
    target_tps: f64,
    congestion_threshold: f64,
}

struct AnomalyDetector {
    baseline_metrics: Arc<RwLock<BaselineMetrics>>,
    detection_threshold: f64,
}

#[derive(Debug, Clone, Default)]
struct BaselineMetrics {
    avg_transaction_size: f64,
    avg_instruction_count: f64,
    common_patterns: HashMap<Vec<u8>, u64>,
}

struct LearningDatabase {
    // In production, this would be SQLite or RocksDB
    records: HashMap<String, LearningRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LearningRecord {
    transaction_hash: String,
    decision: RoutingDecision,
    predicted_metrics: PredictedMetrics,
    actual_metrics: ActualMetrics,
    timestamp: SystemTime,
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

impl EnhancedAIDecisionEngine {
    pub fn new() -> Self {
        Self {
            weights: Arc::new(RwLock::new(DecisionWeights::default())),
            execution_history: Arc::new(DashMap::new()),
            congestion_monitor: Arc::new(CongestionMonitor::new()),
            anomaly_detector: Arc::new(AnomalyDetector::new()),
            learning_db: Arc::new(Mutex::new(LearningDatabase::new())),
        }
    }

    /// Make routing decision with multi-factor analysis
    pub async fn analyze_transaction(&self, data: &[u8]) -> AIResult<TransactionAnalysis> {
        let start = Instant::now();
        
        // Parallel analysis of different factors
        let (complexity, safety, anomaly_score, gas_estimate) = tokio::join!(
            self.analyze_complexity(data),
            self.analyze_safety(data),
            self.detect_anomalies(data),
            self.estimate_gas_cost(data),
        );

        // Get current network congestion
        let congestion = self.congestion_monitor.get_congestion_level();
        
        // Check execution history
        let history_score = self.get_history_score(data).await;
        
        // Calculate weighted decision score
        let weights = self.weights.read();
        let decision_score = 
            complexity * weights.complexity_weight +
            safety * weights.safety_weight +
            (1.0 - anomaly_score) * weights.anomaly_weight +
            (1.0 - congestion) * weights.congestion_weight +
            history_score * weights.history_weight +
            (1.0 / (1.0 + gas_estimate as f64 / 1000.0)) * weights.gas_price_weight;
        
        drop(weights);
        
        // Determine routing decision
        let routing = if decision_score > 0.7 {
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
        };
        
        // Store analysis for learning
        self.store_analysis(&analysis, data).await?;
        
        Ok(analysis)
    }

    async fn analyze_complexity(&self, data: &[u8]) -> f64 {
        // Entropy-based complexity with instruction analysis
        let entropy = calculate_entropy(data);
        let instruction_complexity = self.estimate_instruction_complexity(data);
        let size_factor = (data.len() as f64 / 1024.0).min(1.0);
        
        (entropy * 0.4 + instruction_complexity * 0.4 + size_factor * 0.2).min(1.0)
    }

    async fn analyze_safety(&self, data: &[u8]) -> f64 {
        let mut safety = 0.9; // Start with high safety
        
        // Check for known malicious patterns
        if contains_malicious_patterns(data) {
            safety -= 0.5;
        }
        
        // Check transaction structure validity
        if !is_well_formed_transaction(data) {
            safety -= 0.3;
        }
        
        // Size-based safety (very large transactions are riskier)
        if data.len() > 50_000 {
            safety -= 0.2;
        }
        
        safety.max(0.0f64)
    }

    async fn detect_anomalies(&self, data: &[u8]) -> f64 {
        self.anomaly_detector.detect(data).await
    }

    async fn estimate_gas_cost(&self, data: &[u8]) -> u64 {
        // Simple estimation - would use more sophisticated model in production
        let base_cost = 21_000u64;
        let data_cost = (data.len() as u64) * 16;
        let complexity_multiplier = self.estimate_instruction_complexity(data) * 10.0;
        
        base_cost + data_cost + (complexity_multiplier as u64 * 1000)
    }

    async fn get_history_score(&self, data: &[u8]) -> f64 {
        // Check if we've seen similar transactions
        let hash = hash_transaction(data);
        
        if let Some(history) = self.execution_history.get(&hash) {
            let success_rate = history.successful_executions as f64 / 
                              history.total_executions.max(1) as f64;
            success_rate
        } else {
            0.5 // Neutral score for unknown transactions
        }
    }

    fn estimate_instruction_complexity(&self, data: &[u8]) -> f64 {
        // Analyze instruction patterns
        if data.len() < 4 {
            return 0.1;
        }
        
        // Count unique 4-byte sequences (simplified instruction detection)
        let mut sequences = std::collections::HashSet::new();
        for window in data.windows(4) {
            sequences.insert(window);
        }
        
        let unique_ratio = sequences.len() as f64 / (data.len() / 4).max(1) as f64;
        unique_ratio.min(1.0)
    }

    fn is_high_value_transaction(&self, data: &[u8]) -> bool {
        // Simple heuristic - check for large numbers in data
        data.windows(8)
            .any(|window| {
                let value = u64::from_le_bytes(window.try_into().unwrap_or([0; 8]));
                value > 1_000_000_000 // 1 SOL in lamports
            })
    }

    async fn store_analysis(&self, analysis: &TransactionAnalysis, data: &[u8]) -> AIResult<()> {
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
                success: false, // Will be updated after execution
                compute_units: 0,
                execution_time_ms: 0,
                gas_used: 0,
            },
            timestamp: SystemTime::now(),
        };
        
        let mut db = self.learning_db.lock().await;
        db.records.insert(hash, record);
        
        Ok(())
    }

    /// Update learning database with actual execution results
    pub async fn update_with_results(
        &self, 
        tx_hash: &str, 
        actual: ActualMetrics
    ) -> AIResult<()> {
        let mut db = self.learning_db.lock().await;
        
        if let Some(record) = db.records.get_mut(tx_hash) {
            record.actual_metrics = actual;
            
            // Update model weights based on prediction accuracy
            self.update_weights(&record.predicted_metrics, &record.actual_metrics).await;
        }
        
        Ok(())
    }

    async fn update_weights(&self, predicted: &PredictedMetrics, actual: &ActualMetrics) {
        let mut weights = self.weights.write();
        
        // Simple gradient descent update
        let learning_rate = 0.001;
        let prediction_error = (predicted.success_probability - actual.success as u8 as f64).abs();
        
        // Adjust weights based on prediction accuracy
        if prediction_error > 0.3 {
            // Large error - need to adjust weights
            weights.complexity_weight *= 1.0 - learning_rate * prediction_error;
            weights.safety_weight *= 1.0 + learning_rate * prediction_error;
        }
        
        // Normalize weights
        let sum = weights.complexity_weight + weights.safety_weight + 
                  weights.gas_price_weight + weights.congestion_weight + 
                  weights.history_weight + weights.anomaly_weight;
        
        weights.complexity_weight /= sum;
        weights.safety_weight /= sum;
        weights.gas_price_weight /= sum;
        weights.congestion_weight /= sum;
        weights.history_weight /= sum;
        weights.anomaly_weight /= sum;
    }
}

impl CongestionMonitor {
    fn new() -> Self {
        Self {
            current_tps: Arc::new(RwLock::new(0.0)),
            target_tps: 50_000.0,
            congestion_threshold: 0.8,
        }
    }

    fn get_congestion_level(&self) -> f64 {
        let current = *self.current_tps.read();
        (current / self.target_tps).min(1.0)
    }

    #[allow(dead_code)]
    fn update_tps(&self, new_tps: f64) {
        *self.current_tps.write() = new_tps;
    }
}

impl AnomalyDetector {
    fn new() -> Self {
        Self {
            baseline_metrics: Arc::new(RwLock::new(BaselineMetrics::default())),
            detection_threshold: 2.0, // 2 standard deviations
        }
    }

    async fn detect(&self, data: &[u8]) -> f64 {
        let baseline = self.baseline_metrics.read();
        let mut anomaly_score = 0.0;
        
        // Check transaction size anomaly
        let size_diff = (data.len() as f64 - baseline.avg_transaction_size).abs();
        if size_diff > baseline.avg_transaction_size * self.detection_threshold {
            anomaly_score += 0.3;
        }
        
        // Check for unusual patterns
        let pattern_hash = hash_pattern(data);
        if !baseline.common_patterns.contains_key(&pattern_hash) {
            anomaly_score += 0.2;
        }
        
        // Check for suspicious byte sequences
        if has_suspicious_sequences(data) {
            anomaly_score += 0.5;
        }
        
        anomaly_score.min(1.0f64)
    }
}

impl LearningDatabase {
    fn new() -> Self {
        Self {
            records: HashMap::new(),
        }
    }
    
    #[allow(dead_code)]
    async fn persist(&self) -> AIResult<()> {
        // In production, write to SQLite/RocksDB
        Ok(())
    }
    
    #[allow(dead_code)]
    async fn load() -> AIResult<Self> {
        // In production, load from SQLite/RocksDB
        Ok(Self::new())
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
    
    entropy / 8.0 // Normalize to [0, 1]
}

fn contains_malicious_patterns(data: &[u8]) -> bool {
    // Check for known malicious patterns
    const MALICIOUS_PATTERNS: &[&[u8]] = &[
        b"\x00\x00\x00\x00\x00\x00\x00\x00", // Null bytes
        b"\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF", // All ones
        b"SELFDESTRUCT",                      // Dangerous opcodes
    ];
    
    MALICIOUS_PATTERNS.iter().any(|pattern| {
        data.windows(pattern.len()).any(|window| window == *pattern)
    })
}

fn is_well_formed_transaction(data: &[u8]) -> bool {
    // Basic structure validation
    data.len() >= 32 && data.len() < 1_000_000
}

fn hash_transaction(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn hash_pattern(data: &[u8]) -> Vec<u8> {
    // Create a pattern hash for anomaly detection
    if data.len() < 32 {
        data.to_vec()
    } else {
        let mut pattern = Vec::with_capacity(32);
        pattern.extend_from_slice(&data[0..8]);      // First 8 bytes
        pattern.extend_from_slice(&data[data.len()-8..]); // Last 8 bytes
        pattern.extend_from_slice(&data[data.len()/2-8..data.len()/2]); // Middle 8 bytes
        pattern
    }
}

fn has_suspicious_sequences(data: &[u8]) -> bool {
    // Check for suspicious byte sequences
    let suspicious_count = data.windows(4)
        .filter(|window| {
            // All same byte
            window[0] == window[1] && window[1] == window[2] && window[2] == window[3]
        })
        .count();
    
    suspicious_count > data.len() / 100 // More than 1% suspicious
}

#[async_trait::async_trait]
impl AIAgent for EnhancedAIDecisionEngine {
    async fn make_routing_decision(&self, context: &[u8]) -> AIResult<RoutingDecision> {
        let analysis = self.analyze_transaction(context).await?;
        Ok(analysis.routing_decision)
    }

    async fn store_context(&self, key: &str, context: &[u8]) -> AIResult<()> {
        // Store in learning database
        let mut db = self.learning_db.lock().await;
        db.records.insert(key.to_string(), LearningRecord {
            transaction_hash: key.to_string(),
            decision: RoutingDecision::Skip,
            predicted_metrics: PredictedMetrics::default(),
            actual_metrics: ActualMetrics::default(),
            timestamp: SystemTime::now(),
        });
        Ok(())
    }

    async fn retrieve_context(&self, key: &str) -> AIResult<Option<Vec<u8>>> {
        let db = self.learning_db.lock().await;
        Ok(db.records.get(key).map(|r| r.transaction_hash.as_bytes().to_vec()))
    }

    async fn update_memory(&self, experience: &[u8]) -> AIResult<()> {
        // Update baseline metrics
        let mut baseline = self.baseline_metrics.write();
        
        // Update average transaction size
        let old_avg = baseline.avg_transaction_size;
        let count = baseline.common_patterns.len() as f64 + 1.0;
        baseline.avg_transaction_size = (old_avg * (count - 1.0) + experience.len() as f64) / count;
        
        // Update common patterns
        let pattern = hash_pattern(experience);
        *baseline.common_patterns.entry(pattern).or_insert(0) += 1;
        
        Ok(())
    }
}

// Implement Default for structs that need it
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