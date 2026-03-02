# MultiClaw 原生多 Agent 集群架构方案（整合版）

> **版本**: v2.0 - 整合版
> **创建日期**: 2026 年 2 月 28 日
> **优先级**: P0 - 核心能力
> **状态**: 待审批
> **整合自**: 
> - `NATIVE_MULTI_AGENT_PLAN.md` (技术实现)
> - `agent 集群框架方案.md` (架构理念)
> - `doc/optimization-01-agent-orchestration.md` (参考)

---

## 一、执行摘要

### 1.1 改造愿景

将 MultiClaw 从**单 Agent 静态配置**架构升级为**原生多 Agent 动态编排**架构，实现：

> **"让 AI 自主组织团队，像经验丰富的项目经理一样管理复杂任务"**

### 1.2 核心特性

| 特性 | 目标值 | 说明 |
|------|--------|------|
| **动态创建** | <10ms/Agent | 运行时按需创建，弹性伸缩 |
| **协作模式** | ≥5 种预定义 | 广撒网/分层审批/专家会诊/混合众包/动态自适应 |
| **自主决策** | 语义匹配 | 主 Agent 根据任务自动选择/组合模式 |
| **并发规模** | ≥1000 并发 | 分布式协调，弹性 Agent 池 |
| **消息吞吐** | >10000 msg/s | 异步非阻塞，背压机制 |
| **沙箱隔离** | Firecracker/Wasm | 安全执行，毫秒级启动 |

### 1.3 三层抽象架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        集群层 (Swarm)                            │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Orchestrator (协调与规划) + Scheduler (调度与资源)      │    │
│  │  - 任务解析                                              │    │
│  │  - 模式选择（自主决策）                                   │    │
│  │  - 工作计划生成                                          │    │
│  │  - 进度监控                                              │    │
│  │  - 弹性 Agent 池管理                                       │    │
│  │  - 资源配额控制                                          │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                        团队层 (Team)                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ 模式实例 1   │  │ 模式实例 2   │  │ 模式实例 N   │             │
│  │ (广撒网)    │  │ (分层审批)  │  │ (专家会诊)  │             │
│  │ - Team Lead │  │ - Architect │  │ - Coordinator│            │
│  │ - N Workers │  │ - Developers│  │ - Experts   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  协作协议：任务分配 / 通信规则 / 依赖管理 / 审批流程              │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│                        个体层 (Agent)                            │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Agent 沙箱 (Firecracker/Wasm)                           │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │    │
│  │  │ 角色配置文件 │  │ Agent 内核    │  │ MCP 客户端    │   │    │
│  │  │ - 专属提示词 │  │ - 任务解析   │  │ - 工具调用   │   │    │
│  │  │ - 可调用工具 │  │ - 记忆管理   │  │ - 文件读写   │   │    │
│  │  │ - 协作协议   │  │ - 通信处理   │  │ - 代码执行   │   │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 1.4 与现有方案对比

| 维度 | 改造前 | 改造后 | 提升 |
|------|--------|--------|------|
| **创建方式** | 配置文件静态定义 | 运行时动态创建 + 弹性池 | 100x 灵活 |
| **协作模式** | 仅 delegate 工具 | 5+ 种可插拔模式 | ∞ 扩展 |
| **决策能力** | 用户手动配置 | AI 自主选择模式 | 智能化 |
| **并发能力** | 单 Agent | ≥1000 并发 | 1000x |
| **隔离安全** | 进程隔离 | Firecracker 微 VM | 企业级 |

---

## 二、架构设计

### 2.1 整体架构（分层 + 事件驱动）

```
┌─────────────────────────────────────────────────────────────────┐
│  用户接口层                                                      │
│  - CLI / HTTP API / Telegram / Discord / Slack                  │
│  - 自然语言任务输入 / 结果输出                                   │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│  协调与规划层 (Orchestrator) - 核心大脑                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ 任务解析器   │  │ 模式选择器   │  │ 工作计划生成 │          │
│  │ - 目标分解   │  │ - 语义匹配   │  │ - DAG 构建     │          │
│  │ - 特征提取   │  │ - 模式组合   │  │ - 资源估算   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ 进度监控器   │  │ 模式生成器   │  │ 质量控制     │          │
│  │ - 状态追踪   │  │ - 动态创建   │  │ - 审批管理   │          │
│  │ - 异常检测   │  │ - 经验学习   │  │ - 验收标准   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│  调度与资源层 (Scheduler)                                        │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ Agent 池管理器 │  │ 资源配额控制 │  │ 负载均衡器   │          │
│  │ - 弹性伸缩   │  │ - Token 限额  │  │ - 地理调度   │          │
│  │ - 热备缓存   │  │ - 并发限制   │  │ - 亲和性调度 │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│  通信层 (Message Bus) - 基于发布/订阅                            │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ 任务队列     │  │ 事件总线     │  │ 点对点信箱   │          │
│  │ - 优先级队列 │  │ - 广播/组播  │  │ - 异步通信   │          │
│  │ - 延迟队列   │  │ - 状态订阅   │  │ - 文件附件   │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│  技术选型：Tokio channels + Redis Stream / Kafka                │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│  Agent 执行层 - 隔离沙箱环境                                      │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ Firecracker │  │ Wasm 沙箱    │  │ Docker      │             │
│  │ (微 VM)     │  │ (轻量级)    │  │ (兼容现有) │             │
│  │ 启动：<100ms│  │ 启动：<10ms │  │ 启动：>1s   │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│  每个 Agent 独立环境，通过 MCP 协议调用工具                        │
└─────────────────────────────────────────────────────────────────┘
                              │
┌─────────────────────────────────────────────────────────────────┐
│  工具与数据层                                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ MCP 服务器    │  │ 状态存储     │  │ 结果缓存     │          │
│  │ - 浏览器     │  │ - etcd       │  │ - Redis      │          │
│  │ - 文件系统   │  │ - SQLite     │  │ - 避免重复  │          │
│  │ - 代码解释器 │  │ - 任务状态   │  │ - 节省成本  │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心模块设计（Rust 实现）

#### 2.2.1 模式库（可插拔协作模式）

```rust
// src/agent/orchestrator/patterns/mod.rs

