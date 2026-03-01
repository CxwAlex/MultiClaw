# MultiClaw 多 Agent 集群架构方案 v5.0 - 全局编排版

> **版本**: v5.0 - 全局编排架构（四层设计 + 可观测性增强）
> **创建日期**: 2026 年 3 月 1 日
> **优先级**: P0 - 核心能力
> **状态**: 待审批
> **架构理念**: 全局董事长分身 + 企业组织模式 + 核心硬实现 + 编排 Skills 化

---

## 一、执行摘要

### 1.1 核心演进

**v5.0 相对 v4.0 的关键升级**:

| 维度 | v4.0 混合架构 | v5.0 全局编排 | 解决的问题 |
|------|-------------|-------------|-----------|
| **架构层级** | 三层 (编排/核心/执行) | 四层 (全局/编排/核心/执行) | 多实例管理复杂 |
| **用户角色** | 董事长 (直接沟通 CEO) | 用户 → 董事长分身 → CEO | 信息过载 |
| **可观测性** | 集群状态查询 | 快速创建入口 + 目标设定 | 使用门槛高 |
| **实例管理** | 单实例 | 多实例 (分公司) | 规模化扩展 |

### 1.2 核心架构决策

**问题 1**: 多实例 (分公司) 管理复杂，用户信息过载怎么办？

**答案**: **引入全局董事长 Agent** - 作为用户个人分身，统一管理所有实例

```
用户 (自然人)
    │
    ▼ (唯一交互入口)
┌─────────────────────────────────────────┐
│  董事长 Agent (用户分身)                  │
│  - 启动时自动创建                        │
│  - 管理所有实例 (分公司)                  │
│  - 汇总关键信息                          │
│  - 过滤噪音，只同步重要决策              │
└─────────────────────────────────────────┘
    │
    ├─── 实例 1 (CEO: 市场调研)
    ├─── 实例 2 (CEO: 产品开发)
    ├─── 实例 3 (CEO: 客户服务)
    └─── 实例 N (CEO: ...)
```

**问题 2**: 如何降低使用门槛，快速创建公司 - 团队？

**答案**: **可观测性增强** - 提供快速创建入口，目标/资源预设，后续由 CEO 和团队负责人自动完成

```
用户输入: "我想做市场调研"
    │
    ▼
┌─────────────────────────────────────────┐
│  快速创建入口 (CLI/Telegram/Web)          │
│  - 设定公司目标                          │
│  - 分配初始资源                          │
│  - 选择协作模式 (可选)                    │
│  - 一键启动                              │
└─────────────────────────────────────────┘
    │
    ▼ (自动完成后续配置)
CEO Agent:
    - 生成详细执行计划
    - 创建团队负责人
    - 分配 Worker Agent
    - 启动项目执行
```

### 1.3 整体架构（四层设计）

```
┌─────────────────────────────────────────────────────────────────┐
│                        全局层 (Global)                           │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  董事长 Agent (用户分身)                                   │    │
│  │  - 启动时自动创建，绑定用户终端                           │    │
│  │  - 管理所有实例 (分公司)                                  │    │
│  │  - 汇总关键信息，过滤噪音                                 │    │
│  │  - 审批重大决策 (超预算/模式切换/实例创建)                 │    │
│  │  - 同步重要状态到用户                                     │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│              ┌───────────────┼───────────────┐                   │
│              ▼               ▼               ▼                   │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐    │
│  │   实例 1         │ │   实例 2         │ │   实例 N         │    │
│  │  (市场调研公司)  │ │  (产品开发公司)  │ │  (客户服务公司)  │    │
│  └────────┬────────┘ └────────┬────────┘ └────────┬────────┘    │
│           │                   │                   │              │
│           ▼                   ▼                   ▼              │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              编排层 (Skills) - 每个实例独立                │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │    │
│  │  │  CEO Skills  │  │ Team Skills  │  │Worker Skills │   │    │
│  │  │  - 创建团队   │  │  - 申请资源   │  │  - 贡献知识   │   │    │
│  │  │  - 审批资源   │  │  - 查询状态   │  │  - 请求帮助   │   │    │
│  │  │  - 查询状态   │  │  - 升级到 CEO │  │  - 汇报进度   │   │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼ (Skill 调用核心层 API)             │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              核心层 (硬实现) - 每个实例独立                │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │    │
│  │  │ ClusterCore  │  │ ResourceCore │  │ HealthCore   │   │    │
│  │  │ 集群管理核心 │  │ 资源管理核心 │  │ 健康检查核心 │   │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              执行层 (Agent) - 沙箱隔离                     │    │
│  │  - Firecracker 微 VM / Wasm 沙箱 / Docker 容器             │    │
│  │  - MCP 协议调用工具                                       │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

---

## 二、全局层设计（新增）

### 2.1 董事长 Agent（用户分身）

**定位**: 用户的 AI 分身，统一管理所有 MultiClaw 实例

**创建时机**: MultiClaw 启动时自动创建，绑定用户终端

```rust
// src/agent/chairman.rs

