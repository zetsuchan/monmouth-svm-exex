//! Pre-transaction hook interface that processes agent intents before execution

use super::agent_tx::{AgentTx, AgentTxDecoder};
use super::intent_classifier::{IntentClassifier, IntentClassification};
use crate::errors::*;
use crate::inter_exex::svm_messages::{SvmExExMessage, MemoryQuery, MemoryType};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Pre-execution hook for agent transactions
#[async_trait::async_trait]
pub trait PreExecutionHook: Send + Sync {
    /// Process transaction before execution
    async fn process(&self, ctx: PreExecutionContext) -> Result<PreExecutionResult>;
    
    /// Validate agent authorization
    async fn validate_authorization(&self, tx: &AgentTx) -> Result<bool>;
    
    /// Load required memory context
    async fn load_memory_context(&self, tx: &AgentTx) -> Result<MemoryContext>;
}

/// Pre-execution context
#[derive(Debug, Clone)]
pub struct PreExecutionContext {
    /// The agent transaction
    pub tx: AgentTx,
    
    /// Intent classification
    pub classification: IntentClassification,
    
    /// Current block number
    pub block_number: u64,
    
    /// Timestamp
    pub timestamp: u64,
    
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Pre-execution result
#[derive(Debug, Clone)]
pub struct PreExecutionResult {
    /// Whether to proceed with execution
    pub should_execute: bool,
    
    /// Modified transaction (if any)
    pub modified_tx: Option<AgentTx>,
    
    /// Execution plan
    pub execution_plan: ExecutionPlan,
    
    /// Warnings
    pub warnings: Vec<String>,
}

/// Execution plan for the transaction
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Ordered steps to execute
    pub steps: Vec<ExecutionStep>,
    
    /// Estimated total time
    pub estimated_time_ms: u64,
    
    /// Required confirmations
    pub required_confirmations: u32,
    
    /// Rollback strategy
    pub rollback_strategy: RollbackStrategy,
}

/// Single execution step
#[derive(Debug, Clone)]
pub struct ExecutionStep {
    /// Step ID
    pub id: String,
    
    /// Step type
    pub step_type: StepType,
    
    /// Target chain
    pub chain_id: Option<String>,
    
    /// Required tools
    pub tools: Vec<String>,
    
    /// Dependencies on other steps
    pub dependencies: Vec<String>,
    
    /// Can this step be retried
    pub retriable: bool,
}

/// Step type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepType {
    LoadMemory,
    ValidateConditions,
    ExecuteTool,
    CrossChainMessage,
    StateUpdate,
    Finalize,
}

/// Rollback strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackStrategy {
    /// No rollback needed
    None,
    
    /// Simple state reversion
    StateRevert,
    
    /// Complex multi-step rollback
    MultiStep,
    
    /// Manual intervention required
    Manual,
}

/// Memory context loaded for execution
#[derive(Debug, Clone)]
pub struct MemoryContext {
    /// Loaded memories
    pub memories: Vec<LoadedMemory>,
    
    /// Memory verification proof
    pub proof: Vec<u8>,
    
    /// Total memory size
    pub total_size: usize,
}

/// Loaded memory item
#[derive(Debug, Clone)]
pub struct LoadedMemory {
    /// Memory ID
    pub id: String,
    
    /// Memory content
    pub content: Vec<u8>,
    
    /// Memory type
    pub memory_type: MemoryType,
    
    /// Age in seconds
    pub age_seconds: u64,
}

/// Default implementation of pre-execution hook
pub struct DefaultPreExecutionHook {
    /// Intent classifier
    classifier: Arc<IntentClassifier>,
    
    /// Memory loader
    memory_loader: Arc<RwLock<MemoryLoader>>,
    
    /// Authorization checker
    auth_checker: Arc<AuthorizationChecker>,
}

/// Memory loader component
struct MemoryLoader {
    /// Cache of loaded memories
    cache: lru::LruCache<String, LoadedMemory>,
}

impl MemoryLoader {
    fn new(cache_size: usize) -> Self {
        Self {
            cache: lru::LruCache::new(std::num::NonZeroUsize::new(cache_size).unwrap()),
        }
    }
    
