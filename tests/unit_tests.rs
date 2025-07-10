//! Comprehensive unit tests for SVM ExEx

use monmouth_svm_exex::*;
use alloy_primitives::{Address, B256};
use std::sync::Arc;

#[cfg(test)]
mod svm_processor_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_svm_processor_creation() {
        let processor = create_svm_processor();
        assert!(processor.get_state_root().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_process_transaction() {
        let processor = create_svm_processor();
        let tx_data = vec![1, 2, 3, 4, 5];
        
        let result = processor.process_transaction(&tx_data).await;
        assert!(result.is_ok());
        
        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert_eq!(execution_result.gas_used, 5000);
    }
    
    #[tokio::test]
    async fn test_revert_to_checkpoint() {
        let processor = create_svm_processor();
        
        // Try to revert to non-existent checkpoint
        let result = processor.revert_to_checkpoint(100).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod inter_exex_tests {
    use super::*;
    use monmouth_svm_exex::inter_exex::*;
    
    #[tokio::test]
    async fn test_message_creation() {
        let msg = ExExMessage::new(
            MessageType::Data,
            "test-node".to_string(),
            MessagePayload::Data(vec![1, 2, 3]),
        );
        
        assert!(msg.validate());
        assert!(msg.is_broadcast());
    }
    
    #[tokio::test]
    async fn test_svm_message_conversion() {
        let svm_msg = SvmExExMessage::HealthCheck {
            requester: "node1".to_string(),
            component: HealthCheckComponent::SvmProcessor,
        };
        
        let exex_msg: ExExMessage = svm_msg.clone().into();
        assert_eq!(exex_msg.source, "svm-exex");
        
        // Convert back
        let converted = SvmExExMessage::try_from(exex_msg);
        assert!(converted.is_ok());
    }
    
    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = MessageBusConfig::default();
        let coordinator = InterExExCoordinator::new(config, "test-node".to_string());
        assert!(coordinator.is_ok());
    }
}

#[cfg(test)]
mod rag_integration_tests {
    use super::*;
    use monmouth_svm_exex::rag_integration::*;
    use monmouth_svm_exex::inter_exex::*;
    
    #[tokio::test]
    async fn test_rag_service_creation() {
        let config = MessageBusConfig::default();
        let coordinator = Arc::new(
            InterExExCoordinator::new(config, "test-node".to_string()).unwrap()
        );
        
        let rag_config = RagIntegrationConfig::default();
        let service = RagIntegrationService::new(coordinator, rag_config);
        
        // Test context request (will timeout as no RAG ExEx is running)
        let result = service.get_transaction_context(
            B256::default(),
            "agent1",
            "test query"
        ).await;
        
        // Should return empty context on timeout
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
    
    #[tokio::test]
    async fn test_memory_creation() {
        let config = MessageBusConfig::default();
        let coordinator = Arc::new(
            InterExExCoordinator::new(config, "test-node".to_string()).unwrap()
        );
        
        let rag_config = RagIntegrationConfig::default();
        let service = RagIntegrationService::new(coordinator, rag_config);
        
        let result = SvmExecutionResult {
            tx_hash: B256::default(),
            success: true,
            gas_used: 1000,
            return_data: vec![],
            logs: vec!["test".to_string()],
            state_changes: vec![],
        };
        
        let routing = monmouth_svm_exex::ai::traits::RoutingDecision {
            decision: monmouth_svm_exex::ai::traits::DecisionType::RouteToSVM,
            confidence: 0.9,
            factors: Default::default(),
            reasoning: "test".to_string(),
        };
        
        let memory = service.create_memory_from_execution(
            B256::default(),
            &result,
            &routing,
        );
        
        assert_eq!(memory.memory_type, MemoryType::Episodic);
        assert!(memory.metadata.contains_key("success"));
    }
}

#[cfg(test)]
mod state_sync_tests {
    use super::*;
    use monmouth_svm_exex::state_sync::*;
    use monmouth_svm_exex::enhanced_exex::AccountsLatticeHash;
    
    #[tokio::test]
    async fn test_state_sync_creation() {
        let config = monmouth_svm_exex::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            monmouth_svm_exex::inter_exex::InterExExCoordinator::new(
                config,
                "test-node".to_string()
            ).unwrap()
        );
        
        let alh = AccountsLatticeHash::new();
        let sync_config = StateSyncConfig::default();
        let service = StateSyncService::new(coordinator, alh, sync_config);
        
        let info = service.get_state_info().await;
        assert_eq!(info.account_count, 0);
        assert_eq!(info.total_lamports, 0);
    }
    
    #[tokio::test]
    async fn test_account_update() {
        let config = monmouth_svm_exex::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            monmouth_svm_exex::inter_exex::InterExExCoordinator::new(
                config,
                "test-node".to_string()
            ).unwrap()
        );
        
        let alh = AccountsLatticeHash::new();
        let sync_config = StateSyncConfig::default();
        let service = StateSyncService::new(coordinator, alh, sync_config);
        
        // Update account
        let address = Address::default();
        service.update_account(
            address,
            1000,
            None,
            monmouth_svm_exex::inter_exex::AccountUpdateType::Created,
        ).await.unwrap();
        
        let info = service.get_state_info().await;
        assert_eq!(info.account_count, 1);
        assert_eq!(info.total_lamports, 1000);
    }
    
    #[tokio::test]
    async fn test_state_reversion() {
        let config = monmouth_svm_exex::inter_exex::MessageBusConfig::default();
        let coordinator = Arc::new(
            monmouth_svm_exex::inter_exex::InterExExCoordinator::new(
                config,
                "test-node".to_string()
            ).unwrap()
        );
        
        let alh = AccountsLatticeHash::new();
        let sync_config = StateSyncConfig::default();
        let service = StateSyncService::new(coordinator, alh, sync_config);
        
        // Try to revert to non-existent block
        let result = service.revert_to_block(100).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod ai_decision_tests {
    use super::*;
    
    #[cfg(feature = "ai-agents")]
    #[tokio::test]
    async fn test_decision_engine() {
        use monmouth_svm_exex::ai::decision_engine::*;
        use monmouth_svm_exex::ai::traits::*;
        
        let config = EnhancedAIDecisionEngineConfig::default();
        let engine = EnhancedAIDecisionEngine::new(config);
        
        let tx_data = vec![1, 2, 3, 4, 5];
        let decision = engine.analyze_transaction(&tx_data).await;
        
        assert!(decision.confidence > 0.0);
        assert!(decision.confidence <= 1.0);
        assert!(!decision.reasoning.is_empty());
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;
    use monmouth_svm_exex::errors::*;
    
    #[test]
    fn test_error_conversions() {
        // Test From implementations
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "test");
        let svm_error: SvmExExError = io_error.into();
        assert!(matches!(svm_error, SvmExExError::Unknown(_)));
        
        // Test custom errors
        let checkpoint_error = SvmExExError::CheckpointNotFound(42);
        let error_string = checkpoint_error.to_string();
        assert!(error_string.contains("42"));
    }
    
    #[test]
    fn test_result_type() {
        fn test_function() -> Result<i32> {
            Ok(42)
        }
        
        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}

#[cfg(test)]
mod integration_helpers {
    use super::*;
    
    /// Helper to create a test transaction
    pub fn create_test_transaction() -> Vec<u8> {
        vec![
            0x01, // Version
            0x00, // Instruction count
            0x00, 0x00, 0x00, 0x00, // Data length
        ]
    }
    
    /// Helper to create test context data
    pub fn create_test_context() -> monmouth_svm_exex::ai::traits::ContextData {
        monmouth_svm_exex::ai::traits::ContextData {
            id: "test-context".to_string(),
            content: "Test context content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: Default::default(),
            timestamp: 0,
        }
    }
}