//! 五层看板系统
//! 提供从用户到 Agent 的完整可观测性

pub mod user_dashboard;
pub mod board_dashboard;
pub mod ceo_dashboard;
pub mod team_dashboard;
pub mod agent_dashboard;
pub mod cluster_state;
pub mod company_manager;

// Re-export from cluster_state
pub use cluster_state::{
    ClusterState, ClusterNode, ClusterMetrics, ClusterSummary,
    NodeStatus, NodeType, NodeResourceUsage, CompanySummary,
};

// Re-export from company_manager
pub use company_manager::{
    CompanyManager, CompanyConfig, CompanyType, TeamConfig, TeamType,
    CreateCompanyRequest, CreateTeamRequest, CreateResult,
    CEOConfig, DecisionMode, CollaborationMode,
};

// Re-export from user_dashboard
pub use user_dashboard::{
    UserDashboard, UserDashboardData, InstanceSummary,
    ResourceOverview, TokenUsage, CostUsage, AgentUsage,
    TodayCompletion, CompletedProject, UserAlert, AlertLevel,
    InstanceStatusDisplay,
};

// Re-export from board_dashboard
pub use board_dashboard::{
    BoardDashboard, CompanyOverview, ResourceOverview as BoardResourceOverview,
    ProjectSummary, ProjectStatus, MajorEvent, MajorEventType, 
    EventSeverity, CostAnalysis, CompanyCost, TypeCost, DailyCost,
    BoardDashboardManager,
};

// Re-export from ceo_dashboard
pub use ceo_dashboard::{
    CEODashboard, ProjectDetail, ProjectStatus as CEOProjectStatus,
    ResourceUsageDetail, ApprovalRequest, ApprovalType, Urgency,
    TeamPerformance, CEOAlert, AlertLevel as CEOAlertLevel, 
    CEOAlertType, QuickStats, CEODashboardManager,
};

// Re-export from team_dashboard
pub use team_dashboard::{
    TeamDashboard, ProjectInfo, ProjectStatus as TeamProjectStatus,
    TaskDetail, TaskStatus, TaskPriority, WorkerStatus, 
    WorkerRunningStatus, TeamResourceUsage, KnowledgeEntry, KnowledgeType,
    TeamQuickStats, TeamDashboardManager,
};

// Re-export from agent_dashboard
pub use agent_dashboard::{
    AgentDashboard, AgentInfo, AgentStatus,
    TaskDetail as AgentTaskDetail, TaskStatus as AgentTaskStatus, 
    TaskPriority as AgentTaskPriority,
    TaskSummary, WorkerHealthStatus, HealthStatus,
    HealthAlert, HealthAlertType,
    ExecutionEntry, ExecutionResult, 
    InboxMessage, MessageType, MessagePriority, SenderType,
    AgentQuickStats, AgentDashboardManager,
};