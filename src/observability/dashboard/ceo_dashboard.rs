//! CEODashboard - CEO 看板（L3）
//! 项目列表视图，面向 CEO Agent

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cluster_state::{ClusterState, ClusterNode, NodeStatus};

/// CEO 看板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEODashboard {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 公司 ID
    pub company_id: String,
    /// 公司名称
    pub company_name: String,
    /// 项目列表
    pub projects: Vec<ProjectDetail>,
    /// 资源使用详情
    pub resource_usage: ResourceUsageDetail,
    /// 待审批事项
    pub pending_approvals: Vec<ApprovalRequest>,
    /// 团队表现排名
    pub team_performance_ranking: Vec<TeamPerformance>,
    /// 告警列表
    pub alerts: Vec<CEOAlert>,
    /// 快速统计
    pub quick_stats: QuickStats,
}

/// 项目详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetail {
    /// 项目 ID
    pub id: String,
    /// 项目名称
    pub name: String,
    /// 项目描述
    pub description: String,
    /// 项目状态
    pub status: ProjectStatus,
    /// 进度百分比
    pub progress_percent: f64,
    /// 团队数量
    pub team_count: usize,
    /// Agent 数量
    pub agent_count: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 预计完成时间
    pub estimated_completion: Option<DateTime<Utc>>,
    /// 负责人
    pub owner: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 资源消耗
    pub resource_used: u64,
    /// 资源预算
    pub resource_budget: u64,
}

/// 项目状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    InProgress,
    Review,
    Completed,
    OnHold,
    Cancelled,
}

/// 资源使用详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageDetail {
    /// Token 使用量
    pub token_used: u64,
    /// Token 配额
    pub token_quota: u64,
    /// Token 使用百分比
    pub token_percent: f64,
    /// 活跃 Agent 数
    pub active_agents: usize,
    /// Agent 配额
    pub agent_quota: usize,
    /// Agent 使用百分比
    pub agent_percent: f64,
    /// 本月成本
    pub month_cost: f64,
    /// 日均成本
    pub daily_cost: f64,
}

/// 审批请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// 请求 ID
    pub id: String,
    /// 请求类型
    pub request_type: ApprovalType,
    /// 请求标题
    pub title: String,
    /// 请求描述
    pub description: String,
    /// 请求者
    pub requester: String,
    /// 请求时间
    pub requested_at: DateTime<Utc>,
    /// 紧急程度
    pub urgency: Urgency,
    /// 预期影响
    pub impact: String,
}

/// 审批类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalType {
    ResourceIncrease,
    NewTeam,
    TeamScaling,
    BudgetIncrease,
    CrossTeamCollaboration,
    PriorityChange,
    ScheduleChange,
}

/// 紧急程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Urgency {
    Low,
    Medium,
    High,
    Critical,
}

/// 团队表现
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamPerformance {
    /// 团队 ID
    pub team_id: String,
    /// 团队名称
    pub team_name: String,
    /// 团队类型
    pub team_type: String,
    /// 效率得分 (0-100)
    pub efficiency_score: f64,
    /// 质量得分 (0-100)
    pub quality_score: f64,
    /// 今日完成任务数
    pub tasks_completed_today: usize,
    /// 总完成任务数
    pub tasks_completed_total: usize,
    /// Agent 数量
    pub agent_count: usize,
    /// 资源消耗
    pub resource_used: u64,
    /// 排名
    pub rank: usize,
}

/// CEO 告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEOAlert {
    /// 告警 ID
    pub id: String,
    /// 告警级别
    pub level: AlertLevel,
    /// 告警类型
    pub alert_type: CEOAlertType,
    /// 告警消息
    pub message: String,
    /// 告警时间
    pub timestamp: DateTime<Utc>,
    /// 相关团队
    pub related_team: Option<String>,
    /// 建议操作
    pub suggested_action: Option<String>,
}

/// 告警级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Info,
    Warning,
    Error,
    Critical,
}

/// CEO 告警类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CEOAlertType {
    ResourceLow,
    TeamOverloaded,
    TaskStalled,
    QualityDrop,
    DeadlineRisk,
    AgentFailure,
    CommunicationIssue,
}

/// 快速统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickStats {
    /// 活跃项目数
    pub active_projects: usize,
    /// 运行中团队数
    pub active_teams: usize,
    /// 今日完成任务数
    pub tasks_today: usize,
    /// 待处理审批数
    pub pending_approvals: usize,
    /// 活跃 Agent 数
    pub active_agents: usize,
}

/// CEO 看板管理器
pub struct CEODashboardManager {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// 公司 ID
    company_id: String,
    /// 审批请求存储
    approval_requests: DashMap<String, ApprovalRequest>,
    /// 告警存储
    alerts: DashMap<String, CEOAlert>,
}

impl CEODashboardManager {
    /// 创建 CEO 看板
    pub fn new(cluster_state: Arc<ClusterState>, company_id: String) -> Self {
        Self {
            cluster_state,
            company_id,
            approval_requests: DashMap::new(),
            alerts: DashMap::new(),
        }
    }

