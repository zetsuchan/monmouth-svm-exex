//! Intent classification system to route transactions

use super::agent_tx::{AgentTx, Intent, IntentType};
use crate::errors::*;
use std::collections::HashMap;
use tracing::{debug, info};

/// Intent classifier for agent transactions
pub struct IntentClassifier {
    /// Classification rules
    rules: Vec<ClassificationRule>,
    
    /// Intent patterns
    patterns: HashMap<String, IntentPattern>,
    
    /// Risk assessment configuration
    risk_config: RiskConfig,
}

/// Classification rule
#[derive(Debug, Clone)]
struct ClassificationRule {
    /// Rule name
    name: String,
    
    /// Conditions to match
    conditions: Vec<Condition>,
    
    /// Classification if matched
    classification: IntentCategory,
    
    /// Confidence boost
    confidence_boost: f32,
}

/// Condition for rule matching
#[derive(Debug, Clone)]
enum Condition {
    /// Intent type matches
    IntentTypeIs(IntentType),
    
    /// Parameter exists
    HasParameter(String),
    
    /// Resource requirement
    RequiresResource(ResourceType),
    
    /// Cross-chain operation
    IsCrossChain,
    
    /// High value transaction
    IsHighValue(u64),
}

/// Resource type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResourceType {
    GPU,
    TEE,
    HighMemory,
    LargeStorage,
}

/// Intent pattern for matching
#[derive(Debug, Clone)]
struct IntentPattern {
    /// Pattern name
    name: String,
    
    /// Keywords to match
    keywords: Vec<String>,
    
    /// Category if matched
    category: IntentCategory,
    
    /// Base confidence
    base_confidence: f32,
}

/// Intent category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentCategory {
    DeFi,
    NFT,
    Gaming,
    Identity,
    Governance,
    Infrastructure,
    Unknown,
}

/// Risk configuration
#[derive(Debug, Clone)]
struct RiskConfig {
    /// High value threshold
    high_value_threshold: u64,
    
    /// Maximum cross-chain hops
    max_cross_chain_hops: usize,
    
    /// Suspicious patterns
    suspicious_patterns: Vec<String>,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            high_value_threshold: 1_000_000, // 1M units
            max_cross_chain_hops: 3,
            suspicious_patterns: vec![
                "drain".to_string(),
                "exploit".to_string(),
                "hack".to_string(),
            ],
        }
    }
}

/// Intent classification result
#[derive(Debug, Clone)]
pub struct IntentClassification {
    /// Primary category
    pub category: IntentCategory,
    
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    
    /// Suggested tools for execution
    pub suggested_tools: Vec<String>,
    
    /// Risk factors identified
    pub risk_factors: Vec<String>,
    
    /// Estimated resources
    pub estimated_resources: EstimatedResources,
}

/// Estimated resources for intent execution
#[derive(Debug, Clone)]
pub struct EstimatedResources {
    pub gas_estimate: u64,
    pub compute_units_estimate: u64,
    pub execution_time_ms_estimate: u64,
    pub success_probability: f32,
}

impl IntentClassifier {
    /// Create a new intent classifier
    pub fn new() -> Self {
        let mut classifier = Self {
            rules: Vec::new(),
            patterns: HashMap::new(),
            risk_config: RiskConfig::default(),
        };
        
        classifier.initialize_default_rules();
        classifier.initialize_default_patterns();
        
        classifier
    }
    
    /// Initialize default classification rules
    fn initialize_default_rules(&mut self) {
        // DeFi rules
        self.rules.push(ClassificationRule {
            name: "defi_swap".to_string(),
            conditions: vec![
                Condition::IntentTypeIs(IntentType::Swap),
            ],
            classification: IntentCategory::DeFi,
            confidence_boost: 0.3,
        });
        
        self.rules.push(ClassificationRule {
            name: "defi_stake".to_string(),
            conditions: vec![
                Condition::IntentTypeIs(IntentType::Stake),
            ],
            classification: IntentCategory::DeFi,
            confidence_boost: 0.3,
        });
        
        // Infrastructure rules
        self.rules.push(ClassificationRule {
            name: "infra_compute".to_string(),
            conditions: vec![
                Condition::IntentTypeIs(IntentType::Compute),
                Condition::RequiresResource(ResourceType::GPU),
            ],
            classification: IntentCategory::Infrastructure,
            confidence_boost: 0.4,
        });
        
        // Cross-chain rules
        self.rules.push(ClassificationRule {
            name: "cross_chain_bridge".to_string(),
            conditions: vec![
                Condition::IntentTypeIs(IntentType::Bridge),
                Condition::IsCrossChain,
            ],
            classification: IntentCategory::Infrastructure,
            confidence_boost: 0.5,
        });
    }
    
