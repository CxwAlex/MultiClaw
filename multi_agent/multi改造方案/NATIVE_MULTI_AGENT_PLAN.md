# MultiClaw 原生多 Agent 集群改造方案

> **版本**: v1.0
> **创建日期**: 2026 年 2 月 28 日
> **优先级**: P0 - 核心能力
> **状态**: 待审批
> **参考**: `doc/optimization-01-agent-orchestration.md`

---

## 一、执行摘要

### 1.1 改造目标

将 MultiClaw 从**单 Agent 静态配置**架构升级为**原生多 Agent 动态编排**架构，支持：

- ✅ 运行时动态创建/销毁 Agent
- ✅ N 层嵌套 Agent 树（可配置深度）
- ✅ Agent 间高效通信（消息总线）
- ✅ 共享上下文和记忆（可配置隔离）
- ✅ 工作流引擎（DAG/状态机）
- ✅ 高并发支持（≥100 并发 Agent）

### 1.2 核心价值

| 维度 | 改造前 | 改造后 |
|------|--------|--------|
| **创建方式** | 配置文件静态定义 | 运行时动态创建 |
| **层级限制** | max_depth=2 硬限制 | 可配置（默认 5 层） |
| **通信机制** | 仅 delegate 工具 | 消息总线（pub/sub + P2P） |
| **上下文** | 独立会话 | 可选共享/继承 |
| **工作流** | 无 | DAG/状态机引擎 |
| **并发能力** | 单 Agent | ≥100 并发 Agent |

### 1.3 改造范围

```
修改模块:
  ✅ src/agent/orchestrator/      # 新增：编排核心模块
  ✅ src/tools/delegate.rs        # 改造：集成编排系统
  ✅ src/channels/mod.rs          # 改造：支持多 Agent 路由
  ✅ src/memory/mod.rs            # 改造：支持上下文隔离/共享
  ✅ src/config/schema.rs         # 改造：新增编排配置
  ✅ src/cli/                     # 新增：Agent 管理命令
  ✅ src/http/                    # 新增：HTTP API

新增目录:
  ✅ src/agent/orchestrator/
     ├── mod.rs          # 编排系统入口
     ├── registry.rs     # Agent 注册表
     ├── bus.rs          # 消息总线
     ├── context.rs      # 上下文管理
     ├── workflow.rs     # 工作流引擎
     ├── template.rs     # Agent 模板
     └── quotas.rs       # 资源配额

  ✅ src/cli/agent/
     ├── mod.rs          # agent 命令入口
     ├── spawn.rs        # spawn 命令
     ├── terminate.rs    # terminate 命令
     ├── list.rs         # list 命令
     └── status.rs       # status 命令

  ✅ workflows/           # 工作流定义目录
     └── examples/
```

---

## 二、架构设计

### 2.1 整体架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        MultiClaw Runtime                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │              Agent Orchestrator (新增)                      │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │   Registry   │  │  Message Bus │  │   Context    │     │ │
│  │  │  注册表      │  │  消息总线    │  │  上下文管理  │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │   Template   │  │   Workflow   │  │   Quotas     │     │ │
│  │  │  模板管理    │  │  工作流引擎  │  │  资源配额    │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                              │                                   │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │                  Existing Modules (改造)                    │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │   Delegate   │  │   Channels   │  │   Memory     │     │ │
│  │  │  委托工具    │  │  通信频道    │  │  记忆系统    │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │ │
│  │  │    Tools     │  │   Providers  │  │   Config     │     │ │
│  │  │  工具系统    │  │  模型提供商  │  │  配置系统    │     │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 核心模块设计

#### 2.2.1 Agent Registry（注册表）

**职责**: Agent 生命周期管理、层级关系追踪、资源配额控制

