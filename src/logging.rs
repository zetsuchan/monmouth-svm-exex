//! Logging configuration and utilities for Monmouth SVM ExEx
//! 
//! Provides structured logging for:
//! - Reth ExEx operations
//! - SVM transaction processing
//! - AI agent decision making
//! - Cross-chain state management
//! - Performance metrics

use tracing::{Level, Span};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter, Layer,
};
use std::io;

/// Initialize the logging system for the SVM ExEx
pub fn init_logging() -> eyre::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,monmouth_svm_exex=debug,reth=info,solana=info"));

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .with_writer(io::stderr);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!("Monmouth SVM ExEx logging initialized");
    Ok(())
}

/// Initialize logging with custom configuration for AI agents
pub fn init_ai_agent_logging() -> eyre::Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("info,monmouth_svm_exex=debug,ai_agent=debug,reth=info,solana=info")
        });

    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
        .json()
        .with_writer(io::stderr);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!("AI Agent logging initialized with JSON format");
    Ok(())
}

/// Create a tracing span for SVM transaction processing
pub fn svm_transaction_span(tx_hash: &str, input_size: usize) -> Span {
    tracing::info_span!(
        "svm_transaction",
        tx_hash = %tx_hash,
        input_size = input_size,
        status = tracing::field::Empty,
        execution_time_ms = tracing::field::Empty,
        ai_decision = tracing::field::Empty,
    )
}

/// Create a tracing span for AI agent operations
pub fn ai_agent_span(operation: &str, context_size: usize) -> Span {
    tracing::info_span!(
        "ai_agent_operation",
        operation = operation,
        context_size = context_size,
        decision = tracing::field::Empty,
        confidence = tracing::field::Empty,
        execution_time_ms = tracing::field::Empty,
    )
}

/// Create a tracing span for cross-chain operations
pub fn cross_chain_span(operation: &str, source_chain: &str, target_chain: &str) -> Span {
    tracing::info_span!(
        "cross_chain_operation",
        operation = operation,
        source_chain = source_chain,
        target_chain = target_chain,
        success = tracing::field::Empty,
        error = tracing::field::Empty,
    )
}

/// Create a tracing span for ExEx lifecycle events
pub fn exex_lifecycle_span(event: &str) -> Span {
    tracing::info_span!(
        "exex_lifecycle",
        event = event,
        timestamp = %chrono::Utc::now(),
        build_info = env!("BUILD_TIMESTAMP"),
    )
}

/// Structured logging for performance metrics
pub struct PerformanceLogger {
    start_time: std::time::Instant,
    operation: String,
}

impl PerformanceLogger {
    pub fn start(operation: &str) -> Self {
        tracing::debug!("Starting performance measurement for: {}", operation);
        Self {
            start_time: std::time::Instant::now(),
            operation: operation.to_string(),
        }
    }

    pub fn log_milestone(&self, milestone: &str) {
        let elapsed = self.start_time.elapsed();
        tracing::debug!(
            operation = %self.operation,
            milestone = milestone,
            elapsed_ms = elapsed.as_millis(),
            "Performance milestone reached"
        );
    }

    pub fn finish(self) {
        let total_time = self.start_time.elapsed();
        tracing::info!(
            operation = %self.operation,
            total_time_ms = total_time.as_millis(),
            "Performance measurement completed"
        );
    }
}

/// Log AI agent decision with context
pub fn log_ai_decision(
    decision: &str,
    confidence: f64,
    context_hash: &str,
    reasoning: Option<&str>,
) {
    tracing::info!(
        decision = decision,
        confidence = confidence,
        context_hash = context_hash,
        reasoning = reasoning,
        "AI agent decision made"
    );
}

/// Log transaction routing decision
pub fn log_transaction_routing(
    tx_hash: &str,
    route: &str,
    reason: &str,
    ai_assisted: bool,
) {
    tracing::info!(
        tx_hash = tx_hash,
        route = route,
        reason = reason,
        ai_assisted = ai_assisted,
        "Transaction routing decision"
    );
}

/// Log cross-chain state synchronization
pub fn log_state_sync(
    evm_block: u64,
    svm_slot: u64,
    sync_type: &str,
    success: bool,
    error: Option<&str>,
) {
    if success {
        tracing::info!(
            evm_block = evm_block,
            svm_slot = svm_slot,
            sync_type = sync_type,
            "Cross-chain state synchronization successful"
        );
    } else {
        tracing::error!(
            evm_block = evm_block,
            svm_slot = svm_slot,
            sync_type = sync_type,
            error = error,
            "Cross-chain state synchronization failed"
        );
    }
}

/// Log memory operations for AI agents
pub fn log_memory_operation(
    operation: &str,
    key: &str,
    data_size: Option<usize>,
    success: bool,
    latency_ms: u64,
) {
    tracing::debug!(
        operation = operation,
        key = key,
        data_size = data_size,
        success = success,
        latency_ms = latency_ms,
        "AI agent memory operation"
    );
}

/// Custom macro for ExEx-specific error logging
#[macro_export]
macro_rules! exex_error {
    ($($arg:tt)*) => {
        tracing::error!(
            target: "monmouth_svm_exex",
            component = "exex",
            $($arg)*
        )
    };
}

/// Custom macro for SVM-specific info logging
#[macro_export]
macro_rules! svm_info {
    ($($arg:tt)*) => {
        tracing::info!(
            target: "monmouth_svm_exex",
            component = "svm",
            $($arg)*
        )
    };
}

/// Custom macro for AI agent logging
#[macro_export]
macro_rules! ai_debug {
    ($($arg:tt)*) => {
        tracing::debug!(
            target: "monmouth_svm_exex",
            component = "ai_agent",
            $($arg)*
        )
    };
}