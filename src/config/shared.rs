//! Shared configuration system for cross-ExEx coordination
//! 
//! This module provides configuration structures that can be shared between
//! multiple ExEx instances to ensure consistent behavior and coordination.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use eyre::Result;

/// Shared configuration for all ExEx instances
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedExExConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// Transaction routing rules
    pub routing: RoutingConfig,
    /// AI coordination settings
    pub ai_coordination: AICoordinationConfig,
    /// Load balancing parameters
    pub load_balancing: LoadBalancingConfig,
    /// State synchronization settings
    pub state_sync: StateSyncConfig,
    /// Security policies
    pub security: SecurityConfig,
}

/// Network configuration for inter-ExEx communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Discovery service endpoints
    pub discovery_endpoints: Vec<String>,
    /// Message bus bind address template (e.g., "0.0.0.0:{}")
    pub bind_address_template: String,
    /// Starting port for ExEx instances
    pub base_port: u16,
    /// Enable encryption for inter-ExEx messages
    pub enable_encryption: bool,
    /// Network timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum message size in bytes
    pub max_message_size: usize,
    /// Heartbeat interval in seconds
    pub heartbeat_interval_secs: u64,
}

/// Transaction routing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    /// Routing strategy
    pub strategy: RoutingStrategy,
    /// ExEx type preferences for different transaction types
    pub type_preferences: HashMap<String, ExExTypePreference>,
    /// Maximum transactions per ExEx per block
    pub max_txs_per_exex: usize,
    /// Enable cross-ExEx transaction validation
    pub enable_cross_validation: bool,
}

/// Routing strategies for distributing transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingStrategy {
    /// Round-robin distribution
    RoundRobin,
    /// Load-based distribution
    LoadBased,
    /// AI-driven intelligent routing
    AIOptimized,
    /// Transaction type-based routing
    TypeBased,
    /// Hybrid approach
    Hybrid,
}

/// ExEx type preferences for routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExExTypePreference {
    /// Preferred ExEx type (SVM, RAG, etc.)
    pub preferred_type: String,
    /// Fallback types in order of preference
    pub fallback_types: Vec<String>,
    /// Minimum confidence threshold for routing
    pub confidence_threshold: f64,
}

/// AI coordination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AICoordinationConfig {
    /// Enable cross-ExEx AI coordination
    pub enabled: bool,
    /// Shared AI model parameters
    pub model_params: AIModelParams,
    /// Context sharing settings
    pub context_sharing: ContextSharingConfig,
    /// Decision aggregation method
    pub aggregation_method: AggregationMethod,
}

/// Shared AI model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIModelParams {
    /// Model temperature for decision making
    pub temperature: f64,
    /// Maximum context window size
    pub max_context_size: usize,
    /// Confidence threshold for decisions
    pub confidence_threshold: f64,
    /// Enable learning from cross-ExEx feedback
    pub enable_learning: bool,
    /// Model checkpoint sync interval (seconds)
    pub checkpoint_sync_interval: u64,
}

/// Context sharing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSharingConfig {
    /// Enable context sharing between ExEx instances
    pub enabled: bool,
    /// Maximum context age in seconds
    pub max_context_age_secs: u64,
    /// Context deduplication enabled
    pub deduplicate: bool,
    /// Compression algorithm for context transfer
    pub compression: CompressionType,
}

/// Compression types for data transfer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    None,
    Gzip,
    Zstd,
    Lz4,
}

/// Decision aggregation methods
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AggregationMethod {
    /// Simple majority voting
    Majority,
    /// Weighted by confidence scores
    WeightedConsensus,
    /// AI-based meta-decision
    MetaLearning,
    /// Threshold-based agreement
    ThresholdConsensus,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Enable dynamic load balancing
    pub enabled: bool,
    /// Load check interval in seconds
    pub check_interval_secs: u64,
    /// Maximum load difference percentage before rebalancing
    pub max_load_difference: u8,
    /// Minimum transactions before considering rebalancing
    pub min_tx_threshold: usize,
    /// Load metrics to consider
    pub metrics: LoadMetricsConfig,
}

/// Load metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetricsConfig {
    /// Consider CPU usage
    pub cpu_weight: f32,
    /// Consider memory usage
    pub memory_weight: f32,
    /// Consider queue depth
    pub queue_weight: f32,
    /// Consider processing time
    pub latency_weight: f32,
}

/// State synchronization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSyncConfig {
    /// Enable automatic state synchronization
    pub enabled: bool,
    /// Sync interval in milliseconds
    pub sync_interval_ms: u64,
    /// ALH verification enabled
    pub verify_alh: bool,
    /// Maximum state divergence allowed (blocks)
    pub max_divergence_blocks: u64,
    /// State snapshot interval (blocks)
    pub snapshot_interval: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable message authentication
    pub enable_auth: bool,
    /// Require node attestation
    pub require_attestation: bool,
    /// Allowed node IDs (empty = allow all)
    pub allowed_nodes: Vec<String>,
    /// Banned node IDs
    pub banned_nodes: Vec<String>,
    /// Rate limiting configuration
    pub rate_limits: RateLimitConfig,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum messages per second per node
    pub max_messages_per_second: u32,
    /// Maximum bandwidth per node (bytes/sec)
    pub max_bandwidth_per_node: u64,
    /// Burst allowance factor
    pub burst_factor: f32,
}