    async fn load_memories(&mut self, requirements: &[super::agent_tx::MemoryRequirement]) -> Result<Vec<LoadedMemory>> {
        let mut memories = Vec::new();
        
        for req in requirements {
            // Check cache first
            if let Some(cached) = self.cache.get(&req.query) {
                memories.push(cached.clone());
                continue;
            }
            
            // Load from memory store (simulated)
            let memory = LoadedMemory {
                id: uuid::Uuid::new_v4().to_string(),
                content: vec![0u8; 1024], // Placeholder
                memory_type: match req.memory_type {
                    super::agent_tx::MemoryType::ShortTerm => MemoryType::ShortTerm,
                    super::agent_tx::MemoryType::LongTerm => MemoryType::LongTerm,
                    super::agent_tx::MemoryType::Episodic => MemoryType::Episodic,
                    super::agent_tx::MemoryType::Semantic => MemoryType::Semantic,
                },
                age_seconds: 0,
            };
            
            // Cache the loaded memory
            self.cache.put(req.query.clone(), memory.clone());
            memories.push(memory);
        }
        
        Ok(memories)
    }
}

/// Authorization checker component
struct AuthorizationChecker {
    /// Authorized agents
    authorized_agents: Arc<RwLock<std::collections::HashSet<String>>>,
    
    /// Agent permissions
    agent_permissions: Arc<RwLock<std::collections::HashMap<String, Vec<Permission>>>>,
}

/// Agent permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Permission {
    Execute,
    CrossChain,
    HighValue,
    UseGPU,
    UseTEE,
}

