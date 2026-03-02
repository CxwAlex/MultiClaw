//! ClusterState - 集群状态管理
//! 提供整个集群的实时状态视图

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 集群节点状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNode {
    /// 节点 ID
    pub id: String,
    /// 节点名称
    pub name: String,
    /// 节点类型
    pub node_type: NodeType,
    /// 运行状态
    pub status: NodeStatus,
    /// 实例 ID（如果是公司节点）
    pub instance_id: Option<String>,
    /// CEO Agent ID
    pub ceo_agent_id: Option<String>,
    /// 绑定的通信通道
    pub channel: Option<String>,
    /// 资源使用情况
    pub resource_usage: NodeResourceUsage,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
    /// 标签
    pub labels: std::collections::HashMap<String, String>,
}

/// 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    /// 董事长节点（用户分身）
    Chairman,
    /// 公司节点（实例）
    Company,
    /// 团队节点
    Team,
    /// Agent 节点
    Agent,
}

/// 节点运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 空闲
    Idle,
    /// 忙碌
    Busy,
    /// 不健康
    Unhealthy,
    /// 恢复中
    Recovering,
    /// 恢复失败
    RecoveryFailed,
    /// 已停止
    Stopped,
}

/// 节点资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResourceUsage {
    /// CPU 使用率 (%)
    pub cpu_percent: f64,
    /// 内存使用率 (%)
    pub memory_percent: f64,
    /// Token 使用量
    pub tokens_used: u64,
    /// Token 配额
    pub tokens_quota: u64,
    /// 活跃任务数
    pub active_tasks: usize,
    /// 完成任务数
    pub completed_tasks: usize,
}

impl Default for NodeResourceUsage {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            memory_percent: 0.0,
            tokens_used: 0,
            tokens_quota: 0,
            active_tasks: 0,
            completed_tasks: 0,
        }
    }
}

