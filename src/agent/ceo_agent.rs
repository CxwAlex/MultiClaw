// src/agent/ceo_agent.rs
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// CEO Agent - 公司实例的最高管理者
pub struct CEOAgent {
    /// CEO ID
    pub id: String,
    /// 公司 ID
    pub company_id: String,
    /// 公司名称
    pub company_name: String,
    /// CEO 配置
    pub config: CEOConfig,
    /// 团队管理
    pub teams: Arc<RwLock<Vec<TeamHandle>>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEOConfig {
    /// CEO 名称
    pub name: String,
    /// 模型偏好
    pub model_preference: String,
    /// 个性特征
    pub personality: String,
    /// 资源限制
    pub resource_limits: ResourceQuota,
    /// 决策模式
    pub decision_mode: DecisionMode,
    /// 通信渠道
    pub channel: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub tokens_per_minute: u32,
    pub max_concurrent_agents: u32,
    pub storage_limit_mb: u32,
    pub api_calls_per_minute: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DecisionMode {
    /// 自动决策
    Automatic,
    /// 半自动（需要确认重要决策）
    SemiAutomatic,
    /// 手动（需要确认所有决策）
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamHandle {
    /// 团队 ID
    pub id: String,
    /// 团队名称
    pub name: String,
    /// 团队类型
    pub team_type: TeamType,
    /// 团队领导 ID
    pub lead_id: String,
    /// 团队成员数量
    pub member_count: usize,
    /// 团队状态
    pub status: TeamStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TeamStatus {
    Initializing,
    Running,
    Paused,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub goal: String,
    pub team_type: TeamType,
    pub initial_members: usize,
    pub collaboration_mode: CollaborationMode,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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

impl CEOAgent {
    /// 创建新的 CEO Agent
    pub fn new(company_id: String, company_name: String, config: CEOConfig) -> Self {
        Self {
            id: format!("ceo-{}", Uuid::new_v4()),
            company_id,
            company_name,
            config,
            teams: Arc::new(RwLock::new(Vec::new())),
            created_at: Utc::now(),
            last_active_at: Utc::now(),
        }
    }

    /// 创建团队
    pub async fn create_team(&self, request: CreateTeamRequest) -> Result<TeamHandle, Box<dyn std::error::Error>> {
        let team_handle = TeamHandle {
            id: format!("team-{}", Uuid::new_v4()),
            name: request.name,
            team_type: request.team_type,
            lead_id: format!("lead-{}", Uuid::new_v4()), // 创建团队领导
            member_count: request.initial_members,
            status: TeamStatus::Initializing,
            created_at: Utc::now(),
        };

        // 添加到团队列表
        {
            let mut teams = self.teams.write().await;
            teams.push(team_handle.clone());
        }

        Ok(team_handle)
    }

    /// 获取团队列表
    pub async fn get_teams(&self) -> Vec<TeamHandle> {
        let teams = self.teams.read().await;
        teams.clone()
    }

    /// 获取团队详情
    pub async fn get_team(&self, team_id: &str) -> Option<TeamHandle> {
        let teams = self.teams.read().await;
        teams.iter().find(|t| t.id == team_id).cloned()
    }

    /// 更新最后活跃时间
    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
    }

    /// 获取公司摘要
    pub fn get_company_summary(&self) -> String {
        format!(
            "🏢 公司：{}\n📊 团队数：{}\n👥 总成员预估：{}\n🧠 模型：{}\n🎯 个性：{}",
            self.company_name,
            self.teams.try_read().map(|t| t.len()).unwrap_or(0),
            self.teams
                .try_read()
                .map(|t| t.iter().map(|team| team.member_count).sum::<usize>())
                .unwrap_or(0),
            self.config.model_preference,
            self.config.personality
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ceo_agent_creation() {
        let config = CEOConfig {
            name: "Test CEO".to_string(),
            model_preference: "gpt-4".to_string(),
            personality: "strategic".to_string(),
            resource_limits: ResourceQuota {
                tokens_per_minute: 100000,
                max_concurrent_agents: 20,
                storage_limit_mb: 500,
                api_calls_per_minute: 500,
            },
            decision_mode: DecisionMode::SemiAutomatic,
            channel: Some("telegram:test_bot".to_string()),
        };

        let ceo = CEOAgent::new(
            "company-123".to_string(),
            "Test Company".to_string(),
            config,
        );

        assert!(ceo.id.starts_with("ceo-"));
        assert_eq!(ceo.company_id, "company-123");
        assert_eq!(ceo.company_name, "Test Company");
        assert_eq!(ceo.teams.try_read().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_team_creation() {
        let config = CEOConfig {
            name: "Test CEO".to_string(),
            model_preference: "gpt-4".to_string(),
            personality: "strategic".to_string(),
            resource_limits: ResourceQuota {
                tokens_per_minute: 100000,
                max_concurrent_agents: 20,
                storage_limit_mb: 500,
                api_calls_per_minute: 500,
            },
            decision_mode: DecisionMode::SemiAutomatic,
            channel: Some("telegram:test_bot".to_string()),
        };

        let ceo = CEOAgent::new(
            "company-123".to_string(),
            "Test Company".to_string(),
            config,
        );

        let request = CreateTeamRequest {
            name: "Development Team".to_string(),
            goal: "Develop new features".to_string(),
            team_type: TeamType::Development,
            initial_members: 5,
            collaboration_mode: CollaborationMode::Collaborative,
        };

        let result = ceo.create_team(request).await;
        assert!(result.is_ok());

        let team = result.unwrap();
        assert!(team.id.starts_with("team-"));
        assert_eq!(team.name, "Development Team");
        assert_eq!(team.team_type, TeamType::Development);
        assert_eq!(team.member_count, 5);

        let teams = ceo.get_teams().await;
        assert_eq!(teams.len(), 1);
    }
}