//! A2A (Agent-to-Agent) 通信协议定义
//! 定义 MultiClaw 中 Agent 间通信的标准消息格式和协议

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// A2A 消息 (标准化协议)
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct A2AMessage {
    /// 消息唯一 ID
    pub message_id: String,
    /// 发送者 Agent ID
    pub sender_id: String,
    /// 发送者团队 ID (可选)
    pub sender_team_id: Option<String>,
    /// 发送者实例 ID (可选)
    pub sender_instance_id: Option<String>,
    /// 接收者 Agent ID (单播) 或团队 ID (组播) 或实例 ID (跨实例)
    pub recipient_id: String,
    /// 消息类型
    pub message_type: A2AMessageType,
    /// 消息内容
    pub content: Value,
    /// 优先级
    pub priority: MessagePriority,
    /// 时间戳
    pub timestamp: i64,
    /// 关联任务 ID (可选)
    pub related_task_id: Option<String>,
    /// 需要回复 (可选)
    pub requires_reply: bool,
    /// 超时时间 (可选，秒)
    pub timeout_secs: Option<u64>,
}

/// 消息类型
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum A2AMessageType {
    /// 查询 (请求信息)
    Query { question: String },
    /// 通知 (单向告知)
    Notification { title: String, body: String },
    /// 请求协作 (需要对方行动)
    CollaborationRequest {
        description: String,
        expected_outcome: String,
        deadline: Option<i64>,
    },
    /// 共享知识 (知识传递)
    KnowledgeShare {
        knowledge_type: String,
        content: String,
        applicable_scenarios: Vec<String>,
    },
    /// 响应 (回复查询/请求)
    Response {
        in_reply_to: String,
        content: String,
        success: bool,
    },
    /// 错误 (通信失败)
    Error {
        in_reply_to: String,
        error_code: String,
        error_message: String,
    },
}

/// 消息优先级
#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MessagePriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Urgent = 3,
}

/// A2A 消息优先级辅助方法
impl MessagePriority {
    pub fn as_u8(&self) -> u8 {
        match self {
            MessagePriority::Low => 0,
            MessagePriority::Normal => 1,
            MessagePriority::High => 2,
            MessagePriority::Urgent => 3,
        }
    }
}

/// 消息验证器
pub struct MessageValidator;

impl MessageValidator {
    /// 验证消息是否符合协议规范
    pub fn validate(message: &A2AMessage) -> Result<(), String> {
        if message.message_id.is_empty() {
            return Err("Message ID cannot be empty".to_string());
        }
        
        if message.sender_id.is_empty() {
            return Err("Sender ID cannot be empty".to_string());
        }
        
        if message.recipient_id.is_empty() {
            return Err("Recipient ID cannot be empty".to_string());
        }
        
        if message.timestamp == 0 {
            return Err("Timestamp cannot be zero".to_string());
        }
        
        // 验证消息长度限制
        let content_str = serde_json::to_string(&message.content)
            .map_err(|e| format!("Failed to serialize content: {}", e))?;
        
        if content_str.len() > 1024 * 100 { // 100KB 限制
            return Err("Message content too large (> 100KB)".to_string());
        }
        
        Ok(())
    }
}

/// A2A 消息构建器
pub struct A2AMessageBuilder {
    message: A2AMessage,
}

impl A2AMessageBuilder {
    pub fn new(sender_id: String, recipient_id: String, message_type: A2AMessageType) -> Self {
        Self {
            message: A2AMessage {
                message_id: Uuid::new_v4().to_string(),
                sender_id,
                sender_team_id: None,
                sender_instance_id: None,
                recipient_id,
                message_type,
                content: Value::Null,
                priority: MessagePriority::Normal,
                timestamp: Utc::now().timestamp(),
                related_task_id: None,
                requires_reply: false,
                timeout_secs: None,
            },
        }
    }
    
    pub fn with_content(mut self, content: Value) -> Self {
        self.message.content = content;
        self
    }
    
    pub fn with_priority(mut self, priority: MessagePriority) -> Self {
        self.message.priority = priority;
        self
    }
    
    pub fn with_team_id(mut self, team_id: Option<String>) -> Self {
        self.message.sender_team_id = team_id;
        self
    }
    
    pub fn with_instance_id(mut self, instance_id: Option<String>) -> Self {
        self.message.sender_instance_id = instance_id;
        self
    }
    
    pub fn with_related_task_id(mut self, task_id: Option<String>) -> Self {
        self.message.related_task_id = task_id;
        self
    }
    
    pub fn requires_reply(mut self, requires: bool) -> Self {
        self.message.requires_reply = requires;
        self
    }
    
    pub fn with_timeout(mut self, timeout_secs: Option<u64>) -> Self {
        self.message.timeout_secs = timeout_secs;
        self
    }
    
    pub fn build(self) -> A2AMessage {
        self.message
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_message_validation() {
        let message = A2AMessageBuilder::new(
            "sender123".to_string(),
            "recipient456".to_string(),
            A2AMessageType::Notification {
                title: "Test".to_string(),
                body: "Test body".to_string(),
            }
        )
        .with_content(json!({"key": "value"}))
        .build();

        assert!(MessageValidator::validate(&message).is_ok());
    }

    #[test]
    fn test_message_builder() {
        let message = A2AMessageBuilder::new(
            "sender123".to_string(),
            "recipient456".to_string(),
            A2AMessageType::Query {
                question: "What is the meaning of life?".to_string(),
            }
        )
        .with_content(json!({"additional_data": "some_value"}))
        .with_priority(MessagePriority::High)
        .requires_reply(true)
        .with_timeout(Some(300))
        .build();

        assert_eq!(message.sender_id, "sender123");
        assert_eq!(message.recipient_id, "recipient456");
        assert_eq!(message.priority, MessagePriority::High);
        assert!(message.requires_reply);
        assert_eq!(message.timeout_secs, Some(300));
    }

    #[test]
    fn test_invalid_message() {
        let invalid_message = A2AMessage {
            message_id: "".to_string(), // Empty ID
            sender_id: "".to_string(),  // Empty sender
            sender_team_id: None,
            sender_instance_id: None,
            recipient_id: "".to_string(), // Empty recipient
            message_type: A2AMessageType::Notification {
                title: "Test".to_string(),
                body: "Test".to_string(),
            },
            content: Value::Null,
            priority: MessagePriority::Normal,
            timestamp: 0, // Zero timestamp
            related_task_id: None,
            requires_reply: false,
            timeout_secs: None,
        };

        assert!(MessageValidator::validate(&invalid_message).is_err());
    }
}