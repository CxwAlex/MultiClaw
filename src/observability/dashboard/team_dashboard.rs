//! TeamDashboard - 团队看板（L2）
//! 任务进度视图，面向团队负责人

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cluster_state::{ClusterState, ClusterNode, NodeStatus};

/// 团队看板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamDashboard {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// 团队 ID
    pub team_id: String,
    /// 团队名称
    pub team_name: String,
    /// 团队目标
    pub team_goal: String,
    /// 项目信息
    pub project_info: ProjectInfo,
    /// 任务列表
    pub tasks: Vec<TaskDetail>,
    /// Worker 状态
    pub worker_statuses: Vec<WorkerStatus>,
    /// 资源使用
    pub resource_usage: TeamResourceUsage,
    /// 团队知识
    pub team_knowledge: Vec<KnowledgeEntry>,
    /// 快速统计
    pub quick_stats: TeamQuickStats,
}

/// 项目信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    /// 项目 ID
    pub id: String,
    /// 项目名称
    pub name: String,
    /// 项目描述
    pub description: String,
    /// 项目状态
    pub status: ProjectStatus,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 预计完成时间
    pub estimated_completion: Option<DateTime<Utc>>,
    /// 整体进度
    pub progress_percent: f64,
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

/// 任务详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDetail {
    /// 任务 ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务描述
    pub description: String,
    /// 任务状态
    pub status: TaskStatus,
    /// 任务优先级
    pub priority: TaskPriority,
    /// 分配给的 Agent
    pub assigned_agent: Option<String>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 预计耗时（分钟）
    pub estimated_minutes: Option<u32>,
    /// 实际耗时（分钟）
    pub actual_minutes: Option<u32>,
    /// 依赖任务
    pub dependencies: Vec<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 进度百分比
    pub progress_percent: f64,
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Blocked,
    Review,
    Completed,
    Failed,
    Cancelled,
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

/// Worker 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerStatus {
    /// Agent ID
    pub agent_id: String,
    /// Agent 名称
    pub agent_name: String,
    /// Agent 角色
    pub role: String,
    /// 运行状态
    pub status: WorkerRunningStatus,
    /// 当前任务
    pub current_task: Option<String>,
    /// 今日完成任务数
    pub tasks_completed_today: usize,
    /// 总完成任务数
    pub tasks_completed_total: usize,
    /// 效率得分
    pub efficiency_score: f64,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// CPU 使用率
    pub cpu_percent: f64,
    /// 内存使用率
    pub memory_percent: f64,
}

/// Worker 运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerRunningStatus {
    Idle,
    Busy,
    Waiting,
    Error,
    Offline,
}

/// 团队资源使用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamResourceUsage {
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
    /// 本团队成本
    pub team_cost: f64,
}

/// 知识条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// 条目 ID
    pub id: String,
    /// 条目标题
    pub title: String,
    /// 条目内容
    pub content: String,
    /// 条目类型
    pub entry_type: KnowledgeType,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 创建者
    pub created_by: String,
    /// 标签
    pub tags: Vec<String>,
    /// 引用次数
    pub citation_count: usize,
}

/// 知识类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KnowledgeType {
    Document,
    Pattern,
    LessonLearned,
    BestPractice,
    Issue,
    Solution,
}

/// 团队快速统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamQuickStats {
    /// 总任务数
    pub total_tasks: usize,
    /// 完成任务数
    pub completed_tasks: usize,
    /// 进行中任务数
    pub in_progress_tasks: usize,
    /// 阻塞任务数
    pub blocked_tasks: usize,
    /// Agent 数量
    pub agent_count: usize,
    /// 团队效率
    pub team_efficiency: f64,
}

/// 团队看板管理器
pub struct TeamDashboardManager {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// 团队 ID
    team_id: String,
    /// 任务存储
    tasks: DashMap<String, TaskDetail>,
    /// Worker 状态存储
    workers: DashMap<String, WorkerStatus>,
    /// 知识库
    knowledge: DashMap<String, KnowledgeEntry>,
    /// 团队目标
    team_goal: String,
}

impl TeamDashboardManager {
    /// 创建团队看板
    pub fn new(cluster_state: Arc<ClusterState>, team_id: String, team_goal: String) -> Self {
        Self {
            cluster_state,
            team_id,
            tasks: DashMap::new(),
            workers: DashMap::new(),
            knowledge: DashMap::new(),
            team_goal,
        }
    }