```rust
// src/agent/orchestrator/registry.rs

use dashmap::DashMap;
use tokio::sync::RwLock;
use std::sync::Arc;
use uuid::Uuid;

/// Agent 注册表 - 管理所有 Agent 实例
pub struct AgentRegistry {
    /// Agent 实例存储：agent_id -> AgentInstance
    agents: DashMap<String, AgentInstance>,
    /// Agent 模板：template_name -> AgentTemplate
    templates: Arc<DashMap<String, AgentTemplate>>,
    /// 父子关系：parent_id -> [child_ids]
    hierarchy: DashMap<String, Vec<String>>,
    /// 资源配额管理器
    quotas: Arc<ResourceQuotas>,
    /// 配置
    config: RegistryConfig,
}

/// Agent 实例
pub struct AgentInstance {
    pub id: String,
    pub template_name: String,
    pub parent_id: Option<String>,
    pub child_ids: Vec<String>,
    pub session: Session,
    pub state: AgentState,
    pub context_id: Option<String>,
    pub created_at: Instant,
    pub resource_usage: ResourceUsage,
}

/// Agent 模板 - 用于动态创建
#[derive(Clone, Serialize, Deserialize)]
pub struct AgentTemplate {
    pub name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: String,
    pub allowed_tools: Vec<String>,
    pub max_iterations: u32,
    pub temperature: Option<f64>,
    
    /// 上下文继承配置
    pub inherit_context: bool,      // 是否继承父 Agent 上下文
    pub inherit_memory: bool,       // 是否共享记忆
    pub context_share_ratio: f32,   // 历史消息共享比例 (0.0-1.0)
    
    /// 权限配置
    pub can_spawn: bool,            // 是否允许创建子 Agent
    pub max_spawn_depth: u32,       // 最大 spawn 深度
    pub forbidden_tools: Vec<String>, // 禁止使用的工具
}

impl AgentRegistry {
    /// 从配置初始化
    pub async fn from_config(config: OrchestrationConfig) -> Result<Self>;
    
    /// 动态创建子 Agent
    pub async fn spawn(
        &self,
        parent_id: &str,
        template: AgentTemplate,
        context_config: Option<ContextConfig>,
    ) -> Result<String>;
    
    /// 销毁 Agent 及其子树
    pub async fn terminate(&self, agent_id: &str, graceful: bool) -> Result<()>;
    
    /// 获取层级关系
    pub fn get_hierarchy(&self, agent_id: &str) -> AgentHierarchy;
    
    /// 检查配额
    pub fn check_quota(&self, agent_id: &str, action: &str) -> QuotaResult;
}
```

**关键特性**:
- ✅ 使用 `DashMap` 实现无锁并发访问
- ✅ 支持优雅终止（等待当前任务完成）和强制终止
- ✅ 自动追踪父子关系，支持树形查询
- ✅ 集成配额检查，防止资源耗尽

---

#### 2.2.2 Message Bus（消息总线）

**职责**: Agent 间通信、事件广播、发布订阅

```rust
// src/agent/orchestrator/bus.rs

use tokio::sync::{broadcast, mpsc, watch};
use dashmap::DashMap;

pub struct AgentMessageBus {
    /// 广播通道：发送 AgentEvent
    broadcast_tx: broadcast::Sender<AgentEvent>,
    /// P2P 通道：agent_id -> Sender<Message>
    p2p_channels: DashMap<String, mpsc::Sender<Message>>,
    /// 订阅关系：event_type -> [agent_ids]
    subscriptions: DashMap<EventType, Vec<String>>,
    /// 状态观察器：agent_id -> watch::Sender<AgentState>
    state_watchers: DashMap<String, watch::Sender<AgentState>>,
    config: BusConfig,
}

/// Agent 事件类型
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// 任务委托
    Delegate {
        from: String,
        to: String,
        task_id: String,
        task: String,
    },
    /// 任务完成
    Completed {
        agent_id: String,
        task_id: String,
        result: TaskResult,
        duration_ms: u64,
    },
    /// 状态变更
    StateChanged {
        agent_id: String,
        old_state: AgentState,
        new_state: AgentState,
    },
    /// 上下文更新
    ContextUpdated {
        agent_id: String,
        context_delta: ContextDelta,
    },
    /// 资源告警
    ResourceAlert {
        agent_id: String,
        resource: String,
        usage: f32,
        limit: f32,
    },
}

/// 消息类型
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Message {
    /// 委托任务
    Delegate { task: String, metadata: Value },
    /// 通知
    Notify { content: String },
    /// 查询
    Query { question: String },
    /// 响应
    Response { content: String, in_reply_to: Option<String> },
}

impl AgentMessageBus {
    /// 初始化
    pub fn new(config: BusConfig) -> Self;
    
    /// 发送消息到指定 Agent
    pub async fn send(&self, to: &str, msg: Message) -> Result<()>;
    
    /// 广播事件
    pub fn broadcast(&self, event: AgentEvent) -> Result<()>;
    
    /// 订阅事件类型
    pub fn subscribe(&self, agent_id: &str, event_types: Vec<EventType>) -> broadcast::Receiver<AgentEvent>;
    
    /// 获取状态观察器
    pub fn watch_state(&self, agent_id: &str) -> watch::Receiver<AgentState>;
    
    /// 发布状态变更
    pub fn publish_state(&self, agent_id: &str, new_state: AgentState);
}
```

**关键特性**:
- ✅ 支持广播、组播、点对点三种模式
- ✅ 使用 `watch` channel 实现状态观察
- ✅ 背压机制：通道满时自动丢弃旧消息
- ✅ 事件持久化：可选配置将事件写入 SQLite

---

#### 2.2.3 Context Manager（上下文管理）

**职责**: 共享上下文创建、访问控制、同步

