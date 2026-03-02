//! 董事长 Agent 模块
//! 用户的 AI 分身，统一管理所有 MultiClaw 实例

use crate::a2a::A2AGateway;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

/// 董事长 Agent - 用户个人分身
pub struct ChairmanAgent {
    /// 用户 ID
    pub user_id: String,
    /// 绑定用户终端（主入口）
    pub user_channel: String,
    /// 管理的所有实例
    pub instances: DashMap<String, InstanceHandle>,
    /// 全局资源池
    pub global_resource: Arc<GlobalResourceManager>,
    /// 信息聚合器
    pub aggregator: Arc<InformationAggregator>,
    /// 决策过滤器（过滤噪音）
    pub decision_filter: DecisionFilter,
    /// A2A 网关（跨实例通信）
    pub a2a_gateway: Arc<A2AGateway>,
}

/// 实例句柄
#[derive(Clone, Serialize, Deserialize)]
pub struct InstanceHandle {
    /// 实例 ID
    pub id: String,
    /// 实例名称
    pub name: String,
    /// 实例类型
    pub instance_type: InstanceType,
    /// CEO Agent ID
    pub ceo_agent_id: String,
    /// CEO 绑定的独立通信通道（可选）
    pub ceo_channel: Option<String>,
    /// 实例状态
    pub status: InstanceStatus,
    /// 资源配额
    pub quota: ResourceQuota,
    /// 当前项目数
    pub active_projects: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InstanceType {
    MarketResearch,
    ProductDevelopment,
    CustomerService,
    DataAnalysis,
    General,
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InstanceStatus {
    Initializing,
    Running,
    Idle,
    Busy,
    Unhealthy,
    Recovering,    // 恢复中 (新增)
    RecoveryFailed, // 恢复失败 (新增)
    Stopped,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub tokens_per_minute: u32,
    pub max_concurrent_agents: u32,
    pub storage_limit_mb: u32,
    pub api_calls_per_minute: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CEOConfig {
    pub model_preference: String,
    pub personality: String,
    pub resource_limits: ResourceQuota,
}

#[derive(Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub instance_type: InstanceType,
    pub quota: ResourceQuota,
    pub ceo_config: CEOConfig,
    /// CEO 绑定的独立通信通道（可选）
    pub ceo_channel: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct QuotaIncreaseRequest {
    pub reason: String,
    pub current_quota: ResourceQuota,
    pub new_quota: ResourceQuota,
}

#[derive(Serialize, Deserialize)]
pub struct GlobalStatus {
    pub total_instances: usize,
    pub running_instances: usize,
    pub busy_instances: usize,
    pub total_projects: usize,
    pub global_resource_usage: ResourceUsage,
    pub instances: Vec<InstanceHandle>,
}

#[derive(Serialize, Deserialize)]
pub struct ResourceUsage {
    pub tokens_used: u32,
    pub tokens_remaining: u32,
    pub concurrent_agents: u32,
    pub storage_used_mb: u32,
    pub storage_remaining_mb: u32,
}

#[derive(Serialize, Deserialize)]
pub struct QuickCreateRequest {
    pub instance_name: String,
    pub instance_type: InstanceType,
    pub task_description: String,
    pub team_goal: String,
    pub complexity: u8,
    pub quota: ResourceQuota,
    pub ceo_config: CEOConfig,
    pub ceo_channel: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct QuickCreateResult {
    pub instance_id: String,
    pub team_id: String,
    pub message: String,
}

pub struct GlobalResourceManager {
    global_token_quota: std::sync::atomic::AtomicUsize,
    global_token_used: std::sync::atomic::AtomicUsize,
    max_instances: std::sync::atomic::AtomicUsize,
    current_instances: std::sync::atomic::AtomicUsize,
}

pub struct InformationAggregator;

pub struct DecisionFilter;

pub enum MajorDecision {
    CreateInstance(CreateInstanceRequest),
    IncreaseGlobalQuota(QuotaIncreaseRequest),
    ShutdownInstance(String),
    MergeInstances { from: String, to: String },
    CrossInstanceCollaboration { from: String, to: String, purpose: String },
}

pub enum DecisionResult {
    Approved { message: String },
    Rejected { reason: String },
}

impl ChairmanAgent {
    /// 启动时自动创建
    pub async fn initialize(user_id: String, user_channel: String) -> Result<Self, Box<dyn std::error::Error>> {
        let chairman = Self {
            user_id,
            user_channel,
            instances: DashMap::new(),
            global_resource: Arc::new(GlobalResourceManager::new()),
            aggregator: Arc::new(InformationAggregator::new()),
            decision_filter: DecisionFilter::default(),
            a2a_gateway: Arc::new(A2AGateway::new()),
        };

        // 加载现有实例（如果有）
        chairman.load_existing_instances().await?;
        
        Ok(chairman)
    }

    /// 创建新实例（分公司）
    pub async fn create_instance(
        &self,
        request: &CreateInstanceRequest,
    ) -> Result<InstanceHandle, Box<dyn std::error::Error>> {
        // 1. 检查全局资源
        if !self.global_resource.can_allocate(&request.quota) {
            return Err("全局资源不足，请先释放已有实例或申请增加配额".into());
        }

        // 2. 创建实例
        let instance = InstanceHandle {
            id: Uuid::new_v4().to_string(),
            name: request.name.clone(),
            instance_type: request.instance_type,
            ceo_agent_id: String::new(),
            ceo_channel: request.ceo_channel.clone(), // CEO 独立通信通道
            status: InstanceStatus::Initializing,
            quota: request.quota.clone(),
            active_projects: 0,
            created_at: Utc::now(),
            last_active_at: Utc::now(),
        };

        // 3. 分配全局资源
        self.global_resource.allocate(&request.quota).await?;

        // 4. 创建 CEO Agent (这里简化，实际需要与具体的实例创建逻辑集成)
        // let ceo = self.create_ceo_agent(&instance, request.ceo_config.clone()).await?;
        let mut instance = instance;
        instance.ceo_agent_id = format!("ceo_{}", instance.id); // 简化实现
        instance.status = InstanceStatus::Running;

        // 5. 注册实例
        self.instances.insert(instance.id.clone(), instance.clone());

        // 6. 通知用户
        self.notify_user(&format!(
            "✅ 已创建新实例「{}」(类型：{:?})\n初始资源：{:?}\nCEO 已就绪{}",
            instance.name,
            instance.instance_type,
            instance.quota,
            instance.ceo_channel.as_ref()
                .map(|c| format!("\n独立通信：{}", c))
                .unwrap_or_default()
        )).await?;

        Ok(instance)
    }

    /// 汇总关键信息（定时任务）
    pub async fn aggregate_and_sync(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut summaries = Vec::new();
        for entry in self.instances.iter() {
            let instance = entry.value();
            let summary = self.fetch_instance_summary(instance).await?;
            summaries.push(summary);
        }

        let aggregated = self.aggregator.aggregate(summaries).await?;
        let filtered = self.decision_filter.filter(aggregated);

        if !filtered.is_empty() {
            self.sync_to_user(&filtered).await?;
        }

        Ok(())
    }

    /// 审批重大决策
    pub async fn review_major_decision(
        &self,
        decision: &MajorDecision,
    ) -> Result<DecisionResult, Box<dyn std::error::Error>> {
        match decision {
            MajorDecision::CreateInstance(request) => {
                let instance = self.create_instance(request).await?;
                Ok(DecisionResult::Approved {
                    message: format!("实例「{}」已创建", instance.name),
                })
            }
            MajorDecision::IncreaseGlobalQuota(request) => {
                self.request_user_confirmation(&format!(
                    "申请增加全局资源配额：{}\n当前配额：{:?}\n新配额：{:?}",
                    request.reason,
                    request.current_quota,
                    request.new_quota
                )).await?;
                Ok(DecisionResult::Approved { message: "配额已增加".to_string() })
            }
            MajorDecision::ShutdownInstance(instance_id) => {
                self.shutdown_instance(instance_id).await?;
                Ok(DecisionResult::Approved { message: "实例已关闭".to_string() })
            }
            MajorDecision::MergeInstances { from, to } => {
                self.merge_instances(from, to).await?;
                Ok(DecisionResult::Approved { message: "实例已合并".to_string() })
            }
            MajorDecision::CrossInstanceCollaboration { from, to, purpose } => {
                // 跨实例协作审批
                self.approve_cross_instance_collaboration(from, to, purpose).await?;
                Ok(DecisionResult::Approved { message: "跨实例协作已批准".to_string() })
            }
        }
    }

    /// 查询全局状态
    pub fn get_global_status(&self) -> GlobalStatus {
        let instances: Vec<_> = self.instances.iter().map(|e| e.value().clone()).collect();

        GlobalStatus {
            total_instances: instances.len(),
            running_instances: instances.iter()
                .filter(|i| i.status == InstanceStatus::Running)
                .count(),
            busy_instances: instances.iter()
                .filter(|i| i.status == InstanceStatus::Busy)
                .count(),
            total_projects: instances.iter().map(|i| i.active_projects).sum(),
            global_resource_usage: self.global_resource.get_usage(),
            instances,
        }
    }

    /// 快速创建公司 - 团队入口
    pub async fn quick_create(
        &self,
        request: &QuickCreateRequest,
    ) -> Result<QuickCreateResult, Box<dyn std::error::Error>> {
        let instance = if let Some(existing) = self.get_instance_by_name(&request.instance_name) {
            existing
        } else {
            self.create_instance(&CreateInstanceRequest {
                name: request.instance_name.clone(),
                instance_type: request.instance_type,
                quota: request.quota.clone(),
                ceo_config: request.ceo_config.clone(),
                ceo_channel: request.ceo_channel.clone(),
            }).await?
        };

        // 这里简化实现，实际需要调用 CEO 的技能
        let team_id = format!("team_{}", Uuid::new_v4());
        
        Ok(QuickCreateResult {
            instance_id: instance.id,
            team_id,
            message: format!(
                "✅ 已创建「{}」实例和「{}」团队\n目标：{}\n资源：{:?}",
                instance.name,
                "DefaultTeam", // 简化实现
                request.team_goal,
                instance.quota
            ),
        })
    }

    /// 双通道通信：用户可直接联系 CEO
    pub async fn forward_to_ceo(
        &self,
        instance_id: &str,
        message: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let instance = self.instances.get(instance_id)
            .ok_or("实例不存在")?;

        // 通过 A2A 网关发送消息到 CEO
        use crate::a2a::{A2AMessageBuilder, A2AMessageType, MessagePriority};
        use serde_json::json;

        let a2a_message = A2AMessageBuilder::new(
            "user".to_string(),
            instance.ceo_agent_id.clone(),
            A2AMessageType::Notification {
                title: "用户消息".to_string(),
                body: message.to_string(),
            }
        )
        .with_priority(MessagePriority::High)
        .requires_reply(true)
        .with_timeout(Some(300))
        .build();

        let result = self.a2a_gateway.send(a2a_message).await?;
        Ok(result)
    }

    // 辅助方法
    async fn load_existing_instances(&self) -> Result<(), Box<dyn std::error::Error>> {
        // 这里应该从持久化存储加载现有实例
        // 简化实现：暂时不加载任何实例
        Ok(())
    }

    async fn notify_user(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 这里应该通过用户绑定的渠道发送通知
        println!("Chairman Agent 通知用户: {}", message);
        Ok(())
    }

    async fn sync_to_user(&self, filtered_info: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        // 同步过滤后的信息给用户
        for info in filtered_info {
            self.notify_user(info).await?;
        }
        Ok(())
    }

    async fn fetch_instance_summary(&self, instance: &InstanceHandle) -> Result<String, Box<dyn std::error::Error>> {
        // 获取实例摘要信息
        Ok(format!("实例 {}: 状态={:?}, 项目数={}, 最后活跃={}", 
            instance.name, 
            instance.status, 
            instance.active_projects,
            instance.last_active_at
        ))
    }

    fn get_instance_by_name(&self, name: &str) -> Option<InstanceHandle> {
        for entry in self.instances.iter() {
            if entry.value().name == name {
                return Some(entry.value().clone());
            }
        }
        None
    }

    async fn shutdown_instance(&self, instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(mut instance) = self.instances.get_mut(instance_id) {
            instance.status = InstanceStatus::Stopped;
            // 实际实现中还需要清理资源
        }
        Ok(())
    }

    async fn merge_instances(&self, from: &str, to: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 合并实例的逻辑
        // 实际实现会更复杂
        println!("合并实例 {} 到 {}", from, to);
        Ok(())
    }

    async fn approve_cross_instance_collaboration(&self, from: &str, to: &str, purpose: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 跨实例协作审批
        println!("批准跨实例协作: {} -> {}, 目的: {}", from, to, purpose);
        Ok(())
    }

    async fn request_user_confirmation(&self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 请求用户确认
        println!("请求用户确认: {}", message);
        Ok(())
    }
}

impl GlobalResourceManager {
    pub fn new() -> Self {
        Self {
            global_token_quota: std::sync::atomic::AtomicUsize::new(1000000), // 1M tokens
            global_token_used: std::sync::atomic::AtomicUsize::new(0),
            max_instances: std::sync::atomic::AtomicUsize::new(10),
            current_instances: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn can_allocate(&self, quota: &ResourceQuota) -> bool {
        let current_tokens = self.global_token_used.load(std::sync::atomic::Ordering::Relaxed);
        let total_tokens = self.global_token_quota.load(std::sync::atomic::Ordering::Relaxed);
        current_tokens + (quota.tokens_per_minute as usize) <= total_tokens
    }

    pub async fn allocate(&self, quota: &ResourceQuota) -> Result<(), Box<dyn std::error::Error>> {
        // 更新全局资源使用情况
        self.global_token_used.fetch_add(quota.tokens_per_minute as usize, std::sync::atomic::Ordering::Relaxed);
        self.current_instances.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    pub fn get_usage(&self) -> ResourceUsage {
        ResourceUsage {
            tokens_used: self.global_token_used.load(std::sync::atomic::Ordering::Relaxed) as u32,
            tokens_remaining: (self.global_token_quota.load(std::sync::atomic::Ordering::Relaxed) - 
                              self.global_token_used.load(std::sync::atomic::Ordering::Relaxed)) as u32,
            concurrent_agents: self.current_instances.load(std::sync::atomic::Ordering::Relaxed) as u32,
            storage_used_mb: 0, // 简化实现
            storage_remaining_mb: 0, // 简化实现
        }
    }
}

impl InformationAggregator {
    pub fn new() -> Self {
        Self
    }

    pub async fn aggregate(&self, summaries: Vec<String>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        // 简化实现：直接返回摘要
        Ok(summaries)
    }
}

impl DecisionFilter {
    pub fn filter(&self, aggregated_info: Vec<String>) -> Vec<String> {
        // 过滤噪音，只返回重要信息
        // 简化实现：返回所有信息
        aggregated_info
    }
}

impl Default for DecisionFilter {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_chairman_agent_creation() {
        let chairman = ChairmanAgent::initialize("user123".to_string(), "telegram_abc".to_string()).await.unwrap();
        
        assert_eq!(chairman.user_id, "user123");
        assert_eq!(chairman.user_channel, "telegram_abc");
        assert_eq!(chairman.instances.len(), 0);
    }

    #[tokio::test]
    async fn test_instance_creation() {
        let chairman = ChairmanAgent::initialize("user123".to_string(), "telegram_abc".to_string()).await.unwrap();
        
        let request = CreateInstanceRequest {
            name: "Test Instance".to_string(),
            instance_type: InstanceType::General,
            quota: ResourceQuota {
                tokens_per_minute: 1000,
                max_concurrent_agents: 10,
                storage_limit_mb: 100,
                api_calls_per_minute: 100,
            },
            ceo_config: CEOConfig {
                model_preference: "gpt-4".to_string(),
                personality: "analytical".to_string(),
                resource_limits: ResourceQuota {
                    tokens_per_minute: 1000,
                    max_concurrent_agents: 10,
                    storage_limit_mb: 100,
                    api_calls_per_minute: 100,
                },
            },
            ceo_channel: Some("discord_test".to_string()),
        };

        let result = chairman.create_instance(&request).await;
        assert!(result.is_ok());
        
        let instance = result.unwrap();
        assert_eq!(instance.name, "Test Instance");
        assert_eq!(instance.instance_type, InstanceType::General);
        assert_eq!(chairman.instances.len(), 1);
    }

    #[tokio::test]
    async fn test_global_status() {
        let chairman = ChairmanAgent::initialize("user123".to_string(), "telegram_abc".to_string()).await.unwrap();
        
        let status = chairman.get_global_status();
        assert_eq!(status.total_instances, 0);
        assert_eq!(status.running_instances, 0);
        
        // 创建一个实例
        let request = CreateInstanceRequest {
            name: "Test Instance".to_string(),
            instance_type: InstanceType::General,
            quota: ResourceQuota {
                tokens_per_minute: 1000,
                max_concurrent_agents: 10,
                storage_limit_mb: 100,
                api_calls_per_minute: 100,
            },
            ceo_config: CEOConfig {
                model_preference: "gpt-4".to_string(),
                personality: "analytical".to_string(),
                resource_limits: ResourceQuota {
                    tokens_per_minute: 1000,
                    max_concurrent_agents: 10,
                    storage_limit_mb: 100,
                    api_calls_per_minute: 100,
                },
            },
            ceo_channel: Some("discord_test".to_string()),
        };
        
        chairman.create_instance(&request).await.unwrap();
        
        let status = chairman.get_global_status();
        assert_eq!(status.total_instances, 1);
        assert_eq!(status.running_instances, 1);
    }

    #[tokio::test]
    async fn test_quick_create() {
        let chairman = ChairmanAgent::initialize("user123".to_string(), "telegram_abc".to_string()).await.unwrap();
        
        let request = QuickCreateRequest {
            instance_name: "Quick Test Instance".to_string(),
            instance_type: InstanceType::MarketResearch,
            task_description: "Market analysis for new product".to_string(),
            team_goal: "Analyze market trends".to_string(),
            complexity: 5,
            quota: ResourceQuota {
                tokens_per_minute: 1000,
                max_concurrent_agents: 10,
                storage_limit_mb: 100,
                api_calls_per_minute: 100,
            },
            ceo_config: CEOConfig {
                model_preference: "gpt-4".to_string(),
                personality: "analytical".to_string(),
                resource_limits: ResourceQuota {
                    tokens_per_minute: 1000,
                    max_concurrent_agents: 10,
                    storage_limit_mb: 100,
                    api_calls_per_minute: 100,
                },
            },
            ceo_channel: Some("telegram_quick".to_string()),
        };

        let result = chairman.quick_create(&request).await;
        assert!(result.is_ok());
        
        let quick_result = result.unwrap();
        assert!(!quick_result.instance_id.is_empty());
        assert!(!quick_result.team_id.is_empty());
        assert!(quick_result.message.contains("已创建"));
    }
}