use crate::instance::InstanceHandle;
use dashmap::DashMap;

/// 董事长 Agent - 用户个人分身
pub struct ChairmanAgent {
    /// 用户 ID
    pub user_id: String,
    /// 绑定用户终端
    pub user_channel: ChannelId,
    /// 管理的所有实例
    pub instances: DashMap<String, InstanceHandle>,
    /// 全局资源池
    pub global_resource: Arc<GlobalResourceManager>,
    /// 信息聚合器
    pub aggregator: Arc<InformationAggregator>,
    /// 决策过滤器（过滤噪音）
    pub decision_filter: DecisionFilter,
}

/// 实例句柄
#[derive(Clone)]
pub struct InstanceHandle {
    /// 实例 ID
    pub id: String,
    /// 实例名称
    pub name: String,
    /// 实例类型
    pub instance_type: InstanceType,
    /// CEO Agent ID
    pub ceo_agent_id: String,
    /// 实例状态
    pub status: InstanceStatus,
    /// 资源配额
    pub quota: ResourceQuota,
    /// 当前项目数
    pub active_projects: usize,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活跃时间
    pub last_active_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InstanceType {
    /// 市场调研
    MarketResearch,
    /// 产品开发
    ProductDevelopment,
    /// 客户服务
    CustomerService,
    /// 数据分析
    DataAnalysis,
    /// 通用型
    General,
    /// 自定义
    Custom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InstanceStatus {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 空闲
    Idle,
    /// 忙碌
    Busy,
    /// 异常
    Unhealthy,
    /// 已停止
    Stopped,
}

impl ChairmanAgent {
    /// 启动时自动创建
    pub async fn initialize(user_id: String, user_channel: ChannelId) -> Result<Self> {
        let chairman = Self {
            user_id,
            user_channel,
            instances: DashMap::new(),
            global_resource: Arc::new(GlobalResourceManager::new()),
            aggregator: Arc::new(InformationAggregator::new()),
            decision_filter: DecisionFilter::default(),
        };

        // 加载已有实例
        chairman.load_existing_instances().await?;

        Ok(chairman)
    }

    /// 创建新实例（分公司）
    pub async fn create_instance(
        &self,
        request: &CreateInstanceRequest,
    ) -> Result<InstanceHandle> {
        // 1. 检查全局资源
        if !self.global_resource.can_allocate(&request.quota) {
            return Err("全局资源不足，请先释放已有实例或申请增加配额".into());
        }

        // 2. 创建实例
        let instance = InstanceHandle {
            id: uuid::Uuid::new_v4().to_string(),
            name: request.name.clone(),
            instance_type: request.instance_type,
            ceo_agent_id: String::new(), // 待创建
            status: InstanceStatus::Initializing,
            quota: request.quota.clone(),
            active_projects: 0,
            created_at: Utc::now(),
            last_active_at: Utc::now(),
        };

        // 3. 分配全局资源
        self.global_resource.allocate(&request.quota).await?;

        // 4. 创建 CEO Agent
        let ceo = self.create_ceo_agent(&instance, request.ceo_config.clone()).await?;
        let mut instance = instance.clone();
        instance.ceo_agent_id = ceo.id;
        instance.status = InstanceStatus::Running;

        // 5. 注册实例
        self.instances.insert(instance.id.clone(), instance.clone());

        // 6. 通知用户
        self.notify_user(&format!(
            "✅ 已创建新实例「{}」(类型：{:?})\n初始资源：{}\nCEO 已就绪",
            instance.name,
            instance.instance_type,
            self.format_quota(&instance.quota)
        )).await?;

        Ok(instance)
    }

    /// 汇总关键信息（定时任务）
    pub async fn aggregate_and_sync(&self) -> Result<()> {
        // 1. 收集所有实例的关键信息
        let mut summaries = Vec::new();
        for entry in self.instances.iter() {
            let instance = entry.value();
            let summary = self.fetch_instance_summary(instance).await?;
            summaries.push(summary);
        }

        // 2. 聚合信息
        let aggregated = self.aggregator.aggregate(summaries).await?;

        // 3. 过滤噪音（只保留重要信息）
        let filtered = self.decision_filter.filter(aggregated);

        // 4. 同步给用户
        if !filtered.is_empty() {
            self.sync_to_user(&filtered).await?;
        }

        Ok(())
    }

    /// 审批重大决策
    pub async fn review_major_decision(
        &self,
        decision: &MajorDecision,
    ) -> Result<DecisionResult> {
        match decision {
            MajorDecision::CreateInstance(request) => {
                // 创建新实例
                let instance = self.create_instance(request).await?;
                Ok(DecisionResult::Approved {
                    message: format!("实例「{}」已创建", instance.name),
                })
            }
            MajorDecision::IncreaseGlobalQuota(request) => {
                // 增加全局配额（需要用户确认）
                self.request_user_confirmation(&format!(
                    "申请增加全局资源配额：{}\n当前配额：{}\n新配额：{}",
                    request.reason,
                    self.global_resource.current_quota(),
                    request.new_quota
                )).await?;
                Ok(DecisionResult::Approved { message: "配额已增加".to_string() })
            }
            MajorDecision::ShutdownInstance(instance_id) => {
                // 关闭实例
                self.shutdown_instance(instance_id).await?;
                Ok(DecisionResult::Approved { message: "实例已关闭".to_string() })
            }
            MajorDecision::MergeInstances { from, to } => {
                // 合并实例
                self.merge_instances(from, to).await?;
                Ok(DecisionResult::Approved { message: "实例已合并".to_string() })
            }
        }
    }

    /// 查询全局状态
    pub fn get_global_status(&self) -> GlobalStatus {
        let instances: Vec<_> = self.instances.iter().map(|e| e.value().clone()).collect();

        GlobalStatus {
            total_instances: instances.len(),
            running_instances: instances.iter()
                .filter(|i| i.status == InstanceStatus::Running)
                .count(),
            busy_instances: instances.iter()
                .filter(|i| i.status == InstanceStatus::Busy)
                .count(),
            total_projects: instances.iter().map(|i| i.active_projects).sum(),
            global_resource_usage: self.global_resource.get_usage(),
            instances: instances,
        }
    }

    /// 快速创建公司 - 团队入口
    pub async fn quick_create(
        &self,
        request: &QuickCreateRequest,
    ) -> Result<QuickCreateResult> {
        // 1. 创建实例（如果不存在）
        let instance = if let Some(existing) = self.get_instance_by_name(&request.instance_name) {
            existing
        } else {
            self.create_instance(&CreateInstanceRequest {
                name: request.instance_name.clone(),
                instance_type: request.instance_type,
                quota: request.quota.clone(),
                ceo_config: request.ceo_config.clone(),
            }).await?
        };

        // 2. 调用 CEO Skill 创建团队
        let team = self.invoke_ceo_skill(
            &instance.ceo_agent_id,
            "create_project_team",
            &json!({
                "task": request.task_description,
                "goal": request.team_goal,
                "estimated_complexity": request.complexity,
            }),
        ).await?;

        Ok(QuickCreateResult {
            instance_id: instance.id,
            team_id: team.id,
            message: format!(
                "✅ 已创建「{}」实例和「{}」团队\n目标：{}\n资源：{}",
                instance.name,
                team.name,
                request.team_goal,
                self.format_quota(&team.quota)
            ),
        })
    }
}

/// 创建实例请求
#[derive(Clone, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    /// 实例名称
    pub name: String,
    /// 实例类型
    pub instance_type: InstanceType,
    /// 资源配额
    pub quota: ResourceQuota,
    /// CEO 配置
    pub ceo_config: CEOConfig,
}

/// 全局资源管理器
pub struct GlobalResourceManager {
    /// 全局 Token 配额
    global_token_quota: AtomicUsize,
    global_token_used: AtomicUsize,
    /// 全局并发实例限制
    max_instances: AtomicUsize,
    current_instances: AtomicUsize,
}

/// 信息聚合器
pub struct InformationAggregator {
    /// 聚合策略
    aggregation_strategy: AggregationStrategy,
    /// 时间窗口（秒）
    time_window_secs: u64,
}

/// 决策过滤器（过滤噪音）
pub struct DecisionFilter {
    /// 重要性阈值
    importance_threshold: f32,
    /// 过滤规则
    filter_rules: Vec<FilterRule>,
}

/// 重大决策类型
pub enum MajorDecision {
    CreateInstance(CreateInstanceRequest),
    IncreaseGlobalQuota(QuotaIncreaseRequest),
    ShutdownInstance(String),
    MergeInstances { from: String, to: String },
}

/// 全局状态
#[derive(Clone, Serialize, Deserialize)]
pub struct GlobalStatus {
    pub total_instances: usize,
    pub running_instances: usize,
    pub busy_instances: usize,
    pub total_projects: usize,
    pub global_resource_usage: ResourceUsage,
    pub instances: Vec<InstanceHandle>,
}

/// 快速创建请求
#[derive(Clone, Serialize, Deserialize)]
pub struct QuickCreateRequest {
    /// 实例名称
    pub instance_name: String,
    /// 实例类型
    pub instance_type: InstanceType,
    /// 任务描述
    pub task_description: String,
    /// 团队目标
    pub team_goal: String,
    /// 预估复杂度 (1-10)
    pub complexity: u8,
    /// 资源配额
    pub quota: ResourceQuota,
    /// CEO 配置
    pub ceo_config: CEOConfig,
}

/// 快速创建结果
#[derive(Clone, Serialize, Deserialize)]
pub struct QuickCreateResult {
    pub instance_id: String,
    pub team_id: String,
    pub message: String,
}
```

---

### 2.2 全局层与用户交互

#### 2.2.1 用户交互流程

```
用户启动 MultiClaw
    │
    ▼
自动创建董事长 Agent（用户分身）
    │
    ▼
董事长 Agent 向用户报到
    │
    ▼
用户输入需求
    │
    ├─── "创建市场调研公司" → 创建实例
    ├─── "查看全局状态"   → 返回汇总信息
    ├─── "我想做 XX 任务"  → 快速创建公司 - 团队
    └─── "审批资源申请"   → 转发到对应 CEO
```

#### 2.2.2 信息同步策略

```rust
// 董事长 Agent 定时同步关键信息

/// 信息同步策略
#[derive(Clone)]
pub struct SyncPolicy {
    /// 同步间隔（秒）
    pub sync_interval_secs: u64,
    /// 重要性阈值（0-1）
    pub importance_threshold: f32,
    /// 同步内容类型
    pub sync_content_types: Vec<SyncContentType>,
}

#[derive(Clone, Copy, PartialEq)]
pub enum SyncContentType {
    /// 实例创建/销毁
    InstanceLifecycle,
    /// 重大决策审批
    MajorDecision,
    /// 资源预警（<20%）
    ResourceWarning,
    /// 项目完成
    ProjectCompletion,
    /// 异常情况
    Exception,
}

impl ChairmanAgent {
    /// 定时同步任务
    pub async fn start_sync_loop(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(self.sync_policy.sync_interval_secs));

