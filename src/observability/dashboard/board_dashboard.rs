//! BoardDashboard - 董事长看板（L4）
//! 多实例管理视图，面向董事长 Agent

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cluster_state::{ClusterState, ClusterNode, NodeStatus, NodeType};

/// 公司概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyOverview {
    /// 实例数量
    pub total_instances: usize,
    /// 活跃项目数
    pub active_projects: usize,
    /// 总 Agent 数
    pub total_agents: usize,
    /// 今日完成任务数
    pub tasks_completed_today: usize,
    /// 整体健康度 (0-100)
    pub overall_health_score: f32,
    /// 运行中实例数
    pub running_instances: usize,
    /// 忙碌实例数
    pub busy_instances: usize,
    /// 不健康实例数
    pub unhealthy_instances: usize,
}

/// 资源概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOverview {
    /// 全局 Token 配额
    pub global_token_quota: u64,
    /// 已使用 Token
    pub token_used: u64,
    /// Token 使用百分比
    pub token_usage_percent: f64,
    /// 预计剩余天数
    pub estimated_remaining_days: u32,
    /// 本月成本
    pub month_cost: f64,
    /// 本月预算
    pub month_budget: f64,
    /// 成本百分比
    pub cost_percent: f64,
}

/// 项目摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    /// 项目 ID
    pub id: String,
    /// 项目名称
    pub name: String,
    /// 所属公司 ID
    pub company_id: String,
    /// 所属公司名称
    pub company_name: String,
    /// 项目状态
    pub status: ProjectStatus,
    /// 进度百分比
    pub progress_percent: f64,
    /// 预计完成时间
    pub estimated_completion: Option<DateTime<Utc>>,
    /// 资源消耗
    pub resource_usage: u64,
    /// 团队数量
    pub team_count: usize,
    /// Agent 数量
    pub agent_count: usize,
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

/// 重大事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MajorEvent {
    /// 事件 ID
    pub id: String,
    /// 事件类型
    pub event_type: MajorEventType,
    /// 事件标题
    pub title: String,
    /// 事件描述
    pub description: String,
    /// 来源公司
    pub source_company: Option<String>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 是否已处理
    pub handled: bool,
    /// 严重程度
    pub severity: EventSeverity,
}

/// 重大事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MajorEventType {
    ProjectCompleted,
    ResourceThreshold,
    InstanceCreated,
    InstanceFailed,
    InstanceRecovered,
    CrossInstanceCollaboration,
    BudgetAlert,
    AgentScaling,
    ErrorSpike,
}

/// 事件严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// 成本分析
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostAnalysis {
    /// 本月总成本
    pub month_total: f64,
    /// 按公司分布
    pub by_company: Vec<CompanyCost>,
    /// 按类型分布
    pub by_type: Vec<TypeCost>,
    /// 趋势（最近 7 天）
    pub trend: Vec<DailyCost>,
    /// 预测（本月剩余）
    pub forecast: f64,
}

/// 公司成本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyCost {
    pub company_id: String,
    pub company_name: String,
    pub cost: f64,
    pub percent: f64,
}

/// 类型成本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeCost {
    pub cost_type: String,
    pub cost: f64,
    pub percent: f64,
}

/// 每日成本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String,
    pub cost: f64,
}

/// 董事长看板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardDashboard {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 公司概览
    pub company_overview: CompanyOverview,
    /// 资源概览
    pub resource_overview: ResourceOverview,
    /// 项目列表摘要
    pub projects_summary: Vec<ProjectSummary>,
    /// 重大事件
    pub major_events: Vec<MajorEvent>,
    /// 成本分析
    pub cost_analysis: CostAnalysis,
}

/// 董事长看板
pub struct BoardDashboardManager {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// 重大事件存储
    events: DashMap<String, MajorEvent>,
}

impl BoardDashboardManager {
    /// 创建董事长看板
    pub fn new(cluster_state: Arc<ClusterState>) -> Self {
        Self {
            cluster_state,
            events: DashMap::new(),
        }
    }

    /// 获取看板数据
    pub async fn get_dashboard(&self) -> BoardDashboard {
        let metrics = self.cluster_state.get_metrics().await;
        let companies = self.cluster_state.get_companies();

        // 构建公司概览
        let company_overview = CompanyOverview {
            total_instances: companies.len(),
            active_projects: 0, // TODO: 从项目数据获取
            total_agents: metrics.total_agents,
            tasks_completed_today: metrics.tasks_completed_today,
            overall_health_score: metrics.health_score,
            running_instances: companies.iter().filter(|c| c.status == NodeStatus::Running).count(),
            busy_instances: companies.iter().filter(|c| c.status == NodeStatus::Busy).count(),
            unhealthy_instances: companies.iter().filter(|c| c.status == NodeStatus::Unhealthy).count(),
        };

        // 构建资源概览
        let resource_overview = ResourceOverview {
            global_token_quota: metrics.total_token_quota,
            token_used: metrics.total_token_used,
            token_usage_percent: if metrics.total_token_quota > 0 {
                (metrics.total_token_used as f64 / metrics.total_token_quota as f64) * 100.0
            } else {
                0.0
            },
            estimated_remaining_days: self.estimate_remaining_days(&metrics),
            month_cost: metrics.total_token_used as f64 * 0.00001,
            month_budget: 100.0,
            cost_percent: (metrics.total_token_used as f64 * 0.00001) / 100.0 * 100.0,
        };

        // 构建项目摘要
        let projects_summary = self.build_project_summaries(&companies);

        // 获取重大事件
        let major_events = self.get_recent_events(7);

        // 构建成本分析
        let cost_analysis = self.build_cost_analysis(&companies, &metrics);

        BoardDashboard {
            generated_at: Utc::now(),
            company_overview,
            resource_overview,
            projects_summary,
            major_events,
            cost_analysis,
        }
    }