```rust
// src/agent/orchestrator/context.rs

use dashmap::DashMap;
use std::sync::Arc;

pub struct SharedContextManager {
    /// 上下文存储：context_id -> SharedContext
    contexts: DashMap<String, SharedContext>,
    /// 访问控制
    acl: Arc<ContextACL>,
    /// 记忆系统引用
    memory: Arc<Memory>,
    config: ContextConfig,
}

/// 共享上下文
pub struct SharedContext {
    pub id: String,
    pub parent_id: Option<String>,
    /// 会话历史（可配置共享比例）
    pub history: Arc<RwLock<Vec<ChatMessage>>>,
    /// 工作记忆（临时数据）
    pub working_memory: Arc<DashMap<String, Value>>,
    /// 工具注册表（可继承/隔离）
    pub tools: Arc<ToolRegistry>,
    /// 资源配额
    pub quotas: ResourceQuotas,
    /// 访问者列表
    pub accessors: Arc<RwLock<HashSet<String>>>,
}

/// 访问控制列表
pub struct ContextACL {
    /// 读权限：context_id -> [agent_ids]
    read_allowlist: DashMap<String, HashSet<String>>,
    /// 写权限：context_id -> [agent_ids]
    write_allowlist: DashMap<String, HashSet<String>>,
    /// 继承规则
    inherit_rules: ContextInheritRules,
}

/// 上下文共享模式
#[derive(Clone, Copy, Debug)]
pub enum ContextShareMode {
    /// 不共享（完全隔离）
    None,
    /// 继承（只读父上下文）
    Inherit,
    /// 完全共享（读写）
    Full,
}

impl SharedContextManager {
    /// 创建共享上下文
    pub fn create(
        &self,
        parent_id: Option<&str>,
        mode: ContextShareMode,
        config: ContextConfig,
    ) -> Result<String>;
    
    /// 访问检查
    pub fn can_access(&self, agent_id: &str, context_id: &str, access: AccessType) -> bool;
    
    /// 同步上下文（将本地变更同步到父上下文）
    pub async fn sync(&self, agent_id: &str) -> Result<()>;
    
    /// 获取上下文（带访问检查）
    pub fn get(&self, agent_id: &str, context_id: &str) -> Option<Arc<SharedContext>>;
}
```

**关键特性**:
- ✅ 三种共享模式：None / Inherit / Full
- ✅ 基于 ACL 的访问控制
- ✅ 增量同步：只同步变更部分
- ✅ 与现有 Memory 系统集成

---

#### 2.2.4 Workflow Engine（工作流引擎）

**职责**: 工作流解析、执行、状态管理

```rust
// src/agent/orchestrator/workflow.rs

use petgraph::graph::DiGraph;
use tokio::sync::Semaphore;

pub struct WorkflowEngine {
    /// 工作流定义：workflow_id -> WorkflowDefinition
    workflows: DashMap<String, WorkflowDefinition>,
    /// 执行实例：execution_id -> WorkflowExecution
    executions: DashMap<String, WorkflowExecution>,
    /// Agent 注册表引用
    registry: Arc<AgentRegistry>,
    /// 消息总线引用
    bus: Arc<AgentMessageBus>,
    /// 并发控制
    semaphore: Arc<Semaphore>,
    config: WorkflowConfig,
}

/// 工作流定义（DAG）
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    /// 节点定义
    pub nodes: HashMap<String, WorkflowNode>,
    /// 边（使用 petgraph 存储）
    pub graph: DiGraph<NodeId, EdgeCondition>,
    /// 变量定义
    pub variables: HashMap<String, Value>,
}

/// 工作流节点
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowNode {
    pub node_type: NodeType,
    /// Agent 节点配置
    pub agent_template: Option<String>,
    pub prompt_template: Option<String>,
    /// 并行节点
    pub parallel_nodes: Option<Vec<String>>,
    /// 条件表达式
    pub condition: Option<String>,
    /// 超时和重试
    pub timeout_secs: Option<u64>,
    pub retry_policy: Option<RetryPolicy>,
    /// 输入输出
    pub input_refs: Vec<String>,
    pub output_key: Option<String>,
}

/// 节点类型
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    /// 单 Agent 执行
    Agent,
    /// 并行执行
    Parallel,
    /// 条件分支
    Condition,
    /// 循环
    Loop,
    /// 人工审批
    Approval,
    /// 数据转换
    Transform,
}

/// 执行实例
pub struct WorkflowExecution {
    pub id: String,
    pub workflow_id: String,
    pub current_node: Option<String>,
    pub completed_nodes: HashSet<String>,
    pub failed_nodes: HashSet<String>,
    pub variables: HashMap<String, Value>,
    pub state: ExecutionState,
    pub started_at: Option<Instant>,
    pub completed_at: Option<Instant>,
}

impl WorkflowEngine {
    /// 从目录加载工作流定义
    pub fn load_from_dir(&self, dir: &str) -> Result<()>;
    
    /// 执行工作流
    pub async fn execute(
        &self,
        workflow_id: &str,
        input: Value,
        async_mode: bool,
    ) -> Result<ExecutionResult>;
    
    /// 暂停执行
    pub fn pause(&self, execution_id: &str) -> Result<()>;
    
    /// 恢复执行
    pub fn resume(&self, execution_id: &str) -> Result<()>;
    
    /// 查询状态
    pub fn get_status(&self, execution_id: &str) -> Option<ExecutionStatus>;
    
    /// 列出执行历史
    pub fn list_executions(&self, workflow_id: Option<&str>) -> Vec<ExecutionSummary>;
}
```