            loop {
                interval.tick().await;

                // 聚合所有实例信息
                let aggregated = self.aggregate_and_sync().await;

                match aggregated {
                    Ok(filtered) => {
                        if !filtered.is_empty() {
                            // 同步给用户
                            let _ = self.sync_to_user(&filtered).await;
                        }
                    }
                    Err(e) => {
                        tracing::error!("信息同步失败：{}", e);
                    }
                }
            }
        })
    }

    /// 过滤噪音（核心逻辑）
    fn filter_noise(&self, info: &InstanceInfo) -> Option<FilteredInfo> {
        // 过滤规则：
        // 1. 重要性评分 < 阈值 → 过滤
        // 2. 日常状态更新 → 过滤（除非异常）
        // 3. 资源使用正常 → 过滤（只预警 <20%）
        // 4. 项目进行中 → 过滤（只同步完成/延期）

        let importance = self.calculate_importance(info);

        if importance < self.importance_threshold {
            return None; // 过滤
        }

        match info.info_type {
            InfoType::InstanceCreated | InfoType::InstanceDestroyed => {
                Some(FilteredInfo::Lifecycle(info.clone()))
            }
            InfoType::ResourceWarning { usage_percentage } if usage_percentage < 20.0 => {
                Some(FilteredInfo::ResourceWarning(info.clone()))
            }
            InfoType::ProjectCompleted { quality_score } if quality_score >= 4.0 => {
                Some(FilteredInfo::ProjectSuccess(info.clone()))
            }
            InfoType::Exception { severity } if severity >= Severity::High => {
                Some(FilteredInfo::Exception(info.clone()))
            }
            InfoType::MajorDecisionRequired => {
                Some(FilteredInfo::DecisionRequired(info.clone()))
            }
            _ => None, // 过滤日常信息
        }
    }
}
```

#### 2.2.3 用户视角示例

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
MultiClaw 启动成功！

董事长 Agent 已就绪，我是您的 AI 分身，将协助您管理所有实例。

【全局状态】
- 实例数量：0
- 可用资源：100 万 Token, 10 个并发实例
- 活跃项目：0

您可以：
1. "创建市场调研公司" - 快速创建新实例
2. "我想了解 AI 编程助手市场" - 快速创建公司 - 团队
3. "查看全局状态" - 查看所有实例汇总
4. "审批资源申请" - 处理待审批决策

请告诉我您的需求：
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

用户：我想了解 AI 编程助手市场

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
✅ 已为您创建：

【实例】AI 编程助手市场调研公司
- 类型：市场调研
- 初始资源：50 万 Token, 5 个并发实例
- CEO：已就绪

【团队】AI 编程助手市场调研项目组
- 目标：全面了解 AI 编程助手市场规模、竞争格局、增长趋势
- 协作模式：广撒网并行采集
- 预计团队规模：15-30 个 Worker Agent
- 预计完成时间：2 小时

项目已启动，我会在以下情况同步您：
- 项目完成时
- 资源不足需要审批时
- 遇到重大异常时

您现在可以：
- "查看项目进展" - 查询当前状态
- "查看 AI 编程助手市场调研" - 查看项目详情
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

2 小时后...

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
【项目完成通知】

「AI 编程助手市场调研」项目已完成！

【成果摘要】
- 收集信息：156 条
- 分析报告：1 份（4500 字）
- 关键发现：
  1. 全球 AI 编程助手市场规模 2025 年达$XX 亿
  2. 主要竞争者：GitHub Copilot、Cursor、Codeium
  3. 增长趋势：年复合增长率 XX%

【资源使用】
- Token: 42 万 / 50 万（84%）
- 实际耗时：1 小时 45 分

【交付物】
📄 完整报告：[查看链接]

需要我做什么吗？
- "深入分析 XX 方面" - 启动后续研究
- "导出为 PDF" - 格式转换
- "关闭此实例" - 释放资源
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## 三、快速创建入口设计

### 3.1 多端支持

| 入口 | 命令/交互 | 适用场景 |
|------|---------|---------|
| **CLI** | `multiclaw quick-create --type market --goal "XX"` | 开发者/运维 |
| **Telegram** | `/quick_create market "了解 AI 编程助手市场"` | 日常使用 |
| **Web UI** | 表单填写 + 一键启动 | 企业用户 |
| **HTTP API** | `POST /api/v1/quick-create` | 集成第三方 |

### 3.2 CLI 快速创建

```bash
# 快速创建市场调研公司 - 团队
multiclaw quick-create \
  --type market-research \
  --name "AI 编程助手调研" \
  --goal "全面了解 AI 编程助手市场规模、竞争格局、增长趋势" \
  --quota-tokens 500000 \
  --quota-agents 30 \
  --complexity 7

