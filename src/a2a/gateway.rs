//! A2A (Agent-to-Agent) 通信网关
//! 负责在 MultiClaw 的各个 Agent 之间路由消息
//! 
//! 实现四级通信权限验证：
//! - L1: Agent 内部通信
//! - L2: 团队内通信
//! - L3: 跨团队通信
//! - L4: 跨实例通信

use crate::a2a::protocol::{A2AMessage, MessageValidator};
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 通信层级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommunicationLevel {
    /// L1: Agent 内部通信
    Internal,
    /// L2: 团队内通信
    Team,
    /// L3: 跨团队通信
    CrossTeam,
    /// L4: 跨实例通信
    CrossInstance,
}

/// Agent 角色类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentRole {
    /// Worker Agent
    Worker,
    /// 团队负责人
    TeamLead,
    /// CEO
    CEO,
    /// 董事长（用户分身）
    Chairman,
}

/// Agent 注册信息
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent ID
    pub agent_id: String,
    /// 所属团队 ID
    pub team_id: Option<String>,
    /// 所属实例 ID
    pub instance_id: Option<String>,
    /// Agent 角色
    pub role: AgentRole,
    /// 权限列表
    pub permissions: Vec<String>,
}

/// 权限验证结果
#[derive(Debug, Clone)]
pub struct PermissionResult {
    /// 是否允许
    pub allowed: bool,
    /// 通信层级
    pub level: CommunicationLevel,
    /// 拒绝原因（如果拒绝）
    pub denial_reason: Option<String>,
    /// 是否需要审批
    pub requires_approval: bool,
    /// 审批角色（如果需要审批）
    pub approval_role: Option<String>,
}

/// 审计日志条目
#[derive(Debug, Clone)]
pub struct AuditLogEntry {
    /// 日志 ID
    pub id: String,
    /// 消息 ID
    pub message_id: String,
    /// 发送者 ID
    pub sender_id: String,
    /// 接收者 ID
    pub recipient_id: String,
    /// 通信层级
    pub level: CommunicationLevel,
    /// 是否成功
    pub success: bool,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 详情
    pub details: String,
}

/// A2A 通信网关
pub struct A2AGateway {
    /// 消息队列
    message_queue: DashMap<String, Vec<A2AMessage>>,
    /// 订阅关系 (team_id -> [agent_ids])
    subscriptions: DashMap<String, Vec<String>>,
    /// 实例路由 (instance_id -> ceo_id)
    instance_routes: DashMap<String, String>,
    /// Agent 信息注册表
    agent_registry: DashMap<String, AgentInfo>,
    /// 团队到 Agent 的映射
    team_agents: DashMap<String, Vec<String>>,
    /// 实例到团队的映射
    instance_teams: DashMap<String, Vec<String>>,
    /// 审计日志
    audit_log: Arc<RwLock<Vec<AuditLogEntry>>>,
    /// 正在处理的消息
    processing_messages: DashMap<String, A2AMessage>,
    /// 跨实例通信审批队列
    cross_instance_approvals: DashMap<String, A2AMessage>,
}

impl A2AGateway {
    /// 创建新的 A2A 网关
    pub fn new() -> Self {
        Self {
            message_queue: DashMap::new(),
            subscriptions: DashMap::new(),
            instance_routes: DashMap::new(),
            agent_registry: DashMap::new(),
            team_agents: DashMap::new(),
            instance_teams: DashMap::new(),
            audit_log: Arc::new(RwLock::new(Vec::new())),
            processing_messages: DashMap::new(),
            cross_instance_approvals: DashMap::new(),
        }
    }

    /// 注册 Agent
    pub fn register_agent(&self, info: AgentInfo) {
        let agent_id = info.agent_id.clone();
        let team_id = info.team_id.clone();
        let instance_id = info.instance_id.clone();

        // 注册到 Agent 表
        self.agent_registry.insert(agent_id.clone(), info);

        // 更新团队映射
        if let Some(tid) = team_id {
            let mut agents = self.team_agents.entry(tid).or_insert_with(Vec::new);
            if !agents.contains(&agent_id) {
                agents.push(agent_id.clone());
            }
        }
    }

    /// 注册团队到实例
    pub fn register_team_to_instance(&self, team_id: String, instance_id: String) {
        let mut teams = self.instance_teams.entry(instance_id).or_insert_with(Vec::new);
        if !teams.contains(&team_id) {
            teams.push(team_id);
        }
    }

    /// 获取 Agent 信息
    pub fn get_agent_info(&self, agent_id: &str) -> Option<AgentInfo> {
        self.agent_registry.get(agent_id).map(|info| info.clone())
    }

    /// 确定通信层级
    pub fn determine_communication_level(
        &self,
        sender_id: &str,
        recipient_id: &str,
    ) -> CommunicationLevel {
        let sender = match self.agent_registry.get(sender_id) {
            Some(info) => info,
            None => return CommunicationLevel::CrossInstance, // 未知发送者视为跨实例
        };

        let recipient = match self.agent_registry.get(recipient_id) {
            Some(info) => info,
            None => return CommunicationLevel::CrossInstance, // 未知接收者视为跨实例
        };

        // L1: 同一个 Agent
        if sender_id == recipient_id {
            return CommunicationLevel::Internal;
        }

        // L2: 同一个团队
        if sender.team_id.is_some() && sender.team_id == recipient.team_id {
            return CommunicationLevel::Team;
        }

        // L3: 同一个实例
        if sender.instance_id.is_some() && sender.instance_id == recipient.instance_id {
            return CommunicationLevel::CrossTeam;
        }

        // L4: 跨实例
        CommunicationLevel::CrossInstance
    }

