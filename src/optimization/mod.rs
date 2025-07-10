//! Query Optimization Module
//! 
//! This module provides comprehensive query optimization capabilities for the RAG system,
//! including intelligent query rewriting, advanced multi-tier caching, and performance
//! optimization specifically designed for SVM transaction processing speeds.

pub mod query;
pub mod caching;

pub use query::{
    QueryOptimizer, QueryOptimizerConfig, QueryAnalysis, OptimizedQuery,
    QueryType, QueryIntent, QueryVariant, OptimizationStrategy,
    QueryPlan, QueryStep, QueryStepType, PerformanceEstimate,
    QueryMetrics, QueryExecution,
};

pub use caching::{
    RAGCacheSystem, CacheConfig, CacheKey, CachedResult, CachedData,
    CacheLevel, InvalidationStrategy, CacheMetrics, AnalyticsEvent,
    AnalyticsEventType, CacheReport, CacheRecommendation,
};

use crate::errors::*;
use crate::ai::rag::{SearchResult, VectorStore};
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

/// Integrated optimization service combining query optimization and caching
pub struct OptimizationService {
    /// Query optimizer
    pub query_optimizer: Arc<QueryOptimizer>,
    /// Cache system
    pub cache_system: Arc<RAGCacheSystem>,
    /// Service configuration
    config: OptimizationServiceConfig,
}

/// Configuration for the optimization service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationServiceConfig {
    /// Query optimizer configuration
    pub query_config: QueryOptimizerConfig,
    /// Cache configuration
    pub cache_config: CacheConfig,
    /// Enable integrated optimization
    pub enable_integrated_optimization: bool,
    /// Cache query plans
    pub cache_query_plans: bool,
    /// Cache query results
    pub cache_query_results: bool,
    /// Performance monitoring interval
    pub monitoring_interval: Duration,
}

/// Optimized query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedExecutionResult {
    /// Search results
    pub results: Vec<SearchResult>,
    /// Optimization metadata
    pub optimization_metadata: OptimizationMetadata,
    /// Cache metadata
    pub cache_metadata: Option<CacheMetadata>,
    /// Performance metrics
    pub performance: ExecutionPerformance,
}

/// Optimization metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationMetadata {
    /// Query was optimized
    pub optimized: bool,
    /// Original query
    pub original_query: String,
    /// Final query used
    pub final_query: String,
    /// Optimization strategies applied
    pub strategies_applied: Vec<OptimizationStrategy>,
    /// Optimization confidence
    pub confidence: f64,
}

/// Cache metadata for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    /// Cache hit occurred
    pub cache_hit: bool,
    /// Cache level used
    pub cache_level: Option<CacheLevel>,
    /// Cache key
    pub cache_key: String,
    /// Cache age
    pub cache_age: Option<Duration>,
}

/// Execution performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPerformance {
    /// Total execution time
    pub total_time: Duration,
    /// Query optimization time
    pub optimization_time: Duration,
    /// Cache lookup time
    pub cache_lookup_time: Duration,
    /// Vector search time
    pub vector_search_time: Duration,
    /// Result processing time
    pub result_processing_time: Duration,
    /// Cache efficiency
    pub cache_efficiency: f64,
}

impl OptimizationService {
    /// Create a new optimization service
    pub fn new(config: OptimizationServiceConfig) -> Self {
        let query_optimizer = Arc::new(QueryOptimizer::new(config.query_config.clone()));
        let cache_system = Arc::new(RAGCacheSystem::new(config.cache_config.clone()));
        
        Self {
            query_optimizer,
            cache_system,
            config,
        }
    }
    
    /// Initialize the optimization service
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing optimization service");
        
        // Initialize query optimizer
        self.query_optimizer.initialize().await?;
        
        // Initialize cache system
        self.cache_system.initialize().await?;
        
        // Start monitoring if enabled
        if self.config.enable_integrated_optimization {
            self.start_monitoring().await?;
        }
        