    /// 获取看板数据
    pub async fn get_dashboard(&self) -> TeamDashboard {
        let team = self.cluster_state.get_node(&self.team_id);
        let agents = self.cluster_state.get_agents(Some(&self.team_id));

        let team_name = team
            .as_ref()
            .map(|t| t.name.clone())
            .unwrap_or_else(|| "Unknown Team".to_string());

        // 获取任务列表
        let tasks = self.get_all_tasks();

        // 获取 Worker 状态
        let worker_statuses = self.get_worker_statuses(&agents);

        // 构建资源使用
        let resource_usage = self.build_resource_usage(&team, &agents);

        // 获取团队知识
        let team_knowledge = self.get_recent_knowledge(10);

        // 构建快速统计
        let quick_stats = self.build_quick_stats(&tasks);

        // 构建项目信息
        let project_info = ProjectInfo {
            id: format!("project-{}", self.team_id),
            name: team_name.clone(),
            description: self.team_goal.clone(),
            status: ProjectStatus::InProgress,
            started_at: team.as_ref().map(|t| t.created_at).unwrap_or_else(Utc::now),
            estimated_completion: Some(Utc::now() + chrono::Duration::hours(4)),
            progress_percent: quick_stats.completed_tasks as f64 / quick_stats.total_tasks.max(1) as f64 * 100.0,
        };

        TeamDashboard {
            generated_at: Utc::now(),
            team_id: self.team_id.clone(),
            team_name,
            team_goal: self.team_goal.clone(),
            project_info,
            tasks,
            worker_statuses,
            resource_usage,
            team_knowledge,
            quick_stats,
        }
    }

    /// 添加任务
    pub fn add_task(&self, task: TaskDetail) {
        self.tasks.insert(task.id.clone(), task);
    }

