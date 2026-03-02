//! AgentDashboard - Agent 看板（L1）
//! 执行记录视图，面向 Worker Agent

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::cluster_state::ClusterState;

/// Agent 看板数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDashboard {
    /// 生成时间
    pub generated_at: DateTime<Utc>,
    /// Agent 信息
    pub agent_info: AgentInfo,
    /// 当前任务
    pub current_task: Option<TaskDetail>,
    /// 历史任务（最近 10 个）
    pub task_history: Vec<TaskSummary>,
    /// 健康状态
    pub health_status: WorkerHealthStatus,
    /// 执行记录
    pub execution_log: Vec<ExecutionEntry>,
    /// 收件箱（未读消息）
    pub inbox: Vec<InboxMessage>,
    /// 快速统计
    pub quick_stats: AgentQuickStats,
}

/// Agent 信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub id: String,
    /// Agent 名称
    pub name: String,
    /// Agent 角色
    pub role: String,
    /// 所属团队 ID
    pub team_id: String,
    /// 所属团队名称
    pub team_name: String,
    /// 所属公司 ID
    pub company_id: String,
    /// 所属公司名称
    pub company_name: String,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 运行状态
    pub status: AgentStatus,
    /// 标签
    pub tags: Vec<String>,
    /// 能力列表
    pub capabilities: Vec<String>,
}

/// Agent 状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Idle,
    Busy,
    Waiting,
    Error,
    Offline,
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
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 开始时间
    pub started_at: Option<DateTime<Utc>>,
    /// 预计完成时间
    pub estimated_completion: Option<DateTime<Utc>>,
    /// 进度百分比
    pub progress_percent: f64,
    /// 当前步骤
    pub current_step: Option<String>,
    /// 已完成步骤
    pub completed_steps: Vec<String>,
    /// 待完成步骤
    pub pending_steps: Vec<String>,
    /// 依赖任务
    pub dependencies: Vec<String>,
    /// 标签
    pub tags: Vec<String>,
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

/// 任务摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    /// 任务 ID
    pub id: String,
    /// 任务名称
    pub name: String,
    /// 任务状态
    pub status: TaskStatus,
    /// 完成时间
    pub completed_at: Option<DateTime<Utc>>,
    /// 实际耗时（分钟）
    pub duration_minutes: Option<u32>,
    /// 质量得分
    pub quality_score: Option<f64>,
}

/// Worker 健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHealthStatus {
    /// 整体健康状态
    pub status: HealthStatus,
    /// CPU 使用率
    pub cpu_percent: f64,
    /// 内存使用率
    pub memory_percent: f64,
    /// 网络延迟（毫秒）
    pub network_latency_ms: f64,
    /// 错误计数
    pub error_count: usize,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// 运行时长（秒）
    pub uptime_secs: u64,
    /// 告警列表
    pub alerts: Vec<HealthAlert>,
}

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Unhealthy,
    Unknown,
}

/// 健康告警
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// 告警 ID
    pub id: String,
    /// 告警类型
    pub alert_type: HealthAlertType,
    /// 告警消息
    pub message: String,
    /// 告警时间
    pub timestamp: DateTime<Utc>,
    /// 是否已处理
    pub handled: bool,
}

/// 健康告警类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthAlertType {
    HighCpu,
    HighMemory,
    NetworkIssue,
    TaskFailure,
    TimeoutWarning,
    ResourceLow,
}

/// 执行记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEntry {
    /// 记录 ID
    pub id: String,
    /// 执行动作
    pub action: String,
    /// 执行结果
    pub result: ExecutionResult,
    /// 开始时间
    pub started_at: DateTime<Utc>,
    /// 结束时间
    pub finished_at: Option<DateTime<Utc>>,
    /// 耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 相关任务 ID
    pub related_task_id: Option<String>,
    /// 详细信息
    pub details: serde_json::Value,
    /// 错误信息
    pub error: Option<String>,
}

/// 执行结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionResult {
    Success,
    PartialSuccess,
    Failure,
    Timeout,
    Cancelled,
}

/// 收件箱消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    /// 消息 ID
    pub id: String,
    /// 发送者
    pub sender: String,
    /// 发送者类型
    pub sender_type: SenderType,
    /// 消息类型
    pub message_type: MessageType,
    /// 消息标题
    pub title: String,
    /// 消息内容
    pub content: String,
    /// 发送时间
    pub sent_at: DateTime<Utc>,
    /// 是否已读
    pub read: bool,
    /// 是否需要回复
    pub requires_reply: bool,
    /// 优先级
    pub priority: MessagePriority,
}

