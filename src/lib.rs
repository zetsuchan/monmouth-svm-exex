//! # Monmouth SVM ExEx
//! 
//! A sophisticated Execution Extension (ExEx) for Reth that integrates the Solana Virtual Machine
//! (SVM) to enable AI agent execution within Ethereum nodes. This implementation provides:
//! 
//! - **Hybrid EVM/SVM Execution**: Execute Solana smart contracts within Ethereum environment
//! - **AI Agent Integration**: Built-in support for AI-powered transaction routing and execution
//! - **Cross-Chain State Management**: Seamless state synchronization between EVM and SVM
//! - **Performance Optimization**: High-throughput transaction processing with minimal overhead
//! 
//! ## Features
//! 
//! - Real-time transaction routing based on AI agent decisions
//! - Memory-efficient SVM state management with caching
//! - Comprehensive error handling and structured logging
//! - Cross-chain address mapping and call translation
//! - Extensible architecture for custom AI agent implementations
//! 
//! ## Architecture
//! 
//! The system consists of several key components:
//! 
//! - **ExEx Framework**: Integrates with Reth's execution extension system
//! - **SVM Processor**: Handles Solana transaction sanitization, account loading, and execution
//! - **AI Agent Engine**: Makes intelligent routing decisions and manages context/memory
//! - **Cross-Chain Bridge**: Manages state synchronization and address mapping
//! 
//! ## Usage
//! 
//! ```rust,no_run
//! use monmouth_svm_exex::{EnhancedSvmExEx, SvmState, logging};
//! 
//! #[tokio::main]
//! async fn main() -> eyre::Result<()> {
//!     logging::init_ai_agent_logging()?;
//!     
//!     // Initialize and run the ExEx
//!     // See implementation examples in implementation1/ and implementation2/
//!     Ok(())
//! }
//! ```

pub mod errors;
pub mod logging;
pub mod enhanced_exex;
pub mod enhanced_processor;

// Re-export commonly used types and functions
pub use errors::{
    SvmExExError, SvmExExResult, RethExExError, SvmProcessingError, AIAgentError,
    CrossChainError, ErrorContext, ErrorSeverity,
};

pub use logging::{
    init_logging, init_ai_agent_logging, svm_transaction_span, ai_agent_span,
    cross_chain_span, PerformanceLogger, log_ai_decision, log_transaction_routing,
};

// Version and build information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const BUILD_TIMESTAMP: &str = env!("BUILD_TIMESTAMP");
pub const BUILD_PROFILE: &str = env!("BUILD_PROFILE");

/// Core traits for the SVM ExEx system
pub mod traits {
    use crate::errors::*;
    use async_trait::async_trait;

    /// Trait for SVM transaction processing
    #[async_trait]
    pub trait SvmProcessor: Send + Sync {
        async fn sanitize_transaction(&self, data: &[u8]) -> SvmResult<Vec<u8>>;
        async fn load_accounts(&self, transaction: &[u8]) -> SvmResult<Vec<String>>;
        async fn process_instructions(&self, instructions: &[u8]) -> SvmResult<String>;
        async fn commit_state(&self, state_changes: &[u8]) -> SvmResult<()>;
    }

    /// Trait for AI agent decision making
    #[async_trait]
    pub trait AIAgent: Send + Sync {
        async fn make_routing_decision(&self, context: &[u8]) -> AIResult<RoutingDecision>;
        async fn store_context(&self, key: &str, context: &[u8]) -> AIResult<()>;
        async fn retrieve_context(&self, key: &str) -> AIResult<Option<Vec<u8>>>;
        async fn update_memory(&self, experience: &[u8]) -> AIResult<()>;
    }

    /// Trait for cross-chain bridge operations
    #[async_trait]
    pub trait CrossChainBridge: Send + Sync {
        async fn map_address(&self, eth_address: [u8; 20]) -> CrossChainResult<[u8; 32]>;
        async fn translate_call(&self, evm_call: &[u8]) -> CrossChainResult<Vec<u8>>;
        async fn synchronize_state(&self, evm_block: u64, svm_slot: u64) -> CrossChainResult<()>;
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum RoutingDecision {
        ExecuteOnSvm,
        ExecuteOnEvm,
        Skip,
        Delegate(String),
    }

    impl std::fmt::Display for RoutingDecision {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                RoutingDecision::ExecuteOnSvm => write!(f, "SVM"),
                RoutingDecision::ExecuteOnEvm => write!(f, "EVM"),
                RoutingDecision::Skip => write!(f, "SKIP"),
                RoutingDecision::Delegate(target) => write!(f, "DELEGATE:{}", target),
            }
        }
    }
}

/// Utility functions and helpers
pub mod utils {
    use crate::errors::*;
    use sha2::{Sha256, Digest};

    /// Calculate hash of input data for logging and caching
    pub fn calculate_hash(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Check if data has SVM prefix
    pub fn has_svm_prefix(data: &[u8]) -> bool {
        data.starts_with(b"SVM")
    }

    /// Strip SVM prefix from data
    pub fn strip_svm_prefix(data: &[u8]) -> &[u8] {
        if has_svm_prefix(data) && data.len() > 3 {
            &data[3..]
        } else if has_svm_prefix(data) && data.len() == 3 {
            b""
        } else {
            data
        }
    }

    /// Validate Ethereum address format
    pub fn is_valid_eth_address(address: &[u8]) -> bool {
        address.len() == 20
    }

    /// Validate Solana pubkey format
    pub fn is_valid_solana_pubkey(pubkey: &[u8]) -> bool {
        pubkey.len() == 32
    }

    /// Convert bytes to human-readable size
    pub fn format_bytes(bytes: usize) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }

    /// Generate unique transaction ID
    pub fn generate_tx_id() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Get current timestamp in milliseconds
    pub fn current_timestamp_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Create error with context
    pub fn create_context_error(operation: &str, details: &str) -> SvmExExError {
        SvmExExError::Unknown(format!("{}: {}", operation, details))
    }
}

/// Configuration management
pub mod config {
    use crate::errors::*;
    use serde::{Deserialize, Serialize};
    use std::path::Path;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SvmExExConfig {
        pub svm_router_address: String,
        pub ai_agent: AIAgentConfig,
        pub performance: PerformanceConfig,
        pub logging: LoggingConfig,
        pub cross_chain: CrossChainConfig,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct AIAgentConfig {
        pub enabled: bool,
        pub decision_timeout_ms: u64,
        pub memory_cache_size: usize,
        pub confidence_threshold: f64,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PerformanceConfig {
        pub max_concurrent_transactions: usize,
        pub transaction_timeout_ms: u64,
        pub cache_size: usize,
        pub enable_metrics: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct LoggingConfig {
        pub level: String,
        pub format: String, // "json" or "text"
        pub enable_spans: bool,
        pub enable_ai_debug: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct CrossChainConfig {
        pub enable_state_sync: bool,
        pub sync_interval_ms: u64,
        pub address_mapping_cache_size: usize,
    }

    impl Default for SvmExExConfig {
        fn default() -> Self {
            Self {
                svm_router_address: "0x0000000000000000000000000000000000000000".to_string(),
                ai_agent: AIAgentConfig {
                    enabled: true,
                    decision_timeout_ms: 1000,
                    memory_cache_size: 10000,
                    confidence_threshold: 0.8,
                },
                performance: PerformanceConfig {
                    max_concurrent_transactions: 1000,
                    transaction_timeout_ms: 5000,
                    cache_size: 50000,
                    enable_metrics: true,
                },
                logging: LoggingConfig {
                    level: "info".to_string(),
                    format: "json".to_string(),
                    enable_spans: true,
                    enable_ai_debug: true,
                },
                cross_chain: CrossChainConfig {
                    enable_state_sync: true,
                    sync_interval_ms: 1000,
                    address_mapping_cache_size: 10000,
                },
            }
        }
    }

    impl SvmExExConfig {
        pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigurationError> {
            let content = std::fs::read_to_string(&path).map_err(|_| {
                ConfigurationError::FileNotFound {
                    path: path.as_ref().to_string_lossy().to_string(),
                }
            })?;

            let config: Self = toml::from_str(&content).map_err(|e| {
                ConfigurationError::ParsingFailed {
                    reason: e.to_string(),
                }
            })?;

            config.validate()?;
            Ok(config)
        }

        pub fn load_from_env() -> Result<Self, ConfigurationError> {
            let mut config = Self::default();

            if let Ok(addr) = std::env::var("SVM_ROUTER_ADDRESS") {
                config.svm_router_address = addr;
            }

            if let Ok(enabled) = std::env::var("AI_AGENT_ENABLED") {
                config.ai_agent.enabled = enabled.parse().unwrap_or(true);
            }

            if let Ok(level) = std::env::var("LOG_LEVEL") {
                config.logging.level = level;
            }

            config.validate()?;
            Ok(config)
        }

        fn validate(&self) -> Result<(), ConfigurationError> {
            // Validate SVM router address
            if self.svm_router_address.len() != 42 || !self.svm_router_address.starts_with("0x") {
                return Err(ConfigurationError::InvalidValue {
                    key: "svm_router_address".to_string(),
                    value: self.svm_router_address.clone(),
                    reason: "Invalid Ethereum address format".to_string(),
                });
            }

            // Validate confidence threshold
            if !(0.0..=1.0).contains(&self.ai_agent.confidence_threshold) {
                return Err(ConfigurationError::InvalidValue {
                    key: "ai_agent.confidence_threshold".to_string(),
                    value: self.ai_agent.confidence_threshold.to_string(),
                    reason: "Must be between 0.0 and 1.0".to_string(),
                });
            }

            Ok(())
        }
    }
}

// Feature-gated modules
#[cfg(feature = "ai-agents")]
pub mod ai {
    //! AI agent implementations and utilities
    
    pub use crate::traits::{AIAgent, RoutingDecision};
    
    /// Mock AI agent for testing and development
    pub struct MockAIAgent {
        confidence_threshold: f64,
        memory_cache: std::collections::HashMap<String, Vec<u8>>,
    }
    
    impl MockAIAgent {
        pub fn new(confidence_threshold: f64) -> Self {
            Self {
                confidence_threshold,
                memory_cache: std::collections::HashMap::new(),
            }
        }
    }
}

// Re-export key dependencies for convenience
pub use eyre;
pub use tokio;
pub use tracing;