impl AuthorizationChecker {
    fn new() -> Self {
        Self {
            authorized_agents: Arc::new(RwLock::new(std::collections::HashSet::new())),
            agent_permissions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
    
    async fn check_authorization(&self, tx: &AgentTx) -> Result<bool> {
        let authorized = self.authorized_agents.read().await;
        
        if !authorized.contains(&tx.agent.agent_id) {
            return Ok(false);
        }
        
        // Check specific permissions
        let permissions = self.agent_permissions.read().await;
        if let Some(agent_perms) = permissions.get(&tx.agent.agent_id) {
            // Check high value permission
            if tx.resources.total_gas_estimate > 1_000_000 && 
               !agent_perms.contains(&Permission::HighValue) {
                return Ok(false);
            }
            
            // Check cross-chain permission
            if tx.cross_chain_plans.len() > 1 && 
               !agent_perms.contains(&Permission::CrossChain) {
                return Ok(false);
            }
            
            // Check GPU permission
            if tx.resources.requires_gpu && 
               !agent_perms.contains(&Permission::UseGPU) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

impl DefaultPreExecutionHook {
    /// Create a new default pre-execution hook
    pub fn new() -> Self {
        Self {
            classifier: Arc::new(IntentClassifier::new()),
            memory_loader: Arc::new(RwLock::new(MemoryLoader::new(1000))),
            auth_checker: Arc::new(AuthorizationChecker::new()),
        }
    }
    
    /// Create execution plan from agent transaction
    fn create_execution_plan(&self, tx: &AgentTx, classification: &IntentClassification) -> ExecutionPlan {
        let mut steps = Vec::new();
        let mut step_counter = 0;
        
        // Step 1: Load memory if required
        if !tx.memory_context.required_memories.is_empty() {
            steps.push(ExecutionStep {
                id: format!("step-{}", step_counter),
                step_type: StepType::LoadMemory,
                chain_id: None,
                tools: vec!["memory-loader".to_string()],
                dependencies: vec![],
                retriable: true,
            });
            step_counter += 1;
        }
        
        // Step 2: Validate conditions
        steps.push(ExecutionStep {
            id: format!("step-{}", step_counter),
            step_type: StepType::ValidateConditions,
            chain_id: None,
            tools: vec!["condition-validator".to_string()],
            dependencies: if step_counter > 0 { 
                vec![format!("step-{}", step_counter - 1)] 
            } else { 
                vec![] 
            },
            retriable: false,
        });
        step_counter += 1;
        
        // Step 3: Execute tools for each cross-chain plan
        for (idx, plan) in tx.cross_chain_plans.iter().enumerate() {
            let step_id = format!("step-{}", step_counter);
            
            steps.push(ExecutionStep {
                id: step_id.clone(),
                step_type: StepType::ExecuteTool,
                chain_id: Some(plan.chain_id.clone()),
                tools: plan.tools.iter().map(|t| t.tool_id.clone()).collect(),
                dependencies: if step_counter > 0 { 
                    vec![format!("step-{}", step_counter - 1)] 
                } else { 
                    vec![] 
                },
                retriable: true,
            });
            step_counter += 1;
            
            // Add cross-chain message if not the last plan
            if idx < tx.cross_chain_plans.len() - 1 {
                steps.push(ExecutionStep {
                    id: format!("step-{}", step_counter),
                    step_type: StepType::CrossChainMessage,
                    chain_id: Some(plan.chain_id.clone()),
                    tools: vec!["bridge-messenger".to_string()],
                    dependencies: vec![step_id],
                    retriable: true,
                });
                step_counter += 1;
            }
        }
        
        // Step 4: State update
        steps.push(ExecutionStep {
            id: format!("step-{}", step_counter),
            step_type: StepType::StateUpdate,
            chain_id: None,
            tools: vec!["state-manager".to_string()],
            dependencies: if step_counter > 0 { 
                vec![format!("step-{}", step_counter - 1)] 
            } else { 
                vec![] 
            },
            retriable: false,
        });
        step_counter += 1;
        
        // Step 5: Finalize
        steps.push(ExecutionStep {
            id: format!("step-{}", step_counter),
            step_type: StepType::Finalize,
            chain_id: None,
            tools: vec!["finalizer".to_string()],
            dependencies: vec![format!("step-{}", step_counter - 1)],
            retriable: false,
        });
        
        // Determine rollback strategy
        let rollback_strategy = if tx.cross_chain_plans.len() > 1 {
            RollbackStrategy::MultiStep
        } else if tx.resources.total_gas_estimate > 1_000_000 {
            RollbackStrategy::StateRevert
        } else {
            RollbackStrategy::None
        };
        
        ExecutionPlan {
            steps,
            estimated_time_ms: classification.estimated_resources.execution_time_ms_estimate,
            required_confirmations: if tx.cross_chain_plans.len() > 1 { 2 } else { 1 },
            rollback_strategy,
        }
    }
}

#[async_trait::async_trait]
impl PreExecutionHook for DefaultPreExecutionHook {
    async fn process(&self, ctx: PreExecutionContext) -> Result<PreExecutionResult> {
        info!("Processing pre-execution hook for agent tx: {}", ctx.tx.agent.agent_id);
        
        let mut warnings = Vec::new();
        
        // Check authorization
        let authorized = self.validate_authorization(&ctx.tx).await?;
        if !authorized {
            return Ok(PreExecutionResult {
                should_execute: false,
                modified_tx: None,
                execution_plan: ExecutionPlan {
                    steps: vec![],
                    estimated_time_ms: 0,
                    required_confirmations: 0,
                    rollback_strategy: RollbackStrategy::None,
                },
                warnings: vec!["Agent not authorized".to_string()],
            });
        }
        
        // Check risk factors
        if !ctx.classification.risk_factors.is_empty() {
            warnings.extend(ctx.classification.risk_factors.clone());
            
            // High risk transactions need additional validation
            if ctx.classification.risk_factors.len() > 2 {
                warn!("High risk transaction detected: {:?}", ctx.classification.risk_factors);
            }
        }
        
        // Create execution plan
        let execution_plan = self.create_execution_plan(&ctx.tx, &ctx.classification);
        
        // Determine if we should execute
        let should_execute = ctx.classification.confidence > 0.5 && 
                           ctx.classification.estimated_resources.success_probability > 0.7;
        
        Ok(PreExecutionResult {
            should_execute,
            modified_tx: None, // No modifications in default implementation
            execution_plan,
            warnings,
        })
    }
    
    async fn validate_authorization(&self, tx: &AgentTx) -> Result<bool> {
        self.auth_checker.check_authorization(tx).await
    }
    
    async fn load_memory_context(&self, tx: &AgentTx) -> Result<MemoryContext> {
        let mut memory_loader = self.memory_loader.write().await;
        let memories = memory_loader.load_memories(&tx.memory_context.requirements).await?;
        
        let total_size: usize = memories.iter().map(|m| m.content.len()).sum();
        
        Ok(MemoryContext {
            memories,
            proof: vec![0u8; 32], // Placeholder proof
            total_size,
        })
    }
}

impl Default for DefaultPreExecutionHook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agent_tx::*;
    
    #[tokio::test]
    async fn test_pre_execution_hook() {
        let hook = DefaultPreExecutionHook::new();
        
        let tx = create_test_agent_tx();
        let classification = IntentClassification {
            category: super::super::intent_classifier::IntentCategory::DeFi,
            confidence: 0.8,
            suggested_tools: vec!["svm-executor".to_string()],
            risk_factors: vec![],
            estimated_resources: super::super::intent_classifier::EstimatedResources {
                gas_estimate: 100_000,
                compute_units_estimate: 50_000,
                execution_time_ms_estimate: 25,
                success_probability: 0.95,
            },
        };
        
        let ctx = PreExecutionContext {
            tx,
            classification,
            block_number: 1000,
            timestamp: 0,
            metadata: std::collections::HashMap::new(),
        };
        
        let result = hook.process(ctx).await.unwrap();
        
        // Should not execute because agent is not authorized
        assert!(!result.should_execute);
        assert!(!result.warnings.is_empty());
    }
    
    #[tokio::test]
    async fn test_execution_plan_creation() {
        let hook = DefaultPreExecutionHook::new();
        
        let mut tx = create_test_agent_tx();
        tx.cross_chain_plans = vec![
            CrossChainPlan {
                chain_id: "ethereum".to_string(),
                tools: vec![],
                target_contract: alloy_primitives::Address::ZERO,
                calldata: vec![],
                resources: ResourceAllocation {
                    gas_limit: 100_000,
                    compute_units: 50_000,
                    memory_mb: 256,
                    storage_kb: 0,
                },
            },
        ];
        
        let classification = IntentClassification {
            category: super::super::intent_classifier::IntentCategory::DeFi,
            confidence: 0.8,
            suggested_tools: vec![],
            risk_factors: vec![],
            estimated_resources: super::super::intent_classifier::EstimatedResources {
                gas_estimate: 100_000,
                compute_units_estimate: 50_000,
                execution_time_ms_estimate: 25,
                success_probability: 0.95,
            },
        };
        
        let plan = hook.create_execution_plan(&tx, &classification);
        
        assert!(!plan.steps.is_empty());
        assert_eq!(plan.rollback_strategy, RollbackStrategy::None);
    }
    
    fn create_test_agent_tx() -> AgentTx {
        AgentTx {
            agent: AgentIdentity {
                agent_id: "test-agent".to_string(),
                public_key: vec![0u8; 32],
                agent_type: AgentType::Executor,
                capabilities: vec!["execute".to_string()],
                attributes: HashMap::new(),
            },
            intent: Intent {
                intent_id: uuid::Uuid::new_v4().to_string(),
                intent_type: IntentType::Transfer,
                description: "Test transfer".to_string(),
                parameters: vec![],
                constraints: vec![],
                expiration_timestamp: 0,
            },
            memory_context: super::super::agent_tx::MemoryContext {
                required_memories: vec![],
                memory_root: alloy_primitives::B256::ZERO,
                requirements: vec![],
                max_memory_age_seconds: 3600,
            },
            cross_chain_plans: vec![],
            resources: ResourceRequirements {
                total_gas_estimate: 100_000,
                total_compute_units: 50_000,
                per_chain_allocation: HashMap::new(),
                requires_gpu: false,
                requires_tee: false,
            },
            metadata: AgentTxMetadata {
                nonce: 0,
                timestamp: 0,
                correlation_id: uuid::Uuid::new_v4().to_string(),
                tags: vec![],
                labels: HashMap::new(),
                priority: 1,
            },
            signature: vec![0u8; 64],
        }
    }
}