/// 发送者类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SenderType {
    User,
    Chairman,
    CEO,
    TeamLead,
    Agent,
    System,
}

/// 消息类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    Task,
    Notification,
    Query,
    Collaboration,
    Alert,
    System,
}

/// 消息优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessagePriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Agent 快速统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentQuickStats {
    /// 今日完成任务数
    pub tasks_completed_today: usize,
    /// 总完成任务数
    pub tasks_completed_total: usize,
    /// 平均任务耗时（分钟）
    pub avg_task_duration: f64,
    /// 成功率
    pub success_rate: f64,
    /// 效率得分
    pub efficiency_score: f64,
    /// 未读消息数
    pub unread_messages: usize,
}

/// Agent 看板管理器
pub struct AgentDashboardManager {
    /// 集群状态
    cluster_state: Arc<ClusterState>,
    /// Agent ID
    agent_id: String,
    /// Agent 信息
    agent_info: AgentInfo,
    /// 当前任务
    current_task: DashMap<String, TaskDetail>,
    /// 任务历史
    task_history: DashMap<String, TaskSummary>,
    /// 执行日志
    execution_log: DashMap<String, ExecutionEntry>,
    /// 收件箱
    inbox: DashMap<String, InboxMessage>,
    /// 健康状态
    health_status: DashMap<String, WorkerHealthStatus>,
}

impl AgentDashboardManager {
    /// 创建 Agent 看板
    pub fn new(cluster_state: Arc<ClusterState>, agent_info: AgentInfo) -> Self {
        let agent_id = agent_info.id.clone();
        Self {
            cluster_state,
            agent_id,
            agent_info,
            current_task: DashMap::new(),
            task_history: DashMap::new(),
            execution_log: DashMap::new(),
            inbox: DashMap::new(),
            health_status: DashMap::new(),
        }
    }

    /// 获取看板数据
    pub async fn get_dashboard(&self) -> AgentDashboard {
        // 获取当前任务
        let current_task = self.get_current_task();

        // 获取任务历史
        let task_history = self.get_task_history(10);

        // 获取健康状态
        let health_status = self.get_health_status();

        // 获取执行日志
        let execution_log = self.get_recent_execution_log(20);

        // 获取收件箱
        let inbox = self.get_unread_messages();

        // 构建快速统计
        let quick_stats = self.build_quick_stats(&task_history);

        AgentDashboard {
            generated_at: Utc::now(),
            agent_info: self.agent_info.clone(),
            current_task,
            task_history,
            health_status,
            execution_log,
            inbox,
            quick_stats,
        }
    }

    /// 设置当前任务
    pub fn set_current_task(&self, task: TaskDetail) {
        self.current_task.insert("current".to_string(), task);
    }

    /// 完成当前任务
    pub fn complete_current_task(&self, quality_score: Option<f64>) {
        if let Some((_, task)) = self.current_task.remove("current") {
            let summary = TaskSummary {
                id: task.id.clone(),
                name: task.name,
                status: TaskStatus::Completed,
                completed_at: Some(Utc::now()),
                duration_minutes: task.started_at.map(|s| {
                    (Utc::now() - s).num_minutes() as u32
                }),
                quality_score,
            };
            self.task_history.insert(task.id, summary);
        }
    }

    /// 添加执行日志
    pub fn add_execution_log(&self, entry: ExecutionEntry) {
        self.execution_log.insert(entry.id.clone(), entry);
    }

    /// 接收消息
    pub fn receive_message(&self, message: InboxMessage) {
        self.inbox.insert(message.id.clone(), message);
    }

    /// 标记消息已读
    pub fn mark_message_read(&self, message_id: &str) {
        if let Some(mut msg) = self.inbox.get_mut(message_id) {
            msg.read = true;
        }
    }

    /// 获取当前任务
    fn get_current_task(&self) -> Option<TaskDetail> {
        self.current_task.get("current").map(|t| t.clone())
    }