**关键特性**:
- ✅ 使用 `petgraph` 实现高效 DAG 遍历
- ✅ 支持并行、条件分支、循环
- ✅ 超时和重试机制
- ✅ 人工审批节点（暂停等待用户输入）
- ✅ 执行历史持久化

---

## 三、配置设计

### 3.1 新增配置项

```toml
# ~/.multiclaw/config.toml

# ============================================================
# 多 Agent 编排系统配置
# ============================================================

[orchestration]
# 启用编排系统
enabled = true

# 最大并发 Agent 数
max_concurrent_agents = 100

# 默认最大层级深度
max_depth = 5

# Agent 默认超时（秒）
default_timeout_secs = 300

# 资源配额
[orchestration.quotas]
max_memory_per_agent_mb = 50
max_tool_calls_per_minute = 60
max_tokens_per_request = 8192
max_cost_per_agent_cents = 100

# 消息总线配置
[orchestration.bus]
# 广播通道容量
broadcast_capacity = 1000
# P2P 通道容量
p2p_capacity = 100
# 事件保留时间（秒）
event_retention_secs = 60
# 持久化事件
persist_events = true

# 共享上下文配置
[orchestration.context]
# 默认共享模式：none | inherit | full
default_share_mode = "inherit"
# 历史消息共享比例（0.0-1.0）
history_share_ratio = 0.5
# 工作记忆大小限制
max_working_memory_entries = 100
# 同步间隔（秒）
sync_interval_secs = 30

# 工作流引擎配置
[orchestration.workflow]
# 启用工作流
enabled = true
# 工作流目录
workflows_dir = "~/.multiclaw/workflows"
# 最大并发执行数
max_concurrent_executions = 20
# 执行历史保留天数
execution_history_days = 7
# 自动加载工作流
auto_load_workflows = true

# Agent 模板预定义
[[orchestration.templates]]
name = "researcher"
provider = "qwen-coding-plan"
model = "kimi-k2.5"
system_prompt = "你是专业研究员，擅长信息搜集和整理。"
allowed_tools = ["web_search", "web_fetch", "memory_recall"]
max_iterations = 10
inherit_context = true
inherit_memory = true
can_spawn = true
max_spawn_depth = 3

[[orchestration.templates]]
name = "coder"
provider = "qwen-code"
model = "qwen3-coder-plus"
system_prompt = "你是高级工程师，编写高质量代码。"
allowed_tools = ["file_read", "file_write", "shell", "git"]
max_iterations = 20
inherit_context = false
inherit_memory = true
can_spawn = false

[[orchestration.templates]]
name = "architect"
provider = "qwen-coding-plan"
model = "glm-5"
system_prompt = "你是资深架构师，负责系统设计和评审。"
allowed_tools = ["file_read", "memory_recall"]
max_iterations = 15
inherit_context = true
inherit_memory = true
can_spawn = true
max_spawn_depth = 5
```

### 3.2 工作流定义示例