        info!("Optimization service initialized successfully");
        Ok(())
    }
    
    /// Execute an optimized query with caching
    pub async fn execute_optimized_query(
        &self,
        query: &str,
        vector_store: Arc<dyn VectorStore>,
        limit: usize,
    ) -> Result<OptimizedExecutionResult> {
        let start_time = std::time::Instant::now();
        
        // Generate cache key
        let cache_key = CacheKey::new(query, "query_results");
        
        // Check cache first if enabled
        let cache_start = std::time::Instant::now();
        let cached_result = if self.config.cache_query_results {
            self.cache_system.get(&cache_key).await?
        } else {
            None
        };
        let cache_lookup_time = cache_start.elapsed();
        
        if let Some(cached) = cached_result {
            // Return cached results
            if let CachedData::QueryResults(results) = cached.data {
                debug!("Returning cached query results for: {}", query);
                
                return Ok(OptimizedExecutionResult {
                    results,
                    optimization_metadata: OptimizationMetadata {
                        optimized: false,
                        original_query: query.to_string(),
                        final_query: query.to_string(),
                        strategies_applied: vec![],
                        confidence: 1.0,
                    },
                    cache_metadata: Some(CacheMetadata {
                        cache_hit: true,
                        cache_level: Some(cached.metadata.cache_level),
                        cache_key: format!("{:?}", cache_key),
                        cache_age: Some(cached.metadata.created_at.elapsed().unwrap_or(Duration::ZERO)),
                    }),
                    performance: ExecutionPerformance {
                        total_time: start_time.elapsed(),
                        optimization_time: Duration::ZERO,
                        cache_lookup_time,
                        vector_search_time: Duration::ZERO,
                        result_processing_time: Duration::ZERO,
                        cache_efficiency: 1.0,
                    },
                });
            }
        }
        
        // No cache hit, proceed with optimization and execution
        debug!("Cache miss, optimizing and executing query: {}", query);
        
        // Optimize query
        let optimization_start = std::time::Instant::now();
        let optimized_query = self.query_optimizer.optimize(query).await?;
        let optimization_time = optimization_start.elapsed();
        
        // Execute optimized query
        let vector_search_start = std::time::Instant::now();
        let results = self.query_optimizer.execute(
            &optimized_query,
            vector_store,
            limit,
        ).await?;
        let vector_search_time = vector_search_start.elapsed();
        
        // Process and cache results
        let result_processing_start = std::time::Instant::now();
        
        // Cache the results if enabled
        if self.config.cache_query_results {
            let cached_data = CachedData::QueryResults(results.clone());
            self.cache_system.put(cache_key.clone(), cached_data).await?;
        }
        
        let result_processing_time = result_processing_start.elapsed();
        
        // Create optimization metadata
        let optimization_metadata = OptimizationMetadata {
            optimized: true,
            original_query: query.to_string(),
            final_query: optimized_query.variants.first()
                .map(|v| v.text.clone())
                .unwrap_or_else(|| query.to_string()),
            strategies_applied: optimized_query.variants.iter()
                .map(|v| v.strategy)
                .collect(),
            confidence: optimized_query.metadata.confidence,
        };
        
        Ok(OptimizedExecutionResult {
            results,
            optimization_metadata,
            cache_metadata: Some(CacheMetadata {
                cache_hit: false,
                cache_level: None,
                cache_key: format!("{:?}", cache_key),
                cache_age: None,
            }),
            performance: ExecutionPerformance {
                total_time: start_time.elapsed(),
                optimization_time,
                cache_lookup_time,
                vector_search_time,
                result_processing_time,
                cache_efficiency: 0.0, // No cache hit
            },
        })
    }
    
    /// Warm cache with common queries
    pub async fn warm_cache(&self, queries: Vec<String>) -> Result<()> {
        info!("Warming cache with {} queries", queries.len());
        self.cache_system.warm_cache(queries).await
    }
    
    /// Invalidate cache entries matching pattern
    pub async fn invalidate_cache(&self, pattern: &str) -> Result<u32> {
        info!("Invalidating cache with pattern: {}", pattern);
        self.cache_system.invalidate_pattern(pattern).await
    }
    
    /// Get comprehensive performance metrics
    pub async fn get_performance_metrics(&self) -> Result<PerformanceReport> {
        let query_metrics = self.query_optimizer.get_metrics().await;
        let cache_metrics = self.cache_system.get_stats().await;
        
        Ok(PerformanceReport {
            query_metrics,
            cache_metrics,
            integration_metrics: IntegrationMetrics {
                cache_enabled_queries: 0, // Would track actual metrics
                optimization_success_rate: query_metrics.optimization_success_rate,
                avg_cache_speedup: 0.0, // Would calculate actual speedup
                memory_efficiency: 0.0, // Would calculate actual efficiency
            },
        })
    }
    
    /// Start performance monitoring
    async fn start_monitoring(&self) -> Result<()> {
        let query_optimizer = self.query_optimizer.clone();
        let cache_system = self.cache_system.clone();
        let monitoring_interval = self.config.monitoring_interval;
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(monitoring_interval);
            
            loop {
                interval.tick().await;
                
                // Collect metrics
                let query_metrics = query_optimizer.get_metrics().await;
                let cache_metrics = cache_system.get_stats().await;
                
                debug!(
                    "Performance metrics - Query: avg_latency={}ms, success_rate={:.2}%, Cache: hit_rate={:.2}%",
                    query_metrics.avg_latency_ms,
                    query_metrics.success_rate * 100.0,
                    cache_metrics.overall_hit_rate * 100.0
                );
                
                // TODO: Implement performance alerting and optimization
            }
        });
        
        Ok(())
    }
    
    /// Optimize cache configuration based on usage patterns
    pub async fn optimize_cache_configuration(&self) -> Result<CacheOptimizationReport> {
        let metrics = self.cache_system.get_stats().await;
        let mut recommendations = Vec::new();
        
        // Analyze cache performance and generate recommendations
        if metrics.overall_hit_rate < 0.3 {
            recommendations.push(CacheRecommendation {
                rec_type: caching::RecommendationType::IncreaseL1Size,
                description: "Low hit rate detected. Consider increasing L1 cache size.".to_string(),
                expected_impact: 0.15,
                priority: caching::RecommendationPriority::High,
                complexity: caching::RecommendationComplexity::Low,
            });
        }
        
        if let Some(l1_hit_rate) = metrics.tier_hit_rates.get(&CacheLevel::L1) {
            if *l1_hit_rate > 0.9 {
                recommendations.push(CacheRecommendation {
                    rec_type: caching::RecommendationType::EnablePrefetch,
                    description: "High L1 hit rate suggests good prefetching opportunity.".to_string(),
                    expected_impact: 0.1,
                    priority: caching::RecommendationPriority::Medium,
                    complexity: caching::RecommendationComplexity::Medium,
                });
            }
        }
        
        Ok(CacheOptimizationReport {
            current_performance: metrics,
            recommendations,
            estimated_improvement: recommendations.iter()
                .map(|r| r.expected_impact)
                .sum::<f64>(),
        })
    }
    
    /// Update optimization configuration
    pub async fn update_configuration(&mut self, config: OptimizationServiceConfig) -> Result<()> {
        info!("Updating optimization service configuration");
        self.config = config;
        
        // Reinitialize if needed
        self.initialize().await?;
        
        Ok(())
    }
}