    /// 验证通信权限
    pub async fn verify_permission(
        &self,
        message: &A2AMessage,
    ) -> Result<PermissionResult, Box<dyn std::error::Error>> {
        let level = self.determine_communication_level(&message.sender_id, &message.recipient_id);

        let sender = self.agent_registry.get(&message.sender_id);
        let recipient = self.agent_registry.get(&message.recipient_id);

        match level {
            CommunicationLevel::Internal => {
                // L1: Agent 内部通信，始终允许
                Ok(PermissionResult {
                    allowed: true,
                    level,
                    denial_reason: None,
                    requires_approval: false,
                    approval_role: None,
                })
            }
            CommunicationLevel::Team => {
                // L2: 团队内通信
                // Worker 可以向 TeamLead 发送消息
                // TeamLead 可以向团队成员发送消息
                let sender_role = sender.as_ref().map(|s| s.role.clone());
                let recipient_role = recipient.as_ref().map(|r| r.role.clone());

                let allowed = match (sender_role, recipient_role) {
                    (Some(AgentRole::Worker), Some(AgentRole::TeamLead)) => true,
                    (Some(AgentRole::Worker), Some(AgentRole::Worker)) => true,
                    (Some(AgentRole::TeamLead), Some(AgentRole::Worker)) => true,
                    (Some(AgentRole::TeamLead), Some(AgentRole::TeamLead)) => true,
                    (Some(AgentRole::CEO), _) => true, // CEO 可以向任何人发送
                    (_, Some(AgentRole::CEO)) => true, // 任何人可以向 CEO 发送
                    _ => false,
                };

                Ok(PermissionResult {
                    allowed,
                    level,
                    denial_reason: if allowed { None } else { Some("Team communication permission denied".to_string()) },
                    requires_approval: false,
                    approval_role: None,
                })
            }
            CommunicationLevel::CrossTeam => {
                // L3: 跨团队通信
                // 需要 TeamLead 或 CEO 发起
                let sender_role = sender.as_ref().map(|s| s.role.clone());

                let allowed = match sender_role {
                    Some(AgentRole::CEO) => true,
                    Some(AgentRole::Chairman) => true,
                    Some(AgentRole::TeamLead) => true,
                    _ => false,
                };

                Ok(PermissionResult {
                    allowed,
                    level,
                    denial_reason: if allowed { None } else { Some("Cross-team communication requires TeamLead or higher role".to_string()) },
                    requires_approval: !allowed,
                    approval_role: if !allowed { Some("TeamLead".to_string()) } else { None },
                })
            }
            CommunicationLevel::CrossInstance => {
                // L4: 跨实例通信
                // 只有 CEO 和 Chairman 可以发起，且需要审批
                let sender_role = sender.as_ref().map(|s| s.role.clone());

                let (allowed, requires_approval, approval_role) = match sender_role {
                    Some(AgentRole::Chairman) => (true, false, None),
                    Some(AgentRole::CEO) => (true, true, Some("Chairman".to_string())),
                    _ => (false, true, Some("CEO".to_string())),
                };

                Ok(PermissionResult {
                    allowed,
                    level,
                    denial_reason: if allowed { None } else { Some("Cross-instance communication requires CEO or Chairman role".to_string()) },
                    requires_approval,
                    approval_role,
                })
            }
        }
    }

    /// 发送消息
    pub async fn send(&self, message: A2AMessage) -> Result<String, Box<dyn std::error::Error>> {
        // 1. 消息验证
        MessageValidator::validate(&message)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;

        // 2. 权限验证
        let permission = self.verify_permission(&message).await?;
        
        if !permission.allowed {
            // 记录审计日志
            self.log_audit(&message, permission.level, false, "Permission denied").await;
            return Err(format!("Permission denied: {:?}", permission.denial_reason).into());
        }

        if permission.requires_approval {
            // 放入审批队列
            self.cross_instance_approvals.insert(message.message_id.clone(), message.clone());
            
            // 记录审计日志
            self.log_audit(&message, permission.level, false, "Pending approval").await;
            
            return Err(format!("Message requires approval from {:?}", permission.approval_role).into());
        }

        // 3. 路由消息
        self.route_message(&message).await?;

        // 4. 记录审计日志
        self.log_audit(&message, permission.level, true, "Message delivered").await;

        // 5. 返回消息 ID
        Ok(message.message_id)
    }

    /// 记录审计日志
    async fn log_audit(
        &self,
        message: &A2AMessage,
        level: CommunicationLevel,
        success: bool,
        details: &str,
    ) {
        let entry = AuditLogEntry {
            id: Uuid::new_v4().to_string(),
            message_id: message.message_id.clone(),
            sender_id: message.sender_id.clone(),
            recipient_id: message.recipient_id.clone(),
            level,
            success,
            timestamp: Utc::now(),
            details: details.to_string(),
        };

        let mut log = self.audit_log.write().await;
        log.push(entry);
    }

    /// 获取审计日志
    pub async fn get_audit_log(&self) -> Vec<AuditLogEntry> {
        self.audit_log.read().await.clone()
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

        // 注册发送者和接收者（同一团队，允许通信）
        gateway.register_agent(AgentInfo {
            agent_id: "sender123".to_string(),
            team_id: Some("team1".to_string()),
            instance_id: Some("instance1".to_string()),
            role: AgentRole::Worker,
            permissions: vec![],
        });
        
        gateway.register_agent(AgentInfo {
            agent_id: "recipient456".to_string(),
            team_id: Some("team1".to_string()),
            instance_id: Some("instance1".to_string()),
            role: AgentRole::Worker,
            permissions: vec![],
        });

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