```yaml
# ~/.multiclaw/workflows/research_and_code.yaml

id: research_and_code
name: 研究并实现
description: 研究需求后实现功能的完整工作流
version: "1.0"

variables:
  task: ""
  language: "rust"

nodes:
  # 节点 1: 需求分析
  analyze:
    type: agent
    agent_template: architect
    prompt: |
      分析以下任务需求，输出详细的技术方案：
      任务：{{task}}
      语言：{{language}}
    output_key: analysis

  # 节点 2: 并行研究
  research:
    type: parallel
    nodes:
      - web_research:
          type: agent
          agent_template: researcher
          prompt: "搜索 {{task}} 相关的最佳实践和示例代码"
          tools: ["web_search", "web_fetch"]
          output_key: web_results
      - internal_research:
          type: agent
          agent_template: researcher
          prompt: "查询内部记忆中关于 {{task}} 的记录"
          tools: ["memory_recall"]
          output_key: internal_results

  # 节点 3: 代码实现
  implement:
    type: agent
    agent_template: coder
    prompt: |
      基于以下信息实现功能：
      需求分析：{{analysis}}
      网络研究：{{web_results}}
      内部记录：{{internal_results}}
      语言：{{language}}
    input_refs: ["analysis", "web_results", "internal_results"]
    output_key: code

  # 节点 4: 代码审查
  review:
    type: agent
    agent_template: architect
    prompt: "审查以下代码的质量、安全性和性能：{{code}}"
    input_refs: ["code"]
    output_key: review_feedback

  # 节点 5: 条件分支 - 是否需要修改
  check_review:
    type: condition
    condition: "{{review_feedback.has_issues}} == true"
    if_true: fix_code
    if_false: finalize

  # 节点 6: 修复代码
  fix_code:
    type: agent
    agent_template: coder
    prompt: |
      根据审查意见修复代码：
      原代码：{{code}}
      审查意见：{{review_feedback}}
    input_refs: ["code", "review_feedback"]
    next: review  # 循环回审查节点

  # 节点 7: 完成
  finalize:
    type: transform
    transform: |
      {
        "status": "completed",
        "code": "{{code}}",
        "analysis": "{{analysis}}",
        "review": "{{review_feedback}}"
      }
    output_key: final_result

edges:
  - from: analyze
    to: research
  - from: research
    to: implement
  - from: implement
    to: review
  - from: review
    to: check_review
  - from: check_review
    to: fix_code
    condition: "result == true"
  - from: check_review
    to: finalize
    condition: "result == false"
  - from: fix_code
    to: review
```

---

## 四、与现有系统集成

### 4.1 Delegate 工具集成

**改造点**: 将现有的 `delegate` 工具升级为使用编排系统

```rust
// src/tools/delegate.rs - 改造后

use crate::agent::orchestrator::AgentOrchestrator;

pub struct DelegateTool {
    orchestrator: Arc<AgentOrchestrator>,
}

impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "委托任务给子 Agent。支持动态创建或使用预定义模板。"
    }

    async fn execute(&self, params: Value) -> Result<Value> {
        let task = params["task"].as_str().ok_or("task required")?;
        let template = params["template"].as_str();
        let spawn_new = params["spawn_new"].as_bool().unwrap_or(false);

        if spawn_new {
            // 动态创建新 Agent
            let new_template = parse_template_from_params(&params)?;
            let agent_id = self.orchestrator.spawn(&new_template, None).await?;
            self.orchestrator.send(&agent_id, task).await?;
        } else if let Some(template_name) = template {
            // 使用预定义模板
            let agent_id = self.orchestrator.spawn_from_template(template_name).await?;
            self.orchestrator.send(&agent_id, task).await?;
        } else {
            // 使用默认子 Agent
            let agent_id = self.orchestrator.get_default_child()?;
            self.orchestrator.send(&agent_id, task).await?;
        }

        Ok(json!({
            "status": "delegated",
            "agent_id": agent_id,
        }))
    }
}
```

### 4.2 Channels 集成

**改造点**: 支持消息路由到不同 Agent

```rust
// src/channels/mod.rs - 改造后

async fn handle_channel_message(
    msg: ChannelMessage,
    orchestrator: Arc<AgentOrchestrator>,
) -> Result<()> {
    // 检查消息是否指定了目标 Agent
    let target_agent = extract_agent_mention(&msg.content);

    if let Some(agent_id) = target_agent {
        // 发送到指定 Agent
        orchestrator.send(&agent_id, &msg.content).await?;
    } else {
        // 发送到主 Agent（默认行为）
        orchestrator.send_to_main(&msg.content).await?;
    }

    Ok(())
}
```

### 4.3 Memory 集成

**改造点**: 支持上下文隔离/共享

```rust
// src/memory/mod.rs - 改造后

impl Memory {
    /// 存储消息（带上下文检查）
    pub async fn store_message(
        &self,
        agent_id: &str,
        context_id: &str,
        message: ChatMessage,
    ) -> Result<()> {
        // 检查访问权限
        if !self.context_manager.can_access(agent_id, context_id, AccessType::Write) {
            return Err(Error::AccessDenied);
        }

        // 存储到指定上下文
        self.store_to_context(context_id, message).await?;
        Ok(())
    }

    /// 检索记忆（支持跨上下文）
    pub async fn search(
        &self,
        agent_id: &str,
        query: &str,
        include_shared: bool,
    ) -> Result<Vec<MemoryResult>> {
        let context_ids = self.context_manager.get_accessible_contexts(agent_id);
        
        let mut results = Vec::new();
        for context_id in context_ids {
            let ctx_results = self.search_in_context(&context_id, query).await?;
            results.extend(ctx_results);
        }

        Ok(results)
    }
}
```

---

## 五、CLI 和 HTTP API

### 5.1 CLI 命令

