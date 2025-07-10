//! Configuration management for the SVM ExEx system
//! 
//! This module provides comprehensive configuration management including:
//! - Individual ExEx configuration
//! - Shared cross-ExEx configuration
//! - Environment variable support
//! - Configuration validation and merging

pub mod shared;

pub use shared::{
    SharedExExConfig, NetworkConfig, RoutingConfig, RoutingStrategy,
    AICoordinationConfig, LoadBalancingConfig, StateSyncConfig,
    SecurityConfig, InstanceConfig,
};

use serde::{Deserialize, Serialize};
use std::path::Path;
use eyre::Result;

/// Main configuration structure combining individual and shared configs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExExConfiguration {
    /// Individual ExEx configuration
    pub exex: IndividualExExConfig,
    /// Shared configuration for cross-ExEx coordination
    pub shared: SharedExExConfig,
}

/// Individual ExEx-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualExExConfig {
    /// Unique identifier for this ExEx instance
    pub instance_id: String,
    /// ExEx type (SVM, RAG, Hybrid)
    pub exex_type: ExExType,
    /// Local data directory
    pub data_dir: String,
    /// Log file path
    pub log_file: Option<String>,
    /// Performance tuning
    pub performance: PerformanceConfig,
    /// SVM-specific settings
    pub svm_settings: Option<SvmSettings>,
    /// RAG-specific settings
    pub rag_settings: Option<RagSettings>,
}

/// ExEx types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExExType {
    /// SVM transaction processor
    SVM,
    /// RAG context provider
    RAG,
    /// Hybrid with both capabilities
    Hybrid,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of worker threads
    pub worker_threads: usize,
    /// Maximum memory usage in MB
    pub max_memory_mb: usize,
    /// Cache size in MB
    pub cache_size_mb: usize,
    /// Enable performance profiling
    pub enable_profiling: bool,
}

/// SVM-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvmSettings {
    /// Maximum compute units per transaction
    pub max_compute_units: u64,
    /// Account data cache size
    pub account_cache_size: usize,
    /// Program cache size
    pub program_cache_size: usize,
    /// Enable JIT compilation
    pub enable_jit: bool,
}

/// RAG-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagSettings {
    /// Vector database path
    pub vector_db_path: String,
    /// Embedding model
    pub embedding_model: String,
    /// Context retrieval limit
    pub retrieval_limit: usize,
    /// Similarity threshold
    pub similarity_threshold: f32,
}

impl Default for ExExConfiguration {
    fn default() -> Self {
        Self {
            exex: IndividualExExConfig::default(),
            shared: SharedExExConfig::default(),
        }
    }
}

impl Default for IndividualExExConfig {
    fn default() -> Self {
        Self {
            instance_id: "exex-default".to_string(),
            exex_type: ExExType::SVM,
            data_dir: "./data".to_string(),
            log_file: None,
            performance: PerformanceConfig {
                worker_threads: num_cpus::get(),
                max_memory_mb: 4096,
                cache_size_mb: 512,
                enable_profiling: false,
            },
            svm_settings: Some(SvmSettings {
                max_compute_units: 1_400_000,
                account_cache_size: 10000,
                program_cache_size: 100,
                enable_jit: true,
            }),
            rag_settings: None,
        }
    }
}

impl ExExConfiguration {
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

    /// Load from environment variables
    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        // Individual config from env
        if let Ok(id) = std::env::var("EXEX_INSTANCE_ID") {
            config.exex.instance_id = id;
        }

        if let Ok(data_dir) = std::env::var("EXEX_DATA_DIR") {
            config.exex.data_dir = data_dir;
        }

        if let Ok(threads) = std::env::var("EXEX_WORKER_THREADS") {
            config.exex.performance.worker_threads = threads.parse()?;
        }

        // Shared config from env
        if let Ok(port) = std::env::var("EXEX_BASE_PORT") {
            config.shared.network.base_port = port.parse()?;
        }

        if let Ok(encryption) = std::env::var("EXEX_ENABLE_ENCRYPTION") {
            config.shared.network.enable_encryption = encryption.parse()?;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate individual config
        if self.exex.instance_id.is_empty() {
            return Err(eyre::eyre!("Instance ID cannot be empty"));
        }

        if self.exex.performance.worker_threads == 0 {
            return Err(eyre::eyre!("Worker threads must be at least 1"));
        }

        // Validate shared config
        self.shared.validate()?;

        // Type-specific validation
        match self.exex.exex_type {
            ExExType::SVM | ExExType::Hybrid => {
                if self.exex.svm_settings.is_none() {
                    return Err(eyre::eyre!("SVM settings required for SVM/Hybrid ExEx"));
                }
            }
            ExExType::RAG => {
                if self.exex.rag_settings.is_none() {
                    return Err(eyre::eyre!("RAG settings required for RAG ExEx"));
                }
            }
        }

        Ok(())
    }

    /// Create configuration for a specific instance
    pub fn for_instance(&self, instance_num: usize) -> Self {
        let mut config = self.clone();
        config.exex.instance_id = format!("{}-{}", self.exex.instance_id, instance_num);
        
        // Update shared config for this instance
        let instance_config = config.shared.for_instance(instance_num);
        config.exex.instance_id = instance_config.node_id;
        
        config
    }
}

/// Configuration builder for easier setup
pub struct ConfigBuilder {
    config: ExExConfiguration,
}

impl ConfigBuilder {
    /// Create a new builder with default config
    pub fn new() -> Self {
        Self {
            config: ExExConfiguration::default(),
        }
    }

    /// Set instance ID
    pub fn instance_id(mut self, id: impl Into<String>) -> Self {
        self.config.exex.instance_id = id.into();
        self
    }

    /// Set ExEx type
    pub fn exex_type(mut self, exex_type: ExExType) -> Self {
        self.config.exex.exex_type = exex_type;
        self
    }

    /// Set data directory
    pub fn data_dir(mut self, dir: impl Into<String>) -> Self {
        self.config.exex.data_dir = dir.into();
        self
    }

    /// Set worker threads
    pub fn worker_threads(mut self, threads: usize) -> Self {
        self.config.exex.performance.worker_threads = threads;
        self
    }

    /// Set base port for network
    pub fn base_port(mut self, port: u16) -> Self {
        self.config.shared.network.base_port = port;
        self
    }

    /// Enable/disable encryption
    pub fn encryption(mut self, enabled: bool) -> Self {
        self.config.shared.network.enable_encryption = enabled;
        self
    }

    /// Set routing strategy
    pub fn routing_strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.config.shared.routing.strategy = strategy;
        self
    }

    /// Build the configuration
    pub fn build(self) -> Result<ExExConfiguration> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for ConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ExExConfiguration::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = ConfigBuilder::new()
            .instance_id("test-exex")
            .exex_type(ExExType::SVM)
            .worker_threads(8)
            .base_port(9000)
            .build();
        
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.exex.instance_id, "test-exex");
        assert_eq!(config.exex.performance.worker_threads, 8);
    }

    #[test]
    fn test_instance_config() {
        let config = ExExConfiguration::default();
        let instance = config.for_instance(1);
        assert!(instance.exex.instance_id.contains("-1"));
    }
}