# 快速创建产品开发公司 - 团队
multiclaw quick-create \
  --type product-development \
  --name "新功能开发" \
  --goal "开发 XX 功能，满足 XX 需求" \
  --quota-tokens 1000000 \
  --quota-agents 50 \
  --complexity 8

# 使用预设模板
multiclaw quick-create --template market-research-basic \
  --name "竞品分析"
```

### 3.3 Telegram 快速创建

```
用户：/quick_create market "了解 AI 编程助手市场"

Bot 回复：
━━━━━━━━━━━━━━━━━━━━━━
✅ 快速创建已启动

【实例】AI 编程助手市场调研公司
- 类型：市场调研
- 资源：50 万 Token, 30 个并发 Agent

【团队】AI 编程助手市场调研项目组
- 目标：全面了解 AI 编程助手市场规模、竞争格局、增长趋势
- 协作模式：广撒网并行采集（自动选择）
- 预计完成：2 小时

确认创建？
✅ 确认
❌ 取消
━━━━━━━━━━━━━━━━━━━━━━

用户：✅

Bot 回复：
━━━━━━━━━━━━━━━━━━━━━━
✅ 已创建！

项目已启动，我会在以下情况通知您：
- 项目完成
- 资源不足需要审批
- 重大异常

查看进展：/status AI 编程助手调研
查看详情：/project AI 编程助手调研
━━━━━━━━━━━━━━━━━━━━━━
```

### 3.4 Web UI 快速创建

```html
<!-- 快速创建表单 -->
<div class="quick-create-form">
  <h2>快速创建公司 - 团队</h2>

  <!-- 实例类型选择 -->
  <select name="instance_type">
    <option value="market-research">市场调研</option>
    <option value="product-development">产品开发</option>
    <option value="customer-service">客户服务</option>
    <option value="data-analysis">数据分析</option>
    <option value="custom">自定义</option>
  </select>

  <!-- 实例名称 -->
  <input type="text" name="instance_name" placeholder="实例名称" />

  <!-- 团队目标 -->
  <textarea name="team_goal" placeholder="团队目标（详细描述）"></textarea>

  <!-- 任务描述 -->
  <textarea name="task_description" placeholder="任务描述"></textarea>

  <!-- 资源配额预设 -->
  <div class="quota-presets">
    <button data-preset="small">小型（10 万 Token, 10 Agent）</button>
    <button data-preset="medium">中型（50 万 Token, 30 Agent）</button>
    <button data-preset="large">大型（100 万 Token, 50 Agent）</button>
    <button data-preset="custom">自定义</button>
  </div>

  <!-- 复杂度评估 -->
  <input type="range" name="complexity" min="1" max="10" value="5" />
  <label>预估复杂度：1-10</label>

  <!-- 一键启动 -->
  <button type="submit">🚀 一键启动</button>
