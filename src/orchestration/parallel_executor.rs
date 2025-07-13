//! Parallel tool execution support

use super::tool_executor::{ToolExecutor, ToolExecution, ToolResult};
use crate::errors::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, warn};

/// Parallel executor for tool sequences
pub struct ParallelExecutor {
    /// Tool executor
    tool_executor: Arc<ToolExecutor>,
    
    /// Execution semaphore for concurrency control
    semaphore: Arc<Semaphore>,
    
    /// Execution strategy
    strategy: ExecutionStrategy,
    
    /// Metrics
    metrics: Arc<RwLock<ParallelMetrics>>,
}

/// Execution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStrategy {
    /// Execute all tools in parallel (no dependencies)
    FullParallel,
    
    /// Execute in waves based on dependencies
    WaveBased,
    
    /// Optimize execution order for best performance
    Optimized,
    
    /// Execute based on resource availability
    ResourceAware,
}

/// Parallel execution metrics
#[derive(Debug, Default)]
struct ParallelMetrics {
    total_executions: u64,
    parallel_executions: u64,
    average_parallelism: f64,
    total_execution_time_ms: u64,
    resource_conflicts: u64,
}

/// Execution wave for wave-based strategy
#[derive(Debug, Clone)]
struct ExecutionWave {
    /// Wave number
    wave_number: usize,
    
    /// Executions in this wave
    executions: Vec<ToolExecution>,
    
    /// Estimated execution time
    estimated_time_ms: u64,
}

/// Resource tracker for resource-aware execution
struct ResourceTracker {
    /// Available compute units
    available_compute: Arc<RwLock<u64>>,
    
    /// Available memory in MB
    available_memory: Arc<RwLock<u64>>,
    
    /// Available network bandwidth in Mbps
    available_bandwidth: Arc<RwLock<u64>>,
}

impl ParallelExecutor {
    /// Create a new parallel executor
    pub fn new(
        tool_executor: Arc<ToolExecutor>,
        max_parallel: usize,
        strategy: ExecutionStrategy,
    ) -> Self {
        Self {
            tool_executor,
            semaphore: Arc::new(Semaphore::new(max_parallel)),
            strategy,
            metrics: Arc::new(RwLock::new(ParallelMetrics::default())),
        }
    }
    
    /// Execute tools in parallel based on strategy
    pub async fn execute_parallel(
        &self,
        executions: Vec<ToolExecution>,
    ) -> Result<Vec<ToolResult>> {
        info!("Executing {} tools with {:?} strategy", executions.len(), self.strategy);
        
        let start_time = std::time::Instant::now();
        
        let results = match self.strategy {
            ExecutionStrategy::FullParallel => {
                self.execute_full_parallel(executions).await?
            }
            ExecutionStrategy::WaveBased => {
                self.execute_wave_based(executions).await?
            }
            ExecutionStrategy::Optimized => {
                self.execute_optimized(executions).await?
            }
            ExecutionStrategy::ResourceAware => {
                self.execute_resource_aware(executions).await?
            }
        };
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_executions += results.len() as u64;
            metrics.total_execution_time_ms += start_time.elapsed().as_millis() as u64;
        }
        