impl Default for SharedExExConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig {
                discovery_endpoints: vec!["http://localhost:8545".to_string()],
                bind_address_template: "0.0.0.0:{}".to_string(),
                base_port: 9000,
                enable_encryption: true,
                timeout_ms: 5000,
                max_message_size: 10 * 1024 * 1024, // 10MB
                heartbeat_interval_secs: 30,
            },
            routing: RoutingConfig {
                strategy: RoutingStrategy::LoadBased,
                type_preferences: HashMap::new(),
                max_txs_per_exex: 1000,
                enable_cross_validation: true,
            },
            ai_coordination: AICoordinationConfig {
                enabled: true,
                model_params: AIModelParams {
                    temperature: 0.7,
                    max_context_size: 8192,
                    confidence_threshold: 0.8,
                    enable_learning: true,
                    checkpoint_sync_interval: 300, // 5 minutes
                },
                context_sharing: ContextSharingConfig {
                    enabled: true,
                    max_context_age_secs: 600, // 10 minutes
                    deduplicate: true,
                    compression: CompressionType::Zstd,
                },
                aggregation_method: AggregationMethod::WeightedConsensus,
            },
            load_balancing: LoadBalancingConfig {
                enabled: true,
                check_interval_secs: 10,
                max_load_difference: 20, // 20%
                min_tx_threshold: 100,
                metrics: LoadMetricsConfig {
                    cpu_weight: 0.3,
                    memory_weight: 0.2,
                    queue_weight: 0.3,
                    latency_weight: 0.2,
                },
            },
            state_sync: StateSyncConfig {
                enabled: true,
                sync_interval_ms: 1000,
                verify_alh: true,
                max_divergence_blocks: 10,
                snapshot_interval: 1000,
            },
            security: SecurityConfig {
                enable_auth: true,
                require_attestation: false,
                allowed_nodes: vec![],
                banned_nodes: vec![],
                rate_limits: RateLimitConfig {
                    max_messages_per_second: 1000,
                    max_bandwidth_per_node: 100 * 1024 * 1024, // 100MB/s
                    burst_factor: 1.5,
                },
            },
        }
    }
}

impl SharedExExConfig {
    /// Load configuration from file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(&path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        // Validate network config
        if self.network.base_port == 0 {
            return Err(eyre::eyre!("Invalid base port"));
        }

        if self.network.max_message_size == 0 {
            return Err(eyre::eyre!("Invalid max message size"));
        }

        // Validate AI config
        if self.ai_coordination.model_params.temperature < 0.0 
            || self.ai_coordination.model_params.temperature > 2.0 {
            return Err(eyre::eyre!("Temperature must be between 0.0 and 2.0"));
        }

        if self.ai_coordination.model_params.confidence_threshold < 0.0 
            || self.ai_coordination.model_params.confidence_threshold > 1.0 {
            return Err(eyre::eyre!("Confidence threshold must be between 0.0 and 1.0"));
        }

        // Validate load balancing weights
        let total_weight = self.load_balancing.metrics.cpu_weight
            + self.load_balancing.metrics.memory_weight
            + self.load_balancing.metrics.queue_weight
            + self.load_balancing.metrics.latency_weight;

        if (total_weight - 1.0).abs() > 0.001 {
            return Err(eyre::eyre!("Load balancing weights must sum to 1.0"));
        }

        Ok(())
    }

    /// Merge with another configuration (other takes precedence)
    pub fn merge(&mut self, other: &Self) {
        // This is a simplified merge - in production, you'd want more sophisticated merging
        *self = other.clone();
    }

    /// Get configuration for a specific ExEx instance
    pub fn for_instance(&self, instance_id: usize) -> InstanceConfig {
        InstanceConfig {
            node_id: format!("exex-{}", instance_id),
            bind_port: self.network.base_port + instance_id as u16,
            shared: self.clone(),
        }
    }
}

/// Configuration for a specific ExEx instance
#[derive(Debug, Clone)]
pub struct InstanceConfig {
    /// Unique node identifier
    pub node_id: String,
    /// Bind port for this instance
    pub bind_port: u16,
    /// Shared configuration
    pub shared: SharedExExConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SharedExExConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_instance_config() {
        let config = SharedExExConfig::default();
        let instance = config.for_instance(0);
        assert_eq!(instance.node_id, "exex-0");
        assert_eq!(instance.bind_port, config.network.base_port);
    }

    #[test]
    fn test_validation() {
        let mut config = SharedExExConfig::default();
        config.ai_coordination.model_params.temperature = 3.0;
        assert!(config.validate().is_err());
    }
}