```bash
# Agent 管理
multiclaw agent spawn <template> [--parent <id>] [--config <json>]
multiclaw agent terminate <id> [--graceful]
multiclaw agent list [--tree] [--json]
multiclaw agent status <id> [--json]
multiclaw agent hierarchy <id> [--depth <n>]

# 消息传递
multiclaw agent send <to> <message>
multiclaw agent broadcast <event-type> <payload>
multiclaw agent subscribe <event-type>

# 上下文管理
multiclaw context create [--parent <id>] [--mode inherit|full|none]
multiclaw context share <context-id> --with <agent-id>
multiclaw context sync <agent-id>
multiclaw context list --agent <id>

# 工作流
multiclaw workflow run <workflow-id> --input <json> [--async]
multiclaw workflow pause <execution-id>
multiclaw workflow resume <execution-id>
multiclaw workflow status <execution-id>
multiclaw workflow list [--status <status>]
multiclaw workflow history [--days <n>]

# 模板管理
multiclaw template list
multiclaw template show <name>
multiclaw template create <name> --config <json>
multiclaw template update <name> --config <json>
```

### 5.2 HTTP API

```yaml
# Agent 管理
POST   /api/v1/agents/spawn              # 创建 Agent
DELETE /api/v1/agents/{id}               # 终止 Agent
GET    /api/v1/agents                    # 列出 Agent
GET    /api/v1/agents/{id}               # 获取状态
GET    /api/v1/agents/{id}/hierarchy     # 获取层级
POST   /api/v1/agents/{id}/message       # 发送消息

# 上下文
POST   /api/v1/contexts                  # 创建上下文
GET    /api/v1/contexts/{id}             # 获取上下文
POST   /api/v1/contexts/{id}/share       # 共享上下文
POST   /api/v1/contexts/{id}/sync        # 同步上下文

# 工作流
POST   /api/v1/workflows/{id}/execute    # 执行工作流
GET    /api/v1/workflows                 # 列出工作流
GET    /api/v1/workflows/{id}            # 获取定义
GET    /api/v1/workflows/{id}/executions # 列出执行历史
POST   /api/v1/workflows/{exec_id}/pause # 暂停执行
POST   /api/v1/workflows/{exec_id}/resume # 恢复执行

# 模板
GET    /api/v1/templates                 # 列出模板
POST   /api/v1/templates                 # 创建模板
PUT    /api/v1/templates/{name}          # 更新模板
DELETE /api/v1/templates/{name}          # 删除模板

# 监控
GET    /api/v1/metrics/agents            # Agent 指标
GET    /api/v1/metrics/bus               # 消息总线指标
GET    /api/v1/metrics/workflows         # 工作流指标
```

---

## 六、实现计划

### 6.1 阶段划分

| 阶段 | 内容 | 工期 | 依赖 | 里程碑 |
|------|------|------|------|--------|
| **Phase 1** | Agent Registry + 基础架构 | 1 周 | - | M1: 可动态创建 Agent |
| **Phase 2** | Message Bus | 1 周 | Phase 1 | M2: 消息传递正常 |
| **Phase 3** | Context Manager | 1 周 | Phase 1, 2 | M3: 上下文共享正常 |
| **Phase 4** | 改造 Delegate 工具 | 3 天 | Phase 1-3 | M4: delegate 集成完成 |
| **Phase 5** | Workflow Engine (基础) | 2 周 | Phase 1-3 | M5: 工作流可执行 |
| **Phase 6** | CLI + HTTP API | 1 周 | Phase 1-5 | M6: CLI/API 可用 |
| **Phase 7** | 改造 Channels + Memory | 1 周 | Phase 1-5 | M7: 全系统集成 |
| **Phase 8** | 测试 + 文档 | 1 周 | Phase 1-7 | M8: 测试覆盖>80% |

**总计**: 9 周

### 6.2 详细任务分解

#### Phase 1: Agent Registry (1 周)

```
Day 1-2: 基础结构
  - [ ] 创建 src/agent/orchestrator/ 目录
  - [ ] 实现 AgentTemplate 和 AgentInstance
  - [ ] 实现 DashMap 存储

Day 3-4: 生命周期管理
  - [ ] 实现 spawn() 方法
  - [ ] 实现 terminate() 方法
  - [ ] 实现层级关系追踪

Day 5: 配额管理
  - [ ] 实现 ResourceQuotas
  - [ ] 集成配额检查
  - [ ] 单元测试

Day 6-7: 集成测试
  - [ ] 编写集成测试
  - [ ] 性能基准测试
  - [ ] 修复 bug
```

#### Phase 5: Workflow Engine (2 周)