    /// Initialize default intent patterns
    fn initialize_default_patterns(&mut self) {
        // DeFi patterns
        self.patterns.insert("defi_pattern".to_string(), IntentPattern {
            name: "defi_pattern".to_string(),
            keywords: vec![
                "swap".to_string(),
                "exchange".to_string(),
                "liquidity".to_string(),
                "yield".to_string(),
                "farm".to_string(),
                "stake".to_string(),
            ],
            category: IntentCategory::DeFi,
            base_confidence: 0.7,
        });
        
        // NFT patterns
        self.patterns.insert("nft_pattern".to_string(), IntentPattern {
            name: "nft_pattern".to_string(),
            keywords: vec![
                "mint".to_string(),
                "nft".to_string(),
                "collection".to_string(),
                "metadata".to_string(),
                "royalty".to_string(),
            ],
            category: IntentCategory::NFT,
            base_confidence: 0.7,
        });
        
        // Gaming patterns
        self.patterns.insert("gaming_pattern".to_string(), IntentPattern {
            name: "gaming_pattern".to_string(),
            keywords: vec![
                "game".to_string(),
                "play".to_string(),
                "score".to_string(),
                "achievement".to_string(),
                "item".to_string(),
            ],
            category: IntentCategory::Gaming,
            base_confidence: 0.6,
        });
    }
    
    /// Classify an agent transaction
    pub fn classify(&self, tx: &AgentTx) -> IntentClassification {
        debug!("Classifying intent: {}", tx.intent.intent_id);
        
        let mut category = IntentCategory::Unknown;
        let mut confidence = 0.0;
        let mut suggested_tools = Vec::new();
        let mut risk_factors = Vec::new();
        
        // Apply rule-based classification
        let rule_result = self.apply_rules(tx);
        if rule_result.confidence > confidence {
            category = rule_result.category;
            confidence = rule_result.confidence;
        }
        
        // Apply pattern matching
        let pattern_result = self.match_patterns(&tx.intent);
        if pattern_result.confidence > confidence {
            category = pattern_result.category;
            confidence = pattern_result.confidence;
        }
        
        // Assess risks
        risk_factors.extend(self.assess_risks(tx));
        
        // Suggest tools based on category and intent
        suggested_tools.extend(self.suggest_tools(category, &tx.intent));
        
        // Estimate resources
        let estimated_resources = self.estimate_resources(tx);
        
        IntentClassification {
            category,
            confidence,
            suggested_tools,
            risk_factors,
            estimated_resources,
        }
    }
    
    /// Apply classification rules
    fn apply_rules(&self, tx: &AgentTx) -> ClassificationResult {
        let mut best_match = ClassificationResult {
            category: IntentCategory::Unknown,
            confidence: 0.0,
        };
        
        for rule in &self.rules {
            let mut matches = true;
            
            for condition in &rule.conditions {
                if !self.check_condition(condition, tx) {
                    matches = false;
                    break;
                }
            }
            
            if matches {
                let confidence = 0.5 + rule.confidence_boost;
                if confidence > best_match.confidence {
                    best_match = ClassificationResult {
                        category: rule.classification,
                        confidence,
                    };
                }
            }
        }
        
        best_match
    }
    
    /// Check if a condition matches
    fn check_condition(&self, condition: &Condition, tx: &AgentTx) -> bool {
        match condition {
            Condition::IntentTypeIs(intent_type) => tx.intent.intent_type == *intent_type,
            Condition::HasParameter(param_name) => {
                tx.intent.parameters.iter().any(|p| p.name == *param_name)
            }
            Condition::RequiresResource(resource_type) => {
                match resource_type {
                    ResourceType::GPU => tx.resources.requires_gpu,
                    ResourceType::TEE => tx.resources.requires_tee,
                    ResourceType::HighMemory => {
                        tx.resources.per_chain_allocation.values()
                            .any(|alloc| alloc.memory_mb > 1024)
                    }
                    ResourceType::LargeStorage => {
                        tx.resources.per_chain_allocation.values()
                            .any(|alloc| alloc.storage_kb > 1024 * 1024)
                    }
                }
            }
            Condition::IsCrossChain => tx.cross_chain_plans.len() > 1,
            Condition::IsHighValue(threshold) => tx.resources.total_gas_estimate > *threshold,
        }
    }
    
    /// Match intent patterns
    fn match_patterns(&self, intent: &Intent) -> ClassificationResult {
        let mut best_match = ClassificationResult {
            category: IntentCategory::Unknown,
            confidence: 0.0,
        };
        
        let description_lower = intent.description.to_lowercase();
        
        for pattern in self.patterns.values() {
            let mut keyword_matches = 0;
            
            for keyword in &pattern.keywords {
                if description_lower.contains(keyword) {
                    keyword_matches += 1;
                }
            }
            
            if keyword_matches > 0 {
                let confidence = pattern.base_confidence * 
                    (keyword_matches as f32 / pattern.keywords.len() as f32);
                
                if confidence > best_match.confidence {
                    best_match = ClassificationResult {
                        category: pattern.category,
                        confidence,
                    };
                }
            }
        }
        
        best_match
    }
    
