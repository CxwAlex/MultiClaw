//! CompanyManager - 公司和团队管理
//! 提供创建公司（实例）和团队的能力

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use super::cluster_state::{ClusterState, ClusterNode, NodeType, NodeStatus, NodeResourceUsage};

/// 公司配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyConfig {
    /// 公司名称
    pub name: String,
    /// 公司类型
    pub company_type: CompanyType,
    /// Token 配额
    pub token_quota: u64,
    /// 最大 Agent 数
    pub max_agents: usize,
    /// CEO 配置
    pub ceo_config: CEOConfig,
    /// 绑定的通信通道
    pub channel: Option<String>,
    /// 标签
    pub labels: std::collections::HashMap<String, String>,
}

/// 公司类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompanyType {
    /// 市场调研
    MarketResearch,
    /// 产品开发
    ProductDevelopment,
    /// 客户服务
    CustomerService,
    /// 数据分析
    DataAnalysis,
    /// 通用型
    General,
    /// 自定义
    Custom,
}

impl std::fmt::Display for CompanyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompanyType::MarketResearch => write!(f, "市场调研"),
            CompanyType::ProductDevelopment => write!(f, "产品开发"),
            CompanyType::CustomerService => write!(f, "客户服务"),
            CompanyType::DataAnalysis => write!(f, "数据分析"),
            CompanyType::General => write!(f, "通用型"),
            CompanyType::Custom => write!(f, "自定义"),
        }
    }
}

/// CEO 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CEOConfig {
    /// CEO 名称
    pub name: Option<String>,
    /// 决策模式
    pub decision_mode: DecisionMode,
    /// 自动审批阈值
    pub auto_approval_threshold: f64,
}

/// 决策模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DecisionMode {
    /// 自动决策
    #[default]
    Automatic,
    /// 半自动（需要确认重要决策）
    SemiAutomatic,
    /// 手动（需要确认所有决策）
    Manual,
}

/// 团队配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    /// 团队名称
    pub name: String,
    /// 团队目标
    pub goal: String,
    /// 团队类型
    pub team_type: TeamType,
    /// 预估复杂度 (1-10)
    pub complexity: u8,
    /// 初始 Agent 数量
    pub initial_agents: usize,
    /// 协作模式
    pub collaboration_mode: CollaborationMode,
}

/// 团队类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamType {
    /// 信息收集
    InformationGathering,
    /// 数据分析
    DataAnalysis,
    /// 内容创作
    ContentCreation,
    /// 开发
    Development,
    /// 研究
    Research,
    /// 支持
    Support,
    /// 通用
    General,
}

/// 协作模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollaborationMode {
    /// 并行执行
    Parallel,
    /// 串行执行
    Sequential,
    /// 层级执行
    Hierarchical,
    /// 协同执行
    Collaborative,
}

/// 创建公司请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCompanyRequest {
    /// 公司名称
    pub name: String,
    /// 公司类型
    pub company_type: CompanyType,
    /// Token 配额
    pub token_quota: Option<u64>,
    /// 最大 Agent 数
    pub max_agents: Option<usize>,
    /// CEO 配置
    pub ceo_config: Option<CEOConfig>,
    /// 绑定的通信通道
    pub channel: Option<String>,
    /// 标签
    pub labels: Option<std::collections::HashMap<String, String>>,
}

/// 创建团队请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    /// 所属公司 ID
    pub company_id: String,
    /// 团队名称
    pub name: String,
    /// 团队目标
    pub goal: String,
    /// 团队类型
    pub team_type: TeamType,
    /// 预估复杂度
    pub complexity: Option<u8>,
    /// 初始 Agent 数量
    pub initial_agents: Option<usize>,
    /// 协作模式
    pub collaboration_mode: Option<CollaborationMode>,
}

/// 创建结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResult {
    /// 是否成功
    pub success: bool,
    /// 创建的实体 ID
    pub id: Option<String>,
    /// 消息
    pub message: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 公司管理器
pub struct CompanyManager {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// 董事长 ID（用户分身）
    chairman_id: String,
    /// 公司配置存储
    company_configs: DashMap<String, CompanyConfig>,
    /// 团队配置存储
    team_configs: DashMap<String, TeamConfig>,
}