use serde::{Deserialize, Serialize};

/// 协作模式元数据 - 用于自主决策匹配
#[derive(Clone, Serialize, Deserialize)]
pub struct CollaborationPattern {
    /// 模式唯一标识
    pub id: String,
    /// 模式名称
    pub name: String,
    /// 自然语言描述（用于语义匹配）
    pub description: String,
    /// 适用场景描述
    pub applicable_scenarios: Vec<String>,
    /// 团队结构定义
    pub team_structure: TeamStructure,
    /// 协作协议
    pub collaboration_protocol: CollaborationProtocol,
    /// 生命周期策略
    pub lifecycle: LifecycleStrategy,
    /// 调度策略
    pub scheduling_policy: SchedulingPolicy,
    /// 性能指标（历史统计）
    pub performance_metrics: Option<PatternMetrics>,
}

/// 团队结构定义
#[derive(Clone, Serialize, Deserialize)]
pub struct TeamStructure {
    /// 角色定义列表
    pub roles: Vec<RoleDefinition>,
    /// 最小 Agent 数量
    pub min_agents: usize,
    /// 最大 Agent 数量（可动态伸缩）
    pub max_agents: usize,
    /// 推荐 Agent 数量
    pub recommended_agents: usize,
}

/// 角色定义
#[derive(Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub forbidden_tools: Vec<String>,
    /// 该角色的并发副本数范围
    pub replica_range: (usize, usize),
    /// 是否允许创建子 Agent
    pub can_spawn: bool,
    /// 最大 spawn 深度
    pub max_spawn_depth: u32,
}

/// 协作协议
#[derive(Clone, Serialize, Deserialize)]
pub struct CollaborationProtocol {
    /// 任务分配策略
    pub task_assignment: TaskAssignmentStrategy,
    /// 通信规则
    pub communication_rules: CommunicationRules,
    /// 依赖管理
    pub dependency_management: DependencyManagement,
    /// 审批流程（可选）
    pub approval_workflow: Option<ApprovalWorkflow>,
}

/// 任务分配策略
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAssignmentStrategy {
    /// 竞争领取（适合广撒网）
    Competitive,
    /// 领导者分配（适合分层审批）
    LeaderAssigned,
    /// 轮询分配
    RoundRobin,
    /// 基于负载均衡
    LoadBalanced,
    /// 基于亲和性（专家优先）
    AffinityBased,
}

/// 生命周期策略
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStrategy {
    /// 临时存在（任务结束即解散）
    Ephemeral,
    /// 持久化（可复用）
    Persistent,
    /// 带保活时间
    WithKeepAlive { keep_alive_secs: u64 },
}

/// 调度策略
#[derive(Clone, Serialize, Deserialize)]
pub struct SchedulingPolicy {
    /// 是否允许 Leader 审批子任务
    pub leader_approval: bool,
    /// 是否允许 Worker 自主拆解任务
    pub worker_autonomy: bool,
    /// 最大并行度
    pub max_parallelism: usize,
    /// 超时策略
    pub timeout_policy: TimeoutPolicy,
}
```

#### 2.2.2 模式库预定义模式

```rust
// src/agent/orchestrator/patterns/builtin.rs

use super::*;

/// 预定义协作模式库
pub struct BuiltinPatterns;

