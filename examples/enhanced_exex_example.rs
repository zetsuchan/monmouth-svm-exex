//! Example demonstrating the enhanced SVM ExEx implementation
//! 
//! This example shows how to use the production-ready features including:
//! - FinishedHeight events for safe pruning
//! - Reorg-aware transaction processing
//! - Accounts Lattice Hash (ALH) for efficient state
//! - Enhanced AI decision engine with persistent learning
//! - Multi-tier caching strategy

use monmouth_svm_exex::{
    enhanced_exex::EnhancedSvmExEx,
    enhanced_processor::{
        EnhancedAIDecisionEngine, AccountCache, CacheConfig, EnhancedAccount,
    },
    logging,
};
use reth::cli::Cli;
use reth_node_ethereum::EthereumNode;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Example configuration for the enhanced ExEx
#[derive(Debug)]
struct ExExConfig {
    /// Enable AI-powered transaction routing
    ai_routing_enabled: bool,
    
    /// Cache configuration
    cache_config: CacheConfig,
    
    /// Maximum blocks to process before committing
    max_blocks_before_commit: u64,
    
    /// Enable performance monitoring
    monitoring_enabled: bool,
}

impl Default for ExExConfig {
    fn default() -> Self {
        Self {
            ai_routing_enabled: true,
            cache_config: CacheConfig {
                l1_size: 10_000,
                l2_size: 100_000,
                l3_enabled: true,
                ttl_seconds: 300,
                prefetch_enabled: true,
            },
            max_blocks_before_commit: 100,
            monitoring_enabled: true,
        }
    }
}

/// Example SVM state manager
struct SvmStateManager {
    account_cache: Arc<AccountCache>,
    ai_engine: Arc<EnhancedAIDecisionEngine>,
    state_version: Arc<RwLock<u64>>,
}

impl SvmStateManager {
    fn new(config: &ExExConfig) -> Self {
        Self {
            account_cache: Arc::new(AccountCache::new(config.cache_config.clone())),
            ai_engine: Arc::new(EnhancedAIDecisionEngine::new()),
            state_version: Arc::new(RwLock::new(0)),
        }
    }

    /// Process a transaction with AI routing and caching
    async fn process_transaction(&self, data: &[u8]) -> eyre::Result<()> {
        // Step 1: AI analysis for routing decision
        let analysis = self.ai_engine.analyze_transaction(data).await?;
        
        info!(
            "AI Analysis - Decision: {:?}, Confidence: {:.2}, Complexity: {:.2}",
            analysis.routing_decision, analysis.confidence, analysis.complexity_score
        );

        // Step 2: Only process if AI recommends SVM execution
        match analysis.routing_decision {
            monmouth_svm_exex::traits::RoutingDecision::ExecuteOnSvm => {
                info!("Executing transaction on SVM");
                self.execute_on_svm(data).await?;
            }
            monmouth_svm_exex::traits::RoutingDecision::ExecuteOnEvm => {
                info!("Routing to EVM execution");
                // Would forward to EVM here
            }
            monmouth_svm_exex::traits::RoutingDecision::Skip => {
                warn!("Skipping transaction based on AI analysis");
            }
            monmouth_svm_exex::traits::RoutingDecision::Delegate(target) => {
                info!("Delegating to: {}", target);
            }
        }

        Ok(())
    }

    /// Execute transaction on SVM with caching
    async fn execute_on_svm(&self, data: &[u8]) -> eyre::Result<()> {
        // Parse transaction to get account references
        let account_refs = self.parse_account_refs(data)?;
        
        // Load accounts from cache
        let mut accounts = Vec::new();
        for pubkey in &account_refs {
            if let Some(account) = self.account_cache.get_account(pubkey).await {
                accounts.push(account);
            } else {
                // Create new account if not found
                let new_account = self.create_default_account(*pubkey);
                self.account_cache.insert_account(new_account.clone()).await;
                accounts.push(new_account);
            }
        }

        // Execute transaction (simplified)
        info!("Executing SVM transaction with {} accounts", accounts.len());
        
        // Update state version
        let mut version = self.state_version.write().await;
        *version += 1;
        
        // In real implementation, would:
        // 1. Execute instructions
        // 2. Update account states
        // 3. Update ALH
        // 4. Store results for AI learning
        
        Ok(())
    }

    fn parse_account_refs(&self, data: &[u8]) -> eyre::Result<Vec<[u8; 32]>> {
        // Simplified parsing - in reality would parse transaction format
        let mut refs = Vec::new();
        
        if data.len() >= 32 {
            let mut pubkey = [0u8; 32];
            pubkey.copy_from_slice(&data[0..32]);
            refs.push(pubkey);
        }
        
        Ok(refs)
    }

    fn create_default_account(&self, pubkey: [u8; 32]) -> EnhancedAccount {
        let mut account = EnhancedAccount {
            pubkey,
            lamports: 0,
            data: vec![],
            owner: [0u8; 32],
            executable: false,
            rent_epoch: 0,
            data_hash: [0u8; 32],
            last_modified_slot: 0,
            access_count: 0,
            last_accessed: std::time::SystemTime::now(),
        };
        account.compute_data_hash();
        account
    }
}

/// Example monitoring and metrics
async fn setup_monitoring() -> eyre::Result<()> {
    // In production, would set up:
    // - OpenTelemetry exporter
    // - Prometheus metrics endpoint
    // - Custom dashboards
    
    info!("Monitoring setup complete");
    Ok(())
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    // Initialize enhanced logging
    logging::init_ai_agent_logging()?;
    
    info!("Starting Enhanced SVM ExEx Example");

    // Load configuration
    let config = ExExConfig::default();
    info!("Configuration: {:?}", config);

    // Setup monitoring if enabled
    if config.monitoring_enabled {
        setup_monitoring().await?;
    }

    // Create state manager
    let state_manager = Arc::new(SvmStateManager::new(&config));

    // Run the node with enhanced ExEx
    Cli::parse_args().run(|builder, _| async move {
        let handle = builder
            .node(EthereumNode::default())
            .install_exex("enhanced-svm-exex", |ctx| {
                Ok(EnhancedSvmExEx::new(ctx))
            })
            .launch()
            .await?;

        info!("Enhanced SVM ExEx launched successfully");
        
        // Demonstrate processing a sample transaction
        tokio::spawn({
            let state_manager = state_manager.clone();
            async move {
                // Wait for node to initialize
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                
                // Example transaction data
                let sample_tx = b"SVM\x00\x00\x00\x01"; // SVM prefix + simple data
                
                if let Err(e) = state_manager.process_transaction(sample_tx).await {
                    warn!("Failed to process sample transaction: {}", e);
                }
            }
        });

        handle.wait_for_node_exit().await
    })
}