impl CompanyManager {
    /// 创建新的公司管理器
    pub fn new(cluster_state: Arc<ClusterState>) -> Self {
        Self {
            cluster_state,
            chairman_id: Uuid::new_v4().to_string(),
            company_configs: DashMap::new(),
            team_configs: DashMap::new(),
        }
    }

    /// 获取董事长 ID
    pub fn get_chairman_id(&self) -> &str {
        &self.chairman_id
    }

    /// 创建公司（实例）
    pub async fn create_company(&self, request: CreateCompanyRequest) -> CreateResult {
        // 检查公司名称是否已存在
        let existing = self.cluster_state.get_companies();
        if existing.iter().any(|c| c.name == request.name) {
            return CreateResult {
                success: false,
                id: None,
                message: format!("公司「{}」已存在", request.name),
                created_at: Utc::now(),
            };
        }

        // 生成 ID
        let company_id = format!("company-{}", Uuid::new_v4());
        let ceo_id = format!("ceo-{}", Uuid::new_v4());

        // 创建配置
        let config = CompanyConfig {
            name: request.name.clone(),
            company_type: request.company_type,
            token_quota: request.token_quota.unwrap_or(500_000),
            max_agents: request.max_agents.unwrap_or(30),
            ceo_config: request.ceo_config.unwrap_or_default(),
            channel: request.channel.clone(),
            labels: request.labels.unwrap_or_default(),
        };

        // 创建节点
        let node = ClusterNode {
            id: company_id.clone(),
            name: request.name.clone(),
            node_type: NodeType::Company,
            status: NodeStatus::Initializing,
            instance_id: Some(company_id.clone()),
            ceo_agent_id: Some(ceo_id.clone()),
            channel: request.channel,
            resource_usage: NodeResourceUsage {
                cpu_percent: 0.0,
                memory_percent: 0.0,
                tokens_used: 0,
                tokens_quota: config.token_quota,
                active_tasks: 0,
                completed_tasks: 0,
            },
            created_at: Utc::now(),
            last_active_at: Utc::now(),
            labels: config.labels.clone(),
        };

        // 注册节点
        self.cluster_state.register_node(node);
        self.cluster_state.set_parent(&company_id, &self.chairman_id);
        self.company_configs.insert(company_id.clone(), config);

        // 更新状态为运行中
        self.cluster_state.update_node_status(&company_id, NodeStatus::Running);

        // 更新集群 Token 配额
        self.cluster_state.set_token_quota(
            self.cluster_state.get_metrics().await.total_token_quota + request.token_quota.unwrap_or(500_000),
        );

        CreateResult {
            success: true,
            id: Some(company_id.clone()),
            message: format!(
                "✅ 已创建公司「{}」\n类型：{}\nToken 配额：{}\n最大 Agent 数：{}\nCEO ID：{}",
                request.name,
                request.company_type,
                request.token_quota.unwrap_or(500_000),
                request.max_agents.unwrap_or(30),
                ceo_id
            ),
            created_at: Utc::now(),
        }
    }

