//! Error types and handling for Monmouth SVM ExEx
//! 
//! Comprehensive error handling for:
//! - Reth ExEx operations
//! - SVM transaction processing
//! - AI agent operations
//! - Cross-chain state management
//! - Memory and storage operations

use thiserror::Error;
use std::fmt;

/// Main error type for the Monmouth SVM ExEx
#[derive(Error, Debug)]
pub enum SvmExExError {
    #[error("Reth ExEx error: {0}")]
    RethExEx(#[from] RethExExError),

    #[error("SVM processing error: {0}")]
    SvmProcessing(#[from] SvmProcessingError),

    #[error("AI agent error: {0}")]
    AIAgent(#[from] AIAgentError),

    #[error("Cross-chain error: {0}")]
    CrossChain(#[from] CrossChainError),

    #[error("Configuration error: {0}")]
    Configuration(#[from] ConfigurationError),

    #[error("Memory operation error: {0}")]
    Memory(#[from] MemoryError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

/// Reth ExEx specific errors
#[derive(Error, Debug)]
pub enum RethExExError {
    #[error("ExEx initialization failed: {reason}")]
    InitializationFailed { reason: String },

    #[error("ExEx notification processing failed: {reason}")]
    NotificationProcessingFailed { reason: String },

    #[error("Block processing error: block_number={block_number}, error={error}")]
    BlockProcessingError { block_number: u64, error: String },

    #[error("Transaction processing failed: tx_hash={tx_hash}, error={error}")]
    TransactionProcessingFailed { tx_hash: String, error: String },

    #[error("ExEx context error: {0}")]
    ContextError(String),

    #[error("Future polling error: {0}")]
    FuturePollingError(String),
}

/// SVM processing specific errors
#[derive(Error, Debug)]
pub enum SvmProcessingError {
    #[error("Transaction sanitization failed: {reason}")]
    SanitizationFailed { reason: String },

    #[error("Account loading failed: account={account}, reason={reason}")]
    AccountLoadingFailed { account: String, reason: String },

    #[error("Instruction processing failed: instruction_index={index}, error={error}")]
    InstructionProcessingFailed { index: usize, error: String },

    #[error("eBPF VM execution failed: {reason}")]
    EbpfVmFailed { reason: String },

    #[error("Bank operation failed: {operation}, reason={reason}")]
    BankOperationFailed { operation: String, reason: String },

    #[error("Program cache error: program_id={program_id}, error={error}")]
    ProgramCacheError { program_id: String, error: String },

    #[error("Accounts database error: {0}")]
    AccountsDbError(String),

    #[error("Compute units exceeded: used={used}, limit={limit}")]
    ComputeUnitsExceeded { used: u64, limit: u64 },

    #[error("Invalid transaction format: {reason}")]
    InvalidTransactionFormat { reason: String },
}

/// AI agent specific errors
#[derive(Error, Debug)]
pub enum AIAgentError {
    #[error("AI decision making failed: context_size={context_size}, error={error}")]
    DecisionMakingFailed { context_size: usize, error: String },

    #[error("Memory storage failed: key={key}, error={error}")]
    MemoryStorageFailed { key: String, error: String },

    #[error("Memory retrieval failed: key={key}, error={error}")]
    MemoryRetrievalFailed { key: String, error: String },

    #[error("Context processing failed: {reason}")]
    ContextProcessingFailed { reason: String },

    #[error("Model inference failed: model={model}, error={error}")]
    ModelInferenceFailed { model: String, error: String },

    #[error("Vector operation failed: operation={operation}, error={error}")]
    VectorOperationFailed { operation: String, error: String },

    #[error("Agent configuration invalid: {reason}")]
    InvalidConfiguration { reason: String },

    #[error("Planning failed: {reason}")]
    PlanningFailed { reason: String },

    #[error("Execution timeout: operation={operation}, timeout_ms={timeout_ms}")]
    ExecutionTimeout { operation: String, timeout_ms: u64 },
}

/// Cross-chain operation errors
#[derive(Error, Debug)]
pub enum CrossChainError {
    #[error("Address mapping failed: evm_address={evm_address}, reason={reason}")]
    AddressMappingFailed { evm_address: String, reason: String },

    #[error("Call translation failed: from={from_chain}, to={to_chain}, error={error}")]
    CallTranslationFailed { from_chain: String, to_chain: String, error: String },

    #[error("State bridge operation failed: operation={operation}, error={error}")]
    StateBridgeOperationFailed { operation: String, error: String },

    #[error("State synchronization failed: evm_block={evm_block}, svm_slot={svm_slot}, error={error}")]
    StateSynchronizationFailed { evm_block: u64, svm_slot: u64, error: String },

    #[error("Chain compatibility error: {reason}")]
    ChainCompatibilityError { reason: String },

    #[error("Bridge configuration invalid: {reason}")]
    BridgeConfigurationInvalid { reason: String },
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    #[error("Invalid configuration value: key={key}, value={value}, reason={reason}")]
    InvalidValue { key: String, value: String, reason: String },

    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    #[error("Configuration parsing failed: {reason}")]
    ParsingFailed { reason: String },

    #[error("Environment variable error: {var}, error={error}")]
    EnvironmentVariable { var: String, error: String },
}

/// Memory operation errors
#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Allocation failed: size={size}, reason={reason}")]
    AllocationFailed { size: usize, reason: String },

    #[error("Memory access violation: address={address}")]
    AccessViolation { address: String },

    #[error("Memory corruption detected: region={region}")]
    CorruptionDetected { region: String },

    #[error("Out of memory: requested={requested}, available={available}")]
    OutOfMemory { requested: usize, available: usize },

    #[error("Memory leak detected: component={component}, leaked_bytes={leaked_bytes}")]
    LeakDetected { component: String, leaked_bytes: usize },
}

/// Serialization errors
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("JSON serialization failed: {reason}")]
    JsonFailed { reason: String },

    #[error("Binary serialization failed: format={format}, reason={reason}")]
    BinaryFailed { format: String, reason: String },

    #[error("Deserialization failed: expected_type={expected_type}, reason={reason}")]
    DeserializationFailed { expected_type: String, reason: String },

    #[error("Schema validation failed: {reason}")]
    SchemaValidationFailed { reason: String },

    #[error("Version mismatch: expected={expected}, found={found}")]
    VersionMismatch { expected: String, found: String },
}

/// Network operation errors
#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection failed: endpoint={endpoint}, reason={reason}")]
    ConnectionFailed { endpoint: String, reason: String },

    #[error("Request timeout: endpoint={endpoint}, timeout_ms={timeout_ms}")]
    RequestTimeout { endpoint: String, timeout_ms: u64 },

    #[error("HTTP error: status={status}, endpoint={endpoint}")]
    HttpError { status: u16, endpoint: String },

    #[error("DNS resolution failed: hostname={hostname}")]
    DnsResolutionFailed { hostname: String },

    #[error("Network unreachable: {reason}")]
    NetworkUnreachable { reason: String },
}

/// Result type aliases for convenience
pub type SvmExExResult<T> = Result<T, SvmExExError>;
pub type RethResult<T> = Result<T, RethExExError>;
pub type SvmResult<T> = Result<T, SvmProcessingError>;
pub type AIResult<T> = Result<T, AIAgentError>;
pub type CrossChainResult<T> = Result<T, CrossChainError>;

/// Error context trait for adding context to errors
pub trait ErrorContext<T> {
    fn with_context<F>(self, f: F) -> SvmExExResult<T>
    where
        F: FnOnce() -> String;

    fn with_svm_context(self, operation: &str) -> SvmResult<T>;
    fn with_ai_context(self, operation: &str) -> AIResult<T>;
    fn with_cross_chain_context(self, operation: &str) -> CrossChainResult<T>;
}

impl<T, E> ErrorContext<T> for Result<T, E>
where
    E: Into<SvmExExError>,
{
    fn with_context<F>(self, f: F) -> SvmExExResult<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let base_error: SvmExExError = e.into();
            SvmExExError::Unknown(format!("{}: {}", f(), base_error))
        })
    }