    /// 更新任务状态
    pub fn update_task_status(&self, task_id: &str, status: TaskStatus) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.status = status;
            if status == TaskStatus::Completed {
                task.completed_at = Some(Utc::now());
            }
        }
    }

    /// 分配任务
    pub fn assign_task(&self, task_id: &str, agent_id: &str) {
        if let Some(mut task) = self.tasks.get_mut(task_id) {
            task.assigned_agent = Some(agent_id.to_string());
            task.status = TaskStatus::InProgress;
            task.started_at = Some(Utc::now());
        }
    }

    /// 添加知识
    pub fn add_knowledge(&self, entry: KnowledgeEntry) {
        self.knowledge.insert(entry.id.clone(), entry);
    }

    /// 获取所有任务
    fn get_all_tasks(&self) -> Vec<TaskDetail> {
        self.tasks.iter().map(|t| t.clone()).collect()
    }

    /// 获取 Worker 状态
    fn get_worker_statuses(&self, agents: &[ClusterNode]) -> Vec<WorkerStatus> {
        agents
            .iter()
            .map(|agent| {
                self.workers
                    .get(&agent.id)
                    .map(|w| w.clone())
                    .unwrap_or_else(|| WorkerStatus {
                        agent_id: agent.id.clone(),
                        agent_name: agent.name.clone(),
                        role: "Worker".to_string(),
                        status: WorkerRunningStatus::Idle,
                        current_task: None,
                        tasks_completed_today: 0,
                        tasks_completed_total: 0,
                        efficiency_score: 100.0,
                        last_active_at: agent.last_active_at,
                        cpu_percent: agent.resource_usage.cpu_percent,
                        memory_percent: agent.resource_usage.memory_percent,
                    })
            })
            .collect()
    }

    /// 构建资源使用
    fn build_resource_usage(&self, team: &Option<ClusterNode>, agents: &[ClusterNode]) -> TeamResourceUsage {
        match team {
            Some(t) => {
                let token_used = agents.iter().map(|a| a.resource_usage.tokens_used).sum();
                TeamResourceUsage {
                    token_used,
                    token_quota: t.resource_usage.tokens_quota,
                    token_percent: if t.resource_usage.tokens_quota > 0 {
                        (token_used as f64 / t.resource_usage.tokens_quota as f64) * 100.0
                    } else {
                        0.0
                    },
                    active_agents: agents.iter().filter(|a| a.status == NodeStatus::Running).count(),
                    agent_quota: 10,
                    team_cost: token_used as f64 * 0.00001,
                }
            }
            None => TeamResourceUsage {
                token_used: 0,
                token_quota: 0,
                token_percent: 0.0,
                active_agents: 0,
                agent_quota: 0,
                team_cost: 0.0,
            },
        }
    }

    /// 获取最近的知识条目
    fn get_recent_knowledge(&self, limit: usize) -> Vec<KnowledgeEntry> {
        let mut entries: Vec<_> = self.knowledge.iter().map(|k| k.clone()).collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.into_iter().take(limit).collect()
    }

    /// 构建快速统计
    fn build_quick_stats(&self, tasks: &[TaskDetail]) -> TeamQuickStats {
        TeamQuickStats {
            total_tasks: tasks.len(),
            completed_tasks: tasks.iter().filter(|t| t.status == TaskStatus::Completed).count(),
            in_progress_tasks: tasks.iter().filter(|t| t.status == TaskStatus::InProgress).count(),
            blocked_tasks: tasks.iter().filter(|t| t.status == TaskStatus::Blocked).count(),
            agent_count: self.workers.len(),
            team_efficiency: 85.0,
        }
    }

    /// 格式化为报告
    pub async fn format_report(&self) -> String {
        let dashboard = self.get_dashboard().await;

        let progress_bar = self.format_progress_bar(dashboard.project_info.progress_percent);

        format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 团队看板 - {}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

【项目信息】
{}
{}

【任务统计】
总任务：{}  完成：{}  进行中：{}  阻塞：{}

【任务列表】
{}

【Worker 状态】
{}

【资源使用】
Token: {} / {} ({:.1}%)
活跃 Agent: {} / {}
成本: ${:.4}

【团队知识】{}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"#,
            dashboard.team_name,
            dashboard.team_goal,
            progress_bar,
            dashboard.quick_stats.total_tasks,
            dashboard.quick_stats.completed_tasks,
            dashboard.quick_stats.in_progress_tasks,
            dashboard.quick_stats.blocked_tasks,
            self.format_tasks(&dashboard.tasks),
            self.format_workers(&dashboard.worker_statuses),
            dashboard.resource_usage.token_used,
            dashboard.resource_usage.token_quota,
            dashboard.resource_usage.token_percent,
            dashboard.resource_usage.active_agents,
            dashboard.resource_usage.agent_quota,
            dashboard.resource_usage.team_cost,
            self.format_knowledge(&dashboard.team_knowledge),
        )
    }

    fn format_progress_bar(&self, percent: f64) -> String {
        let filled = (percent / 10.0).round() as usize;
        let empty = 10 - filled;
        format!(
            "进度：[{}{}] {:.0}%",
            "█".repeat(filled),
            "░".repeat(empty),
            percent
        )
    }

    fn format_tasks(&self, tasks: &[TaskDetail]) -> String {
        if tasks.is_empty() {
            return "暂无任务".to_string();
        }

        tasks
            .iter()
            .take(10)
            .map(|t| {
                let status_emoji = match t.status {
                    TaskStatus::Pending => "⏳",
                    TaskStatus::InProgress => "🔄",
                    TaskStatus::Blocked => "🚫",
                    TaskStatus::Review => "📋",
                    TaskStatus::Completed => "✅",
                    TaskStatus::Failed => "❌",
                    TaskStatus::Cancelled => "⏹️",
                };
                let priority_emoji = match t.priority {
                    TaskPriority::Low => "",
                    TaskPriority::Medium => "",
                    TaskPriority::High => "⚡",
                    TaskPriority::Critical => "🔥",
                };
                format!(
                    "{} {} {}{}",
                    status_emoji,
                    t.name,
                    priority_emoji,
                    t.assigned_agent.as_ref().map(|a| format!(" ({})", a)).unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_workers(&self, workers: &[WorkerStatus]) -> String {
        if workers.is_empty() {
            return "暂无 Agent".to_string();
        }

        workers
            .iter()
            .take(5)
            .map(|w| {
                let status_emoji = match w.status {
                    WorkerRunningStatus::Idle => "🟢",
                    WorkerRunningStatus::Busy => "🟡",
                    WorkerRunningStatus::Waiting => "⚪",
                    WorkerRunningStatus::Error => "🔴",
                    WorkerRunningStatus::Offline => "⚫",
                };
                format!(
                    "{} {} - {} 任务 | 效率 {:.0}%",
                    status_emoji,
                    w.agent_name,
                    w.tasks_completed_today,
                    w.efficiency_score
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_knowledge(&self, knowledge: &[KnowledgeEntry]) -> String {
        if knowledge.is_empty() {
            return "\n暂无知识条目".to_string();
        }

        let formatted = knowledge
            .iter()
            .take(3)
            .map(|k| format!("- {}", k.title))
            .collect::<Vec<_>>()
            .join("\n");

        format!("\n{}\n...", formatted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_team_dashboard() {
        let cluster_state = Arc::new(ClusterState::new());
        let dashboard = TeamDashboardManager::new(
            cluster_state,
            "team-1".to_string(),
            "Build awesome features".to_string(),
        );

        // 添加任务
        dashboard.add_task(TaskDetail {
            id: "task-1".to_string(),
            name: "Implement feature A".to_string(),
            description: "Implement feature A for the product".to_string(),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            assigned_agent: Some("agent-1".to_string()),
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            estimated_minutes: Some(60),
            actual_minutes: None,
            dependencies: vec![],
            tags: vec!["feature".to_string()],
            progress_percent: 50.0,
        });

        let data = dashboard.get_dashboard().await;
        assert_eq!(data.tasks.len(), 1);
        assert_eq!(data.quick_stats.in_progress_tasks, 1);
    }
}