//! UserDashboard - 用户看板（L5）
//! 全局摘要视图，面向用户（自然人）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cluster_state::{ClusterState, ClusterSummary, NodeStatus};

/// 用户看板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserDashboardData {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 集群摘要
    pub cluster_summary: ClusterSummary,
    /// 资源概览
    pub resource_overview: ResourceOverview,
    /// 今日完成
    pub today_completion: TodayCompletion,
    /// 最近完成的项目
    pub recent_completed: Vec<CompletedProject>,
    /// 建议
    pub suggestions: Vec<String>,
    /// 告警
    pub alerts: Vec<UserAlert>,
}

/// 资源概览
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOverview {
    /// Token 使用情况
    pub token_usage: TokenUsage,
    /// 成本情况
    pub cost_usage: CostUsage,
    /// Agent 使用情况
    pub agent_usage: AgentUsage,
}

/// Token 使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 已使用
    pub used: u64,
    /// 总配额
    pub quota: u64,
    /// 使用百分比
    pub percent: f64,
}

/// 成本使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostUsage {
    /// 本月已用
    pub month_used: f64,
    /// 本月预算
    pub month_budget: f64,
    /// 使用百分比
    pub percent: f64,
}

/// Agent 使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    /// 活跃 Agent 数
    pub active: usize,
    /// 总 Agent 数
    pub total: usize,
    /// 利用率
    pub utilization_percent: f64,
}

/// 今日完成情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodayCompletion {
    /// 完成任务数
    pub tasks: usize,
    /// 生成报告数
    pub reports: usize,
    /// 消耗成本
    pub cost: f64,
}

/// 已完成项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedProject {
    /// 项目 ID
    pub id: String,
    /// 项目名称
    pub name: String,
    /// 所属公司
    pub company_name: String,
    /// 完成时间
    pub completed_at: DateTime<Utc>,
    /// 质量评分
    pub quality_score: f64,
}

/// 用户告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAlert {
    /// 告警级别
    pub level: AlertLevel,
    /// 告警消息
    pub message: String,
    /// 告警时间
    pub timestamp: DateTime<Utc>,
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

/// 实例摘要（用于用户看板展示）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceSummary {
    /// 实例 ID
    pub id: String,
    /// 实例名称
    pub name: String,
    /// 状态
    pub status: InstanceStatusDisplay,
    /// 活跃项目数
    pub active_projects: usize,
    /// 进度百分比
    pub progress_percent: f64,
    /// 资源使用百分比
    pub resource_usage_percent: f64,
    /// 通信通道
    pub channel: Option<String>,
}

/// 实例状态展示
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceStatusDisplay {
    Running,
    Busy,
    Idle,
    Unhealthy,
    Recovering,
}

impl From<NodeStatus> for InstanceStatusDisplay {
    fn from(status: NodeStatus) -> Self {
        match status {
            NodeStatus::Running => InstanceStatusDisplay::Running,
            NodeStatus::Busy => InstanceStatusDisplay::Busy,
            NodeStatus::Idle => InstanceStatusDisplay::Idle,
            NodeStatus::Unhealthy | NodeStatus::RecoveryFailed => InstanceStatusDisplay::Unhealthy,
            NodeStatus::Recovering => InstanceStatusDisplay::Recovering,
            NodeStatus::Initializing => InstanceStatusDisplay::Running,
            NodeStatus::Stopped => InstanceStatusDisplay::Idle,
        }
    }
}

/// 用户看板
pub struct UserDashboard {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// 用户 ID
    user_id: String,
}

impl UserDashboard {
    /// 创建用户看板
    pub fn new(cluster_state: Arc<ClusterState>, user_id: String) -> Self {
        Self {
            cluster_state,
            user_id,
        }
    }

