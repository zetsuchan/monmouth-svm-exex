//! Framework for multi-step tool operations

use crate::errors::*;
use crate::agent::agent_tx::{ToolExecution as AgentToolExecution, CrossChainPlan};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn, instrument};

/// Tool executor for managing multi-step operations
pub struct ToolExecutor {
    /// Registered tools
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
    
    /// Execution state
    state: Arc<RwLock<ExecutionState>>,
    
    /// Configuration
    config: ToolExecutorConfig,
}

/// Tool trait that all tools must implement
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Execute the tool
    async fn execute(&self, inputs: HashMap<String, Vec<u8>>) -> Result<ToolResult>;
    
    /// Validate inputs before execution
    async fn validate_inputs(&self, inputs: &HashMap<String, Vec<u8>>) -> Result<()>;
    
    /// Get tool metadata
    fn metadata(&self) -> ToolMetadata;
    
    /// Can this tool be retried on failure
    fn is_retriable(&self) -> bool {
        true
    }
}

/// Tool metadata
#[derive(Debug, Clone)]
pub struct ToolMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
    pub required_inputs: Vec<String>,
    pub optional_inputs: Vec<String>,
    pub output_format: OutputFormat,
}

/// Output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Binary,
    Json,
    Protobuf,
    Custom,
}

/// Tool execution result
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// Success status
    pub success: bool,
    
    /// Output data
    pub outputs: HashMap<String, Vec<u8>>,
    
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    
    /// Gas used (if applicable)
    pub gas_used: Option<u64>,
    
    /// Error message (if failed)
    pub error: Option<String>,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Tool execution
#[derive(Debug, Clone)]
pub struct ToolExecution {
    /// Execution ID
    pub id: String,
    
    /// Tool ID
    pub tool_id: String,
    
    /// Inputs
    pub inputs: HashMap<String, Vec<u8>>,
    
    /// Dependencies on other executions
    pub dependencies: Vec<String>,
    
    /// Can fail without stopping the sequence
    pub can_fail: bool,
    
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    
    /// Retry configuration
    pub retry_config: RetryConfig,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_ms: 1000,
            exponential_backoff: true,
        }
    }
}

/// Execution state
#[derive(Debug)]
struct ExecutionState {
    /// Current executions
    executions: HashMap<String, ExecutionRecord>,
    
    /// Completed results
    results: HashMap<String, ToolResult>,
    
    /// Failed executions
    failures: HashMap<String, FailureRecord>,
}

/// Execution record
#[derive(Debug, Clone)]
struct ExecutionRecord {
    execution: ToolExecution,
    status: ExecutionStatus,
    start_time: std::time::Instant,
    attempts: u32,
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retrying,
}

/// Failure record
#[derive(Debug, Clone)]
struct FailureRecord {
    execution_id: String,
    error: String,
    timestamp: std::time::Instant,
    attempts: u32,
}

/// Tool executor configuration
#[derive(Debug, Clone)]
pub struct ToolExecutorConfig {
    /// Maximum concurrent executions
    pub max_concurrent: usize,
    
    /// Global timeout for all executions
    pub global_timeout_ms: u64,
    
    /// Enable detailed logging
    pub enable_detailed_logging: bool,
    
    /// Fail fast on first error
    pub fail_fast: bool,
}