    /// Assess risk factors
    fn assess_risks(&self, tx: &AgentTx) -> Vec<String> {
        let mut risks = Vec::new();
        
        // Check for high value
        if tx.resources.total_gas_estimate > self.risk_config.high_value_threshold {
            risks.push("High value transaction".to_string());
        }
        
        // Check cross-chain complexity
        if tx.cross_chain_plans.len() > self.risk_config.max_cross_chain_hops {
            risks.push(format!("Complex cross-chain operation ({} hops)", 
                tx.cross_chain_plans.len()));
        }
        
        // Check for suspicious patterns
        let description_lower = tx.intent.description.to_lowercase();
        for pattern in &self.risk_config.suspicious_patterns {
            if description_lower.contains(pattern) {
                risks.push(format!("Suspicious pattern detected: {}", pattern));
            }
        }
        
        // Check agent capabilities
        if tx.agent.agent_type == super::agent_tx::AgentType::Orchestrator &&
           tx.cross_chain_plans.len() > 2 {
            risks.push("High-privilege agent with complex operation".to_string());
        }
        
        risks
    }
    
    /// Suggest tools for execution
    fn suggest_tools(&self, category: IntentCategory, intent: &Intent) -> Vec<String> {
        let mut tools = Vec::new();
        
        match category {
            IntentCategory::DeFi => {
                tools.push("svm-executor".to_string());
                tools.push("price-oracle".to_string());
                if intent.intent_type == IntentType::Swap {
                    tools.push("dex-aggregator".to_string());
                }
            }
            IntentCategory::NFT => {
                tools.push("nft-minter".to_string());
                tools.push("metadata-processor".to_string());
            }
            IntentCategory::Gaming => {
                tools.push("game-state-manager".to_string());
                tools.push("score-verifier".to_string());
            }
            IntentCategory::Infrastructure => {
                tools.push("resource-allocator".to_string());
                if intent.intent_type == IntentType::Bridge {
                    tools.push("bridge-connector".to_string());
                }
            }
            _ => {
                tools.push("generic-executor".to_string());
            }
        }
        
        // Always include memory loader for agent transactions
        tools.push("memory-loader".to_string());
        
        tools
    }
    
    /// Estimate resources needed
    fn estimate_resources(&self, tx: &AgentTx) -> EstimatedResources {
        let base_gas = tx.resources.total_gas_estimate;
        let base_compute = tx.resources.total_compute_units;
        
        // Adjust based on complexity
        let complexity_factor = 1.0 + (tx.cross_chain_plans.len() as f64 * 0.2);
        
        // Estimate execution time
        let base_time = 25; // Base 25ms
        let cross_chain_time = tx.cross_chain_plans.len() as u64 * 50; // 50ms per chain
        let tool_time: u64 = tx.cross_chain_plans.iter()
            .map(|p| p.tools.len() as u64 * 10)
            .sum();
        
        EstimatedResources {
            gas_estimate: (base_gas as f64 * complexity_factor) as u64,
            compute_units_estimate: (base_compute as f64 * complexity_factor) as u64,
            execution_time_ms_estimate: base_time + cross_chain_time + tool_time,
            success_probability: self.calculate_success_probability(tx),
        }
    }
    
    /// Calculate success probability
    fn calculate_success_probability(&self, tx: &AgentTx) -> f32 {
        let mut probability = 0.95; // Base probability
        
        // Reduce for cross-chain complexity
        probability -= tx.cross_chain_plans.len() as f32 * 0.05;
        
        // Reduce for high resource requirements
        if tx.resources.requires_gpu {
            probability -= 0.1;
        }
        if tx.resources.requires_tee {
            probability -= 0.05;
        }
        
        // Ensure probability stays in valid range
        probability.max(0.1).min(0.99)
    }
}

/// Classification result
#[derive(Debug)]
struct ClassificationResult {
    category: IntentCategory,
    confidence: f32,
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::agent_tx::*;
    
    #[test]
    fn test_classifier_creation() {
        let classifier = IntentClassifier::new();
        assert!(!classifier.rules.is_empty());
        assert!(!classifier.patterns.is_empty());
    }
    
    #[test]
    fn test_defi_classification() {
        let classifier = IntentClassifier::new();
        
        let mut tx = create_test_agent_tx();
        tx.intent.intent_type = IntentType::Swap;
        tx.intent.description = "Swap USDC for ETH on Uniswap".to_string();
        
        let result = classifier.classify(&tx);
        
        assert_eq!(result.category, IntentCategory::DeFi);
        assert!(result.confidence > 0.7);
        assert!(result.suggested_tools.contains(&"dex-aggregator".to_string()));
    }
    
    #[test]
    fn test_risk_assessment() {
        let classifier = IntentClassifier::new();
        
        let mut tx = create_test_agent_tx();
        tx.resources.total_gas_estimate = 2_000_000; // High value
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
            CrossChainPlan {
                chain_id: "polygon".to_string(),
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
        
        let result = classifier.classify(&tx);
        
        assert!(!result.risk_factors.is_empty());
        assert!(result.risk_factors.iter().any(|r| r.contains("High value")));
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
            memory_context: MemoryContext {
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