</div>
```

---

## 四、可观测性增强

### 4.1 全局可观测性 Dashboard

```
┌─────────────────────────────────────────────────────────────────┐
│                    MultiClaw 全局 Dashboard                        │
├─────────────────────────────────────────────────────────────────┤
│  【全局概览】                                                    │
│  - 实例数量：5 运行中：3 忙碌：1 空闲：1                         │
│  - 全局资源：250 万 / 500 万 Token (50%)                         │
│  - 活跃项目：12  今日完成：3                                     │
│                                                                  │
│  【实例列表】                                                    │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ 实例名称      │ 类型     │ 状态  │ 项目 │ 资源使用 │ 操作 │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ AI 编程助手调研 │ 市场调研 │ 🟢运行│  1   │ 84%     │ 查看 │    │
│  │ 新功能开发    │ 产品开发 │ 🟡忙碌│  3   │ 45%     │ 查看 │    │
│  │ 客户服务系统  │ 客户服务 │ 🟢空闲│  0   │ 12%     │ 查看 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  【快速创建】                                                    │
│  [创建市场调研] [创建产品开发] [创建客户服务] [自定义创建]        │
│                                                                  │
│  【待审批决策】                                                  │
│  - "新功能开发"申请增加资源配额 [审批] [拒绝]                    │
│  - 创建新实例"数据分析公司" [审批] [拒绝]                        │
│                                                                  │
│  【最近完成项目】                                                │
│  - ✅ AI 编程助手市场调研 (质量：4.8/5) [查看报告]               │
│  - ✅ 竞品分析报告 (质量：4.5/5) [查看报告]                      │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 实例级可观测性