    /// 获取任务历史
    fn get_task_history(&self, limit: usize) -> Vec<TaskSummary> {
        let mut history: Vec<_> = self.task_history.iter().map(|t| t.clone()).collect();
        history.sort_by(|a, b| {
            b.completed_at.unwrap_or(Utc::now()).cmp(&a.completed_at.unwrap_or(Utc::now()))
        });
        history.into_iter().take(limit).collect()
    }

    /// 获取健康状态
    fn get_health_status(&self) -> WorkerHealthStatus {
        self.health_status
            .get("health")
            .map(|h| h.clone())
            .unwrap_or_else(|| WorkerHealthStatus {
                status: HealthStatus::Healthy,
                cpu_percent: 0.0,
                memory_percent: 0.0,
                network_latency_ms: 0.0,
                error_count: 0,
                last_heartbeat: Utc::now(),
                uptime_secs: 0,
                alerts: vec![],
            })
    }

    /// 获取最近执行日志
    fn get_recent_execution_log(&self, limit: usize) -> Vec<ExecutionEntry> {
        let mut logs: Vec<_> = self.execution_log.iter().map(|l| l.clone()).collect();
        logs.sort_by(|a, b| b.started_at.cmp(&a.started_at));
        logs.into_iter().take(limit).collect()
    }

    /// 获取未读消息
    fn get_unread_messages(&self) -> Vec<InboxMessage> {
        self.inbox
            .iter()
            .filter(|m| !m.read)
            .map(|m| m.clone())
            .collect()
    }

    /// 构建快速统计
    fn build_quick_stats(&self, task_history: &[TaskSummary]) -> AgentQuickStats {
        let total = task_history.len();
        let completed = task_history.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = task_history.iter().filter(|t| t.status == TaskStatus::Failed).count();

        let avg_duration = task_history
            .iter()
            .filter_map(|t| t.duration_minutes)
            .sum::<u32>() as f64 / completed.max(1) as f64;

        let success_rate = if total > 0 {
            (completed as f64 / total as f64) * 100.0
        } else {
            100.0
        };

        let unread_messages = self.inbox.iter().filter(|m| !m.read).count();

        AgentQuickStats {
            tasks_completed_today: completed,
            tasks_completed_total: total,
            avg_task_duration: avg_duration,
            success_rate,
            efficiency_score: 85.0,
            unread_messages,
        }
    }

    /// 更新健康状态
    pub fn update_health(&self, status: WorkerHealthStatus) {
        self.health_status.insert("health".to_string(), status);
    }

    /// 格式化为报告
    pub async fn format_report(&self) -> String {
        let dashboard = self.get_dashboard().await;

        let status_emoji = match dashboard.agent_info.status {
            AgentStatus::Idle => "🟢",
            AgentStatus::Busy => "🟡",
            AgentStatus::Waiting => "⚪",
            AgentStatus::Error => "🔴",
            AgentStatus::Offline => "⚫",
        };

        let health_emoji = match dashboard.health_status.status {
            HealthStatus::Healthy => "🟢",
            HealthStatus::Warning => "🟡",
            HealthStatus::Unhealthy => "🔴",
            HealthStatus::Unknown => "⚪",
        };

        format!(
            r#"━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🤖 Agent 看板
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

【Agent 信息】
ID: {}
名称: {}
角色: {}
团队: {}
状态: {} {}

【当前任务】
{}

【今日统计】
完成任务: {} 个
成功率: {:.1}%
效率得分: {:.1}%
未读消息: {} 条

【执行日志】（最近 5 条）
{}

【收件箱】
{}

【健康状态】
{} CPU: {:.1}% | 内存: {:.1}%
错误数: {} | 运行时长: {}s
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"#,
            dashboard.agent_info.id,
            dashboard.agent_info.name,
            dashboard.agent_info.role,
            dashboard.agent_info.team_name,
            status_emoji,
            match dashboard.agent_info.status {
                AgentStatus::Idle => "空闲",
                AgentStatus::Busy => "忙碌",
                AgentStatus::Waiting => "等待",
                AgentStatus::Error => "错误",
                AgentStatus::Offline => "离线",
            },
            self.format_current_task(&dashboard.current_task),
            dashboard.quick_stats.tasks_completed_today,
            dashboard.quick_stats.success_rate,
            dashboard.quick_stats.efficiency_score,
            dashboard.quick_stats.unread_messages,
            self.format_execution_log(&dashboard.execution_log),
            self.format_inbox(&dashboard.inbox),
            health_emoji,
            dashboard.health_status.cpu_percent,
            dashboard.health_status.memory_percent,
            dashboard.health_status.error_count,
            dashboard.health_status.uptime_secs,
        )
    }