impl BuiltinPatterns {
    /// 模式 1: 广撒网并行采集（Kimi 式蜂群）
    pub fn swarm_collection() -> CollaborationPattern {
        CollaborationPattern {
            id: "swarm_collection".to_string(),
            name: "广撒网并行采集".to_string(),
            description: "大规模信息搜集、数据爬取、文档批量处理。高度并行，适合无依赖的重复任务。".to_string(),
            applicable_scenarios: vec![
                "大规模信息搜集".to_string(),
                "数据爬取".to_string(),
                "文档批量处理".to_string(),
                "搜索结果聚合".to_string(),
            ],
            team_structure: TeamStructure {
                roles: vec![
                    RoleDefinition {
                        name: "调度员".to_string(),
                        description: "负责任务拆分和结果汇总".to_string(),
                        system_prompt: "你是一个高效的调度员，擅长将大任务拆分为独立子任务，并汇总结果。".to_string(),
                        allowed_tools: vec!["memory_recall".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (1, 1),
                        can_spawn: true,
                        max_spawn_depth: 2,
                    },
                    RoleDefinition {
                        name: "采集员".to_string(),
                        description: "负责执行具体的采集任务".to_string(),
                        system_prompt: "你是一个勤奋的采集员，专注于高效完成分配的采集任务。".to_string(),
                        allowed_tools: vec!["web_search".to_string(), "web_fetch".to_string()],
                        forbidden_tools: vec!["file_write".to_string()],
                        replica_range: (10, 1000),
                        can_spawn: false,
                        max_spawn_depth: 0,
                    },
                ],
                min_agents: 11,
                max_agents: 1001,
                recommended_agents: 50,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::Competitive,
                communication_rules: CommunicationRules {
                    worker_to_worker: false,
                    worker_to_leader: true,
                    broadcast_enabled: true,
                },
                dependency_management: DependencyManagement::None,
                approval_workflow: None,
            },
            lifecycle: LifecycleStrategy::Ephemeral,
            scheduling_policy: SchedulingPolicy {
                leader_approval: false,
                worker_autonomy: false,
                max_parallelism: 500,
                timeout_policy: TimeoutPolicy::PerTask { timeout_secs: 60 },
            },
            performance_metrics: None,
        }
    }

    /// 模式 2: 分层审批团队（Claude 式开发团队）
    pub fn hierarchical_team() -> CollaborationPattern {
        CollaborationPattern {
            id: "hierarchical_team".to_string(),
            name: "分层审批团队".to_string(),
            description: "软件开发、方案设计、内容创作等需要质量控制的复杂任务。可控性好，质量高。".to_string(),
            applicable_scenarios: vec![
                "软件开发".to_string(),
                "方案设计".to_string(),
                "内容创作".to_string(),
                "需要多角色协作的工程任务".to_string(),
            ],
            team_structure: TeamStructure {
                roles: vec![
                    RoleDefinition {
                        name: "Team Lead".to_string(),
                        description: "负责任务拆解、分配和审批".to_string(),
                        system_prompt: "你是经验丰富的技术负责人，擅长任务拆解、代码审查和质量管理。".to_string(),
                        allowed_tools: vec!["file_read".to_string(), "memory_recall".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (1, 1),
                        can_spawn: true,
                        max_spawn_depth: 3,
                    },
                    RoleDefinition {
                        name: "Developer".to_string(),
                        description: "负责具体开发任务".to_string(),
                        system_prompt: "你是优秀的软件工程师，编写高质量、可维护的代码。".to_string(),
                        allowed_tools: vec!["file_read".to_string(), "file_write".to_string(), "shell".to_string(), "git".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (1, 20),
                        can_spawn: false,
                        max_spawn_depth: 0,
                    },
                    RoleDefinition {
                        name: "QA".to_string(),
                        description: "负责质量测试".to_string(),
                        system_prompt: "你是细致的测试工程师，确保产品质量。".to_string(),
                        allowed_tools: vec!["shell".to_string(), "file_read".to_string()],
                        forbidden_tools: vec!["file_write".to_string()],
                        replica_range: (0, 5),
                        can_spawn: false,
                        max_spawn_depth: 0,
                    },
                ],
                min_agents: 2,
                max_agents: 26,
                recommended_agents: 7,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::LeaderAssigned,
                communication_rules: CommunicationRules {
                    worker_to_worker: true,
                    worker_to_leader: true,
                    broadcast_enabled: true,
                },
                dependency_management: DependencyManagement::DAG,
                approval_workflow: Some(ApprovalWorkflow {
                    required_for: vec!["code_merge".to_string(), "deployment".to_string()],
                    approvers: vec!["Team Lead".to_string()],
                }),
            },
            lifecycle: LifecycleStrategy::WithKeepAlive { keep_alive_secs: 3600 },
            scheduling_policy: SchedulingPolicy {
                leader_approval: true,
                worker_autonomy: true,
                max_parallelism: 10,
                timeout_policy: TimeoutPolicy::PerTask { timeout_secs: 300 },
            },
            performance_metrics: None,
        }
    }

    /// 模式 3: 专家会诊模式
    pub fn expert_consultation() -> CollaborationPattern {
        CollaborationPattern {
            id: "expert_consultation".to_string(),
            name: "专家会诊模式".to_string(),
            description: "复杂问题诊断（如系统故障排查、医疗诊断）。利用多领域知识交叉验证。".to_string(),
            applicable_scenarios: vec![
                "系统故障排查".to_string(),
                "医疗诊断".to_string(),
                "复杂问题诊断".to_string(),
                "多领域知识交叉验证".to_string(),
            ],
            team_structure: TeamStructure {
                roles: vec![
                    RoleDefinition {
                        name: "协调员".to_string(),
                        description: "负责收集症状和综合诊断".to_string(),
                        system_prompt: "你是经验丰富的协调员，擅长整合多方意见给出综合结论。".to_string(),
                        allowed_tools: vec!["memory_recall".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (1, 1),
                        can_spawn: true,
                        max_spawn_depth: 2,
                    },
                    RoleDefinition {
                        name: "领域专家".to_string(),
                        description: "负责特定领域的诊断".to_string(),
                        system_prompt: "你是某个领域的资深专家，提供专业诊断意见。".to_string(),
                        allowed_tools: vec!["web_search".to_string(), "memory_recall".to_string(), "file_read".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (2, 10),
                        can_spawn: false,
                        max_spawn_depth: 0,
                    },
                ],
                min_agents: 3,
                max_agents: 11,
                recommended_agents: 5,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::AffinityBased,
                communication_rules: CommunicationRules {
                    worker_to_worker: false,
                    worker_to_leader: true,
                    broadcast_enabled: true,
                },
                dependency_management: DependencyManagement::Parallel,
                approval_workflow: None,
            },
            lifecycle: LifecycleStrategy::Ephemeral,
            scheduling_policy: SchedulingPolicy {
                leader_approval: false,
                worker_autonomy: true,
                max_parallelism: 10,
                timeout_policy: TimeoutPolicy::PerTask { timeout_secs: 180 },
            },
            performance_metrics: None,
        }
    }

    /// 模式 4: 混合众包模式
    pub fn hybrid_crowdsourcing() -> CollaborationPattern {
        CollaborationPattern {
            id: "hybrid_crowdsourcing".to_string(),
            name: "混合众包模式".to_string(),
            description: "需要创意或多元视角的任务，如产品命名、市场策略头脑风暴。激发多样性，避免思维定势。".to_string(),
            applicable_scenarios: vec![
                "产品命名".to_string(),
                "市场策略头脑风暴".to_string(),
                "创意设计".to_string(),
                "需要多元视角的任务".to_string(),
            ],
            team_structure: TeamStructure {
                roles: vec![
                    RoleDefinition {
                        name: "主持人".to_string(),
                        description: "负责发布问题和评选最佳方案".to_string(),
                        system_prompt: "你是公正的主持人，擅长激发创意和评选最佳方案。".to_string(),
                        allowed_tools: vec!["memory_recall".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (1, 1),
                        can_spawn: true,
                        max_spawn_depth: 1,
                    },
                    RoleDefinition {
                        name: "创意 Agent".to_string(),
                        description: "负责提供创意想法".to_string(),
                        system_prompt: "你是富有创意的思考者，提供独特、多元的视角。".to_string(),
                        allowed_tools: vec!["web_search".to_string(), "memory_recall".to_string()],
                        forbidden_tools: vec![],
                        replica_range: (5, 50),
                        can_spawn: false,
                        max_spawn_depth: 0,
                    },
                ],
                min_agents: 6,
                max_agents: 51,
                recommended_agents: 20,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::Competitive,
                communication_rules: CommunicationRules {
                    worker_to_worker: false,
                    worker_to_leader: true,
                    broadcast_enabled: true,
                },
                dependency_management: DependencyManagement::None,
                approval_workflow: Some(ApprovalWorkflow {
                    required_for: vec!["final_selection".to_string()],
                    approvers: vec!["主持人".to_string(), "user".to_string()],
                }),
            },
            lifecycle: LifecycleStrategy::Ephemeral,
            scheduling_policy: SchedulingPolicy {
                leader_approval: false,
                worker_autonomy: true,
                max_parallelism: 50,
                timeout_policy: TimeoutPolicy::PerTask { timeout_secs: 120 },
            },
            performance_metrics: None,
        }
    }

    /// 模式 5: 动态自适应模式（元模式）
    pub fn dynamic_adaptive() -> CollaborationPattern {
        CollaborationPattern {
            id: "dynamic_adaptive".to_string(),
            name: "动态自适应模式".to_string(),
            description: "当无法匹配任何现有模式时，动态生成新模式。极致的灵活性，面向未知任务。".to_string(),
            applicable_scenarios: vec![
                "未知任务类型".to_string(),
                "需要定制化协作的任务".to_string(),
                "现有模式无法满足的场景".to_string(),
            ],
            team_structure: TeamStructure {
                roles: vec![],
                min_agents: 1,
                max_agents: 100,
                recommended_agents: 5,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::LeaderAssigned,
                communication_rules: CommunicationRules {
                    worker_to_worker: true,
                    worker_to_leader: true,
                    broadcast_enabled: true,
                },
                dependency_management: DependencyManagement::DAG,
                approval_workflow: None,
            },
            lifecycle: LifecycleStrategy::Ephemeral,
            scheduling_policy: SchedulingPolicy {
                leader_approval: true,
                worker_autonomy: true,
                max_parallelism: 20,
                timeout_policy: TimeoutPolicy::Adaptive,
            },
            performance_metrics: None,
        }
    }

    /// 获取所有预定义模式
    pub fn all_patterns() -> Vec<CollaborationPattern> {
        vec![
            Self::swarm_collection(),
            Self::hierarchical_team(),
            Self::expert_consultation(),
            Self::hybrid_crowdsourcing(),
            Self::dynamic_adaptive(),
        ]
    }
}
```

#### 2.2.3 自主决策引擎

```rust
// src/agent/orchestrator/decision.rs

use crate::agent::orchestrator::patterns::*;
use serde_json::Value;

/// 自主决策引擎 - 让主 Agent 自主选择协作模式
pub struct DecisionEngine {
    /// 模式库
    patterns: Vec<CollaborationPattern>,
    /// 任务解析器
    task_parser: TaskParser,
    /// 历史统计数据（用于性能评估）
    historical_stats: PatternStatistics,
}

/// 任务解析结果
#[derive(Debug)]
pub struct TaskAnalysis {
    /// 任务类型
    pub task_type: TaskType,
    /// 复杂度评分 (1-10)
    pub complexity: u8,
    /// 所需领域知识
    pub required_domains: Vec<String>,
    /// 质量要求 (1-10)
    pub quality_requirement: u8,
    /// 预估规模 (子任务数量)
    pub estimated_scale: usize,
    /// 成本预算（Token 上限）
    pub cost_budget: Option<usize>,
    /// 时间预算（秒）
    pub time_budget: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskType {
    InformationGathering,
    ContentGeneration,
    CodeDevelopment,
    DecisionAnalysis,
    CreativeDesign,
    ProblemDiagnosis,
    Other,
}

impl DecisionEngine {
    /// 创建决策引擎
    pub fn new(patterns: Vec<CollaborationPattern>) -> Self {
        Self {
            patterns,
            task_parser: TaskParser::default(),
            historical_stats: PatternStatistics::default(),
        }
    }

    /// 自主选择协作模式
    pub async fn select_pattern(&self, task_description: &str) -> Result<PatternSelection> {
        // 步骤 1: 任务解析
        let analysis = self.task_parser.analyze(task_description).await?;

        // 步骤 2: 模式匹配（向量相似度）
        let mut candidates = Vec::new();
        for pattern in &self.patterns {
            let similarity = self.calculate_similarity(&analysis, pattern);
            if similarity > 0.3 {
                candidates.push((pattern, similarity));
            }
        }
        candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        // 步骤 3: 模式组合（如果单一模式不足）
        if candidates.len() > 1 && analysis.complexity > 7 {
            return self.combine_patterns(&candidates, &analysis);
        }

        // 步骤 4: 模拟评估
        if let Some(best) = candidates.first() {
            let simulation = self.simulate_execution(best.0, &analysis).await?;
            if simulation.score > 0.6 {
                return Ok(PatternSelection::Single(best.0.clone()));
            }
        }

        // 步骤 5: 动态生成新模式
        Ok(PatternSelection::Dynamic(
            self.generate_custom_pattern(&analysis).await?
        ))
    }

    /// 计算任务与模式的相似度
    fn calculate_similarity(&self, analysis: &TaskAnalysis, pattern: &CollaborationPattern) -> f32 {
        let mut score = 0.0;

        // 任务类型匹配
        for scenario in &pattern.applicable_scenarios {
            if self.scenario_matches_task_type(scenario, &analysis.task_type) {
                score += 0.4;
            }
        }

        // 复杂度匹配
        if pattern.team_structure.max_agents >= analysis.estimated_scale {
            score += 0.2;
        }

        // 质量要求匹配
        if pattern.collaboration_protocol.approval_workflow.is_some() 
            && analysis.quality_requirement > 7 {
            score += 0.2;
        }

        // 历史性能
        if let Some(metrics) = &pattern.performance_metrics {
            score += metrics.success_rate * 0.2;
        }

        score.min(1.0)
    }

    /// 模拟执行评估
    async fn simulate_execution(&self, pattern: &CollaborationPattern, analysis: &TaskAnalysis) 
        -> Result<SimulationResult> {
        // 轻量级模拟，估算所需资源、时间、成本
        let estimated_agents = self.estimate_agent_count(pattern, analysis);
        let estimated_time = self.estimate_time(pattern, analysis);
        let estimated_cost = self.estimate_cost(pattern, analysis);

        let mut score = 1.0;

        // 检查是否超出预算
        if let Some(budget) = analysis.cost_budget {
            if estimated_cost > budget {
                score -= 0.3;
            }
        }

        if let Some(time_budget) = analysis.time_budget {
            if estimated_time > time_budget {
                score -= 0.3;
            }
        }

        // 检查并发度
        if estimated_agents > pattern.scheduling_policy.max_parallelism {
            score -= 0.2;
        }

        Ok(SimulationResult {
            score,
            estimated_agents,
            estimated_time,
            estimated_cost,
        })
    }

    /// 动态生成自定义模式
    async fn generate_custom_pattern(&self, analysis: &TaskAnalysis) 
        -> Result<CollaborationPattern> {
        // 调用大模型生成临时模式定义
        // 这里简化实现，实际应调用 LLM
        Ok(CollaborationPattern {
            id: format!("custom_{}", uuid::Uuid::new_v4()),
            name: "自定义模式".to_string(),
            description: format!("为任务动态生成的定制模式：{:?}", analysis.task_type),
            applicable_scenarios: vec!["定制任务".to_string()],
            team_structure: TeamStructure {
                roles: self.generate_roles_for_task(analysis).await?,
                min_agents: 1,
                max_agents: 50,
                recommended_agents: 5,
            },
            collaboration_protocol: CollaborationProtocol {
                task_assignment: TaskAssignmentStrategy::LeaderAssigned,
                communication_rules: CommunicationRules::default(),
                dependency_management: DependencyManagement::DAG,
                approval_workflow: None,
            },
            lifecycle: LifecycleStrategy::Ephemeral,
            scheduling_policy: SchedulingPolicy::default(),
            performance_metrics: None,
        })
    }
}

/// 模式选择结果
#[derive(Clone)]
pub enum PatternSelection {
    /// 单一模式
    Single(CollaborationPattern),
    /// 模式组合
    Combined(Vec<CollaborationPattern>),
    /// 动态生成
    Dynamic(CollaborationPattern),
}
```

---

## 三、大规模集群管理策略

### 3.1 弹性 Agent 池

```rust
// src/agent/scheduler/pool.rs

pub struct AgentPool {
    /// 活跃 Agent
    active_agents: DashMap<String, AgentHandle>,
    /// 热备 Agent（按角色分类）
    warm_standby: DashMap<String, Vec<AgentHandle>>,
    /// 角色定义
    role_definitions: DashMap<String, RoleDefinition>,
    /// 配置
    config: PoolConfig,
}

pub struct PoolConfig {
    /// 最小热备数量（按角色）
    pub min_standby_per_role: usize,
    /// 最大热备数量
    pub max_standby_total: usize,
    /// 空闲超时（秒）
    pub idle_timeout_secs: u64,
    /// 启动超时（秒）
    pub startup_timeout_secs: u64,
}

impl AgentPool {
    /// 获取或创建 Agent
    pub async fn acquire(&self, role: &str) -> Result<AgentHandle> {
        // 1. 尝试从热备池获取
        if let Some(standby) = self.warm_standby.get_mut(role) {
            if let Some(agent) = standby.pop() {
                return Ok(agent);
            }
        }

        // 2. 动态创建（Firecracker/Wasm）
        let agent = self.spawn_agent(role).await?;
        
        // 3. 补充热备池
        self.replenish_standby(role).await?;

        Ok(agent)
    }

    /// 释放 Agent（返回热备池或销毁）
    pub async fn release(&self, agent: AgentHandle) {
        // 检查是否超出热备上限
        let total_standby: usize = self.warm_standby.iter()
            .map(|r| r.value().len())
            .sum();

        if total_standby < self.config.max_standby_total {
            // 返回热备池
            let role = agent.role.clone();
            if let Some(mut standby) = self.warm_standby.get_mut(&role) {
                standby.push(agent);
            }
        } else {
            // 销毁
            self.destroy_agent(agent).await;
        }
    }

    /// 动态创建 Agent（Firecracker 微 VM）
    async fn spawn_agent(&self, role: &str) -> Result<AgentHandle> {
        let start = Instant::now();
        
        // 1. 启动 Firecracker 微 VM（<100ms）
        let vm = FirecrackerVM::spawn(&self.get_vm_config(role)).await?;
        
        // 2. 初始化 Agent 内核
        let agent = AgentHandle::new(vm, role.to_string());
        
        // 3. 注册到活跃池
        self.active_agents.insert(agent.id.clone(), agent.clone());

        tracing::info!(
            agent_id = %agent.id,
            role = %role,
            startup_ms = %start.elapsed().as_millis(),
            "Agent spawned"
        );

        Ok(agent)
    }

    /// 补充热备池
    async fn replenish_standby(&self, role: &str) -> Result<()> {
        let current = self.warm_standby.get(role).map(|r| r.len()).unwrap_or(0);
        let target = self.config.min_standby_per_role;

        if current < target {
            for _ in current..target {
                let agent = self.spawn_agent(role).await?;
                if let Some(mut standby) = self.warm_standby.get_mut(role) {
                    standby.push(agent);
                }
            }
        }

        Ok(())
    }
}
```

### 3.2 分布式协调

```rust
// src/agent/scheduler/distributed.rs

use etcd_client::{Client, PutOptions};

pub struct DistributedCoordinator {
    /// etcd 客户端
    etcd: Client,
    /// 本节点 ID
    node_id: String,
    /// 选主状态
    leader_state: LeaderState,
}

enum LeaderState {
    Leader,
    Follower { leader_key: String },
    Candidate,
}

impl DistributedCoordinator {
    /// 初始化（参与选主）
    pub async fn init(etcd_endpoints: &[String]) -> Result<Self> {
        let client = Client::connect(etcd_endpoints, None).await?;
        let node_id = uuid::Uuid::new_v4().to_string();

        let mut coordinator = Self {
            etcd: client,
            node_id,
            leader_state: LeaderState::Candidate,
        };

        // 参与选主
        coordinator.join_leader_election().await?;

        Ok(coordinator)
    }

    /// 注册 Agent 状态（全局可见）
    pub async fn register_agent(&self, agent: &AgentInfo) -> Result<()> {
        let key = format!("/multiclaw/agents/{}", agent.id);
        let value = serde_json::to_string(agent)?;
        
        self.etcd.put(key, value, None).await?;
        Ok(())
    }

    /// 更新任务状态
    pub async fn update_task(&self, task_id: &str, state: TaskState) -> Result<()> {
        let key = format!("/multiclaw/tasks/{}", task_id);
        let value = serde_json::to_string(&state)?;
        
        self.etcd.put(key, value, None).await?;
        Ok(())
    }

    /// 心跳保活
    pub async fn heartbeat(&self) -> Result<()> {
        let key = format!("/multiclaw/nodes/{}", self.node_id);
        let value = serde_json::json!({
            "timestamp": chrono::Utc::now().timestamp(),
            "status": "healthy"
        });

        // 带 TTL 的键（60 秒过期）
        let lease = self.etcd.grant(60).await?;
        self.etcd.put(key, value, Some(PutOptions::new().with_lease(lease.id()))).await?;

        Ok(())
    }
}
```

### 3.3 任务分片与合并（MapReduce 模式）

```rust
// src/agent/orchestrator/mapreduce.rs

pub struct MapReduceEngine {
    scheduler: Arc<Scheduler>,
    bus: Arc<MessageBus>,
}

impl MapReduceEngine {
    /// 执行 MapReduce 任务
    pub async fn execute(&self, task: MapReduceTask) -> Result<Value> {
        // Map 阶段：分片
        let shards = self.split_task(&task)?;
        
        // 创建大量 Worker Agent
        let mut handles = Vec::new();
        for shard in shards {
            let agent = self.scheduler.acquire("worker").await?;
            handles.push(self.execute_map(agent, shard));
        }

        // 等待所有 Map 完成
        let map_results = futures::future::join_all(handles).await;
        
        // Reduce 阶段：合并
        let final_result = self.reduce(map_results, &task.reduce_function).await?;

        Ok(final_result)
    }

    /// 分片策略
    fn split_task(&self, task: &MapReduceTask) -> Result<Vec<TaskShard>> {
        // 根据数据量智能分片
        let shard_size = self.calculate_optimal_shard_size(task.total_size);
        let num_shards = (task.total_size + shard_size - 1) / shard_size;

        let mut shards = Vec::new();
        for i in 0..num_shards {
            shards.push(TaskShard {
                id: i,
                offset: i * shard_size,
                size: shard_size.min(task.total_size - i * shard_size),
            });
        }

        Ok(shards)
    }

    /// 计算最优分片大小
    fn calculate_optimal_shard_size(&self, total_size: usize) -> usize {
        // 启发式算法：目标 100-1000 个分片
        let target_shards = 500;
        (total_size + target_shards - 1) / target_shards
    }
}
```

---

## 四、与现有系统集成

### 4.1 Delegate 工具升级

```rust
// src/tools/delegate.rs - 改造后

use crate::agent::orchestrator::{AgentOrchestrator, PatternSelection};

pub struct DelegateTool {
    orchestrator: Arc<AgentOrchestrator>,
}

impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "委托任务给子 Agent 或 Agent 团队。支持：
        1. 使用预定义协作模式（如广撒网、分层审批）
        2. 动态创建自定义团队
        3. 指定特定 Agent 角色"
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let task = params["task"].as_str().ok_or("task required")?;
        
        // 模式 1: 自主决策（默认）
        if params["auto"].as_bool().unwrap_or(true) {
            let selection = self.orchestrator.select_pattern(task).await?;
            return self.orchestrator.execute_with_pattern(task, selection).await;
        }

        // 模式 2: 指定模式
        if let Some(pattern_name) = params["pattern"].as_str() {
            let pattern = self.orchestrator.get_pattern(pattern_name)?;
            return self.orchestrator.execute_with_pattern(task, PatternSelection::Single(pattern)).await;
        }

        // 模式 3: 指定 Agent 角色
        if let Some(role) = params["role"].as_str() {
            let agent = self.orchestrator.spawn_agent(role).await?;
            return self.orchestrator.send(&agent.id, task).await;
        }

        Err("必须指定 auto=true 或 pattern 或 role".into())
    }
}
```

### 4.2 MCP 协议集成

```rust
// src/tools/mcp_client.rs

use mcp_sdk::{Client, ToolDefinition};

pub struct MCPAgentEnvironment {
    /// MCP 客户端
    mcp_client: Client,
    /// 挂载的工具
    available_tools: Vec<ToolDefinition>,
    /// 沙箱环境
    sandbox: SandboxHandle,
}

impl MCPAgentEnvironment {
    /// 创建 Agent 环境（带工具挂载）
    pub async fn create(role: &RoleDefinition) -> Result<Self> {
        // 1. 创建沙箱（Firecracker/Wasm）
        let sandbox = SandboxHandle::create().await?;

        // 2. 连接 MCP 服务器
        let mcp_client = Client::connect("unix:///var/run/mcp.sock").await?;

        // 3. 根据角色挂载工具
        let mut available_tools = Vec::new();
        for tool_name in &role.allowed_tools {
            let tool = mcp_client.get_tool(tool_name).await?;
            available_tools.push(tool);
        }

        Ok(Self {
            mcp_client,
            available_tools,
            sandbox,
        })
    }

    /// 调用工具
    pub async fn call_tool(&self, tool_name: &str, args: Value) -> Result<Value> {
        // 检查权限
        if !self.available_tools.iter().any(|t| t.name == tool_name) {
            return Err(format!("工具 {} 未挂载", tool_name).into());
        }

        // 通过 MCP 调用
        let result = self.mcp_client.call_tool(tool_name, args).await?;
        Ok(result)
    }
}
```

---

## 五、实现计划（12 周）

| 阶段 | 内容 | 工期 | 里程碑 |
|------|------|------|--------|
| **Phase 1** | 模式库 + 自主决策引擎 | 2 周 | M1: 可自主选择模式 |
| **Phase 2** | Agent Registry + 弹性池 | 2 周 | M2: 动态创建<10ms |
| **Phase 3** | Message Bus (Redis/Kafka) | 1 周 | M3: 消息>10000 msg/s |
| **Phase 4** | Context Manager | 1 周 | M4: 上下文共享正常 |
| **Phase 5** | Firecracker 沙箱集成 | 2 周 | M5: 沙箱启动<100ms |
| **Phase 6** | MCP 协议集成 | 1 周 | M6: 工具挂载正常 |
| **Phase 7** | 分布式协调 (etcd) | 1 周 | M7: 多节点协调 |
| **Phase 8** | MapReduce 引擎 | 1 周 | M8: 千级并发 |
| **Phase 9** | CLI + HTTP API | 1 周 | M9: CLI/API 可用 |
| **Phase 10** | 测试 + 文档 | 2 周 | M10: 测试覆盖>80% |

**总计**: 12 周

---

## 六、验收标准

### 6.1 功能验收

- [ ] 5 种预定义协作模式可用
- [ ] 自主决策引擎可正确匹配模式
- [ ] 动态创建 Agent <10ms
- [ ] 支持≥1000 并发 Agent
- [ ] 消息传递延迟 <1ms
- [ ] Firecracker 沙箱启动 <100ms
- [ ] MCP 工具挂载正常
- [ ] 分布式协调正常（多节点）

### 6.2 性能验收

| 指标 | 目标值 | 测试方法 |
|------|--------|---------|
| Agent 创建延迟 | <10ms | 基准测试 |
| 消息吞吐量 | >10000 msg/s | 压力测试 |
| 并发 Agent 数 | ≥1000 | 负载测试 |
| 沙箱启动时间 | <100ms | 基准测试 |
| 内存占用 | <50MB/Agent | 资源监控 |

### 6.3 质量验收

- [ ] 单元测试覆盖 >80%
- [ ] 集成测试通过
- [ ] 性能测试达标
- [ ] 文档完整（API 文档 + 用户指南 + 模式库说明）
- [ ] 示例模式 ≥5 个
- [ ] 示例工作流 ≥10 个

---

## 七、风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **循环 spawn 导致资源耗尽** | 高 | 中 | 最大深度检查 + 配额限制 + 实时监控告警 |
| **消息总线成为瓶颈** | 高 | 中 | Redis Cluster/Kafka + 背压机制 + 分片 |
| **沙箱逃逸** | 高 | 低 | Firecracker 微 VM + seccomp + 定期安全审计 |
| **etcd 单点故障** | 高 | 低 | etcd 集群（3 节点）+ 自动故障转移 |
| **模式匹配不准确** | 中 | 高 | 人工反馈循环 + 持续优化 + 支持手动覆盖 |
| **大规模任务 OOM** | 高 | 中 | 流式处理 + 分批次 + 内存限制 |

---

## 八、总结

本方案整合了两个原始方案的优点：

### 核心优势

1. **三层抽象架构** - 个体/团队/集群，清晰分离关注点
2. **可插拔模式库** - 5 种预定义模式 + 动态生成，灵活适配各种场景
3. **自主决策引擎** - 让 AI 自主选择协作模式，像项目经理一样思考
4. **弹性 Agent 池** - Firecracker 微 VM，毫秒级启动，千级并发
5. **分布式协调** - etcd 全局状态，多节点高可用
6. **MCP 协议集成** - 标准化接口，工具即插即用

### 与竞品对比

| 特性 | MultiClaw | Kimi | Claude |
|------|----------|------|--------|
| **动态创建** | ✅ <10ms | ✅ | ✅ |
| **协作模式** | ✅ 5+ 种 | ✅ 蜂群 | ✅ 团队 |
| **自主决策** | ✅ 语义匹配 | ❌ | ❌ |
| **沙箱隔离** | ✅ Firecracker | ❓ | ❓ |
| **分布式** | ✅ etcd | ❓ | ❓ |
| **MCP 协议** | ✅ | ❌ | ❌ |

**MultiClaw 通过本方案可实现业界领先的多 Agent 集群能力！**

---

**审批状态**: 待审批  
**负责人**: 待定  
**最后更新**: 2026 年 2 月 28 日