    fn with_svm_context(self, operation: &str) -> SvmResult<T> {
        self.map_err(|e| {
            SvmProcessingError::InstructionProcessingFailed {
                index: 0,
                error: format!("{}: {}", operation, format!("{:?}", e)),
            }
        })
    }

    fn with_ai_context(self, operation: &str) -> AIResult<T> {
        self.map_err(|e| {
            AIAgentError::ContextProcessingFailed {
                reason: format!("{}: {}", operation, format!("{:?}", e)),
            }
        })
    }

    fn with_cross_chain_context(self, operation: &str) -> CrossChainResult<T> {
        self.map_err(|e| {
            CrossChainError::StateBridgeOperationFailed {
                operation: operation.to_string(),
                error: format!("{:?}", e),
            }
        })
    }
}

/// Utility functions for error handling
pub mod utils {
    use super::*;
    use crate::logging::*;

    /// Log and return an error
    pub fn log_and_return_error<T>(error: SvmExExError) -> SvmExExResult<T> {
        exex_error!(error = %error, "SVM ExEx error occurred");
        Err(error)
    }

    /// Convert a standard error to SvmExExError with context
    pub fn convert_error<E: std::error::Error + Send + Sync + 'static>(
        error: E,
        context: &str,
    ) -> SvmExExError {
        SvmExExError::Unknown(format!("{}: {}", context, error))
    }

    /// Check if error is recoverable
    pub fn is_recoverable_error(error: &SvmExExError) -> bool {
        match error {
            SvmExExError::Network(_) => true,
            SvmExExError::Memory(MemoryError::OutOfMemory { .. }) => false,
            SvmExExError::Configuration(_) => false,
            SvmExExError::SvmProcessing(SvmProcessingError::ComputeUnitsExceeded { .. }) => true,
            _ => false,
        }
    }

    /// Get error severity level
    pub fn get_error_severity(error: &SvmExExError) -> ErrorSeverity {
        match error {
            SvmExExError::Memory(MemoryError::CorruptionDetected { .. }) => ErrorSeverity::Critical,
            SvmExExError::Configuration(_) => ErrorSeverity::High,
            SvmExExError::CrossChain(_) => ErrorSeverity::Medium,
            SvmExExError::Network(_) => ErrorSeverity::Low,
            _ => ErrorSeverity::Medium,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorSeverity::Low => write!(f, "LOW"),
            ErrorSeverity::Medium => write!(f, "MEDIUM"),
            ErrorSeverity::High => write!(f, "HIGH"),
            ErrorSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}