    fn format_current_task(&self, task: &Option<TaskDetail>) -> String {
        match task {
            Some(t) => {
                let progress_bar = self.format_progress_bar(t.progress_percent);
                format!(
                    "{}\n{}\n状态: {:?} | 优先级: {:?}\n{}",
                    t.name, t.description, t.status, t.priority, progress_bar
                )
            }
            None => "暂无任务".to_string(),
        }
    }

    fn format_progress_bar(&self, percent: f64) -> String {
        let filled = (percent / 10.0).round() as usize;
        let empty = 10 - filled;
        format!(
            "[{}{}] {:.0}%",
            "█".repeat(filled),
            "░".repeat(empty),
            percent
        )
    }

    fn format_execution_log(&self, logs: &[ExecutionEntry]) -> String {
        if logs.is_empty() {
            return "暂无执行记录".to_string();
        }

        logs.iter()
            .take(5)
            .map(|l| {
                let result_emoji = match l.result {
                    ExecutionResult::Success => "✅",
                    ExecutionResult::PartialSuccess => "⚠️",
                    ExecutionResult::Failure => "❌",
                    ExecutionResult::Timeout => "⏱️",
                    ExecutionResult::Cancelled => "⏹️",
                };
                format!(
                    "{} {} - {}ms",
                    result_emoji,
                    l.action,
                    l.duration_ms.unwrap_or(0)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn format_inbox(&self, messages: &[InboxMessage]) -> String {
        if messages.is_empty() {
            return "收件箱为空".to_string();
        }

        messages
            .iter()
            .take(5)
            .map(|m| {
                let priority_emoji = match m.priority {
                    MessagePriority::Low => "",
                    MessagePriority::Normal => "",
                    MessagePriority::High => "⚡",
                    MessagePriority::Urgent => "🔥",
                };
                let sender_emoji = match m.sender_type {
                    SenderType::User => "👤",
                    SenderType::Chairman => "👔",
                    SenderType::CEO => "💼",
                    SenderType::TeamLead => "👨‍💼",
                    SenderType::Agent => "🤖",
                    SenderType::System => "⚙️",
                };
                format!(
                    "{}{} {} - {}",
                    sender_emoji,
                    priority_emoji,
                    m.sender,
                    m.title
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
    async fn test_agent_dashboard() {
        let cluster_state = Arc::new(ClusterState::new());
        let agent_info = AgentInfo {
            id: "agent-1".to_string(),
            name: "Worker-1".to_string(),
            role: "Developer".to_string(),
            team_id: "team-1".to_string(),
            team_name: "Dev Team".to_string(),
            company_id: "company-1".to_string(),
            company_name: "Test Company".to_string(),
            created_at: Utc::now(),
            status: AgentStatus::Idle,
            tags: vec!["backend".to_string()],
            capabilities: vec!["rust".to_string(), "python".to_string()],
        };

        let dashboard = AgentDashboardManager::new(cluster_state, agent_info);

        // 设置当前任务
        dashboard.set_current_task(TaskDetail {
            id: "task-1".to_string(),
            name: "Implement feature".to_string(),
            description: "Implement a new feature".to_string(),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            estimated_completion: Some(Utc::now() + chrono::Duration::hours(1)),
            progress_percent: 50.0,
            current_step: Some("Coding".to_string()),
            completed_steps: vec!["Design".to_string()],
            pending_steps: vec!["Testing".to_string()],
            dependencies: vec![],
            tags: vec!["feature".to_string()],
        });

        // 接收消息
        dashboard.receive_message(InboxMessage {
            id: "msg-1".to_string(),
            sender: "Team Lead".to_string(),
            sender_type: SenderType::TeamLead,
            message_type: MessageType::Task,
            title: "New Task".to_string(),
            content: "Please review the PR".to_string(),
            sent_at: Utc::now(),
            read: false,
            requires_reply: true,
            priority: MessagePriority::High,
        });

        let data = dashboard.get_dashboard().await;
        assert!(data.current_task.is_some());
        assert_eq!(data.inbox.len(), 1);
        assert_eq!(data.quick_stats.unread_messages, 1);
    }
}