    /// 获取看板数据
    pub async fn get_dashboard(&self) -> CEODashboard {
        let company = self.cluster_state.get_node(&self.company_id);
        let metrics = self.cluster_state.get_metrics().await;
        let teams = self.cluster_state.get_teams(Some(&self.company_id));

        let company_name = company
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // 构建项目列表
        let projects = self.build_projects(&teams);

        // 构建资源使用详情
        let resource_usage = self.build_resource_usage(&company, &metrics);

        // 获取待审批事项
        let pending_approvals = self.get_pending_approvals();

        // 构建团队表现排名
        let team_performance_ranking = self.build_team_ranking(&teams);

        // 获取告警
        let alerts = self.get_active_alerts();

        // 构建快速统计
        let quick_stats = QuickStats {
            active_projects: projects.iter().filter(|p| p.status == ProjectStatus::InProgress).count(),
            active_teams: teams.iter().filter(|t| t.status == NodeStatus::Running).count(),
            tasks_today: metrics.tasks_completed_today,
            pending_approvals: pending_approvals.len(),
            active_agents: metrics.total_agents,
        };

        CEODashboard {
            generated_at: Utc::now(),
            company_id: self.company_id.clone(),
            company_name,
            projects,
            resource_usage,
            pending_approvals,
            team_performance_ranking,
            alerts,
            quick_stats,
        }
    }

    /// 添加审批请求
    pub fn add_approval_request(&self, request: ApprovalRequest) {
        self.approval_requests.insert(request.id.clone(), request);
    }

    /// 处理审批请求
    pub fn process_approval(&self, request_id: &str, approved: bool) -> Option<ApprovalRequest> {
        if approved {
            self.approval_requests.remove(request_id).map(|(_, r)| r)
        } else {
            self.approval_requests.get(request_id).map(|r| r.clone())
        }
    }

    /// 添加告警
    pub fn add_alert(&self, alert: CEOAlert) {
        self.alerts.insert(alert.id.clone(), alert);
    }

    /// 清除告警
    pub fn clear_alert(&self, alert_id: &str) {
        self.alerts.remove(alert_id);
    }

    /// 获取待审批事项
    fn get_pending_approvals(&self) -> Vec<ApprovalRequest> {
        self.approval_requests
            .iter()
            .map(|r| r.clone())
            .collect()
    }

    /// 获取活跃告警
    fn get_active_alerts(&self) -> Vec<CEOAlert> {
        let cutoff = Utc::now() - chrono::Duration::hours(24);
        self.alerts
            .iter()
            .filter(|a| a.timestamp >= cutoff)
            .map(|a| a.clone())
            .collect()
    }

    /// 构建项目列表
    fn build_projects(&self, teams: &[ClusterNode]) -> Vec<ProjectDetail> {
        // 简化实现：每个团队对应一个项目
        teams
            .iter()
            .map(|team| ProjectDetail {
                id: format!("project-{}", team.id),
                name: team.name.clone(),
                description: String::new(),
                status: ProjectStatus::InProgress,
                progress_percent: 50.0,
                team_count: 1,
                agent_count: 0,
                created_at: team.created_at,
                estimated_completion: Some(Utc::now() + chrono::Duration::hours(4)),
                owner: team.ceo_agent_id.clone(),
                tags: vec![],
                resource_used: team.resource_usage.tokens_used,
                resource_budget: team.resource_usage.tokens_quota,
            })
            .collect()
    }

    /// 构建资源使用详情
    fn build_resource_usage(
        &self,
        company: &Option<ClusterNode>,
        metrics: &super::cluster_state::ClusterMetrics,
    ) -> ResourceUsageDetail {
        match company {
            Some(c) => ResourceUsageDetail {
                token_used: c.resource_usage.tokens_used,
                token_quota: c.resource_usage.tokens_quota,
                token_percent: if c.resource_usage.tokens_quota > 0 {
                    (c.resource_usage.tokens_used as f64 / c.resource_usage.tokens_quota as f64) * 100.0
                } else {
                    0.0
                },
                active_agents: metrics.total_agents,
                agent_quota: 30,
                agent_percent: (metrics.total_agents as f64 / 30.0) * 100.0,
                month_cost: c.resource_usage.tokens_used as f64 * 0.00001,
                daily_cost: c.resource_usage.tokens_used as f64 * 0.00001 / 7.0,
            },
            None => ResourceUsageDetail {
                token_used: 0,
                token_quota: 0,
                token_percent: 0.0,
                active_agents: 0,
                agent_quota: 0,
                agent_percent: 0.0,
                month_cost: 0.0,
                daily_cost: 0.0,
            },
        }
    }