/// Comprehensive performance report
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceReport {
    /// Query optimization metrics
    pub query_metrics: QueryMetrics,
    /// Cache performance metrics
    pub cache_metrics: CacheMetrics,
    /// Integration-specific metrics
    pub integration_metrics: IntegrationMetrics,
}

/// Integration metrics between query optimization and caching
#[derive(Debug, Clone, Serialize)]
pub struct IntegrationMetrics {
    /// Number of queries that used cache
    pub cache_enabled_queries: u64,
    /// Optimization success rate
    pub optimization_success_rate: f64,
    /// Average speedup from caching
    pub avg_cache_speedup: f64,
    /// Memory efficiency
    pub memory_efficiency: f64,
}

/// Cache optimization report
#[derive(Debug, Clone, Serialize)]
pub struct CacheOptimizationReport {
    /// Current cache performance
    pub current_performance: CacheMetrics,
    /// Optimization recommendations
    pub recommendations: Vec<CacheRecommendation>,
    /// Estimated performance improvement
    pub estimated_improvement: f64,
}

impl Default for OptimizationServiceConfig {
    fn default() -> Self {
        Self {
            query_config: QueryOptimizerConfig::default(),
            cache_config: CacheConfig::default(),
            enable_integrated_optimization: true,
            cache_query_plans: true,
            cache_query_results: true,
            monitoring_interval: Duration::from_secs(60),
        }
    }
}

/// Utility functions for optimization
pub mod utils {
    use super::*;
    
    /// Calculate cache efficiency score
    pub fn calculate_cache_efficiency(
        cache_hits: u64,
        cache_misses: u64,
        avg_hit_latency: f64,
        avg_miss_latency: f64,
    ) -> f64 {
        if cache_hits + cache_misses == 0 {
            return 0.0;
        }
        
        let hit_rate = cache_hits as f64 / (cache_hits + cache_misses) as f64;
        let latency_improvement = if avg_miss_latency > 0.0 {
            1.0 - (avg_hit_latency / avg_miss_latency)
        } else {
            0.0
        };
        
        hit_rate * latency_improvement
    }
    
