#[cfg(test)]
mod integration_tests {
    use super::*;
    use tokio;

    /// 测试 A2A 通信协议功能
    #[tokio::test]
    async fn test_a2a_protocol_implementation() {
        use crate::a2a::{A2AGateway, A2AMessageBuilder, A2AMessageType, MessagePriority};
        
        let gateway = A2AGateway::new();
        
        // 创建测试消息
        let message = A2AMessageBuilder::new(
            "test_sender".to_string(),
            "test_recipient".to_string(),
            A2AMessageType::Query { 
                question: "Can you help me?".to_string() 
            }
        )
        .with_content(serde_json::json!({"data": "test"}))
        .with_priority(MessagePriority::Normal)
        .build();

        // 验证消息创建
        assert_eq!(message.sender_id, "test_sender");
        assert_eq!(message.recipient_id, "test_recipient");
        assert!(matches!(message.message_type, A2AMessageType::Query { .. }));
        
        println!("✓ A2A 通信协议功能测试通过");
    }

    /// 测试董事长 Agent 基本功能
    #[tokio::test]
    async fn test_chairman_agent_implementation() {
        use crate::agent::ChairmanAgent;
        use crate::a2a::A2AGateway;
        use crate::core::{MemoryCore, ResourceCore, HealthCore};
        use std::sync::Arc;

        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = Arc::new(MemoryCore::new(a2a_gateway.clone()));
        let resource_core = Arc::new(ResourceCore::new());
        let health_core = Arc::new(HealthCore::new());

        let chairman = ChairmanAgent::new(
            "user123".to_string(),
            "telegram_bot".to_string(),
            a2a_gateway,
            memory_core,
            resource_core,
            health_core,
        );

        // 验证董事长 Agent 创建
        assert_eq!(chairman.user_id, "user123");
        assert_eq!(chairman.user_channel, "telegram_bot");
        
        println!("✓ 董事长 Agent 基本功能测试通过");
    }

    /// 测试 MemoryCore 分级记忆功能
    #[tokio::test]
    async fn test_memory_core_levels() {
        use crate::a2a::A2AGateway;
        use crate::core::memory_core::{MemoryCore, MemoryEntry, MemoryLevel, AccessRole, AccessPermissions};
        use std::collections::HashSet;
        use std::sync::Arc;
        
        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = MemoryCore::new(a2a_gateway);

        // 创建不同级别的记忆条目
        let mut tags = HashSet::new();
        tags.insert("test".to_string());
        tags.insert("memory".to_string());

        let entry = crate::core::memory_core::MemoryEntry {
            id: "test_entry_1".to_string(),
            key: "test_key".to_string(),
            content: "This is a test memory entry".to_string(),
            level: crate::core::memory_core::MemoryLevel::Team,
            team_id: Some("test_team".to_string()),
            instance_id: Some("test_instance".to_string()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            expires_at: None,
            access_permissions: crate::core::memory_core::AccessPermissions {
                read_roles: [crate::core::memory_core::AccessRole::Chairman, 
                             crate::core::memory_core::AccessRole::CEO].iter().cloned().collect(),
                write_roles: [crate::core::memory_core::AccessRole::Chairman].iter().cloned().collect(),
                delete_roles: [crate::core::memory_core::AccessRole::Chairman].iter().cloned().collect(),
            },
            tags,
            importance: 85,
        };

        // 存储记忆
        let id = memory_core.store_memory(entry, crate::core::memory_core::AccessRole::Chairman).await.unwrap();
        assert!(!id.is_empty());
        
        println!("✓ MemoryCore 分级记忆功能测试通过");
    }

    /// 测试 ResourceCore 资源管理功能
    #[tokio::test]
    async fn test_resource_core_basic() {
        use crate::core::resource_core::{ResourceCore, ResourceType, ResourceQuota, ResourceUsage};
        
        let resource_core = ResourceCore::new();
        
        // 设置资源总量
        resource_core.set_total_resource(ResourceType::Compute, 1000);
        resource_core.set_total_resource(ResourceType::Memory, 8192); // 8GB
        
        // 验证资源设置
        let compute_usage = resource_core.get_resource_usage(ResourceType::Compute).await;
        assert!(compute_usage.is_some());
        
        let memory_usage = resource_core.get_resource_usage(ResourceType::Memory).await;
        assert!(memory_usage.is_some());
        
        println!("✓ ResourceCore 资源管理功能测试通过");
    }

    /// 测试 HealthCore 健康检查功能
    #[tokio::test]
    async fn test_health_core_basic() {
        use crate::core::health_core::{HealthCore, HealthStatus, HealthComponentType};
        
        let health_core = HealthCore::new();
        
        // 设置组件状态
        health_core.set_component_status("test_component".to_string(), HealthStatus::Healthy, HealthComponentType::Service);
        
        // 获取组件状态
        let status = health_core.get_component_status("test_component");
        assert_eq!(status, Some(HealthStatus::Healthy));
        
        // 获取整体状态
        let overall_status = health_core.get_overall_status().await;
        assert!(matches!(overall_status, HealthStatus::Healthy | HealthStatus::Warning));
        
        println!("✓ HealthCore 健康检查功能测试通过");
    }

    /// 测试 Skills 系统编排功能
    #[tokio::test]
    async fn test_skills_system_basic() {
        use crate::a2a::A2AGateway;
        use crate::core::{MemoryCore, ResourceCore, HealthCore};
        use crate::skills::{SkillsOrchestration, InformationGatheringSkill};
        use std::sync::Arc;
        
        let a2a_gateway = Arc::new(A2AGateway::new());
        let memory_core = Arc::new(MemoryCore::new(a2a_gateway.clone()));
        let resource_core = Arc::new(ResourceCore::new());
        let health_core = Arc::new(HealthCore::new());
        
        let skills_orchestration = SkillsOrchestration::new(
            a2a_gateway,
            memory_core,
            resource_core,
            health_core,
        );
        
        // 注册示例技能
        skills_orchestration.register_skill(Arc::new(InformationGatheringSkill::new()));
        
        // 验证技能注册
        let skill = skills_orchestration.find_skill("information_gathering");
        assert!(skill.is_some());
        
        println!("✓ Skills 系统编排功能测试通过");
    }
}