impl Default for ToolExecutorConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 10,
            global_timeout_ms: 300_000, // 5 minutes
            enable_detailed_logging: true,
            fail_fast: false,
        }
    }
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new(config: ToolExecutorConfig) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            state: Arc::new(RwLock::new(ExecutionState {
                executions: HashMap::new(),
                results: HashMap::new(),
                failures: HashMap::new(),
            })),
            config,
        }
    }
    
    /// Register a tool
    pub async fn register_tool(&self, tool: Box<dyn Tool>) -> Result<()> {
        let metadata = tool.metadata();
        let mut tools = self.tools.write().await;
        
        if tools.contains_key(&metadata.name) {
            return Err(SvmExExError::ProcessingError(
                format!("Tool {} already registered", metadata.name)
            ));
        }
        
        info!("Registered tool: {} v{}", metadata.name, metadata.version);
        tools.insert(metadata.name.clone(), tool);
        Ok(())
    }
    
    /// Execute a sequence of tools
    #[instrument(skip(self, executions))]
    pub async fn execute_sequence(&self, executions: Vec<ToolExecution>) -> Result<Vec<ToolResult>> {
        info!("Executing tool sequence with {} steps", executions.len());
        
        // Validate all executions first
        self.validate_executions(&executions).await?;
        
        // Initialize execution state
        {
            let mut state = self.state.write().await;
            state.executions.clear();
            state.results.clear();
            state.failures.clear();
            
            for execution in &executions {
                state.executions.insert(execution.id.clone(), ExecutionRecord {
                    execution: execution.clone(),
                    status: ExecutionStatus::Pending,
                    start_time: std::time::Instant::now(),
                    attempts: 0,
                });
            }
        }
        
        // Execute with global timeout
        let results = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.global_timeout_ms),
            self.execute_all()
        )
        .await
        .map_err(|_| SvmExExError::ProcessingError("Global timeout exceeded".to_string()))??;
        
        Ok(results)
    }
    
    /// Validate executions
    async fn validate_executions(&self, executions: &[ToolExecution]) -> Result<()> {
        let tools = self.tools.read().await;
        
        for execution in executions {
            // Check tool exists
            if !tools.contains_key(&execution.tool_id) {
                return Err(SvmExExError::ProcessingError(
                    format!("Tool {} not found", execution.tool_id)
                ));
            }
            
            // Validate dependencies
            for dep in &execution.dependencies {
                if !executions.iter().any(|e| e.id == *dep) {
                    return Err(SvmExExError::ProcessingError(
                        format!("Dependency {} not found", dep)
                    ));
                }
            }
        }
        
        Ok(())
    }
    
    /// Execute all tools
    async fn execute_all(&self) -> Result<Vec<ToolResult>> {
        let mut results = Vec::new();
        let mut completed = std::collections::HashSet::new();
        
        loop {
            // Find ready executions
            let ready = self.find_ready_executions(&completed).await?;
            
            if ready.is_empty() {
                // Check if all are completed
                let state = self.state.read().await;
                if state.executions.values().all(|r| 
                    r.status == ExecutionStatus::Completed || 
                    (r.status == ExecutionStatus::Failed && r.execution.can_fail)
                ) {
                    break;
                }
                
                // Check for failures
                if self.config.fail_fast && !state.failures.is_empty() {
                    return Err(SvmExExError::ProcessingError(
                        "Execution failed in fail-fast mode".to_string()
                    ));
                }
                
                // Wait a bit before checking again
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                continue;
            }
            
            // Execute ready tools in parallel
            let mut handles = Vec::new();
            
            for execution_id in ready {
                let executor = self.clone();
                let handle = tokio::spawn(async move {
                    executor.execute_single(&execution_id).await
                });
                handles.push((execution_id, handle));
            }
            
            // Wait for executions to complete
            for (execution_id, handle) in handles {
                match handle.await {
                    Ok(Ok(result)) => {
                        completed.insert(execution_id.clone());
                        results.push(result);
                    }
                    Ok(Err(e)) => {
                        error!("Execution {} failed: {}", execution_id, e);
                        if self.config.fail_fast {
                            return Err(e);
                        }
                    }
                    Err(e) => {
                        error!("Execution {} panicked: {}", execution_id, e);
                        if self.config.fail_fast {
                            return Err(SvmExExError::ProcessingError(
                                format!("Execution {} panicked", execution_id)
                            ));
                        }
                    }
                }
            }
        }
        
        Ok(results)
    }
    
    /// Find executions ready to run
    async fn find_ready_executions(
        &self,
        completed: &std::collections::HashSet<String>
    ) -> Result<Vec<String>> {
        let state = self.state.read().await;
        let mut ready = Vec::new();
        
        for (id, record) in &state.executions {
            if record.status != ExecutionStatus::Pending {
                continue;
            }
            
            // Check if all dependencies are completed
            let deps_satisfied = record.execution.dependencies.iter()
                .all(|dep| completed.contains(dep));
            
            if deps_satisfied {
                ready.push(id.clone());
            }
        }
        
        // Limit by max concurrent
        ready.truncate(self.config.max_concurrent);
        
        Ok(ready)
    }
    
    /// Execute a single tool
    async fn execute_single(&self, execution_id: &str) -> Result<ToolResult> {
        // Update status to running
        {
            let mut state = self.state.write().await;
            if let Some(record) = state.executions.get_mut(execution_id) {
                record.status = ExecutionStatus::Running;
                record.attempts += 1;
            }
        }
        
        // Get execution details
        let execution = {
            let state = self.state.read().await;
            state.executions.get(execution_id)
                .ok_or_else(|| SvmExExError::ProcessingError(
                    format!("Execution {} not found", execution_id)
                ))?
                .execution.clone()
        };
        
        // Get tool
        let tools = self.tools.read().await;
        let tool = tools.get(&execution.tool_id)
            .ok_or_else(|| SvmExExError::ProcessingError(
                format!("Tool {} not found", execution.tool_id)
            ))?;
        
        // Execute with timeout
        let start_time = std::time::Instant::now();
        
        let result = match tokio::time::timeout(
            std::time::Duration::from_millis(execution.timeout_ms),
            tool.execute(execution.inputs.clone())
        ).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => {
                return self.handle_execution_error(execution_id, e).await;
            }
            Err(_) => {
                return self.handle_execution_error(
                    execution_id,
                    SvmExExError::ProcessingError("Execution timeout".to_string())
                ).await;
            }
        };
        
        // Update state
        {
            let mut state = self.state.write().await;
            if let Some(record) = state.executions.get_mut(execution_id) {
                record.status = ExecutionStatus::Completed;
            }
            state.results.insert(execution_id.to_string(), result.clone());
        }
        
        info!("Execution {} completed in {}ms", 
            execution_id, 
            start_time.elapsed().as_millis()
        );
        
        Ok(result)
    }
    
    /// Handle execution error
    async fn handle_execution_error(
        &self,
        execution_id: &str,
        error: SvmExExError
    ) -> Result<ToolResult> {
        let mut state = self.state.write().await;
        
        if let Some(record) = state.executions.get_mut(execution_id) {
            // Check if we should retry
            if record.attempts < record.execution.retry_config.max_attempts {
                record.status = ExecutionStatus::Retrying;
                warn!("Execution {} failed (attempt {}), retrying", 
                    execution_id, record.attempts);
                
                // Calculate backoff
                let backoff = if record.execution.retry_config.exponential_backoff {
                    record.execution.retry_config.backoff_ms * 
                        2u64.pow(record.attempts - 1)
                } else {
                    record.execution.retry_config.backoff_ms
                };
                
                drop(state);
                tokio::time::sleep(std::time::Duration::from_millis(backoff)).await;
                
                // Retry execution
                return self.execute_single(execution_id).await;
            }
            
            // Mark as failed
            record.status = ExecutionStatus::Failed;
            
            // Record failure
            state.failures.insert(execution_id.to_string(), FailureRecord {
                execution_id: execution_id.to_string(),
                error: error.to_string(),
                timestamp: std::time::Instant::now(),
                attempts: record.attempts,
            });
            
            // Check if can fail
            if record.execution.can_fail {
                warn!("Execution {} failed but can_fail is true", execution_id);
                return Ok(ToolResult {
                    success: false,
                    outputs: HashMap::new(),
                    execution_time_ms: 0,
                    gas_used: None,
                    error: Some(error.to_string()),
                    metadata: HashMap::new(),
                });
            }
        }
        
        Err(error)
    }
    
    /// Get execution status
    pub async fn get_status(&self) -> ExecutionStatusReport {
        let state = self.state.read().await;
        
        let total = state.executions.len();
        let completed = state.executions.values()
            .filter(|r| r.status == ExecutionStatus::Completed)
            .count();
        let failed = state.executions.values()
            .filter(|r| r.status == ExecutionStatus::Failed)
            .count();
        let running = state.executions.values()
            .filter(|r| r.status == ExecutionStatus::Running)
            .count();
        
        ExecutionStatusReport {
            total,
            completed,
            failed,
            running,
            pending: total - completed - failed - running,
        }
    }
}

