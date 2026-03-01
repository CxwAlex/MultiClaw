//! A2A (Agent-to-Agent) 通信网关
//! 负责在 MultiClaw 的各个 Agent 之间路由消息

use crate::a2a::protocol::{A2AMessage, MessageValidator};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A2A 通信网关
pub struct A2AGateway {
    /// 消息队列
    message_queue: DashMap<String, Vec<A2AMessage>>,
    /// 订阅关系 (team_id -> [agent_ids])
    subscriptions: DashMap<String, Vec<String>>,
    /// 实例路由 (instance_id -> ceo_id)
    instance_routes: DashMap<String, String>,
    /// 正在处理的消息
    processing_messages: DashMap<String, A2AMessage>,
}

impl A2AGateway {
    /// 创建新的 A2A 网关
    pub fn new() -> Self {
        Self {
            message_queue: DashMap::new(),
            subscriptions: DashMap::new(),
            instance_routes: DashMap::new(),
            processing_messages: DashMap::new(),
        }
    }

    /// 发送消息
    pub async fn send(&self, message: A2AMessage) -> Result<String, Box<dyn std::error::Error>> {
        // 1. 消息验证
        MessageValidator::validate(&message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        // 2. 路由消息
        self.route_message(&message).await?;

        // 3. 返回消息 ID
        Ok(message.message_id)
    }

    /// 路由消息到适当的接收者
    async fn route_message(&self, message: &A2AMessage) -> Result<(), Box<dyn std::error::Error>> {
        // 这里是简化的路由逻辑，实际实现会更复杂
        // 根据接收者类型决定路由方式：
        // - Agent ID: 直接发送到指定 Agent
        // - Team ID: 发送到团队中的所有成员
        // - Instance ID: 发送到实例的 CEO
        
        // 暂时只是将消息放入队列，等待后续处理
        let mut recipient_queue = self.message_queue.entry(message.recipient_id.clone())
            .or_insert_with(Vec::new);
        
        recipient_queue.push(message.clone());
        
        Ok(())
    }

    /// 注册团队订阅关系
    pub fn register_team_subscription(&self, team_id: String, agent_ids: Vec<String>) {
        self.subscriptions.insert(team_id, agent_ids);
    }

    /// 注册实例路由
    pub fn register_instance_route(&self, instance_id: String, ceo_id: String) {
        self.instance_routes.insert(instance_id, ceo_id);
    }

    /// 获取指定接收者的消息队列
    pub fn get_message_queue(&self, recipient_id: &str) -> Vec<A2AMessage> {
        self.message_queue
            .get(recipient_id)
            .map(|queue| queue.value().clone())
            .unwrap_or_default()
    }

    /// 清空指定接收者的消息队列
    pub fn clear_message_queue(&self, recipient_id: &str) -> Vec<A2AMessage> {
        self.message_queue
            .remove(recipient_id)
            .map(|(_, queue)| queue)
            .unwrap_or_default()
    }
}

impl Default for A2AGateway {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a::protocol::{A2AMessageBuilder, A2AMessageType, MessagePriority};
    use serde_json::json;

    #[tokio::test]
    async fn test_a2a_gateway_creation() {
        let gateway = A2AGateway::new();
        
        assert_eq!(gateway.message_queue.len(), 0);
        assert_eq!(gateway.subscriptions.len(), 0);
    }

    #[tokio::test]
    async fn test_send_message() {
        let gateway = A2AGateway::new();
        
        let message = A2AMessageBuilder::new(
            "sender123".to_string(),
            "recipient456".to_string(),
            A2AMessageType::Notification {
                title: "Test".to_string(),
                body: "Test notification".to_string(),
            }
        )
        .with_content(json!({"data": "test"}))
        .with_priority(MessagePriority::High)
        .build();

        let result = gateway.send(message).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_message_routing() {
        let gateway = A2AGateway::new();
        
        let message = A2AMessageBuilder::new(
            "sender123".to_string(),
            "recipient456".to_string(),
            A2AMessageType::Query {
                question: "Test query".to_string(),
            }
        )
        .build();

        // 直接测试路由功能
        let result = gateway.route_message(&message).await;
        assert!(result.is_ok());
        
        // 检查消息是否被添加到队列
        let queue = gateway.get_message_queue("recipient456");
        assert_eq!(queue.len(), 1);
        assert_eq!(queue[0].sender_id, "sender123");
    }

    #[tokio::test]
    async fn test_team_subscription() {
        let gateway = A2AGateway::new();
        
        let team_id = "team_alpha".to_string();
        let agent_ids = vec!["agent1".to_string(), "agent2".to_string()];
        
        gateway.register_team_subscription(team_id.clone(), agent_ids.clone());
        
        let stored_agents = gateway.subscriptions.get(&team_id).unwrap();
        assert_eq!(stored_agents.value().len(), 2);
        assert_eq!(stored_agents.value()[0], "agent1");
    }
}