```
Week 1: 基础引擎
  - [ ] 实现 WorkflowDefinition 解析
  - [ ] 使用 petgraph 构建 DAG
  - [ ] 实现节点执行器
  - [ ] 实现边遍历

Week 2: 高级功能
  - [ ] 实现 Parallel 节点
  - [ ] 实现 Condition 节点
  - [ ] 实现 Approval 节点
  - [ ] 实现超时和重试
  - [ ] 持久化执行历史
```

---

## 七、测试策略

### 7.1 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_agent() {
        let registry = AgentRegistry::test_instance();
        let template = create_test_template();

        let agent_id = registry.spawn("parent-1", template, None).await.unwrap();

        assert!(registry.exists(&agent_id));
        assert_eq!(registry.get_parent(&agent_id), Some("parent-1"));
    }

    #[tokio::test]
    async fn test_message_bus_broadcast() {
        let bus = AgentMessageBus::test_instance();
        let mut rx = bus.subscribe("agent-1", vec![EventType::Completed]);

        bus.broadcast(AgentEvent::Completed {
            agent_id: "agent-2".to_string(),
            result: TaskResult::Success("done".to_string()),
        });

        let event = rx.recv().await.unwrap();
        assert!(matches!(event, AgentEvent::Completed { .. }));
    }

    #[tokio::test]
    async fn test_context_acl() {
        let mgr = SharedContextManager::test_instance();
        let ctx_id = mgr.create(None, ContextShareMode::Full).unwrap();

        assert!(mgr.can_access("agent-1", &ctx_id, AccessType::Write));
        assert!(!mgr.can_access("agent-2", &ctx_id, AccessType::Read));
    }
}
```

### 7.2 集成测试

```rust
#[tokio::test]
async fn test_agent_hierarchy() {
    let orchestrator = AgentOrchestrator::test_instance();

    // 创建 3 层 Agent 树
    let root = orchestrator.spawn(&root_template, None).await?;
    let child1 = orchestrator.spawn(&child_template, Some(&root)).await?;
    let grandchild = orchestrator.spawn(&grandchild_template, Some(&child1)).await?;

    // 验证层级关系
    let hierarchy = orchestrator.get_hierarchy(&root);
    assert_eq!(hierarchy.depth, 3);
    assert!(hierarchy.contains(&grandchild));

    // 测试消息传递
    orchestrator.send(&grandchild, "Hello").await?;
    // ... 验证消息到达
}

#[tokio::test]
async fn test_workflow_execution() {
    let engine = WorkflowEngine::test_instance();
    engine.load_from_dir("tests/workflows").unwrap();

    let result = engine
        .execute("simple_workflow", json!({"input": "test"}))
        .await
        .unwrap();

    assert_eq!(result.state, ExecutionState::Completed);
    assert!(result.output["result"].is_string());
}
```

### 7.3 性能测试

```rust
#[tokio::test]
async fn benchmark_spawn_latency() {
    let registry = AgentRegistry::test_instance();
    let template = create_minimal_template();

    let start = Instant::now();
    for i in 0..100 {
        registry.spawn("root", template.clone(), None).await?;
    }
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(1)); // <10ms/Agent
}

#[tokio::test]
async fn benchmark_message_throughput() {
    let bus = AgentMessageBus::test_instance();

    let start = Instant::now();
    for i in 0..10000 {
        bus.send(&format!("agent-{}", i), Message::text("test")).await?;
    }
    let elapsed = start.elapsed();

    assert!(elapsed < Duration::from_secs(5)); // >2000 msg/s
}

#[tokio::test]
async fn benchmark_concurrent_agents() {
    let orchestrator = AgentOrchestrator::test_instance();
    let template = create_active_template();

    let mut handles = Vec::new();
    for i in 0..100 {
        let agent_id = orchestrator.spawn(&template, None).await?;
        handles.push(orchestrator.send(&agent_id, format!("task {}", i)));
    }

    let results = futures::future::join_all(handles).await;
    assert!(results.iter().all(|r| r.is_ok()));
}
```

---

## 八、风险与缓解

| 风险 | 影响 | 概率 | 缓解措施 |
|------|------|------|---------|
| **循环 spawn 导致资源耗尽** | 高 | 中 | 实现最大深度检查 + 配额限制 + 监控告警 |
| **消息总线成为瓶颈** | 高 | 中 | 使用多通道 + 背压机制 + 性能监控 |
| **上下文同步延迟** | 中 | 高 | 增量同步 + 懒加载 + 异步处理 |
| **工作流死锁** | 高 | 低 | DAG 验证 + 超时机制 + 死锁检测 |
| **内存泄漏** | 高 | 中 | 严格生命周期管理 + 泄漏检测工具 |
| **与现有系统冲突** | 中 | 中 | 渐进式改造 + 充分测试 + 回滚方案 |

---

## 九、验收标准

### 9.1 功能验收

- [ ] 可动态创建/销毁 Agent
- [ ] 支持至少 5 层嵌套
- [ ] 消息传递延迟 <1ms
- [ ] 上下文共享可配置（None/Inherit/Full）
- [ ] 工作流可定义和执行（DAG）
- [ ] CLI 和 HTTP API 可用
- [ ] 与现有 delegate 工具集成
- [ ] 与 Channels 集成（消息路由）
- [ ] 与 Memory 集成（上下文隔离）

### 9.2 性能验收

- [ ] Agent 创建延迟 <10ms
- [ ] 支持≥100 并发 Agent
- [ ] 消息吞吐量 >2000 msg/s
- [ ] 内存占用 <10MB/Agent
- [ ] 工作流执行开销 <5%

### 9.3 质量验收

- [ ] 单元测试覆盖 >80%
- [ ] 集成测试通过
- [ ] 性能测试达标
- [ ] 文档完整（API 文档 + 用户指南）
- [ ] 示例工作流 ≥5 个

---

## 十、监控与可观测性

### 10.1 指标收集

```rust
// 使用 Prometheus 收集指标