    /// Estimate query complexity score
    pub fn estimate_query_complexity(query: &str) -> f64 {
        let word_count = query.split_whitespace().count() as f64;
        let char_count = query.len() as f64;
        let has_operators = query.contains("AND") || query.contains("OR") || query.contains("NOT");
        
        let base_score = (word_count * 0.1 + char_count * 0.01).min(5.0);
        if has_operators {
            base_score * 1.5
        } else {
            base_score
        }
    }
    
    /// Generate cache key for query with parameters
    pub fn generate_cache_key(query: &str, params: &std::collections::HashMap<String, String>) -> CacheKey {
        use std::hash::{Hash, Hasher};
        
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        query.hash(&mut hasher);
        let query_hash = hasher.finish();
        
        let mut param_keys: Vec<&String> = params.keys().collect();
        param_keys.sort();
        
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for key in param_keys {
            key.hash(&mut hasher);
            params[key].hash(&mut hasher);
        }
        let params_hash = hasher.finish();
        
        CacheKey {
            query_hash,
            params_hash,
            namespace: "optimized_queries".to_string(),
            version: 1,
        }
    }
    
    /// Analyze query patterns for optimization opportunities
    pub fn analyze_query_patterns(queries: &[String]) -> QueryPatternAnalysis {
        let mut pattern_counts = std::collections::HashMap::new();
        let mut total_complexity = 0.0;
        
        for query in queries {
            // Extract common patterns
            let words: Vec<&str> = query.split_whitespace().collect();
            if words.len() >= 2 {
                for window in words.windows(2) {
                    let pattern = format!("{} {}", window[0], window[1]);
                    *pattern_counts.entry(pattern).or_insert(0) += 1;
                }
            }
            
            total_complexity += estimate_query_complexity(query);
        }
        
        let common_patterns: Vec<String> = pattern_counts
            .into_iter()
            .filter(|(_, count)| *count >= 2)
            .map(|(pattern, _)| pattern)
            .collect();
        
        QueryPatternAnalysis {
            total_queries: queries.len(),
            average_complexity: if queries.is_empty() { 0.0 } else { total_complexity / queries.len() as f64 },
            common_patterns,
            optimization_potential: if total_complexity > queries.len() as f64 * 2.0 { 0.8 } else { 0.3 },
        }
    }
}

/// Query pattern analysis result
#[derive(Debug, Clone, Serialize)]
pub struct QueryPatternAnalysis {
    /// Total number of queries analyzed
    pub total_queries: usize,
    /// Average complexity score
    pub average_complexity: f64,
    /// Common query patterns
    pub common_patterns: Vec<String>,
    /// Optimization potential (0.0-1.0)
    pub optimization_potential: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::rag::SearchResult;

    #[tokio::test]
    async fn test_optimization_service_creation() {
        let config = OptimizationServiceConfig::default();
        let service = OptimizationService::new(config);
        
        assert!(service.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_cache_efficiency_calculation() {
        let efficiency = utils::calculate_cache_efficiency(80, 20, 10.0, 100.0);
        assert!(efficiency > 0.7); // Should be high efficiency
        
        let low_efficiency = utils::calculate_cache_efficiency(10, 90, 50.0, 60.0);
        assert!(low_efficiency < 0.3); // Should be low efficiency
    }

    #[tokio::test]
    async fn test_query_complexity_estimation() {
        let simple_query = "find transaction";
        let complex_query = "find all transactions with value greater than 1000 AND status equals confirmed OR pending";
        
        let simple_score = utils::estimate_query_complexity(simple_query);
        let complex_score = utils::estimate_query_complexity(complex_query);
        
        assert!(complex_score > simple_score);
    }

    #[tokio::test]
    async fn test_query_pattern_analysis() {
        let queries = vec![
            "find transaction".to_string(),
            "find transaction by hash".to_string(),
            "search for contracts".to_string(),
            "find transaction details".to_string(),
        ];
        
        let analysis = utils::analyze_query_patterns(&queries);
        
        assert_eq!(analysis.total_queries, 4);
        assert!(analysis.common_patterns.contains(&"find transaction".to_string()));
    }
}