/// 集群整体指标
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterMetrics {
    /// 总节点数
    pub total_nodes: usize,
    /// 运行中节点数
    pub running_nodes: usize,
    /// 公司（实例）数量
    pub total_companies: usize,
    /// 团队数量
    pub total_teams: usize,
    /// Agent 数量
    pub total_agents: usize,
    /// 总 Token 配额
    pub total_token_quota: u64,
    /// 已使用 Token
    pub total_token_used: u64,
    /// 活跃任务数
    pub active_tasks: usize,
    /// 今日完成任务数
    pub tasks_completed_today: usize,
    /// 整体健康度 (0-100)
    pub health_score: f32,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 集群状态
pub struct ClusterState {
    /// 所有节点
    nodes: DashMap<String, ClusterNode>,
    /// 节点父子关系（parent_id -> [child_ids]）
    hierarchy: DashMap<String, Vec<String>>,
    /// 集群指标
    metrics: Arc<RwLock<ClusterMetrics>>,
    /// Token 使用计数器
    token_used: AtomicU64,
    /// Token 配额计数器
    token_quota: AtomicU64,
}

impl ClusterState {
    /// 创建新的集群状态
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            hierarchy: DashMap::new(),
            metrics: Arc::new(RwLock::new(ClusterMetrics::default())),
            token_used: AtomicU64::new(0),
            token_quota: AtomicU64::new(0),
        }
    }

    /// 注册节点
    pub fn register_node(&self, node: ClusterNode) {
        let node_id = node.id.clone();
        self.nodes.insert(node_id.clone(), node);
        self.update_metrics();
    }

    /// 注销节点
    pub fn unregister_node(&self, node_id: &str) {
        self.nodes.remove(node_id);
        self.hierarchy.remove(node_id);
        // 从所有父子关系中移除
        for mut entry in self.hierarchy.iter_mut() {
            entry.value_mut().retain(|id| id != node_id);
        }
        self.update_metrics();
    }

    /// 获取节点
    pub fn get_node(&self, node_id: &str) -> Option<ClusterNode> {
        self.nodes.get(node_id).map(|n| n.clone())
    }

    /// 更新节点状态
    pub fn update_node_status(&self, node_id: &str, status: NodeStatus) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.status = status;
            node.last_active_at = Utc::now();
        }
        self.update_metrics();
    }

    /// 更新节点资源使用
    pub fn update_node_resources(&self, node_id: &str, usage: NodeResourceUsage) {
        if let Some(mut node) = self.nodes.get_mut(node_id) {
            node.resource_usage = usage;
            node.last_active_at = Utc::now();
        }
    }

    /// 设置父子关系
    pub fn set_parent(&self, child_id: &str, parent_id: &str) {
        // 从旧父节点移除
        for mut entry in self.hierarchy.iter_mut() {
            entry.value_mut().retain(|id| id != child_id);
        }
        
        // 添加到新父节点
        self.hierarchy
            .entry(parent_id.to_string())
            .or_default()
            .push(child_id.to_string());
    }

    /// 获取子节点
    pub fn get_children(&self, parent_id: &str) -> Vec<ClusterNode> {
        if let Some(child_ids) = self.hierarchy.get(parent_id) {
            child_ids
                .iter()
                .filter_map(|id| self.nodes.get(id).map(|n| n.clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取父节点
    pub fn get_parent(&self, child_id: &str) -> Option<ClusterNode> {
        for entry in self.hierarchy.iter() {
            if entry.value().contains(&child_id.to_string()) {
                return self.nodes.get(entry.key()).map(|n| n.clone());
            }
        }
        None
    }

    /// 获取所有公司节点
    pub fn get_companies(&self) -> Vec<ClusterNode> {
        self.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Company)
            .map(|n| n.clone())
            .collect()
    }

    /// 获取所有团队节点
    pub fn get_teams(&self, company_id: Option<&str>) -> Vec<ClusterNode> {
        let teams: Vec<_> = self.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Team)
            .map(|n| n.clone())
            .collect();

        match company_id {
            Some(cid) => {
                let child_ids = self.hierarchy.get(cid).map(|v| v.clone()).unwrap_or_default();
                teams.into_iter()
                    .filter(|t| child_ids.contains(&t.id))
                    .collect()
            }
            None => teams,
        }
    }

    /// 获取所有 Agent 节点
    pub fn get_agents(&self, team_id: Option<&str>) -> Vec<ClusterNode> {
        let agents: Vec<_> = self.nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Agent)
            .map(|n| n.clone())
            .collect();

        match team_id {
            Some(tid) => {
                let child_ids = self.hierarchy.get(tid).map(|v| v.clone()).unwrap_or_default();
                agents.into_iter()
                    .filter(|a| child_ids.contains(&a.id))
                    .collect()
            }
            None => agents,
        }
    }

    /// 增加 Token 使用量
    pub fn add_token_usage(&self, tokens: u64) {
        self.token_used.fetch_add(tokens, Ordering::Relaxed);
    }

    /// 设置 Token 配额
    pub fn set_token_quota(&self, quota: u64) {
        self.token_quota.store(quota, Ordering::Relaxed);
    }

    /// 获取集群指标
    pub async fn get_metrics(&self) -> ClusterMetrics {
        self.metrics.read().await.clone()
    }

    /// 更新集群指标
    fn update_metrics(&self) {
        let nodes: Vec<_> = self.nodes.iter().map(|n| n.clone()).collect();
        
        let total_nodes = nodes.len();
        let running_nodes = nodes.iter().filter(|n| n.status == NodeStatus::Running).count();
        let total_companies = nodes.iter().filter(|n| n.node_type == NodeType::Company).count();
        let total_teams = nodes.iter().filter(|n| n.node_type == NodeType::Team).count();
        let total_agents = nodes.iter().filter(|n| n.node_type == NodeType::Agent).count();
        
        let total_token_used = self.token_used.load(Ordering::Relaxed);
        let total_token_quota = self.token_quota.load(Ordering::Relaxed);
        
        let active_tasks: usize = nodes.iter().map(|n| n.resource_usage.active_tasks).sum();
        let tasks_completed_today: usize = nodes.iter().map(|n| n.resource_usage.completed_tasks).sum();
        
        // 计算健康度
        let health_score = if total_nodes == 0 {
            100.0
        } else {
            let healthy_ratio = running_nodes as f32 / total_nodes as f32;
            let token_ratio = if total_token_quota > 0 {
                1.0 - (total_token_used as f32 / total_token_quota as f32).min(1.0)
            } else {
                1.0
            };
            (healthy_ratio * 0.6 + token_ratio * 0.4) * 100.0
        };

        if let Ok(mut metrics) = self.metrics.try_write() {
            *metrics = ClusterMetrics {
                total_nodes,
                running_nodes,
                total_companies,
                total_teams,
                total_agents,
                total_token_quota,
                total_token_used,
                active_tasks,
                tasks_completed_today,
                health_score,
                last_updated: Utc::now(),
            };
        }
    }

    /// 获取集群摘要（用于用户看板）
    pub async fn get_cluster_summary(&self) -> ClusterSummary {
        let metrics = self.get_metrics().await;
        let companies = self.get_companies();

        ClusterSummary {
            total_companies: metrics.total_companies,
            running_companies: companies.iter()
                .filter(|c| c.status == NodeStatus::Running)
                .count(),
            busy_companies: companies.iter()
                .filter(|c| c.status == NodeStatus::Busy)
                .count(),
            total_teams: metrics.total_teams,
            total_agents: metrics.total_agents,
            token_usage_percent: if metrics.total_token_quota > 0 {
                (metrics.total_token_used as f64 / metrics.total_token_quota as f64) * 100.0
            } else {
                0.0
            },
            health_score: metrics.health_score,
            companies: companies.into_iter().map(|c| CompanySummary {
                id: c.id,
                name: c.name,
                status: c.status,
                active_tasks: c.resource_usage.active_tasks,
                token_usage_percent: if c.resource_usage.tokens_quota > 0 {
                    (c.resource_usage.tokens_used as f64 / c.resource_usage.tokens_quota as f64) * 100.0
                } else {
                    0.0
                },
            }).collect(),
        }
    }
}