    /// 创建团队
    pub async fn create_team(&self, request: CreateTeamRequest) -> CreateResult {
        // 检查公司是否存在
        let company = self.cluster_state.get_node(&request.company_id);
        if company.is_none() {
            return CreateResult {
                success: false,
                id: None,
                message: format!("公司 {} 不存在", request.company_id),
                created_at: Utc::now(),
            };
        }

        let company = company.unwrap();
        if company.node_type != NodeType::Company {
            return CreateResult {
                success: false,
                id: None,
                message: "指定的节点不是公司节点".to_string(),
                created_at: Utc::now(),
            };
        }

        // 生成 ID
        let team_id = format!("team-{}", Uuid::new_v4());
        let lead_id = format!("lead-{}", Uuid::new_v4());

        // 创建配置
        let config = TeamConfig {
            name: request.name.clone(),
            goal: request.goal.clone(),
            team_type: request.team_type,
            complexity: request.complexity.unwrap_or(5),
            initial_agents: request.initial_agents.unwrap_or(5),
            collaboration_mode: request.collaboration_mode.unwrap_or(CollaborationMode::Parallel),
        };

        // 创建节点
        let node = ClusterNode {
            id: team_id.clone(),
            name: request.name.clone(),
            node_type: NodeType::Team,
            status: NodeStatus::Initializing,
            instance_id: Some(request.company_id.clone()),
            ceo_agent_id: Some(lead_id.clone()),
            channel: None,
            resource_usage: NodeResourceUsage {
                cpu_percent: 0.0,
                memory_percent: 0.0,
                tokens_used: 0,
                tokens_quota: 0,
                active_tasks: 0,
                completed_tasks: 0,
            },
            created_at: Utc::now(),
            last_active_at: Utc::now(),
            labels: std::collections::HashMap::new(),
        };

        // 注册节点
        self.cluster_state.register_node(node);
        self.cluster_state.set_parent(&team_id, &request.company_id);
        self.team_configs.insert(team_id.clone(), config);

        // 更新状态为运行中
        self.cluster_state.update_node_status(&team_id, NodeStatus::Running);

        CreateResult {
            success: true,
            id: Some(team_id.clone()),
            message: format!(
                "✅ 已在公司「{}」下创建团队「{}」\n目标：{}\n团队类型：{:?}\n复杂度：{}\n初始 Agent 数：{}",
                company.name,
                request.name,
                request.goal,
                request.team_type,
                request.complexity.unwrap_or(5),
                request.initial_agents.unwrap_or(5)
            ),
            created_at: Utc::now(),
        }
    }

    /// 快速创建（公司 + 团队）
    pub async fn quick_create(
        &self,
        company_name: String,
        company_type: CompanyType,
        team_name: String,
        team_goal: String,
        team_type: TeamType,
    ) -> CreateResult {
        // 创建公司
        let company_result = self.create_company(CreateCompanyRequest {
            name: company_name.clone(),
            company_type,
            token_quota: None,
            max_agents: None,
            ceo_config: None,
            channel: None,
            labels: None,
        }).await;

        if !company_result.success {
            return company_result;
        }

        let company_id = company_result.id.clone().unwrap();

        // 创建团队
        let team_result = self.create_team(CreateTeamRequest {
            company_id: company_id.clone(),
            name: team_name.clone(),
            goal: team_goal.clone(),
            team_type,
            complexity: None,
            initial_agents: None,
            collaboration_mode: None,
        }).await;

        if !team_result.success {
            return CreateResult {
                success: false,
                id: Some(company_id),
                message: format!(
                    "公司「{}」已创建，但团队创建失败：{}",
                    company_name,
                    team_result.message
                ),
                created_at: Utc::now(),
            };
        }

        CreateResult {
            success: true,
            id: team_result.id,
            message: format!(
                "✅ 快速创建完成\n\n【公司】{}\n类型：{}\n\n【团队】{}\n目标：{}",
                company_name,
                company_type,
                team_name,
                team_goal
            ),
            created_at: Utc::now(),
        }
    }

    /// 列出所有公司
    pub fn list_companies(&self) -> Vec<(ClusterNode, Option<CompanyConfig>)> {
        self.cluster_state
            .get_companies()
            .into_iter()
            .map(|node| {
                let config = self.company_configs.get(&node.id).map(|c| c.clone());
                (node, config)
            })
            .collect()
    }

    /// 列出公司的所有团队
    pub fn list_teams(&self, company_id: &str) -> Vec<(ClusterNode, Option<TeamConfig>)> {
        self.cluster_state
            .get_teams(Some(company_id))
            .into_iter()
            .map(|node| {
                let config = self.team_configs.get(&node.id).map(|c| c.clone());
                (node, config)
            })
            .collect()
    }

    /// 删除公司
    pub async fn delete_company(&self, company_id: &str) -> CreateResult {
        // 检查公司是否存在
        let company = self.cluster_state.get_node(company_id);
        if company.is_none() {
            return CreateResult {
                success: false,
                id: None,
                message: format!("公司 {} 不存在", company_id),
                created_at: Utc::now(),
            };
        }

        let company = company.unwrap();

        // 先删除所有子团队
        let teams = self.cluster_state.get_teams(Some(company_id));
        for team in teams {
            self.cluster_state.unregister_node(&team.id);
            self.team_configs.remove(&team.id);
        }

        // 删除公司
        self.cluster_state.unregister_node(company_id);
        self.company_configs.remove(company_id);

        CreateResult {
            success: true,
            id: Some(company_id.to_string()),
            message: format!("✅ 已删除公司「{}」及其所有团队", company.name),
            created_at: Utc::now(),
        }
    }