```
┌─────────────────────────────────────────────────────────────────┐
│  实例详情：AI 编程助手市场调研公司                               │
├─────────────────────────────────────────────────────────────────┤
│  【基本信息】                                                    │
│  - 实例 ID：inst_xxx                                             │
│  - 类型：市场调研                                                │
│  - 创建时间：2026-03-01 14:00                                   │
│  - CEO Agent：agent_xxx                                          │
│                                                                  │
│  【资源使用】                                                    │
│  - Token: 42 万 / 50 万 (84%)                                    │
│  - 并发 Agent: 22 / 30                                           │
│  - 成本：$0.42 / $0.50                                           │
│  - 时间：1h45m / 2h                                              │
│                                                                  │
│  【项目列表】                                                    │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │ 项目名称          │ 状态  │ 进度 │ 团队规模 │ 操作      │    │
│  ├─────────────────────────────────────────────────────────┤    │
│  │ AI 编程助手市场调研│ 🟢完成│ 100% │   22     │ 查看报告  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                  │
│  【团队结构】                                                    │
│  - 团队负责人：市场研究专家                                      │
│  - Worker Agent: 22 个                                           │
│    - 信息收集：15 个                                              │
│    - 数据分析：5 个                                               │
│    - 报告撰写：2 个                                               │
│                                                                  │
│  【操作】                                                        │
│  [查看项目详情] [申请增加资源] [关闭实例] [导出报告]             │
└─────────────────────────────────────────────────────────────────┘
```

