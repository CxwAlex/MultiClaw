// src/agent/chairman_config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanConfig {
    /// 董事长名称
    pub name: String,
    /// 董事长个性描述
    pub personality: String,
    /// 默认通信渠道
    pub default_channel: String,
    /// 系统管理模式
    pub system_mode: SystemMode,
    /// 通知设置
    pub notifications: NotificationSettings,
    /// 审批设置
    pub approvals: ApprovalSettings,
    /// 资源管理设置
    pub resource_management: ResourceManagerSettings,
    /// 技能配置
    pub skills: ChairmanSkillsConfig,
    /// 安全设置
    pub security: ChairmanSecurityConfig,
    /// 可观测性设置
    pub observability: ObservabilitySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMode {
    /// 完全自动模式（最小干预）
    FullyAutomated,
    /// 半自动模式（重要决策需要确认）
    SemiAutomated,
    /// 手动模式（所有操作都需要确认）
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 重要事件通知
    pub important_events: bool,
    /// 资源预警通知
    pub resource_warnings: bool,
    /// 系统异常通知
    pub system_alerts: bool,
    /// 每日摘要通知
    pub daily_summary: bool,
    /// 每周报告通知
    pub weekly_report: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSettings {
    /// 资源配额变更阈值（超过此值需要手动审批）
    pub resource_quota_threshold: u64,
    /// 实例创建阈值（超过此值需要手动审批）
    pub instance_creation_threshold: u32,
    /// 跨实例通信阈值
    pub cross_instance_threshold: u32,
    /// 自动审批时间窗口（小时）
    pub auto_approval_window_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagerSettings {
    /// 全局资源配额
    pub global_resource_quota: GlobalResourceQuota,
    /// 实例资源分配策略
    pub allocation_strategy: AllocationStrategy,
    /// 资源监控频率（秒）
    pub monitoring_frequency_sec: u64,
    /// 资源回收策略
    pub recycling_policy: RecyclingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalResourceQuota {
    pub total_tokens_per_minute: u64,
    pub total_max_concurrent_agents: u32,
    pub total_storage_limit_mb: u64,
    pub max_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// 公平分配
    Fair,
    /// 优先级分配
    PriorityBased,
    /// 需求驱动分配
    DemandDriven,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecyclingPolicy {
    /// 不回收
    None,
    /// 轻度回收（仅空闲资源）
    Light,
    /// 中度回收（空闲+低效资源）
    Moderate,
    /// 激进回收（所有可回收资源）
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanSkillsConfig {
    /// 启用的技能
    pub enabled_skills: Vec<String>,
    /// 禁用的技能
    pub disabled_skills: Vec<String>,
    /// 技能执行权限
    pub skill_permissions: HashMap<String, Vec<String>>,
    /// 技能执行限制
    pub skill_limits: HashMap<String, SkillLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLimit {
    /// 每小时最大执行次数
    pub max_executions_per_hour: u32,
    /// 最大并发执行数
    pub max_concurrent_executions: u32,
    /// 最大资源消耗
    pub max_resource_consumption: ResourceConsumption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    pub tokens: u64,
    pub memory_mb: u64,
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanSecurityConfig {
    /// API 密钥管理
    pub api_key_management: ApiKeyManagement,
    /// 访问控制
    pub access_control: AccessControlConfig,
    /// 审计日志
    pub audit_logging: AuditLoggingConfig,
    /// 加密设置
    pub encryption: EncryptionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyManagement {
    /// 密钥轮换周期（天）
    pub rotation_period_days: u32,
    /// 密钥有效期（天）
    pub validity_period_days: u32,
    /// 最大密钥数量
    pub max_keys_per_user: u32,
    /// 密钥作用域限制
    pub scope_restrictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    /// 角色定义
    pub roles: Vec<RoleDefinition>,
    /// 权限矩阵
    pub permissions: HashMap<String, Vec<String>>,
    /// 访问频率限制
    pub rate_limits: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每分钟最大请求数
    pub requests_per_minute: u32,
    /// 每小时最大请求数
    pub requests_per_hour: u32,
    /// IP 级别限制
    pub per_ip_limits: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// 启用审计日志
    pub enabled: bool,
    /// 日志保留天数
    pub retention_days: u32,
    /// 敏感操作日志
    pub sensitive_operations_only: bool,
    /// 日志存储位置
    pub log_location: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// 启用端到端加密
    pub end_to_end_encryption: bool,
    /// 密钥长度
    pub key_length_bits: u32,
    /// 加密算法
    pub algorithm: String,
    /// 密钥管理方式
    pub key_management: KeyManagementType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyManagementType {
    /// 本地管理
    Local,
    /// HSM 硬件安全模块
    Hsm,
    /// 云 KMS 服务
    CloudKms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilitySettings {
    /// 启用指标收集
    pub metrics_enabled: bool,
    /// 指标收集间隔（秒）
    pub metrics_interval_sec: u64,
    /// 启用分布式追踪
    pub tracing_enabled: bool,
    /// 追踪采样率
    pub tracing_sample_rate: f64,
    /// 启用健康检查
    pub health_checks_enabled: bool,
    /// 健康检查间隔（秒）
    pub health_check_interval_sec: u64,
    /// 仪表板配置
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// 仪表板主题
    pub theme: String,
    /// 显示的语言
    pub language: String,
    /// 默认视图
    pub default_view: DashboardView,
    /// 刷新间隔（秒）
    pub refresh_interval_sec: u64,
    /// 启用实时更新
    pub real_time_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardView {
    /// 用户视图（L5）
    UserView,
    /// 董事长视图（L4）
    ChairmanView,
    /// CEO 视图（L3）
    CeoView,
    /// 团队视图（L2）
    TeamView,
    /// Agent 视图（L1）
    AgentView,
}

impl ChairmanConfig {
    /// 创建默认的董事长配置
    pub fn default() -> Self {
        Self {
            name: "Global Chairman".to_string(),
            personality: "Strategic and oversight-focused AI assistant managing multiple MultiClaw instances".to_string(),
            default_channel: "cli".to_string(),
            system_mode: SystemMode::SemiAutomated,
            notifications: NotificationSettings {
                important_events: true,
                resource_warnings: true,
                system_alerts: true,
                daily_summary: true,
                weekly_report: false,
            },
            approvals: ApprovalSettings {
                resource_quota_threshold: 500_000,
                instance_creation_threshold: 5,
                cross_instance_threshold: 3,
                auto_approval_window_hours: 24,
            },
            resource_management: ResourceManagerSettings {
                global_resource_quota: GlobalResourceQuota {
                    total_tokens_per_minute: 1_000_000,
                    total_max_concurrent_agents: 100,
                    total_storage_limit_mb: 10_000,
                    max_instances: 10,
                },
                allocation_strategy: AllocationStrategy::Fair,
                monitoring_frequency_sec: 60,
                recycling_policy: RecyclingPolicy::Moderate,
            },
            skills: ChairmanSkillsConfig {
                enabled_skills: vec![
                    "create_company".to_string(),
                    "company_creation_guide".to_string(),
                    "resource_allocation".to_string(),
                    "instance_monitoring".to_string(),
                    "cross_instance_communication".to_string(),
                ],
                disabled_skills: vec![
                    "direct_memory_access".to_string(),  // 董事长不应直接访问实例内存
                ],
                skill_permissions: HashMap::from([
                    ("create_company".to_string(), vec!["admin".to_string(), "executive".to_string()]),
                    ("resource_allocation".to_string(), vec!["admin".to_string()]),
                ]),
                skill_limits: HashMap::from([
                    ("create_company".to_string(), SkillLimit {
                        max_executions_per_hour: 10,
                        max_concurrent_executions: 2,
                        max_resource_consumption: ResourceConsumption {
                            tokens: 10_000,
                            memory_mb: 100,
                            duration_minutes: 5,
                        },
                    }),
                ]),
            },
            security: ChairmanSecurityConfig {
                api_key_management: ApiKeyManagement {
                    rotation_period_days: 30,
                    validity_period_days: 90,
                    max_keys_per_user: 5,
                    scope_restrictions: vec!["instance_management".to_string(), "resource_allocation".to_string()],
                },
                access_control: AccessControlConfig {
                    roles: vec![
                        RoleDefinition {
                            name: "super_admin".to_string(),
                            description: "超级管理员，拥有所有权限".to_string(),
                            permissions: vec!["*".to_string()],
                        },
                        RoleDefinition {
                            name: "instance_admin".to_string(),
                            description: "实例管理员，可以管理实例".to_string(),
                            permissions: vec![
                                "instance:create".to_string(),
                                "instance:start".to_string(),
                                "instance:stop".to_string(),
                                "instance:delete".to_string(),
                            ],
                        },
                        RoleDefinition {
                            name: "resource_manager".to_string(),
                            description: "资源管理员，可以分配资源".to_string(),
                            permissions: vec![
                                "resource:allocate".to_string(),
                                "resource:view".to_string(),
                            ],
                        },
                    ],
                    permissions: HashMap::new(),
                    rate_limits: RateLimitConfig {
                        requests_per_minute: 60,
                        requests_per_hour: 1000,
                        per_ip_limits: true,
                    },
                },
                audit_logging: AuditLoggingConfig {
                    enabled: true,
                    retention_days: 90,
                    sensitive_operations_only: false,
                    log_location: PathBuf::from("./logs/audit.log"),
                },
                encryption: EncryptionConfig {
                    end_to_end_encryption: true,
                    key_length_bits: 256,
                    algorithm: "AES-256-GCM".to_string(),
                    key_management: KeyManagementType::Local,
                },
            },
            observability: ObservabilitySettings {
                metrics_enabled: true,
                metrics_interval_sec: 30,
                tracing_enabled: true,
                tracing_sample_rate: 1.0,
                health_checks_enabled: true,
                health_check_interval_sec: 60,
                dashboard: DashboardConfig {
                    theme: "dark".to_string(),
                    language: "zh-CN".to_string(),
                    default_view: DashboardView::ChairmanView,
                    refresh_interval_sec: 10,
                    real_time_updates: true,
                },
            },
        }
    }

    /// 从文件加载配置
    pub async fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        use tokio::fs;
        
        let content = fs::read_to_string(path).await?;
        let config: ChairmanConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::fs;
        
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }
}

// 董事长 Agent 的专用提示词模板
pub const CHAIRMAN_AGENT_PROMPTS: &str = r#"
# 董事长 Agent 系统提示

## 角色定义
你是 MultiClaw 系统的董事长 Agent，用户的 AI 分身，负责统一管理所有 MultiClaw 实例。

## 核心职责
1. 管理所有 MultiClaw 实例（公司）
2. 监控全局资源使用情况
3. 审批重要决策（超阈值操作）
4. 协调跨实例通信
5. 维护系统整体健康

## 行为准则
- 始终从全局视角考虑问题
- 优先保护系统稳定性和安全性
- 在必要时寻求用户确认
- 提供清晰的状态报告

## 交互流程

### 实例创建
当用户希望创建新公司/实例时：
1. 询问公司名称和类型
2. 了解资源需求（token配额、agent数量等）
3. 检查全局资源是否充足
4. 使用 create_company 技能创建实例
5. 配置相应的通信渠道

### 资源管理
- 监控全局资源使用情况
- 在资源接近阈值时发出警告
- 根据需要重新分配资源

### 决策审批
对于以下操作需要寻求用户确认：
- 创建超过阈值的新实例
- 分配超过阈值的资源
- 跨实例敏感数据共享

## 可用技能
- create_company: 创建新公司实例
- company_creation_guide: 交互式创建引导
- resource_allocation: 分配和管理资源
- instance_monitoring: 监控实例状态
- cross_instance_communication: 管理跨实例通信

## 回复格式
保持专业、简洁、信息丰富：
- 重要操作前先解释
- 操作完成后报告结果
- 定期提供状态摘要

请始终记住：你是用户在 MultiClaw 系统中的代表，需要平衡效率与安全性。
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_chairman_config() {
        let config = ChairmanConfig::default();
        
        // 测试基本属性
        assert_eq!(config.name, "Global Chairman");
        assert_eq!(config.system_mode, SystemMode::SemiAutomated);
        assert!(config.notifications.important_events);
        assert_eq!(config.skills.enabled_skills.len(), 5);
        
        // 测试保存和加载
        let temp_path = PathBuf::from("/tmp/test_chairman_config.toml");
        config.save_to_file(&temp_path).await.unwrap();
        
        let loaded_config = ChairmanConfig::from_file(&temp_path).await.unwrap();
        assert_eq!(loaded_config.name, config.name);
        
        // 清理测试文件
        let _ = tokio::fs::remove_file(temp_path).await;
    }
}