    /// 删除团队
    pub async fn delete_team(&self, team_id: &str) -> CreateResult {
        // 检查团队是否存在
        let team = self.cluster_state.get_node(team_id);
        if team.is_none() {
            return CreateResult {
                success: false,
                id: None,
                message: format!("团队 {} 不存在", team_id),
                created_at: Utc::now(),
            };
        }

        let team = team.unwrap();

        // 删除团队
        self.cluster_state.unregister_node(team_id);
        self.team_configs.remove(team_id);

        CreateResult {
            success: true,
            id: Some(team_id.to_string()),
            message: format!("✅ 已删除团队「{}」", team.name),
            created_at: Utc::now(),
        }
    }

    /// 更新公司状态
    pub fn update_company_status(&self, company_id: &str, status: NodeStatus) {
        self.cluster_state.update_node_status(company_id, status);
    }

    /// 获取公司配置
    pub fn get_company_config(&self, company_id: &str) -> Option<CompanyConfig> {
        self.company_configs.get(company_id).map(|c| c.clone())
    }

    /// 获取团队配置
    pub fn get_team_config(&self, team_id: &str) -> Option<TeamConfig> {
        self.team_configs.get(team_id).map(|c| c.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_company() {
        let cluster_state = Arc::new(ClusterState::new());
        let manager = CompanyManager::new(cluster_state.clone());

        let result = manager.create_company(CreateCompanyRequest {
            name: "Test Company".to_string(),
            company_type: CompanyType::MarketResearch,
            token_quota: Some(1000000),
            max_agents: Some(50),
            ceo_config: None,
            channel: Some("telegram:@TestBot".to_string()),
            labels: None,
        }).await;

        assert!(result.success);
        assert!(result.id.is_some());

        let companies = manager.list_companies();
        assert_eq!(companies.len(), 1);
    }

    #[tokio::test]
    async fn test_create_team() {
        let cluster_state = Arc::new(ClusterState::new());
        let manager = CompanyManager::new(cluster_state.clone());

        // 先创建公司
        let company_result = manager.create_company(CreateCompanyRequest {
            name: "Parent Company".to_string(),
            company_type: CompanyType::General,
            token_quota: None,
            max_agents: None,
            ceo_config: None,
            channel: None,
            labels: None,
        }).await;

        let company_id = company_result.id.unwrap();

        // 创建团队
        let result = manager.create_team(CreateTeamRequest {
            company_id: company_id.clone(),
            name: "Development Team".to_string(),
            goal: "Build awesome features".to_string(),
            team_type: TeamType::Development,
            complexity: Some(7),
            initial_agents: Some(10),
            collaboration_mode: Some(CollaborationMode::Collaborative),
        }).await;

        assert!(result.success);

        let teams = manager.list_teams(&company_id);
        assert_eq!(teams.len(), 1);
    }

    #[tokio::test]
    async fn test_quick_create() {
        let cluster_state = Arc::new(ClusterState::new());
        let manager = CompanyManager::new(cluster_state.clone());

        let result = manager.quick_create(
            "AI Research Company".to_string(),
            CompanyType::MarketResearch,
            "Market Analysis Team".to_string(),
            "Analyze AI market trends".to_string(),
            TeamType::Research,
        ).await;

        assert!(result.success);

        let companies = manager.list_companies();
        assert_eq!(companies.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_company() {
        let cluster_state = Arc::new(ClusterState::new());
        let manager = CompanyManager::new(cluster_state.clone());

        // 创建公司
        let company_result = manager.create_company(CreateCompanyRequest {
            name: "To Delete".to_string(),
            company_type: CompanyType::General,
            token_quota: None,
            max_agents: None,
            ceo_config: None,
            channel: None,
            labels: None,
        }).await;

        let company_id = company_result.id.unwrap();

        // 删除公司
        let result = manager.delete_company(&company_id).await;
        assert!(result.success);

        let companies = manager.list_companies();
        assert!(companies.is_empty());
    }
}