### 4.3 项目级可观测性

```
┌─────────────────────────────────────────────────────────────────┐
│  项目详情：AI 编程助手市场调研                                   │
├─────────────────────────────────────────────────────────────────┤
│  【项目状态】✅ 已完成                                           │
│  - 启动时间：2026-03-01 14:00                                   │
│  - 完成时间：2026-03-01 15:45                                   │
│  - 协作模式：广撒网并行采集                                      │
│                                                                  │
│  【任务分解】                                                    │
│  ✅ 需求分析和搜索策略 (完成)                                    │
│  ✅ 信息收集 (完成) - 156 条                                      │
│  ✅ 信息筛选和验证 (完成) - 保留 89 条                            │
│  ✅ 分析和综合 (完成)                                            │
│  ✅ 报告定稿 (完成)                                              │
│                                                                  │
│  【交付物】                                                      │
│  📄 完整报告 (4500 字) [查看] [下载]                             │
│  📊 原始数据 (156 条) [查看] [下载]                              │
│  📋 信息来源列表 [查看]                                          │
│                                                                  │
│  【团队表现】                                                    │
│  - 进度：⭐⭐⭐⭐⭐ (5/5)                                           │
│  - 质量：⭐⭐⭐⭐⭐ (5/5)                                           │
│  - 效率：⭐⭐⭐⭐☆ (4.5/5)                                         │
│  - 沟通：⭐⭐⭐⭐⭐ (5/5)                                           │
│                                                                  │
│  【后续操作】                                                    │
│  [深入分析 XX 方面] [导出为 PDF] [启动后续研究] [关闭项目]       │
└─────────────────────────────────────────────────────────────────┘
```

---

## 五、核心层与编排层（延续 v4.0）

### 5.1 核心层（硬实现）

**每个实例独立运行**：

| 核心组件 | 职责 | 性能目标 |
|---------|------|---------|
| **ClusterCore** | Agent 创建/销毁、团队管理 | Agent 创建 <10ms |
| **ResourceCore** | 资源配额检查、原子操作 | 配额检查 <1ms |
| **HealthCore** | 健康检查循环、自动恢复 | 每 10 秒准时执行 |
| **MemoryCore** | 记忆存储、检索 | 检索 <50ms |
| **MessageCore** | 消息路由、广播 | 路由 <1ms |
| **AuditCore** | 审计追踪、日志 | 异步无阻塞 |

### 5.2 编排层（Skills 化）

**每个实例独立 Skills**：

| Skill 类型 | 技能列表 | 权限 |
|-----------|---------|------|
| **CEO Skills** | 创建团队、审批资源、查询状态、推送经验 | CeoOnly |
| **Team Lead Skills** | 申请资源、共享知识、查询状态、升级到 CEO | TeamLeadOnly |
| **Worker Skills** | 贡献知识、请求帮助、汇报进度 | WorkerOnly |

### 5.3 全局层与实例层通信

```rust
// 全局层调用实例层 Skills

impl ChairmanAgent {
    /// 调用实例 CEO Skill
    async fn invoke_ceo_skill(
        &self,
        ceo_id: &str,
        skill_name: &str,
        params: &Value,
    ) -> Result<Value> {
        // 1. 查找实例
        let instance = self.instances.get(ceo_id)
            .ok_or("实例不存在")?;

        // 2. 发送 Skill 调用请求
        let request = SkillInvocationRequest {
            skill_name: skill_name.to_string(),
            params: params.clone(),
            caller: Caller::Chairman,
        };

        // 3. 等待响应（带超时）
        let response = tokio::time::timeout(
            Duration::from_secs(30),
            self.skill_channel.send_request(request),
        ).await??;

        Ok(response.result)
    }

    /// 订阅实例事件
    async fn subscribe_instance_events(&self, instance_id: &str) -> Result<EventStream> {
        // 订阅实例事件（创建/销毁/异常/完成）
        let stream = self.event_bus.subscribe(format!("instance:{}", instance_id));
        Ok(stream)
    }
}
```