    /// 构建团队排名
    fn build_team_ranking(&self, teams: &[ClusterNode]) -> Vec<TeamPerformance> {
        teams
            .iter()
            .enumerate()
            .map(|(i, team)| TeamPerformance {
                team_id: team.id.clone(),
                team_name: team.name.clone(),
                team_type: "General".to_string(),
                efficiency_score: 85.0 - i as f64 * 5.0,
                quality_score: 90.0 - i as f64 * 3.0,
                tasks_completed_today: 10 - i,
                tasks_completed_total: 100 - i * 10,
                agent_count: 5,
                resource_used: team.resource_usage.tokens_used,
                rank: i + 1,
            })
            .collect()
    }

    /// 格式化为报告
    pub async fn format_report(&self) -> String {
        let dashboard = self.get_dashboard().await;

        format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 CEO 看板 - {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

【快速统计】
活跃项目：{}  运行中团队：{}  活跃 Agent：{}
今日完成：{} 个任务  待审批：{} 项

【资源使用】
Token: {:.1} 万 / {:.1} 万 ({:.1}%)
Agent: {} / {} ({:.1}%)
本月成本: ${:.2}

【项目列表】
{}

【团队表现排名】
{}

【待审批】（{} 项）
{}

【告警】（{} 条）
{}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"#,
            dashboard.company_name,
            dashboard.quick_stats.active_projects,
            dashboard.quick_stats.active_teams,
            dashboard.quick_stats.active_agents,
            dashboard.quick_stats.tasks_today,
            dashboard.quick_stats.pending_approvals,
            dashboard.resource_usage.token_used as f64 / 10000.0,
            dashboard.resource_usage.token_quota as f64 / 10000.0,
            dashboard.resource_usage.token_percent,
            dashboard.resource_usage.active_agents,
            dashboard.resource_usage.agent_quota,
            dashboard.resource_usage.agent_percent,
            dashboard.resource_usage.month_cost,
            self.format_projects(&dashboard.projects),
            self.format_team_ranking(&dashboard.team_performance_ranking),
            dashboard.pending_approvals.len(),
            self.format_approvals(&dashboard.pending_approvals),
            dashboard.alerts.len(),
            self.format_alerts(&dashboard.alerts),
        )
    }

    fn format_projects(&self, projects: &[ProjectDetail]) -> String {
        if projects.is_empty() {
            return "暂无项目".to_string();
        }

        projects
            .iter()
            .take(5)
            .map(|p| {
                let status_emoji = match p.status {
                    ProjectStatus::InProgress => "🔄",
                    ProjectStatus::Review => "📋",
                    ProjectStatus::Completed => "✅",
                    ProjectStatus::OnHold => "⏸️",
                    ProjectStatus::Planning => "📝",
                    ProjectStatus::Cancelled => "❌",
                };
                format!(
                    "{} {} - {:.0}% | {} Agent",
                    status_emoji,
                    p.name,
                    p.progress_percent,
                    p.agent_count
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_team_ranking(&self, ranking: &[TeamPerformance]) -> String {
        if ranking.is_empty() {
            return "暂无团队".to_string();
        }

        ranking
            .iter()
            .take(5)
            .map(|t| {
                format!(
                    "{}. {} - 效率 {:.0}% 质量 {:.0}%",
                    t.rank,
                    t.team_name,
                    t.efficiency_score,
                    t.quality_score
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_approvals(&self, approvals: &[ApprovalRequest]) -> String {
        if approvals.is_empty() {
            return "暂无待审批事项".to_string();
        }

        approvals
            .iter()
            .take(3)
            .map(|a| {
                let urgency_emoji = match a.urgency {
                    Urgency::Low => "🟢",
                    Urgency::Medium => "🟡",
                    Urgency::High => "🟠",
                    Urgency::Critical => "🔴",
                };
                format!("{} {} - {}", urgency_emoji, a.title, a.requester)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_alerts(&self, alerts: &[CEOAlert]) -> String {
        if alerts.is_empty() {
            return "暂无告警".to_string();
        }

        alerts
            .iter()
            .take(3)
            .map(|a| {
                let level_emoji = match a.level {
                    AlertLevel::Info => "ℹ️",
                    AlertLevel::Warning => "⚠️",
                    AlertLevel::Error => "❌",
                    AlertLevel::Critical => "🚨",
                };
                format!("{} {}", level_emoji, a.message)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ceo_dashboard() {
        let cluster_state = Arc::new(ClusterState::new());
        let dashboard = CEODashboardManager::new(cluster_state, "company-1".to_string());

        let data = dashboard.get_dashboard().await;
        assert_eq!(data.company_id, "company-1");

        // 添加审批请求
        dashboard.add_approval_request(ApprovalRequest {
            id: "approval-1".to_string(),
            request_type: ApprovalType::ResourceIncrease,
            title: "增加 Token 配额".to_string(),
            description: "需要更多 Token".to_string(),
            requester: "team-1".to_string(),
            requested_at: Utc::now(),
            urgency: Urgency::Medium,
            impact: "可加速项目进度".to_string(),
        });

        let data = dashboard.get_dashboard().await;
        assert_eq!(data.pending_approvals.len(), 1);
    }
}