    /// 添加重大事件
    pub fn add_event(&self, event: MajorEvent) {
        self.events.insert(event.id.clone(), event);
    }

    /// 获取最近的事件
    fn get_recent_events(&self, days: u32) -> Vec<MajorEvent> {
        let cutoff = Utc::now() - chrono::Duration::days(days as i64);
        self.events
            .iter()
            .filter(|e| e.timestamp >= cutoff)
            .map(|e| e.clone())
            .collect()
    }

    /// 构建项目摘要
    fn build_project_summaries(&self, companies: &[ClusterNode]) -> Vec<ProjectSummary> {
        companies
            .iter()
            .map(|company| ProjectSummary {
                id: format!("project-{}", company.id),
                name: format!("{} 项目", company.name),
                company_id: company.id.clone(),
                company_name: company.name.clone(),
                status: ProjectStatus::InProgress,
                progress_percent: 50.0, // TODO: 实际计算
                estimated_completion: Some(Utc::now() + chrono::Duration::hours(2)),
                resource_usage: company.resource_usage.tokens_used,
                team_count: 0, // TODO: 从实际数据获取
                agent_count: 0,
            })
            .collect()
    }

    /// 构建成本分析
    fn build_cost_analysis(&self, companies: &[ClusterNode], metrics: &super::cluster_state::ClusterMetrics) -> CostAnalysis {
        let total_cost = metrics.total_token_used as f64 * 0.00001;

        let by_company: Vec<CompanyCost> = companies
            .iter()
            .map(|c| {
                let cost = c.resource_usage.tokens_used as f64 * 0.00001;
                CompanyCost {
                    company_id: c.id.clone(),
                    company_name: c.name.clone(),
                    cost,
                    percent: if total_cost > 0.0 { cost / total_cost * 100.0 } else { 0.0 },
                }
            })
            .collect();

        CostAnalysis {
            month_total: total_cost,
            by_company,
            by_type: vec![
                TypeCost { cost_type: "LLM API".to_string(), cost: total_cost * 0.8, percent: 80.0 },
                TypeCost { cost_type: "Storage".to_string(), cost: total_cost * 0.15, percent: 15.0 },
                TypeCost { cost_type: "Compute".to_string(), cost: total_cost * 0.05, percent: 5.0 },
            ],
            trend: vec![], // TODO: 从历史数据获取
            forecast: total_cost * 1.2,
        }
    }

    /// 估算剩余天数
    fn estimate_remaining_days(&self, metrics: &super::cluster_state::ClusterMetrics) -> u32 {
        if metrics.total_token_used == 0 {
            return 30; // 默认 30 天
        }

        let remaining = metrics.total_token_quota.saturating_sub(metrics.total_token_used);
        let daily_usage = metrics.total_token_used / 7; // 假设过去 7 天的使用

        if daily_usage == 0 {
            return 30;
        }

        (remaining / daily_usage) as u32
    }

    /// 格式化报告
    pub async fn format_report(&self) -> String {
        let dashboard = self.get_dashboard().await;

        format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 董事长看板
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

【公司概览】
实例总数：{}
├─ 🟢 运行中：{}
├─ 🟡 忙碌：{}
└─ 🔴 不健康：{}

总 Agent 数：{}
今日完成任务：{} 个
整体健康度：{:.1}%

【资源概览】
Token 配额：{:.1} 万
Token 已用：{:.1} 万 ({:.1}%)
预计剩余：{} 天

本月成本：${:.2} / ${:.2} ({:.1}%)

【重大事件】（最近 7 天）
{}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"#,
            dashboard.company_overview.total_instances,
            dashboard.company_overview.running_instances,
            dashboard.company_overview.busy_instances,
            dashboard.company_overview.unhealthy_instances,
            dashboard.company_overview.total_agents,
            dashboard.company_overview.tasks_completed_today,
            dashboard.company_overview.overall_health_score,
            dashboard.resource_overview.global_token_quota as f64 / 10000.0,
            dashboard.resource_overview.token_used as f64 / 10000.0,
            dashboard.resource_overview.token_usage_percent,
            dashboard.resource_overview.estimated_remaining_days,
            dashboard.resource_overview.month_cost,
            dashboard.resource_overview.month_budget,
            dashboard.resource_overview.cost_percent,
            self.format_events(&dashboard.major_events),
        )
    }

    /// 格式化事件列表
    fn format_events(&self, events: &[MajorEvent]) -> String {
        if events.is_empty() {
            return "暂无重大事件".to_string();
        }

        events
            .iter()
            .take(5)
            .map(|e| {
                let severity_emoji = match e.severity {
                    EventSeverity::Info => "ℹ️",
                    EventSeverity::Warning => "⚠️",
                    EventSeverity::Error => "❌",
                    EventSeverity::Critical => "🚨",
                };
                format!(
                    "{} {} - {}",
                    severity_emoji,
                    e.title,
                    e.timestamp.format("%m-%d %H:%M")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_board_dashboard() {
        let cluster_state = Arc::new(ClusterState::new());
        let dashboard = BoardDashboardManager::new(cluster_state);

        let data = dashboard.get_dashboard().await;
        assert_eq!(data.company_overview.total_instances, 0);

        // 添加事件
        dashboard.add_event(MajorEvent {
            id: "event-1".to_string(),
            event_type: MajorEventType::InstanceCreated,
            title: "测试事件".to_string(),
            description: "这是一个测试事件".to_string(),
            source_company: None,
            timestamp: Utc::now(),
            handled: false,
            severity: EventSeverity::Info,
        });

        let data = dashboard.get_dashboard().await;
        assert_eq!(data.major_events.len(), 1);
    }
}