    /// 获取用户看板数据
    pub async fn get_data(&self) -> UserDashboardData {
        let cluster_summary = self.cluster_state.get_cluster_summary().await;
        let metrics = self.cluster_state.get_metrics().await;

        // 构建资源概览
        let resource_overview = ResourceOverview {
            token_usage: TokenUsage {
                used: metrics.total_token_used,
                quota: metrics.total_token_quota,
                percent: if metrics.total_token_quota > 0 {
                    (metrics.total_token_used as f64 / metrics.total_token_quota as f64) * 100.0
                } else {
                    0.0
                },
            },
            cost_usage: CostUsage {
                month_used: metrics.total_token_used as f64 * 0.00001, // 假设每 token 成本
                month_budget: 100.0,
                percent: (metrics.total_token_used as f64 * 0.00001) / 100.0 * 100.0,
            },
            agent_usage: AgentUsage {
                active: metrics.total_agents,
                total: metrics.total_agents,
                utilization_percent: if metrics.total_agents > 0 {
                    75.0 // 假设利用率
                } else {
                    0.0
                },
            },
        };

        // 构建今日完成情况
        let today_completion = TodayCompletion {
            tasks: metrics.tasks_completed_today,
            reports: metrics.tasks_completed_today / 10, // 假设每 10 个任务生成 1 份报告
            cost: metrics.total_token_used as f64 * 0.00001,
        };

        // 生成建议
        let suggestions = self.generate_suggestions(&cluster_summary, &resource_overview);

        // 生成告警
        let alerts = self.generate_alerts(&cluster_summary, &resource_overview);

        UserDashboardData {
            generated_at: Utc::now(),
            cluster_summary: cluster_summary.clone(),
            resource_overview,
            today_completion,
            recent_completed: Vec::new(), // TODO: 从历史记录获取
            suggestions,
            alerts,
        }
    }

    /// 生成建议
    fn generate_suggestions(
        &self,
        summary: &ClusterSummary,
        resource: &ResourceOverview,
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        // Token 使用建议
        if resource.token_usage.percent > 80.0 {
            suggestions.push(format!(
                "Token 使用已达 {:.1}%，预计 {} 天后达到 90%",
                resource.token_usage.percent,
                self.estimate_days_to_threshold(&resource.token_usage, 90.0)
            ));
        }

        // 空闲公司建议
        let idle_companies: Vec<_> = summary.companies.iter()
            .filter(|c| c.status == NodeStatus::Idle || c.status == NodeStatus::Stopped)
            .collect();

        if !idle_companies.is_empty() {
            suggestions.push(format!(
                "有 {} 个公司处于空闲状态，可考虑关闭以释放资源",
                idle_companies.len()
            ));
        }

        // 健康度建议
        if summary.health_score < 80.0 {
            suggestions.push(format!(
                "整体健康度为 {:.1}%，建议检查异常公司",
                summary.health_score
            ));
        }

        suggestions
    }

    /// 生成告警
    fn generate_alerts(
        &self,
        summary: &ClusterSummary,
        resource: &ResourceOverview,
    ) -> Vec<UserAlert> {
        let mut alerts = Vec::new();

        // Token 告警
        if resource.token_usage.percent > 90.0 {
            alerts.push(UserAlert {
                level: AlertLevel::Critical,
                message: format!("Token 配额即将耗尽（{:.1}%）", resource.token_usage.percent),
                timestamp: Utc::now(),
                suggested_action: Some("申请增加配额或关闭部分公司".to_string()),
            });
        } else if resource.token_usage.percent > 80.0 {
            alerts.push(UserAlert {
                level: AlertLevel::Warning,
                message: format!("Token 使用超过 80%（{:.1}%）", resource.token_usage.percent),
                timestamp: Utc::now(),
                suggested_action: Some("监控使用情况".to_string()),
            });
        }

        // 不健康公司告警
        for company in &summary.companies {
            if company.status == NodeStatus::Unhealthy {
                alerts.push(UserAlert {
                    level: AlertLevel::Error,
                    message: format!("公司「{}」处于不健康状态", company.id),
                    timestamp: Utc::now(),
                    suggested_action: Some("检查日志或重启公司".to_string()),
                });
            }
        }

        alerts
    }