---

## 六、实现计划（12 周）

| 阶段 | 内容 | 工期 | 里程碑 |
|------|------|------|--------|
| **Phase 1** | 全局层 (ChairmanAgent) | 2 周 | M1: 董事长 Agent 完成 |
| **Phase 2** | 全局资源管理 | 1 周 | M2: 全局配额管理完成 |
| **Phase 3** | 信息聚合与过滤 | 1 周 | M3: 噪音过滤完成 |
| **Phase 4** | 快速创建入口 (CLI) | 1 周 | M4: CLI 快速创建完成 |
| **Phase 5** | 快速创建入口 (Telegram) | 1 周 | M5: Telegram Bot 完成 |
| **Phase 6** | 快速创建入口 (Web UI) | 2 周 | M6: Web Dashboard 完成 |
| **Phase 7** | 可观测性增强 | 1 周 | M7: Dashboard 完成 |
| **Phase 8** | 核心层优化 | 1 周 | M8: 性能优化完成 |
| **Phase 9** | 测试 + 文档 | 2 周 | M9: 测试覆盖>80% |

**总计**: 12 周

---

## 七、验收标准

### 7.1 全局层验收

- [ ] 董事长 Agent 启动时自动创建
- [ ] 支持多实例管理（≥10 个实例）
- [ ] 信息聚合定时执行（每 60 秒）
- [ ] 噪音过滤正确率 >90%
- [ ] 重大决策审批流程完整

### 7.2 快速创建验收

- [ ] CLI 快速创建命令可用
- [ ] Telegram Bot 快速创建可用
- [ ] Web UI 表单提交可用
- [ ] 创建后 CEO 自动完成后续配置
- [ ] 创建时间 <5 秒

### 7.3 可观测性验收

- [ ] 全局 Dashboard 显示所有实例
- [ ] 实例详情页显示资源使用
- [ ] 项目详情页显示任务分解
- [ ] 待审批决策列表实时更新
- [ ] 支持多端访问（CLI/Telegram/Web）

### 7.4 性能验收

- [ ] 全局状态查询 <100ms
- [ ] 快速创建 <5 秒
- [ ] 信息同步延迟 <60 秒
- [ ] 单实例性能符合 v4.0 标准

---

## 八、架构对比总结

| 维度 | v3.0 企业组织 | v4.0 混合架构 | v5.0 全局编排 |
|------|-------------|-------------|-------------|
| **架构层级** | 三层 | 三层 | **四层** |
| **用户角色** | 董事长 | 董事长 | **用户→董事长分身** |
| **实例管理** | 单实例 | 单实例 | **多实例** |
| **信息同步** | 直接沟通 | 查询 | **定时聚合 + 过滤** |
| **快速创建** | ❌ | ❌ | ✅ |
| **可观测性** | 基础 | 增强 | **Dashboard** |
| **性能** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **灵活性** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **易用性** | ⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |

---

## 九、总结

### v5.0 核心优势

| 特性 | 说明 |
|------|------|
| **全局董事长** | 用户分身，统一管理多实例，过滤噪音 |
| **快速创建** | CLI/Telegram/Web 多端支持，一键启动 |
| **可观测性** | 全局/实例/项目三级 Dashboard |
| **四层架构** | 全局/编排/核心/执行，职责清晰 |
| **性能保障** | 核心层硬实现，延续 v4.0 性能优势 |
| **灵活性** | 编排层 Skills 化，动态扩展 |

### 架构演进路线

```
v3.0 企业组织模式
    │
    │ + 核心层硬实现 (性能)
    │ + 编排层 Skills 化 (灵活)
    ▼
v4.0 混合架构
    │
    │ + 全局董事长 (多实例管理)
    │ + 快速创建入口 (易用性)
    │ + 可观测性 Dashboard
    ▼
v5.0 全局编排架构
```

### 最终定位

**MultiClaw v5.0** 在 v4.0 混合架构的基础上，增加了全局编排层，形成了完整的四层架构：

- ✅ **全局层**：董事长 Agent，用户分身，多实例管理
- ✅ **编排层**：Skills 化，灵活决策
- ✅ **核心层**：硬实现，性能保障
- ✅ **执行层**：沙箱隔离，安全执行

**v5.0 实现了性能、灵活性、易用性的最佳平衡！**

---

**审批状态**: 待审批
**负责人**: 待定
**最后更新**: 2026 年 3 月 1 日