impl Default for ClusterState {
    fn default() -> Self {
        Self::new()
    }
}

/// 集群摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSummary {
    /// 总公司数
    pub total_companies: usize,
    /// 运行中的公司数
    pub running_companies: usize,
    /// 忙碌的公司数
    pub busy_companies: usize,
    /// 总团队数
    pub total_teams: usize,
    /// 总 Agent 数
    pub total_agents: usize,
    /// Token 使用百分比
    pub token_usage_percent: f64,
    /// 健康度
    pub health_score: f32,
    /// 公司列表
    pub companies: Vec<CompanySummary>,
}

/// 公司摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanySummary {
    pub id: String,
    pub name: String,
    pub status: NodeStatus,
    pub active_tasks: usize,
    pub token_usage_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_state_basic() {
        let cluster = ClusterState::new();

        // 注册公司节点
        let company = ClusterNode {
            id: "company-1".to_string(),
            name: "Test Company".to_string(),
            node_type: NodeType::Company,
            status: NodeStatus::Running,
            instance_id: Some("inst-1".to_string()),
            ceo_agent_id: Some("ceo-1".to_string()),
            channel: Some("telegram:@TestBot".to_string()),
            resource_usage: NodeResourceUsage {
                cpu_percent: 45.0,
                memory_percent: 60.0,
                tokens_used: 1000,
                tokens_quota: 5000,
                active_tasks: 3,
                completed_tasks: 10,
            },
            created_at: Utc::now(),
            last_active_at: Utc::now(),
            labels: std::collections::HashMap::new(),
        };

        cluster.register_node(company.clone());
        
        let retrieved = cluster.get_node("company-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Company");

        // 注册团队节点
        let team = ClusterNode {
            id: "team-1".to_string(),
            name: "Development Team".to_string(),
            node_type: NodeType::Team,
            status: NodeStatus::Running,
            instance_id: None,
            ceo_agent_id: None,
            channel: None,
            resource_usage: NodeResourceUsage::default(),
            created_at: Utc::now(),
            last_active_at: Utc::now(),
            labels: std::collections::HashMap::new(),
        };

        cluster.register_node(team.clone());
        cluster.set_parent("team-1", "company-1");

        let children = cluster.get_children("company-1");
        assert_eq!(children.len(), 1);

        // 获取指标
        let metrics = cluster.get_metrics().await;
        assert_eq!(metrics.total_companies, 1);
        assert_eq!(metrics.total_teams, 1);
    }
}