    /// 估算达到阈值的天数
    fn estimate_days_to_threshold(&self, usage: &TokenUsage, threshold: f64) -> u32 {
        if usage.percent >= threshold {
            return 0;
        }
        // 简单估算：假设每天使用 5%
        let remaining_percent = threshold - usage.percent;
        (remaining_percent / 5.0).ceil() as u32
    }

    /// 格式化为 Telegram 消息
    pub async fn format_for_telegram(&self) -> String {
        let data = self.get_data().await;

        let status_emoji = match data.cluster_summary.health_score {
            s if s >= 90.0 => "🟢",
            s if s >= 70.0 => "🟡",
            _ => "🔴",
        };

        let mut output = format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 MultiClaw 全局概览
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

【我的公司】
{status_emoji} 实例数量：{}
🟢 运行中：{}  🟡 忙碌：{}  ⚪ 空闲：{}
📈 活跃团队：{}

【资源总览】
Token: {:.1} 万 / {:.1} 万 ({:.1}%)
本月成本：${:.2} / ${:.2} ({:.1}%)

【今日完成】
✅ 任务：{} 个
📄 报告：{} 份
💰 成本：${:.2}

"#,
            data.cluster_summary.total_companies,
            data.cluster_summary.running_companies,
            data.cluster_summary.busy_companies,
            data.cluster_summary.total_companies - data.cluster_summary.running_companies - data.cluster_summary.busy_companies,
            data.cluster_summary.total_teams,
            data.resource_overview.token_usage.used as f64 / 10000.0,
            data.resource_overview.token_usage.quota as f64 / 10000.0,
            data.resource_overview.token_usage.percent,
            data.resource_overview.cost_usage.month_used,
            data.resource_overview.cost_usage.month_budget,
            data.resource_overview.cost_usage.percent,
            data.today_completion.tasks,
            data.today_completion.reports,
            data.today_completion.cost,
        );

        // 添加公司列表
        if !data.cluster_summary.companies.is_empty() {
            output.push_str("【实例列表】\n");
            for (i, company) in data.cluster_summary.companies.iter().enumerate() {
                let status_icon = match company.status {
                    NodeStatus::Running => "🟢",
                    NodeStatus::Busy => "🟡",
                    NodeStatus::Idle | NodeStatus::Stopped => "⚪",
                    NodeStatus::Unhealthy | NodeStatus::RecoveryFailed => "🔴",
                    NodeStatus::Recovering => "🔄",
                    NodeStatus::Initializing => "⏳",
                };
                output.push_str(&format!(
                    "{}. {} {} \n   任务：{}  资源：{:.1}%\n",
                    i + 1,
                    status_icon,
                    company.id, // 实际应用中应该用名称
                    company.active_tasks,
                    company.token_usage_percent
                ));
            }
            output.push('\n');
        }

        // 添加建议
        if !data.suggestions.is_empty() {
            output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            output.push_str("💡 建议\n");
            output.push_str("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
            for suggestion in &data.suggestions {
                output.push_str(&format!("- {}\n", suggestion));
            }
        }

        // 添加告警
        if !data.alerts.is_empty() {
            output.push_str("\n⚠️ 告警\n");
            for alert in &data.alerts {
                let level_emoji = match alert.level {
                    AlertLevel::Info => "ℹ️",
                    AlertLevel::Warning => "⚠️",
                    AlertLevel::Error => "❌",
                    AlertLevel::Critical => "🚨",
                };
                output.push_str(&format!("{} {}\n", level_emoji, alert.message));
            }
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_user_dashboard() {
        let cluster_state = Arc::new(ClusterState::new());
        let dashboard = UserDashboard::new(cluster_state, "user-1".to_string());

        let data = dashboard.get_data().await;
        assert_eq!(data.cluster_summary.total_companies, 0);

        // 测试 Telegram 格式化
        let telegram_output = dashboard.format_for_telegram().await;
        assert!(telegram_output.contains("MultiClaw 全局概览"));
    }
}