pub struct OrchestratorMetrics {
    /// 活跃 Agent 数
    active_agents: IntGauge,
    /// Agent 创建延迟
    spawn_latency: Histogram,
    /// 消息传递延迟
    message_latency: Histogram,
    /// 工作流执行时间
    workflow_duration: Histogram,
    /// 资源使用
    memory_usage: IntGauge,
    /// 错误计数
    error_count: Counter,
}

impl OrchestratorMetrics {
    pub fn record_spawn(&self, duration: Duration) {
        self.spawn_latency.observe(duration.as_secs_f64());
        self.active_agents.inc();
    }

    pub fn record_message(&self, duration: Duration) {
        self.message_latency.observe(duration.as_secs_f64());
    }
}
```

### 10.2 日志

```rust
// 结构化日志

tracing::info!(
    target: "multiclaw::orchestrator",
    agent_id = %agent_id,
    parent_id = ?parent_id,
    template = %template_name,
    duration_ms = %elapsed.as_millis(),
    "Agent spawned successfully"
);

tracing::warn!(
    target: "multiclaw::orchestrator::quotas",
    agent_id = %agent_id,
    resource = "memory",
    usage = %current_usage,
    limit = %limit,
    "Agent approaching resource limit"
);
```

### 10.3 分布式追踪

```rust
// 使用 OpenTelemetry 追踪

#[instrument(skip(self, task), fields(task_id = %Uuid::new_v4()))]
async fn delegate_task(&self, agent_id: &str, task: &str) -> Result<()> {
    let span = Span::current();
    span.set_attribute("agent_id", agent_id.to_string());
    span.set_attribute("task", task.to_string());

    // ... 执行逻辑
}
```

---

## 十一、示例场景

### 11.1 场景 1: 动态研究团队

```bash
# 创建研究团队
multiclaw agent spawn researcher --config '{"name": "researcher-1"}'
multiclaw agent spawn researcher --config '{"name": "researcher-2"}'

# 委托任务
multiclaw agent send researcher-1 "搜索 Rust 异步编程资料"
multiclaw agent send researcher-2 "搜索 Rust 性能优化资料"

# 查看结果
multiclaw agent status researcher-1
multiclaw agent status researcher-2
```

### 11.2 场景 2: 工作流执行

```bash
# 执行工作流
multiclaw workflow run research_and_code --input '{"task": "实现 Web 爬虫", "language": "rust"}'

# 查看状态
multiclaw workflow status <execution-id>

# 暂停/恢复（人工审核节点）
multiclaw workflow pause <execution-id>
# ... 审核后 ...
multiclaw workflow resume <execution-id>
```

### 11.3 场景 3: 嵌套 Agent 树

```
主 Agent
├── architect (架构师)
│   └── reviewer (审查员)
├── researcher (研究员)
│   ├── web_researcher (网络研究)
│   └── internal_researcher (内部研究)
└── coder (程序员)
    ├── senior_dev (高级)
    └── junior_dev (初级)
```

---

## 十二、参考资源

- [nanobot SubagentManager](../../nanobot/nanobot/agent/subagents.py)
- [nanoclaw Agent Swarms](../../nanoclaw/src/task-scheduler.ts)
- [MultiClaw 现有 delegate 工具](src/tools/delegate.rs)
- [Tokio 异步模式](https://tokio.rs/tokio/tutorial)
- [Rust DashMap](https://docs.rs/dashmap/latest/dashmap/)
- [petgraph](https://docs.rs/petgraph/latest/petgraph/)
- [OpenTelemetry Rust](https://docs.rs/opentelemetry/latest/opentelemetry/)

---

**审批状态**: 待审批  
**负责人**: 待定  
**最后更新**: 2026 年 2 月 28 日