impl Clone for ToolExecutor {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            state: self.state.clone(),
            config: self.config.clone(),
        }
    }
}

/// Execution status report
#[derive(Debug, Clone)]
pub struct ExecutionStatusReport {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub running: usize,
    pub pending: usize,
}

/// Example tool implementation
pub struct ExampleTool {
    name: String,
}

#[async_trait::async_trait]
impl Tool for ExampleTool {
    async fn execute(&self, inputs: HashMap<String, Vec<u8>>) -> Result<ToolResult> {
        // Simulate some work
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        
        let mut outputs = HashMap::new();
        outputs.insert("result".to_string(), b"success".to_vec());
        
        Ok(ToolResult {
            success: true,
            outputs,
            execution_time_ms: 100,
            gas_used: Some(50000),
            error: None,
            metadata: HashMap::new(),
        })
    }
    
    async fn validate_inputs(&self, inputs: &HashMap<String, Vec<u8>>) -> Result<()> {
        if inputs.is_empty() {
            return Err(SvmExExError::ValidationError("No inputs provided".to_string()));
        }
        Ok(())
    }
    
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            name: self.name.clone(),
            version: "1.0.0".to_string(),
            description: "Example tool for testing".to_string(),
            required_inputs: vec![],
            optional_inputs: vec!["data".to_string()],
            output_format: OutputFormat::Binary,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_tool_executor_creation() {
        let executor = ToolExecutor::new(ToolExecutorConfig::default());
        let status = executor.get_status().await;
        
        assert_eq!(status.total, 0);
        assert_eq!(status.completed, 0);
    }
    
    #[tokio::test]
    async fn test_tool_registration() {
        let executor = ToolExecutor::new(ToolExecutorConfig::default());
        let tool = Box::new(ExampleTool {
            name: "test-tool".to_string(),
        });
        
        let result = executor.register_tool(tool).await;
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_simple_execution() {
        let executor = ToolExecutor::new(ToolExecutorConfig::default());
        
        // Register tool
        let tool = Box::new(ExampleTool {
            name: "test-tool".to_string(),
        });
        executor.register_tool(tool).await.unwrap();
        
        // Create execution
        let execution = ToolExecution {
            id: "exec-1".to_string(),
            tool_id: "test-tool".to_string(),
            inputs: HashMap::new(),
            dependencies: vec![],
            can_fail: false,
            timeout_ms: 5000,
            retry_config: RetryConfig::default(),
        };
        
        // Execute
        let results = executor.execute_sequence(vec![execution]).await.unwrap();
        
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
    }
}