        Ok(results)
    }
    
    /// Execute all tools in full parallel (no dependencies)
    async fn execute_full_parallel(
        &self,
        executions: Vec<ToolExecution>,
    ) -> Result<Vec<ToolResult>> {
        // Validate no dependencies
        for execution in &executions {
            if !execution.dependencies.is_empty() {
                return Err(SvmExExError::ProcessingError(
                    "Full parallel strategy requires no dependencies".to_string()
                ));
            }
        }
        
        let mut handles = Vec::new();
        
        for execution in executions {
            let executor = self.tool_executor.clone();
            let permit = self.semaphore.clone().acquire_owned().await.unwrap();
            
            let handle = tokio::spawn(async move {
                let _permit = permit; // Hold permit until done
                executor.execute_sequence(vec![execution]).await
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut all_results = Vec::new();
        for handle in handles {
            let results = handle.await
                .map_err(|e| SvmExExError::ProcessingError(e.to_string()))??;
            all_results.extend(results);
        }
        
        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.parallel_executions += all_results.len() as u64;
            metrics.average_parallelism = 
                (metrics.average_parallelism * (metrics.total_executions - all_results.len() as u64) as f64 + 
                 all_results.len() as f64) / metrics.total_executions as f64;
        }
        
        Ok(all_results)
    }
    
    /// Execute in waves based on dependencies
    async fn execute_wave_based(
        &self,
        executions: Vec<ToolExecution>,
    ) -> Result<Vec<ToolResult>> {
        let waves = self.build_execution_waves(&executions)?;
        info!("Built {} execution waves", waves.len());
        
        let mut all_results = Vec::new();
        
        for wave in waves {
            debug!("Executing wave {} with {} tools", 
                wave.wave_number, wave.executions.len());
            
            // Execute wave in parallel
            let wave_results = self.execute_full_parallel(wave.executions).await?;
            all_results.extend(wave_results);
        }
        
        Ok(all_results)
    }
    
    /// Build execution waves from dependencies
    fn build_execution_waves(
        &self,
        executions: &[ToolExecution],
    ) -> Result<Vec<ExecutionWave>> {
        let mut waves = Vec::new();
        let mut assigned = std::collections::HashSet::new();
        let mut wave_number = 0;
        
        // Create execution map
        let exec_map: HashMap<String, &ToolExecution> = executions.iter()
            .map(|e| (e.id.clone(), e))
            .collect();
        
        while assigned.len() < executions.len() {
            let mut wave_executions = Vec::new();
            
            for execution in executions {
                if assigned.contains(&execution.id) {
                    continue;
                }
                
                // Check if all dependencies are assigned
                let deps_satisfied = execution.dependencies.iter()
                    .all(|dep| assigned.contains(dep));
                
                if deps_satisfied {
                    wave_executions.push(execution.clone());
                }
            }
            
            if wave_executions.is_empty() {
                return Err(SvmExExError::ProcessingError(
                    "Circular dependencies detected".to_string()
                ));
            }
            
            // Mark as assigned
            for execution in &wave_executions {
                assigned.insert(execution.id.clone());
            }
            
            waves.push(ExecutionWave {
                wave_number,
                executions: wave_executions,
                estimated_time_ms: 0, // TODO: Estimate based on tool metadata
            });
            
            wave_number += 1;
        }
        
        Ok(waves)
    }
    
    /// Execute with optimized order
    async fn execute_optimized(
        &self,
        executions: Vec<ToolExecution>,
    ) -> Result<Vec<ToolResult>> {
        // Analyze execution graph
        let optimization = self.optimize_execution_order(&executions)?;
        
        info!("Optimized execution plan: {} parallel groups", 
            optimization.parallel_groups.len());
        
        let mut all_results = Vec::new();
        
        for group in optimization.parallel_groups {
            let group_results = self.execute_full_parallel(group).await?;
            all_results.extend(group_results);
        }
        
        Ok(all_results)
    }
    
    /// Optimize execution order
    fn optimize_execution_order(
        &self,
        executions: &[ToolExecution],
    ) -> Result<ExecutionOptimization> {
        // Build dependency graph
        let graph = self.build_dependency_graph(executions);
        
        // Find critical path
        let critical_path = self.find_critical_path(&graph)?;
        
        // Group by parallelization opportunity
        let parallel_groups = self.group_by_parallelization(&graph, &critical_path)?;
        
        Ok(ExecutionOptimization {
            critical_path,
            parallel_groups,
            estimated_time_ms: 0, // TODO: Calculate
        })
    }
    
    /// Build dependency graph
    fn build_dependency_graph(
        &self,
        executions: &[ToolExecution],
    ) -> DependencyGraph {
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        
        // Create nodes
        for execution in executions {
            nodes.insert(execution.id.clone(), GraphNode {
                id: execution.id.clone(),
                execution: execution.clone(),
                in_degree: 0,
                out_degree: 0,
            });
        }
        
        // Create edges
        for execution in executions {
            for dep in &execution.dependencies {
                edges.push((dep.clone(), execution.id.clone()));
                
                if let Some(node) = nodes.get_mut(&execution.id) {
                    node.in_degree += 1;
                }
                if let Some(node) = nodes.get_mut(dep) {
                    node.out_degree += 1;
                }
            }
        }
        
        DependencyGraph { nodes, edges }
    }
    
    /// Find critical path in dependency graph
    fn find_critical_path(&self, graph: &DependencyGraph) -> Result<Vec<String>> {
        // Simple topological sort for now
        let mut sorted = Vec::new();
        let mut in_degrees = HashMap::new();
        
        for (id, node) in &graph.nodes {
            in_degrees.insert(id.clone(), node.in_degree);
        }
        
        let mut queue = Vec::new();
        for (id, &degree) in &in_degrees {
            if degree == 0 {
                queue.push(id.clone());
            }
        }
        
        while let Some(node_id) = queue.pop() {
            sorted.push(node_id.clone());
            
            for (from, to) in &graph.edges {
                if from == &node_id {
                    if let Some(degree) = in_degrees.get_mut(to) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push(to.clone());
                        }
                    }
                }
            }
        }
        
        if sorted.len() != graph.nodes.len() {
            return Err(SvmExExError::ProcessingError(
                "Graph contains cycles".to_string()
            ));
        }
        
        Ok(sorted)
    }
    
    /// Group executions by parallelization opportunity
    fn group_by_parallelization(
        &self,
        graph: &DependencyGraph,
        critical_path: &[String],
    ) -> Result<Vec<Vec<ToolExecution>>> {
        // Group by level in dependency graph
        let levels = self.calculate_levels(graph)?;
        let mut groups: HashMap<usize, Vec<ToolExecution>> = HashMap::new();
        
        for (id, level) in levels {
            if let Some(node) = graph.nodes.get(&id) {
                groups.entry(level)
                    .or_insert_with(Vec::new)
                    .push(node.execution.clone());
            }
        }
        
        let mut sorted_groups: Vec<_> = groups.into_iter().collect();
        sorted_groups.sort_by_key(|(level, _)| *level);
        
        Ok(sorted_groups.into_iter().map(|(_, group)| group).collect())
    }
    
    /// Calculate levels in dependency graph
    fn calculate_levels(&self, graph: &DependencyGraph) -> Result<HashMap<String, usize>> {
        let mut levels = HashMap::new();
        let mut queue = Vec::new();
        
        // Start with nodes that have no dependencies
        for (id, node) in &graph.nodes {
            if node.in_degree == 0 {
                levels.insert(id.clone(), 0);
                queue.push((id.clone(), 0));
            }
        }
        
        while let Some((node_id, level)) = queue.pop() {
            for (from, to) in &graph.edges {
                if from == &node_id {
                    let new_level = level + 1;
                    levels.entry(to.clone())
                        .and_modify(|l| *l = (*l).max(new_level))
                        .or_insert(new_level);
                    
                    queue.push((to.clone(), new_level));
                }
            }
        }
        
        Ok(levels)
    }
    
    /// Execute with resource awareness
    async fn execute_resource_aware(
        &self,
        executions: Vec<ToolExecution>,
    ) -> Result<Vec<ToolResult>> {
        let resource_tracker = ResourceTracker {
            available_compute: Arc::new(RwLock::new(1_000_000)), // 1M compute units
            available_memory: Arc::new(RwLock::new(16_384)), // 16GB
            available_bandwidth: Arc::new(RwLock::new(1_000)), // 1Gbps
        };
        
        // Sort by resource requirements (simple heuristic)
        let mut sorted_executions = executions;
        sorted_executions.sort_by_key(|e| e.timeout_ms); // Use timeout as proxy for resource needs
        
        let mut handles = Vec::new();
        
        for execution in sorted_executions {
            // Wait for resources
            self.wait_for_resources(&resource_tracker, &execution).await?;
            
            let executor = self.tool_executor.clone();
            let tracker = resource_tracker.clone();
            let exec_clone = execution.clone();
            
            let handle = tokio::spawn(async move {
                let result = executor.execute_sequence(vec![execution]).await;
                
                // Release resources
                Self::release_resources(&tracker, &exec_clone).await;
                
                result
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let mut all_results = Vec::new();
        for handle in handles {
            let results = handle.await
                .map_err(|e| SvmExExError::ProcessingError(e.to_string()))??;
            all_results.extend(results);
        }
        
        Ok(all_results)
    }
    
    /// Wait for resources to become available
    async fn wait_for_resources(
        &self,
        tracker: &ResourceTracker,
        execution: &ToolExecution,
    ) -> Result<()> {
        // Simple resource estimation based on timeout
        let required_compute = execution.timeout_ms * 100; // Rough estimate
        let required_memory = 256; // 256MB default
        
        loop {
            let compute_available = {
                let available = tracker.available_compute.read().await;
                *available >= required_compute
            };
            
            let memory_available = {
                let available = tracker.available_memory.read().await;
                *available >= required_memory
            };
            
            if compute_available && memory_available {
                // Reserve resources
                {
                    let mut compute = tracker.available_compute.write().await;
                    *compute -= required_compute;
                }
                {
                    let mut memory = tracker.available_memory.write().await;
                    *memory -= required_memory;
                }
                break;
            }
            
            // Wait a bit before checking again
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
        
        Ok(())
    }
    
    /// Release resources after execution
    async fn release_resources(tracker: &ResourceTracker, execution: &ToolExecution) {
        let required_compute = execution.timeout_ms * 100;
        let required_memory = 256;
        
        {
            let mut compute = tracker.available_compute.write().await;
            *compute += required_compute;
        }
        {
            let mut memory = tracker.available_memory.write().await;
            *memory += required_memory;
        }
    }
    
    /// Get execution metrics
    pub async fn get_metrics(&self) -> ParallelMetrics {
        self.metrics.read().await.clone()
    }
}

/// Dependency graph
struct DependencyGraph {
    nodes: HashMap<String, GraphNode>,
    edges: Vec<(String, String)>, // (from, to)
}

/// Graph node
struct GraphNode {
    id: String,
    execution: ToolExecution,
    in_degree: usize,
    out_degree: usize,
}

/// Execution optimization result
struct ExecutionOptimization {
    critical_path: Vec<String>,
    parallel_groups: Vec<Vec<ToolExecution>>,
    estimated_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::tool_executor::ToolExecutorConfig;
    
    #[tokio::test]
    async fn test_parallel_executor_creation() {
        let tool_executor = Arc::new(ToolExecutor::new(ToolExecutorConfig::default()));
        let parallel_executor = ParallelExecutor::new(
            tool_executor,
            10,
            ExecutionStrategy::FullParallel
        );
        
        let metrics = parallel_executor.get_metrics().await;
        assert_eq!(metrics.total_executions, 0);
    }
    
    #[tokio::test]
    async fn test_wave_building() {
        let tool_executor = Arc::new(ToolExecutor::new(ToolExecutorConfig::default()));
        let parallel_executor = ParallelExecutor::new(
            tool_executor,
            10,
            ExecutionStrategy::WaveBased
        );
        
        let executions = vec![
            ToolExecution {
                id: "1".to_string(),
                tool_id: "tool1".to_string(),
                inputs: HashMap::new(),
                dependencies: vec![],
                can_fail: false,
                timeout_ms: 1000,
                retry_config: Default::default(),
            },
            ToolExecution {
                id: "2".to_string(),
                tool_id: "tool2".to_string(),
                inputs: HashMap::new(),
                dependencies: vec!["1".to_string()],
                can_fail: false,
                timeout_ms: 1000,
                retry_config: Default::default(),
            },
            ToolExecution {
                id: "3".to_string(),
                tool_id: "tool3".to_string(),
                inputs: HashMap::new(),
                dependencies: vec!["1".to_string()],
                can_fail: false,
                timeout_ms: 1000,
                retry_config: Default::default(),
            },
        ];
        
        let waves = parallel_executor.build_execution_waves(&executions).unwrap();
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0].executions.len(), 1);
        assert_eq!(waves[1].executions.len(), 2);
    }
}