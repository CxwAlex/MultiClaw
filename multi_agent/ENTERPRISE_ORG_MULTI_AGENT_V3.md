# MultiClaw 多 Agent 集群架构方案 v3.0 - 企业组织模式

> **版本**: v3.0 - 企业组织模式
> **创建日期**: 2026 年 2 月 28 日
> **优先级**: P0 - 核心能力
> **状态**: 待审批
> **核心理念**: 企业组织架构（董事长→CEO→项目负责人→Worker Agent）

---

## 一、核心愿景

### 1.1 用户角色定位

**用户 = 投资人/董事长**
- 拥有多家公司（多个 MultiClaw 实例）
- 只与 CEO 直接沟通
- 可以查看公司运营状况（项目列表、资源使用、产出报告）
- 重大决策审批（超预算、模式切换）

### 1.2 组织架构

```
┌─────────────────────────────────────────────────────────────────┐
│                        集群层 (Swarm)                            │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    董事长 (用户)                          │    │
│  │              - 拥有多家"公司"（实例）                      │    │
│  │              - 只与 CEO 直接沟通                          │    │
│  │              - 查看公司报告/审批重大决策                   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              CEO (实例管理 Agent)                         │    │
│  │              - 绑定用户终端（Telegram 等）                 │    │
│  │              - 理解董事长需求                            │    │
│  │              - 决定：自己做/创建简单 Agent/创建项目团队    │    │
│  │              - 管理资源配额和模式库                       │    │
│  │              - 审批团队负责人的资源申请                   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        团队层 (Team)                             │
│                                                                  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐             │
│  │ 项目 1       │  │ 项目 2       │  │ 项目 N       │             │
│  │ 市场调研    │  │ 产品开发    │  │ 专家会诊    │             │
│  │             │  │             │  │             │             │
│  │ ┌─────────┐ │  │ ┌─────────┐ │  │ ┌─────────┐ │             │
│  │ │负责人  │ │  │ │负责人  │ │  │ │负责人  │ │             │
│  │ │(主Agent)│ │  │ │(主Agent)│ │  │ │(主Agent)│ │             │
│  │ └────┬────┘ │  │ └────┬────┘ │  │ └────┬────┘ │             │
│  │      │      │  │      │      │  │      │      │             │
│  │ ┌────┴────┐ │  │ ┌────┴────┐ │  │ ┌────┴────┐ │             │
│  │ │Worker  │ │  │ │Worker  │ │  │ │Worker  │ │             │
│  │ │Agents  │ │  │ │Agents  │ │  │ │Agents  │ │             │
│  │ └─────────┘ │  │ └─────────┘ │  │ └─────────┘ │             │
│  └─────────────┘  └─────────────┘  └─────────────┘             │
│                                                                  │
│  团队负责人职责：                                                │
│  - 动态生成 Worker Agent 定义（详细角色职责）                     │
│  - 决定团队规模和结构                                           │
│  - 协调 Worker Agent 合作                                        │
│  - 向 CEO 申请资源调整                                           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                        个体层 (Agent)                            │
│                                                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Worker Agent (动态生成)                                  │    │
│  │  ┌──────────────────────────────────────────────────┐   │    │
│  │  │ 详细角色定义（由团队负责人生成）                    │   │    │
│  │  │ - 角色名称和职责描述（200+字）                     │   │    │
│  │  │ - 工作流程和协作协议                              │   │    │
│  │  │ - 可用工具列表和调用规范                          │   │    │
│  │  │ - 输出质量标准和验收条件                          │   │    │
│  │  │ - 升级/求助机制                                   │   │    │
│  │  └──────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 核心设计原则

| 原则 | 说明 | 实现方式 |
|------|------|---------|
| **智能决策** | 核心管理 Agent 智能决策，非固定匹配 | CEO Agent + 团队负责人 Agent |
| **动态生成** | Agent 定义由主 Agent 动态生成 | LLM 生成详细角色定义 |
| **弹性规模** | 团队大小由团队负责人决定 | 运行时动态调整 |
| **分级管理** | 董事长→CEO→负责人→Worker | 四层架构 |
| **资源审批** | 团队可向 CEO 申请资源调整 | 审批工作流 |
| **简单优先** | 简单任务不启动集群 | CEO 决策：自己做/单 Agent/集群 |

---

## 二、核心 Agent 设计

### 2.1 CEO Agent（实例管理 Agent）

**职责**: 理解董事长需求，决定执行策略，管理资源和模式库

```rust
// src/agent/ceo.rs

use serde::{Deserialize, Serialize};

/// CEO Agent - 实例级别的管理者
pub struct CEOAgent {
    /// 绑定用户终端
    pub user_channel: ChannelId,
    /// 资源配额管理
    pub resource_manager: Arc<ResourceManager>,
    /// 模式库（可动态扩展）
    pub pattern_library: Arc<PatternLibrary>,
    /// 当前项目列表
    pub active_projects: DashMap<String, ProjectHandle>,
    /// 历史决策记录（用于学习）
    pub decision_history: Vec<DecisionRecord>,
}

/// CEO 决策结果
#[derive(Clone, Serialize, Deserialize)]
pub enum CEODecision {
    /// 简单任务：CEO 自己处理
    HandleByMyself {
        response: String,
    },
    /// 创建单个 Agent 处理
    CreateSingleAgent {
        role_definition: DetailedRoleDefinition,
        task: String,
    },
    /// 创建项目团队
    CreateProjectTeam {
        project_name: String,
        pattern_id: String,
        initial_quota: ResourceQuota,
        team_lead_prompt: String,
    },
    /// 转交现有团队
    AssignToExistingProject {
        project_id: String,
        task: String,
    },
    /// 需要董事长审批
    RequiresBoardApproval {
        reason: String,
        proposal: Value,
    },
}

/// 详细角色定义（由 CEO 或团队负责人生成）
#[derive(Clone, Serialize, Deserialize)]
pub struct DetailedRoleDefinition {
    /// 角色名称
    pub role_name: String,
    /// 详细职责描述（200+ 字）
    pub detailed_responsibilities: String,
    /// 工作流程
    pub workflow: Vec<WorkflowStep>,
    /// 可用工具及调用规范
    pub tools_with_specs: Vec<ToolSpecification>,
    /// 输出质量标准
    pub quality_standards: Vec<QualityCriterion>,
    /// 协作协议
    pub collaboration_protocol: CollaborationProtocol,
    /// 升级/求助机制
    pub escalation_policy: EscalationPolicy,
    /// 系统提示词（完整）
    pub system_prompt: String,
}

/// 工作流程步骤
#[derive(Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_number: u8,
    pub description: String,
    pub input_requirements: Vec<String>,
    pub output_format: String,
    pub quality_check: Option<String>,
}

/// 工具规格（详细说明）
#[derive(Clone, Serialize, Deserialize)]
pub struct ToolSpecification {
    pub tool_name: String,
    pub description: String,
    pub usage_guidelines: Vec<String>,
    pub parameter_constraints: Vec<String>,
    pub error_handling: String,
}

/// 质量标准
#[derive(Clone, Serialize, Deserialize)]
pub struct QualityCriterion {
    pub criterion: String,
    pub measurement: String,
    pub threshold: String,
}

/// 升级/求助机制
#[derive(Clone, Serialize, Deserialize)]
pub struct EscalationPolicy {
    /// 何时升级
    pub trigger_conditions: Vec<String>,
    /// 升级对象
    pub escalate_to: String,
    /// 升级时需要提供的信息
    pub required_context: Vec<String>,
}

impl CEOAgent {
    /// 处理董事长需求
    pub async fn handle_board_request(&self, request: &str) -> Result<CEODecision> {
        // 步骤 1: 任务复杂度分析
        let analysis = self.analyze_task(request).await?;
        
        // 步骤 2: 决策（简单任务自己处理，复杂任务创建团队）
        if analysis.complexity < 3 {
            // 简单任务：自己处理或创建单 Agent
            if analysis.estimated_tokens < 5000 {
                return Ok(CEODecision::HandleByMyself {
                    response: self.execute_simple_task(request).await?,
                });
            } else {
                return Ok(CEODecision::CreateSingleAgent {
                    role_definition: self.generate_specialist_role(request).await?,
                    task: request.to_string(),
                });
            }
        }
        
        // 步骤 3: 复杂任务：选择模式并创建项目团队
        let pattern = self.select_pattern(request, &analysis).await?;
        let project_name = self.generate_project_name(request);
        
        // 步骤 4: 生成团队负责人提示词
        let team_lead_prompt = self.generate_team_lead_prompt(&pattern, request).await?;
        
        // 步骤 5: 分配初始资源配额
        let initial_quota = self.allocate_quota(&analysis, &pattern);
        
        Ok(CEODecision::CreateProjectTeam {
            project_name,
            pattern_id: pattern.id,
            initial_quota,
            team_lead_prompt,
        })
    }

    /// 生成专家角色定义（详细）
    async fn generate_specialist_role(&self, task: &str) -> Result<DetailedRoleDefinition> {
        // 调用 LLM 生成详细角色定义
        let prompt = format!(r#"
请为以下任务生成一个专家角色定义：

任务：{task}

请生成以下内容：
1. 角色名称（简洁专业）
2. 详细职责描述（200 字以上，说明具体工作内容）
3. 工作流程（3-5 个步骤，每步说明输入输出）
4. 可用工具及调用规范（每个工具说明使用场景和注意事项）
5. 输出质量标准（3-5 条可衡量的标准）
6. 协作协议（如何与 CEO 和其他 Agent 沟通）
7. 升级/求助机制（什么情况下需要求助，向谁求助）
8. 完整的系统提示词（可直接用于初始化 Agent）

要求：
- 职责描述要具体，避免空泛
- 工作流程要可执行
- 工具使用要有明确规范
- 质量标准要可衡量
"#);

        let response = self.llm.generate(&prompt).await?;
        self.parse_role_definition(&response)
    }

    /// 生成团队负责人提示词
    async fn generate_team_lead_prompt(&self, pattern: &CollaborationPattern, task: &str) -> Result<String> {
        Ok(format!(r#"
你是"{project_name}"项目的负责人，向 CEO 汇报。

## 项目背景
{task}

## 协作模式
模式名称：{pattern_name}
模式描述：{pattern_description}

## 你的职责
1. 根据项目需求，动态生成 Worker Agent 的详细角色定义
2. 决定团队规模和结构（需要多少 Agent，各什么角色）
3. 协调 Worker Agent 之间的合作
4. 监控项目进度，确保按时交付
5. 如遇资源不足，可向 CEO 申请调整配额

## 生成 Worker Agent 的要求
每个 Worker Agent 的定义必须包含：
- 角色名称和详细职责（200+ 字）
- 工作流程（3-5 步）
- 可用工具及调用规范
- 输出质量标准
- 协作协议
- 升级/求助机制

## 资源配额
初始配额：{quota}

## 汇报机制
- 定期向 CEO 汇报进度（每完成一个里程碑）
- 遇到以下情况立即汇报：
  - 资源不足需要追加
  - 需要切换协作模式
  - 遇到无法解决的技术问题
  - 项目可能延期

现在，请开始你的工作：
1. 分析项目需求
2. 设计团队结构
3. 生成 Worker Agent 定义
4. 启动项目执行
"#,
            project_name = self.generate_project_name(task),
            pattern_name = pattern.name,
            pattern_description = pattern.description,
            quota = self.format_quota(&pattern),
        ))
    }
}
```

---

### 2.2 团队负责人 Agent（Project Lead）

**职责**: 动态生成 Worker Agent 定义，协调团队合作

```rust
// src/agent/team_lead.rs

use crate::agent::ceo::DetailedRoleDefinition;

/// 团队负责人 Agent
pub struct TeamLeadAgent {
    /// 项目名称
    pub project_name: String,
    /// 协作模式
    pub pattern: CollaborationPattern,
    /// CEO 分配的资源配额
    pub quota: ResourceQuota,
    /// Worker Agent 列表
    pub workers: DashMap<String, WorkerAgent>,
    /// 任务分解
    pub task_breakdown: Vec<SubTask>,
    /// 与 CEO 的通信通道
    pub ceo_channel: ChannelId,
}

/// Worker Agent 实例
pub struct WorkerAgent {
    pub id: String,
    pub role_definition: DetailedRoleDefinition,
    pub status: AgentStatus,
    pub current_task: Option<SubTask>,
    pub completed_tasks: Vec<SubTask>,
}

impl TeamLeadAgent {
    /// 启动项目：分析需求，生成团队结构
    pub async fn kickoff(&self, project_brief: &str) -> Result<ProjectPlan> {
        // 步骤 1: 项目需求分析
        let analysis = self.analyze_project(project_brief).await?;
        
        // 步骤 2: 设计团队结构（动态决定需要哪些角色）
        let team_structure = self.design_team_structure(&analysis).await?;
        
        // 步骤 3: 为每个角色生成详细的 Worker Agent 定义
        let mut worker_definitions = Vec::new();
        for role in &team_structure.roles {
            let definition = self.generate_worker_definition(role, &analysis).await?;
            worker_definitions.push(definition);
        }
        
        // 步骤 4: 任务分解
        let tasks = self.breakdown_tasks(project_brief, &team_structure).await?;
        
        // 步骤 5: 向 CEO 汇报计划（可选，根据模式要求）
        if self.pattern.requires_ceo_approval {
            self.report_to_ceo(&ProjectPlan {
                team_structure: team_structure.clone(),
                worker_definitions: worker_definitions.clone(),
                tasks: tasks.clone(),
                estimated_completion: self.estimate_completion(&tasks),
            }).await?;
        }
        
        // 步骤 6: 创建 Worker Agent 实例
        for definition in &worker_definitions {
            let worker = self.spawn_worker(definition).await?;
            self.workers.insert(worker.id.clone(), worker);
        }
        
        // 步骤 7: 分配任务并启动执行
        self.assign_and_start_tasks(&tasks).await?;
        
        Ok(ProjectPlan {
            team_structure,
            worker_definitions,
            tasks,
            estimated_completion: self.estimate_completion(&tasks),
        })
    }

    /// 生成 Worker Agent 详细定义（核心方法）
    async fn generate_worker_definition(
        &self,
        role: &TeamRole,
        analysis: &ProjectAnalysis,
    ) -> Result<DetailedRoleDefinition> {
        let prompt = format!(r#"
你是"{project_name}"项目的团队负责人，需要为团队成员生成详细的角色定义。

## 项目信息
项目名称：{project_name}
项目描述：{project_brief}
项目类型：{project_type}
预计规模：{scale}

## 需要定义的角色
角色名称：{role_name}
角色概述：{role_overview}

## 请生成完整的角色定义

### 1. 角色名称
（简洁专业的名称）

### 2. 详细职责描述
（200 字以上，具体说明：
- 这个角色的核心职责是什么
- 在项目中的定位和价值
- 需要完成的具体工作内容
- 与其他角色的协作关系
）

### 3. 工作流程
（3-5 个步骤，每步说明：
- 步骤编号和名称
- 输入要求（需要什么信息/资源）
- 具体操作
- 输出格式（交付什么成果）
- 质量检查点（如何确认完成质量）
）

### 4. 可用工具及调用规范
（针对每个工具说明：
- 工具名称
- 使用场景（什么时候用）
- 调用规范（参数要求、格式要求）
- 注意事项（常见错误、限制条件）
- 错误处理（出错了怎么办）
）

可用工具列表：{available_tools}

### 5. 输出质量标准
（3-5 条可衡量的标准，例如：
- 代码覆盖率 > 80%
- 文档完整度：包含 API 说明、使用示例、注意事项
- 响应时间 < 2 秒
）

### 6. 协作协议
（说明：
- 如何接收任务（从谁那里，什么格式）
- 如何汇报进度（频率、格式、内容）
- 如何请求帮助（什么情况、向谁、提供什么信息）
- 如何交付成果（格式、验收流程）
）

### 7. 升级/求助机制
（明确说明：
- 什么情况下需要升级（技术难题、资源不足、需求变更等）
- 向谁升级（团队负责人/CEO）
- 升级时需要提供的信息（问题描述、已尝试方案、需要的支持）
- 升级响应时间预期
）

### 8. 完整系统提示词
（将以上内容整合为一段可直接用于初始化 Agent 的系统提示词，
语气专业但友好，结构清晰，便于 Agent 理解和执行）

## 输出要求
- 使用 JSON 格式输出
- 每个部分都要详细具体，避免空泛
- 工作流程要可执行
- 质量标准要可衡量
"#,
            project_name = self.project_name,
            project_brief = analysis.brief,
            project_type = analysis.task_type,
            scale = analysis.estimated_scale,
            role_name = role.name,
            role_overview = role.overview,
            available_tools = self.format_available_tools(&role.allowed_tools),
        );

        let response = self.llm.generate(&prompt).await?;
        self.parse_role_definition(&response)
    }

    /// 申请资源调整
    pub async fn request_quota_adjustment(&self, request: &QuotaAdjustmentRequest) -> Result<()> {
        let message = format!(r#"
CEO 您好，我是"{project_name}"项目负责人。

## 当前状况
- 已完成：{completed_percentage}%
- 当前阶段：{current_phase}
- 团队规模：{current_agents} 个 Agent

## 资源申请
申请类型：{request_type}
当前配额：{current_quota}
申请配额：{requested_quota}
申请原因：{reason}

## 项目进展
{progress_summary}

## 预期影响
如批准：{expected_benefit}
如不批准：{risk_if_denied}

请审批，谢谢。
"#,
            project_name = self.project_name,
            completed_percentage = self.get_completion_percentage(),
            current_phase = self.get_current_phase(),
            current_agents = self.workers.len(),
            request_type = request.request_type,
            current_quota = self.format_quota(&self.quota),
            requested_quota = self.format_quota(&request.new_quota),
            reason = request.reason,
            progress_summary = self.get_progress_summary(),
            expected_benefit = request.expected_benefit,
            risk_if_denied = request.risk_if_denied,
        );

        self.send_to_ceo(&message).await
    }
}
```

---

### 2.3 Worker Agent（执行 Agent）

**特点**: 由团队负责人动态生成详细定义

```rust
// src/agent/worker.rs

use crate::agent::ceo::DetailedRoleDefinition;

/// Worker Agent - 具体执行任务的 Agent
pub struct WorkerAgent {
    pub id: String,
    /// 详细角色定义（由团队负责人生成）
    pub role_definition: DetailedRoleDefinition,
    /// 当前状态
    pub status: AgentStatus,
    /// 当前任务
    pub current_task: Option<SubTask>,
    /// 沙箱环境
    pub sandbox: SandboxHandle,
    /// MCP 工具客户端
    pub mcp_client: MCPClient,
}

impl WorkerAgent {
    /// 初始化（使用团队负责人生成的详细定义）
    pub async fn initialize(&self) -> Result<()> {
        // 使用完整的系统提示词初始化
        let system_prompt = &self.role_definition.system_prompt;
        
        // 初始化 LLM 会话
        self.llm.init_with_system_prompt(system_prompt).await?;
        
        // 挂载工具
        for tool_spec in &self.role_definition.tools_with_specs {
            self.mcp_client.register_tool(tool_spec).await?;
        }
        
        // 向团队负责人报到
        self.report_to_lead("已初始化，准备接收任务").await?;
        
        Ok(())
    }

    /// 执行任务
    pub async fn execute_task(&self, task: &SubTask) -> Result<TaskResult> {
        // 检查任务是否符合角色职责
        if !self.is_task_in_scope(task) {
            return Err("任务超出角色职责范围".into());
        }

        // 按照工作流程执行
        let mut result = TaskResult::new();
        for step in &self.role_definition.workflow {
            let step_result = self.execute_workflow_step(step, task).await?;
            result.steps.push(step_result);
            
            // 质量检查
            if let Some(check) = &step.quality_check {
                if !self.verify_quality(&step_result, check).await? {
                    // 质量不达标，重试或升级
                    return self.handle_quality_failure(step, &step_result).await;
                }
            }
        }

        // 交付成果
        self.deliver_result(&result).await?;
        
        Ok(result)
    }

    /// 请求帮助（触发升级机制）
    pub async fn request_help(&self, issue: &str) -> Result<()> {
        // 检查是否符合升级条件
        let should_escalate = self.role_definition.escalation_policy
            .trigger_conditions
            .iter()
            .any(|condition| self.matches_condition(issue, condition));

        if should_escalate {
            // 向团队负责人升级
            self.escalate_to_lead(issue).await?;
        } else {
            // 尝试自己解决或询问同事
            self.try_self_resolve(issue).await?;
        }

        Ok(())
    }
}
```

---

## 三、任务决策流程

### 3.1 CEO 决策树

```
用户任务
    │
    ▼
┌─────────────────┐
│ CEO 分析任务     │
│ - 复杂度评分    │
│ - 预估 Token 数   │
│ - 所需领域知识  │
│ - 时间敏感性    │
└────────┬────────┘
         │
    ┌────┴────┐
    ▼         ▼
┌──────┐  ┌──────┐
│简单  │  │复杂  │
│<3 分 │  │≥3 分 │
└──┬───┘  └──┬───┘
   │         │
   ▼         ▼
┌─────────┐ ┌──────────────┐
│Token<5K │ │选择协作模式   │
└────┬────┘ │- 广撒网       │
     │      │- 分层审批     │
     ▼      │- 专家会诊     │
┌─────────┐ │- 混合众包     │
│CEO 自己  │ │- 动态自适应   │
│处理     │ └──────┬───────┘
└─────────┘        │
                   ▼
            ┌─────────────┐
            │生成团队负责人│
            │提示词        │
            └──────┬──────┘
                   │
                   ▼
            ┌─────────────┐
            │创建项目团队  │
            │分配初始配额  │
            └─────────────┘
```

### 3.2 团队负责人决策树

```
项目任务
    │
    ▼
┌─────────────────┐
│团队负责人分析   │
│- 任务分解       │
│- 所需角色       │
│- 工作量评估     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│设计团队结构     │
│- 需要哪些角色   │
│- 每个角色几人   │
│- 协作流程       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│为每个角色生成    │
│详细 Worker 定义   │
│(调用 LLM)        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│创建 Worker Agent │
│实例             │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│分配任务并启动   │
│执行             │
└─────────────────┘
```

---

## 四、详细角色定义示例

### 4.1 示例：市场调研 Worker Agent

```json
{
  "role_name": "高级市场研究分析师",
  "detailed_responsibilities": "你是一名资深市场研究分析师，专注于科技行业的市场情报收集和分析。你的核心职责包括：1) 通过多种渠道（网络搜索、行业报告、社交媒体）收集目标市场的信息；2) 分析市场规模、增长趋势、竞争格局；3) 识别关键市场参与者和他们的策略；4) 发现市场机会和潜在风险；5) 将收集的信息整理成结构化的研究报告。你需要确保信息的准确性、时效性和相关性，为项目决策提供可靠的数据支持。",
  "workflow": [
    {
      "step_number": 1,
      "description": "需求理解和搜索策略制定",
      "input_requirements": ["研究主题", "目标市场", "关键问题列表"],
      "output_format": "搜索计划文档，包含关键词列表、目标网站列表、预期输出",
      "quality_check": "搜索计划需覆盖至少 5 个信息源，关键词不少于 10 个"
    },
    {
      "step_number": 2,
      "description": "信息收集",
      "input_requirements": ["搜索计划"],
      "output_format": "原始信息集合（URL、标题、摘要、发布时间）",
      "quality_check": "收集不少于 50 条相关信息，时间范围在最近 2 年内"
    },
    {
      "step_number": 3,
      "description": "信息筛选和验证",
      "input_requirements": ["原始信息集合"],
      "output_format": "筛选后的信息列表（带可信度评分）",
      "quality_check": "剔除重复和不可靠来源，保留至少 30 条高质量信息"
    },
    {
      "step_number": 4,
      "description": "分析和综合",
      "input_requirements": ["筛选后的信息列表"],
      "output_format": "分析报告草稿（包含市场规模、趋势、竞争格局）",
      "quality_check": "报告需包含数据支撑的结论，每个结论至少 2 个来源"
    },
    {
      "step_number": 5,
      "description": "报告定稿",
      "input_requirements": ["分析报告草稿", "团队反馈"],
      "output_format": "最终研究报告（Markdown 格式）",
      "quality_check": "通过团队负责人审核，格式规范，数据准确"
    }
  ],
  "tools_with_specs": [
    {
      "tool_name": "web_search",
      "description": "网络搜索引擎，用于查找相关信息",
      "usage_guidelines": [
        "用于查找市场报告、行业新闻、公司信息",
        "每次搜索使用 3-5 个关键词组合",
        "优先使用英文搜索获取国际视野"
      ],
      "parameter_constraints": [
        "query 参数不超过 100 字符",
        "num_results 建议设置为 10-20",
        "time_range 建议设置为'year'获取最新信息"
      ],
      "error_handling": "如搜索结果不理想，尝试更换关键词或搜索平台"
    },
    {
      "tool_name": "web_fetch",
      "description": "网页内容抓取，用于获取详细信息",
      "usage_guidelines": [
        "用于获取搜索结果的详细内容",
        "优先抓取官方报告、权威媒体",
        "注意版权和引用规范"
      ],
      "parameter_constraints": [
        "url 必须是 http/https 开头",
        "max_size 建议设置为 500KB",
        "timeout 设置为 30 秒"
      ],
      "error_handling": "如网页无法访问，记录并尝试其他来源"
    },
    {
      "tool_name": "memory_recall",
      "description": "查询内部记忆，获取历史项目信息",
      "usage_guidelines": [
        "用于查找类似项目的历史数据",
        "避免重复工作",
        "引用历史信息需注明来源"
      ],
      "parameter_constraints": [
        "query 需具体明确",
        "max_results 建议设置为 5-10"
      ],
      "error_handling": "如无相关记忆，继续外部搜索"
    }
  ],
  "quality_standards": [
    "信息准确性：所有数据需有明确来源，关键数据需交叉验证",
    "时效性：80% 以上的信息应在最近 2 年内发布",
    "完整性：报告需覆盖市场规模、增长趋势、竞争格局、机会风险四个维度",
    "可读性：报告结构清晰，使用图表辅助说明，字数 3000-5000 字"
  ],
  "collaboration_protocol": {
    "task_reception": "从团队负责人接收任务，任务格式为 JSON，包含研究主题、关键问题、截止时间",
    "progress_reporting": "每完成一个工作流步骤，向团队负责人发送进度更新（步骤编号、完成状态、遇到的问题）",
    "help_request": "遇到以下情况请求帮助：1) 无法找到足够信息（<30 条）；2) 信息来源可信度低；3) 分析结论存在明显矛盾",
    "deliverable": "最终报告以 Markdown 格式提交，附带信息来源列表和原始数据"
  },
  "escalation_policy": {
    "trigger_conditions": [
      "搜索 2 小时后收集的有效信息少于 20 条",
      "发现相互矛盾的关键数据无法判断",
      "研究范围超出初始定义需要调整",
      "预计无法在截止时间前完成"
    ],
    "escalate_to": "团队负责人",
    "required_context": [
      "问题详细描述",
      "已尝试的解决方案和结果",
      "需要的具体支持（更多时间、更多资源、调整范围等）",
      "建议的解决方案（如有）"
    ]
  },
  "system_prompt": "你是一名高级市场研究分析师，专注于科技行业的市场情报收集和分析。\n\n【你的职责】\n通过多种渠道收集目标市场信息，分析市场规模、增长趋势、竞争格局，识别关键市场参与者和他们的策略，发现市场机会和潜在风险，将收集的信息整理成结构化的研究报告。\n\n【工作流程】\n1. 需求理解和搜索策略制定 → 2. 信息收集 → 3. 信息筛选和验证 → 4. 分析和综合 → 5. 报告定稿\n\n【可用工具】\n- web_search: 网络搜索（每次 3-5 个关键词，优先英文）\n- web_fetch: 网页抓取（优先官方报告、权威媒体）\n- memory_recall: 查询内部记忆（避免重复工作）\n\n【质量标准】\n- 信息准确性：所有数据需有明确来源，关键数据需交叉验证\n- 时效性：80% 以上信息在最近 2 年内\n- 完整性：覆盖市场规模、增长趋势、竞争格局、机会风险\n- 可读性：结构清晰，3000-5000 字\n\n【协作方式】\n- 从团队负责人接收任务（JSON 格式）\n- 每完成一步发送进度更新\n- 遇到困难及时请求帮助\n- 最终提交 Markdown 报告\n\n【升级机制】\n以下情况向团队负责人升级：搜索 2 小时信息<20 条、发现矛盾数据无法判断、研究范围需调整、预计无法按时完成。升级时提供：问题描述、已尝试方案、需要支持、建议方案。\n\n请专业、高效地完成你的工作，为项目决策提供可靠的数据支持。"
}
```

---

## 五、资源管理和审批

### 5.1 资源配额模型

```rust
// src/agent/resource_manager.rs

/// 资源配额
#[derive(Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Token 配额
    pub token_quota: usize,
    /// 最大并发 Agent 数
    pub max_concurrent_agents: usize,
    /// 时间预算（秒）
    pub time_budget_secs: u64,
    /// 成本预算（美分）
    pub cost_budget_cents: u32,
    /// 工具调用次数限制
    pub tool_call_limit: usize,
}

/// 资源使用统计
#[derive(Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub tokens_used: usize,
    pub current_agents: usize,
    pub elapsed_secs: u64,
    pub cost_cents: u32,
    pub tool_calls: usize,
}

impl ResourceManager {
    /// CEO 分配初始配额
    pub fn allocate_initial_quota(&self, analysis: &TaskAnalysis, pattern: &CollaborationPattern) -> ResourceQuota {
        ResourceQuota {
            token_quota: self.estimate_token_usage(analysis) * 120 / 100, // 20% 缓冲
            max_concurrent_agents: pattern.team_structure.recommended_agents,
            time_budget_secs: self.estimate_time(analysis) * 150 / 100,
            cost_budget_cents: self.estimate_cost(analysis) * 150 / 100,
            tool_call_limit: analysis.estimated_scale * 3,
        }
    }

    /// 审批团队的资源调整申请
    pub async fn review_quota_adjustment(
        &self,
        request: &QuotaAdjustmentRequest,
        project_status: &ProjectStatus,
    ) -> QuotaDecision {
        // 评估团队表现
        let performance_score = self.evaluate_performance(project_status);
        
        // 检查资源余量
        let available_resources = self.get_available_resources();
        
        // 决策逻辑
        if performance_score < 0.5 {
            // 表现不佳，拒绝或减少
            QuotaDecision::Denied {
                reason: "团队表现不佳，建议先优化执行效率".to_string(),
                counter_offer: None,
            }
        } else if available_resources < request.requested_increase {
            // 资源不足，部分批准
            QuotaDecision::PartialApproval {
                approved_quota: self.calculate_partial_quota(&request, &available_resources),
                reason: "资源紧张，部分批准".to_string(),
            }
        } else if performance_score > 0.8 && request.reason.contains("进展顺利") {
            // 表现优秀，额外奖励
            QuotaDecision::ApprovedWithBonus {
                approved_quota: request.new_quota.clone(),
                bonus_percentage: 10,
                reason: "团队表现优秀，额外奖励 10% 资源".to_string(),
            }
        } else {
            // 正常批准
            QuotaDecision::Approved {
                approved_quota: request.new_quota.clone(),
            }
        }
    }

    /// 评估团队表现
    fn evaluate_performance(&self, status: &ProjectStatus) -> f32 {
        let mut score = 0.0;
        
        // 进度评分（40%）
        let progress_score = status.completed_percentage as f32 / 100.0;
        score += progress_score * 0.4;
        
        // 质量评分（30%）
        let quality_score = status.average_quality_score / 10.0;
        score += quality_score * 0.3;
        
        // 效率评分（20%）
        let efficiency_score = self.calculate_efficiency_score(status);
        score += efficiency_score * 0.2;
        
        // 沟通评分（10%）
        let communication_score = status.communication_score / 10.0;
        score += communication_score * 0.1;
        
        score
    }
}

/// 配额决策结果
#[derive(Clone, Serialize, Deserialize)]
pub enum QuotaDecision {
    Approved {
        approved_quota: ResourceQuota,
    },
    ApprovedWithBonus {
        approved_quota: ResourceQuota,
        bonus_percentage: u8,
        reason: String,
    },
    PartialApproval {
        approved_quota: ResourceQuota,
        reason: String,
    },
    Denied {
        reason: String,
        counter_offer: Option<ResourceQuota>,
    },
}
```

---

## 六、用户交互设计

### 6.1 用户（董事长）视角

```
用户 ↔ CEO（Telegram Bot）

用户发送："我想了解 AI 编程助手市场"

CEO 回复：
━━━━━━━━━━━━━━━━━━━━━━
收到，董事长。

这是一个复杂的市场调研任务，我将启动"市场调研项目"。

【项目信息】
- 项目名称：AI 编程助手市场调研
- 协作模式：广撒网并行采集
- 团队负责人：市场研究专家 Agent
- 预计团队规模：15-30 个 Worker Agent
- 预计完成时间：2 小时
- 资源配额：50 万 Token

项目已启动，我会持续监控进展并在完成时向您汇报。

您随时可以询问：
- "项目进展如何" - 查看进度
- "查看 AI 编程助手市场调研" - 查看项目详情
━━━━━━━━━━━━━━━━━━━━━━

2 小时后...

CEO 回复：
━━━━━━━━━━━━━━━━━━━━━━
董事长，"AI 编程助手市场调研"项目已完成。

【项目成果】
- 收集信息：156 条
- 分析报告：1 份（4500 字）
- 关键发现：
  1. 全球 AI 编程助手市场规模 2025 年达$XX 亿
  2. 主要竞争者：GitHub Copilot、Cursor、Codeium
  3. 增长趋势：年复合增长率 XX%
  4. 市场机会：...

【资源使用】
- Token 使用：42 万 / 50 万（84%）
- 实际耗时：1 小时 45 分
- 团队规模：峰值 22 个 Agent

【交付物】
📄 完整报告：[查看链接]

请审阅，如有其他需求请随时告知。
━━━━━━━━━━━━━━━━━━━━━━
```

### 6.2 查看项目详情

```
用户发送："查看 AI 编程助手市场调研"

CEO 回复：
━━━━━━━━━━━━━━━━━━━━━━
【项目详情】AI 编程助手市场调研

【基本信息】
- 状态：✅ 已完成
- 启动时间：2026-02-28 14:00
- 完成时间：2026-02-28 15:45
- 协作模式：广撒网并行采集

【团队结构】
- 团队负责人：市场研究专家
- Worker Agent: 22 个
  - 信息收集 Agent: 15 个
  - 数据分析 Agent: 5 个
  - 报告撰写 Agent: 2 个

【任务分解】
✅ 需求分析和搜索策略（完成）
✅ 信息收集（完成）- 156 条
✅ 信息筛选和验证（完成）- 保留 89 条
✅ 分析和综合（完成）
✅ 报告定稿（完成）

【资源使用】
- Token: 42 万 / 50 万（84%）
- 时间：1h45m / 2h（87.5%）
- 成本：$0.42 / $0.50（84%）

【交付物】
📄 完整报告：[查看链接]
📊 原始数据：[下载链接]
📋 信息来源列表：[查看链接]

【团队表现】
- 进度：⭐⭐⭐⭐⭐ (5/5)
- 质量：⭐⭐⭐⭐⭐ (5/5)
- 效率：⭐⭐⭐⭐☆ (4.5/5)
- 沟通：⭐⭐⭐⭐⭐ (5/5)

需要我做什么吗？
- "重新生成报告" - 调整方向重新分析
- "深入分析 XX 方面" - 启动后续研究
- "导出为 PDF" - 格式转换
━━━━━━━━━━━━━━━━━━━━━━
```

---

## 六、补充策略

### 6.1 团队记忆共享机制

#### 设计理念

借鉴企业知识管理系统，实现**三层记忆共享**：

```
┌─────────────────────────────────────────────────────────────────┐
│                        集群记忆 (CEO 层)                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  集群经验库                                               │    │
│  │  - 成功项目案例（可复用的模式/流程）                       │    │
│  │  - 失败项目复盘（避免的错误）                             │    │
│  │  - 最佳实践库（各领域的标准工作流程）                      │    │
│  │  - 资源使用统计（各模式的平均消耗）                       │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              ↑                                   │
│              CEO 主动推送 / 项目负责人查询                        │
└─────────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────────┐
│                        项目记忆 (团队层)                          │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  项目知识库                                               │    │
│  │  - 当前项目文档（所有 Worker 的产出）                       │    │
│  │  - 中间成果（未完成但有价值的发现）                       │    │
│  │  - 问题解决方案（已解决的技术难题）                       │    │
│  │  - Worker 协作记录（沟通历史、决策记录）                   │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              ↑                                   │
│              项目负责人主动共享 / Worker 自动贡献                  │
└─────────────────────────────────────────────────────────────────┘
                              ↑
┌─────────────────────────────────────────────────────────────────┐
│                        Worker 记忆 (个体层)                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  工作记忆                                                 │    │
│  │  - 当前任务上下文                                        │    │
│  │  - 已尝试方案和结果                                      │    │
│  │  - 临时数据和中间结果                                    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

#### 记忆共享流程

```rust
// src/agent/memory/sharing.rs

/// 记忆共享管理器
pub struct MemorySharingManager {
    /// 集群经验库
    cluster_experience: Arc<DashMap<String, ExperienceEntry>>,
    /// 项目知识库
    project_knowledge: Arc<DashMap<String, ProjectKnowledge>>,
    /// 记忆访问统计（用于优化）
    access_stats: DashMap<String, AccessStats>,
}

/// 经验条目（集群层）
#[derive(Clone, Serialize, Deserialize)]
pub struct ExperienceEntry {
    /// 经验类型
    pub entry_type: ExperienceType,
    /// 来源项目
    pub source_project: String,
    /// 经验描述
    pub description: String,
    /// 适用场景
    pub applicable_scenarios: Vec<String>,
    /// 可复用的模式/流程
    pub reusable_pattern: Option<CollaborationPattern>,
    /// 避免的错误
    pub pitfalls_to_avoid: Vec<String>,
    /// 贡献者（项目负责人）
    pub contributor: String,
    /// 贡献时间
    pub contributed_at: DateTime<Utc>,
    /// 被引用次数
    pub citation_count: usize,
    /// 有效性评分（0-1）
    pub effectiveness_score: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ExperienceType {
    /// 成功经验（可复用）
    SuccessStory,
    /// 失败复盘（避免错误）
    FailureReview,
    /// 最佳实践
    BestPractice,
    /// 资源使用统计
    ResourceStatistics,
}

/// 项目知识（团队层）
#[derive(Clone, Serialize, Deserialize)]
pub struct ProjectKnowledge {
    /// 项目 ID
    pub project_id: String,
    /// 知识条目
    pub entries: Vec<KnowledgeEntry>,
    /// 共享状态
    pub sharing_status: SharingStatus,
    /// 最后更新
    pub last_updated: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    /// 知识类型
    pub entry_type: KnowledgeType,
    /// 贡献者（Worker ID）
    pub contributor: String,
    /// 知识内容
    pub content: String,
    /// 关联任务
    pub related_task: Option<String>,
    /// 被访问次数
    pub access_count: usize,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum KnowledgeType {
    /// 项目文档
    Documentation,
    /// 中间成果
    IntermediateResult,
    /// 问题解决方案
    ProblemSolution,
    /// 协作记录
    CollaborationRecord,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SharingStatus {
    /// 私有（仅项目内可见）
    Private,
    /// 已申请共享（等待 CEO 审批）
    PendingApproval,
    /// 已共享到集群
    SharedToCluster,
    /// 推荐共享（CEO 标记为有价值）
    RecommendedForSharing,
}

impl MemorySharingManager {
    /// Worker 贡献知识到项目记忆
    pub async fn worker_contribute(
        &self,
        project_id: &str,
        worker_id: &str,
        knowledge: KnowledgeEntry,
    ) -> Result<()> {
        let mut project = self.project_knowledge
            .get_mut(project_id)
            .ok_or("项目不存在")?;
        
        project.entries.push(knowledge);
        project.last_updated = Utc::now();
        
        // 自动检查是否值得共享到集群
        if self.is_worth_sharing(&knowledge) {
            project.sharing_status = SharingStatus::RecommendedForSharing;
            // 通知项目负责人
            self.notify_team_lead(project_id, "发现潜在有价值知识，建议共享到集群").await?;
        }
        
        Ok(())
    }

    /// 项目负责人主动共享到集群
    pub async fn team_lead_share_to_cluster(
        &self,
        project_id: &str,
        knowledge_ids: Vec<String>,
        share_reason: &str,
    ) -> Result<()> {
        let project = self.project_knowledge
            .get(project_id)
            .ok_or("项目不存在")?;
        
        // 提取知识条目
        let entries: Vec<_> = project.entries.iter()
            .filter(|e| knowledge_ids.contains(&e.contributor))
            .collect();
        
        // 创建集群经验条目
        let experience = ExperienceEntry {
            entry_type: ExperienceType::BestPractice,
            source_project: project_id.clone(),
            description: format!("来自{}项目的经验分享：{}", project_id, share_reason),
            applicable_scenarios: self.extract_applicable_scenarios(&entries),
            reusable_pattern: None, // 可选
            pitfalls_to_avoid: vec![],
            contributor: project_id.clone(),
            contributed_at: Utc::now(),
            citation_count: 0,
            effectiveness_score: 0.5, // 初始值
        };
        
        // 添加到集群经验库
        self.cluster_experience.insert(
            format!("{}_{}", project_id, Utc::now().timestamp()),
            experience,
        );
        
        // 更新项目共享状态
        let mut project = self.project_knowledge.get_mut(project_id).unwrap();
        project.sharing_status = SharingStatus::SharedToCluster;
        
        Ok(())
    }

    /// CEO 推送集群经验到项目
    pub async fn ceo_push_to_project(
        &self,
        project_id: &str,
        experience_id: &str,
        push_reason: &str,
    ) -> Result<()> {
        let experience = self.cluster_experience
            .get(experience_id)
            .ok_or("经验不存在")?;
        
        // 创建项目知识条目
        let knowledge = KnowledgeEntry {
            entry_type: KnowledgeType::ProblemSolution,
            contributor: "CEO".to_string(),
            content: format!(
                "【集群经验推荐】{}\n适用场景：{}\n{}\n推荐原因：{}",
                experience.description,
                experience.applicable_scenarios.join(", "),
                experience.description,
                push_reason
            ),
            related_task: None,
            access_count: 0,
        };
        
        // 添加到项目知识
        let mut project = self.project_knowledge.get_mut(project_id).unwrap();
        project.entries.push(knowledge);
        project.last_updated = Utc::now();
        
        // 通知项目负责人
        self.notify_team_lead(project_id, &format!(
            "CEO 推送集群经验：{}，请查阅参考",
            experience.description
        )).await?;
        
        Ok(())
    }

    /// 项目完成时自动复盘
    pub async fn auto_review_on_completion(
        &self,
        project_id: &str,
        project_result: &ProjectResult,
    ) -> Result<()> {
        // 分析项目结果
        let review = self.generate_project_review(project_result).await?;
        
        // 创建经验条目
        let experience = ExperienceEntry {
            entry_type: if project_result.success {
                ExperienceType::SuccessStory
            } else {
                ExperienceType::FailureReview
            },
            source_project: project_id.to_string(),
            description: review.summary,
            applicable_scenarios: review.applicable_scenarios,
            reusable_pattern: review.reusable_pattern,
            pitfalls_to_avoid: review.pitfalls,
            contributor: project_id.to_string(),
            contributed_at: Utc::now(),
            citation_count: 0,
            effectiveness_score: if project_result.success { 0.8 } else { 0.5 },
        };
        
        // 自动添加到集群经验库
        self.cluster_experience.insert(
            format!("review_{}_{}", project_id, Utc::now().timestamp()),
            experience,
        );
        
        Ok(())
    }

    /// 查询集群经验（供项目负责人参考）
    pub async fn query_cluster_experience(
        &self,
        project_context: &str,
        top_k: usize,
    ) -> Vec<ExperienceEntry> {
        let mut all_entries: Vec<_> = self.cluster_experience.iter()
            .map(|e| e.value().clone())
            .collect();
        
        // 按相关性排序
        all_entries.sort_by(|a, b| {
            let score_a = self.calculate_relevance(a, project_context);
            let score_b = self.calculate_relevance(b, project_context);
            score_b.partial_cmp(&score_a).unwrap()
        });
        
        // 返回 Top-K
        all_entries.into_iter().take(top_k).collect()
    }

    /// 检查知识是否值得共享
    fn is_worth_sharing(&self, knowledge: &KnowledgeEntry) -> bool {
        // 启发式规则：
        // 1. 问题解决方案类型
        // 2. 内容长度>500 字（详细说明）
        // 3. 包含关键词（"突破"、"创新"、"首次"等）
        
        if knowledge.entry_type != KnowledgeType::ProblemSolution {
            return false;
        }
        
        if knowledge.content.len() < 500 {
            return false;
        }
        
        let valuable_keywords = ["突破", "创新", "首次", "最佳实践", "推荐", "重要"];
        valuable_keywords.iter().any(|kw| knowledge.content.contains(kw))
    }

    /// 计算经验相关性
    fn calculate_relevance(&self, experience: &ExperienceEntry, context: &str) -> f32 {
        let mut score = 0.0;
        
        // 场景匹配（40%）
        for scenario in &experience.applicable_scenarios {
            if context.contains(scenario) {
                score += 0.4;
                break;
            }
        }
        
        // 引用次数（30%）
        score += (experience.citation_count.min(10) as f32) * 0.03;
        
        // 有效性评分（30%）
        score += experience.effectiveness_score * 0.3;
        
        score.min(1.0)
    }
}

/// 项目复盘报告
struct ProjectReview {
    summary: String,
    applicable_scenarios: Vec<String>,
    reusable_pattern: Option<CollaborationPattern>,
    pitfalls: Vec<String>,
}
```

#### 记忆共享示例

```
场景 1: Worker 贡献知识

Worker Agent (市场研究员):
"我发现了一个新的数据来源，可以获取更准确的市场规模数据。"
→ 自动贡献到项目记忆
→ 项目负责人标记为"有价值"
→ CEO 审批后共享到集群经验库
→ 其他项目可以查询使用

场景 2: CEO 推送经验

CEO:
"检测到你们正在进行市场调研，集群经验库中有 3 个相关成功案例，
已推送到项目记忆，请参考。"
→ 项目负责人查看推送
→ 应用到当前项目
→ 提升效率

场景 3: 项目完成自动复盘

项目完成:
"AI 编程助手市场调研"项目完成，质量评分 4.8/5
→ 自动创建成功经验条目
→ 添加到集群经验库
→ 标记为"市场调研最佳实践"
→ 后续类似项目可参考
```

---

### 6.2 Worker 自动重试与状态检查机制

#### 设计理念

借鉴大数据集群（如 Kubernetes、Spark）的**健康检查 + 自动恢复**机制：

```
┌─────────────────────────────────────────────────────────────────┐
│                    Worker 生命周期管理                            │
│                                                                  │
│  创建 → 初始化 → 运行中 → 完成任务 → 销毁                        │
│           ↑        │                                              │
│           │        ▼                                              │
│           │    ┌─────────┐                                       │
│           │    │状态检查 │                                       │
│           │    └────┬────┘                                       │
│           │         │                                            │
│           │    ┌────┴────┐                                       │
│           │    ▼         ▼                                       │
│           │  健康      异常                                       │
│           │   │          │                                        │
│           │   │    ┌─────┴─────┐                                 │
│           │   │    │ 自动重试  │                                 │
│           │   │    │ (最多 3 次) │                                 │
│           │   │    └─────┬─────┘                                 │
│           │   │          │                                        │
│           │   │    ┌─────┴─────┐                                 │
│           │   │    ▼         ▼                                   │
│           │   │  成功     失败                                   │
│           │   │   │        │                                     │
│           │   │   │    ┌───┴───┐                                 │
│           │   │   │    │升级   │                                 │
│           │   │   │    │负责人 │                                 │
│           │   │   │    └───────┘                                 │
│           │   │                                                  │
│           │  继续运行                                            │
│           │                                                      │
│           └──────────────────────────────────────────────────────┘
│                        心跳保活（每 30 秒）
└─────────────────────────────────────────────────────────────────┘
```

#### 状态检查与重试实现

```rust
// src/agent/worker/health_check.rs

use tokio::time::{interval, Duration};
use std::sync::atomic::{AtomicU8, Ordering};

/// Worker 健康状态
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealthStatus {
    /// 初始化中
    Initializing,
    /// 正常运行
    Healthy,
    /// 忙碌中（执行任务）
    Busy,
    /// 响应缓慢
    Slow,
    /// 异常（可恢复）
    Unhealthy,
    /// 失败（不可恢复）
    Failed,
    /// 已终止
    Terminated,
}

/// Worker 健康检查器
pub struct WorkerHealthChecker {
    /// Worker ID
    worker_id: String,
    /// 当前状态
    status: AtomicU8, // 使用 AtomicU8 存储 WorkerHealthStatus
    /// 心跳间隔（秒）
    heartbeat_interval_secs: u64,
    /// 超时阈值（秒）
    timeout_threshold_secs: u64,
    /// 最大重试次数
    max_retry_count: u8,
    /// 当前重试次数
    current_retry_count: AtomicU8,
    /// 任务执行统计
    task_stats: TaskStatistics,
}

/// 任务统计
#[derive(Clone, Default)]
pub struct TaskStatistics {
    /// 总任务数
    pub total_tasks: usize,
    /// 成功任务数
    pub successful_tasks: usize,
    /// 失败任务数
    pub failed_tasks: usize,
    /// 重试任务数
    pub retried_tasks: usize,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: f64,
}

/// 自动重试策略
#[derive(Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_retries: u8,
    /// 初始退避时间（毫秒）
    pub initial_backoff_ms: u64,
    /// 最大退避时间（毫秒）
    pub max_backoff_ms: u64,
    /// 退避乘数
    pub backoff_multiplier: f32,
    /// 可重试的错误类型
    pub retryable_errors: Vec<WorkerErrorType>,
}

/// Worker 错误类型
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WorkerErrorType {
    /// 网络超时
    NetworkTimeout,
    /// API 限流
    RateLimited,
    /// 临时服务不可用
    TemporaryUnavailable,
    /// 资源不足
    InsufficientResources,
    /// 任务执行超时
    TaskTimeout,
    /// 其他可恢复错误
    OtherRecoverable,
    /// 不可恢复错误
    Unrecoverable,
}

impl WorkerHealthChecker {
    /// 创建健康检查器
    pub fn new(worker_id: String, config: &HealthCheckConfig) -> Self {
        Self {
            worker_id,
            status: AtomicU8::new(WorkerHealthStatus::Initializing as u8),
            heartbeat_interval_secs: config.heartbeat_interval_secs,
            timeout_threshold_secs: config.timeout_threshold_secs,
            max_retry_count: config.max_retry_count,
            current_retry_count: AtomicU8::new(0),
            task_stats: TaskStatistics::default(),
        }
    }

    /// 启动健康检查循环
    pub async fn start_health_check_loop(
        self: Arc<Self>,
        worker: Arc<WorkerAgent>,
    ) -> Result<()> {
        let mut heartbeat_interval = interval(Duration::from_secs(self.heartbeat_interval_secs));
        let mut status_check_interval = interval(Duration::from_secs(10)); // 每 10 秒检查状态

        loop {
            tokio::select! {
                // 心跳
                _ = heartbeat_interval.tick() => {
                    self.send_heartbeat(&worker).await?;
                }
                // 状态检查
                _ = status_check_interval.tick() => {
                    self.check_status(&worker).await?;
                }
            }
        }
    }

    /// 发送心跳
    async fn send_heartbeat(&self, worker: &WorkerAgent) -> Result<()> {
        let status = self.get_status();
        
        // 向团队负责人发送心跳
        worker.report_to_lead(&format!(
            "心跳：状态={:?}, 任务统计={:?}",
            status, self.task_stats
        )).await?;
        
        Ok(())
    }

    /// 检查状态
    async fn check_status(&self, worker: &WorkerAgent) -> Result<()> {
        let current_status = self.get_status();
        
        match current_status {
            WorkerHealthStatus::Healthy | WorkerHealthStatus::Busy => {
                // 正常，无需操作
            }
            WorkerHealthStatus::Slow => {
                // 响应缓慢，记录日志
                tracing::warn!(worker_id = %self.worker_id, "Worker 响应缓慢");
            }
            WorkerHealthStatus::Unhealthy => {
                // 异常，尝试自动恢复
                self.attempt_auto_recovery(worker).await?;
            }
            WorkerHealthStatus::Failed => {
                // 失败，通知团队负责人
                worker.escalate_to_lead(&format!(
                    "Worker {} 失败，任务统计：{:?}",
                    self.worker_id, self.task_stats
                )).await?;
            }
            _ => {}
        }
        
        Ok(())
    }

    /// 尝试自动恢复
    async fn attempt_auto_recovery(&self, worker: &WorkerAgent) -> Result<()> {
        let current_retry = self.current_retry_count.load(Ordering::Relaxed);
        
        if current_retry >= self.max_retry_count {
            // 超过最大重试次数，标记为失败
            self.set_status(WorkerHealthStatus::Failed);
            return Err("超过最大重试次数".into());
        }
        
        // 计算退避时间（指数退避）
        let backoff_ms = self.calculate_backoff(current_retry);
        tracing::info!(
            worker_id = %self.worker_id,
            retry_count = current_retry,
            backoff_ms = backoff_ms,
            "尝试自动恢复"
        );
        
        // 等待退避时间
        tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
        
        // 重置 Worker 状态
        worker.reset_state().await?;
        
        // 重试当前任务
        if let Some(task) = &worker.current_task {
            self.current_retry_count.fetch_add(1, Ordering::Relaxed);
            self.task_stats.retried_tasks += 1;
            
            match worker.retry_task(task).await {
                Ok(_) => {
                    // 重试成功
                    self.set_status(WorkerHealthStatus::Healthy);
                    self.current_retry_count.store(0, Ordering::Relaxed);
                }
                Err(e) => {
                    // 重试失败，继续累加重试次数
                    tracing::error!(worker_id = %self.worker_id, error = %e, "重试失败");
                }
            }
        }
        
        Ok(())
    }

    /// 计算退避时间（指数退避 + 抖动）
    fn calculate_backoff(&self, retry_count: u8) -> u64 {
        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 60000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                WorkerErrorType::NetworkTimeout,
                WorkerErrorType::RateLimited,
                WorkerErrorType::TemporaryUnavailable,
            ],
        };
        
        // 指数退避
        let backoff = policy.initial_backoff_ms as f64
            * policy.backoff_multiplier.pow(retry_count as u32) as f64;
        
        // 限制最大值
        let capped_backoff = backoff.min(policy.max_backoff_ms as f64);
        
        // 添加抖动（±20%），避免多个 Worker 同时重试
        let jitter = (rand::random::<f64>() * 0.4 - 0.2) * capped_backoff;
        
        (capped_backoff + jitter) as u64
    }

    /// 执行任务（带状态跟踪）
    pub async fn execute_task_with_monitoring(
        &self,
        worker: &WorkerAgent,
        task: &SubTask,
    ) -> Result<TaskResult> {
        let start_time = Instant::now();
        
        // 设置状态为忙碌
        self.set_status(WorkerHealthStatus::Busy);
        
        // 执行任务
        let result = match tokio::time::timeout(
            Duration::from_secs(self.timeout_threshold_secs),
            worker.execute_task(task),
        ).await {
            Ok(Ok(result)) => {
                // 成功
                let duration = start_time.elapsed().as_millis() as f64;
                self.update_stats(true, duration);
                self.set_status(WorkerHealthStatus::Healthy);
                result
            }
            Ok(Err(e)) => {
                // 执行错误
                let error_type = self.classify_error(&e);
                if self.is_retryable(&error_type) {
                    self.set_status(WorkerHealthStatus::Unhealthy);
                } else {
                    self.set_status(WorkerHealthStatus::Failed);
                }
                self.update_stats(false, 0.0);
                return Err(e);
            }
            Err(_) => {
                // 超时
                tracing::error!(worker_id = %self.worker_id, "任务执行超时");
                self.set_status(WorkerHealthStatus::Unhealthy);
                self.update_stats(false, 0.0);
                return Err("任务执行超时".into());
            }
        };
        
        Ok(result)
    }

    /// 更新统计
    fn update_stats(&self, success: bool, execution_time_ms: f64) {
        self.task_stats.total_tasks += 1;
        if success {
            self.task_stats.successful_tasks += 1;
        } else {
            self.task_stats.failed_tasks += 1;
        }
        
        // 更新平均执行时间（移动平均）
        let n = self.task_stats.total_tasks as f64;
        self.task_stats.avg_execution_time_ms =
            (self.task_stats.avg_execution_time_ms * (n - 1.0) + execution_time_ms) / n;
    }

    /// 错误分类
    fn classify_error(&self, error: &Error) -> WorkerErrorType {
        let error_str = error.to_string().to_lowercase();
        
        if error_str.contains("timeout") || error_str.contains("timed out") {
            WorkerErrorType::NetworkTimeout
        } else if error_str.contains("rate limit") || error_str.contains("429") {
            WorkerErrorType::RateLimited
        } else if error_str.contains("unavailable") || error_str.contains("503") {
            WorkerErrorType::TemporaryUnavailable
        } else if error_str.contains("memory") || error_str.contains("quota") {
            WorkerErrorType::InsufficientResources
        } else {
            WorkerErrorType::Unrecoverable
        }
    }

    /// 检查是否可重试
    fn is_retryable(&self, error_type: &WorkerErrorType) -> bool {
        let policy = RetryPolicy {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 60000,
            backoff_multiplier: 2.0,
            retryable_errors: vec![
                WorkerErrorType::NetworkTimeout,
                WorkerErrorType::RateLimited,
                WorkerErrorType::TemporaryUnavailable,
            ],
        };
        
        policy.retryable_errors.contains(error_type)
    }

    /// 获取状态
    fn get_status(&self) -> WorkerHealthStatus {
        let status_code = self.status.load(Ordering::Relaxed);
        match status_code {
            0 => WorkerHealthStatus::Initializing,
            1 => WorkerHealthStatus::Healthy,
            2 => WorkerHealthStatus::Busy,
            3 => WorkerHealthStatus::Slow,
            4 => WorkerHealthStatus::Unhealthy,
            5 => WorkerHealthStatus::Failed,
            _ => WorkerHealthStatus::Terminated,
        }
    }

    /// 设置状态
    fn set_status(&self, status: WorkerHealthStatus) {
        self.status.store(status as u8, Ordering::Relaxed);
    }
}

/// 健康检查配置
#[derive(Clone)]
pub struct HealthCheckConfig {
    /// 心跳间隔（秒）
    pub heartbeat_interval_secs: u64,
    /// 超时阈值（秒）
    pub timeout_threshold_secs: u64,
    /// 最大重试次数
    pub max_retry_count: u8,
    /// 缓慢阈值（毫秒）
    pub slow_threshold_ms: u64,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: 30,
            timeout_threshold_secs: 300,
            max_retry_count: 3,
            slow_threshold_ms: 10000,
        }
    }
}
```

#### 重试机制示例

```
场景：Worker 执行任务失败自动重试

Worker A 执行任务 "搜索 AI 编程助手市场数据"

第 1 次尝试:
→ 调用 web_search API
→ 网络超时（>30 秒）
→ 错误分类：NetworkTimeout（可重试）
→ 状态设置为 Unhealthy
→ 等待 1 秒（初始退避）

第 2 次尝试:
→ 调用 web_search API
→ API 返回 429 Rate Limited
→ 错误分类：RateLimited（可重试）
→ 等待 2 秒（指数退避）

第 3 次尝试:
→ 调用 web_search API
→ 成功获取数据
→ 状态恢复为 Healthy
→ 重试计数清零
→ 任务完成

如果第 3 次仍失败:
→ 等待 4 秒（继续指数退避）
→ 第 4 次尝试（超过最大重试次数 3）
→ 状态设置为 Failed
→ 升级通知团队负责人
→ 负责人决定：重新分配任务/调整策略
```

---

## 七、实现计划（10 周）

| 阶段 | 内容 | 工期 | 里程碑 |
|------|------|------|--------|
| **Phase 1** | CEO Agent 核心逻辑 | 2 周 | M1: CEO 可智能决策 |
| **Phase 2** | 团队负责人 Agent | 2 周 | M2: 可生成详细角色定义 |
| **Phase 3** | Worker Agent 动态生成 | 1 周 | M3: Worker 定义>200 字 |
| **Phase 4** | 资源管理和审批 | 1 周 | M4: 配额审批流程正常 |
| **Phase 5** | 用户交互（Telegram） | 1 周 | M5: 用户可查看项目 |
| **Phase 6** | 模式库（5 种模式） | 1 周 | M6: 模式可动态选择 |
| **Phase 7** | 项目状态追踪 | 1 周 | M7: 进度报告正常 |
| **Phase 8** | 测试 + 文档 | 1 周 | M8: 测试覆盖>80% |

**总计**: 10 周

---

## 八、验收标准

### 8.1 功能验收

- [ ] CEO 可智能决策（自己做/单 Agent/集群）
- [ ] 团队负责人可生成详细 Worker 定义（>200 字）
- [ ] Worker 定义包含：职责/流程/工具/质量标准/协作协议/升级机制
- [ ] 团队可向 CEO 申请资源调整
- [ ] CEO 可审批资源申请（批准/部分批准/拒绝）
- [ ] 用户可查看项目列表和详情
- [ ] 5 种协作模式可用

### 8.2 质量验收

| 指标 | 目标值 | 测试方法 |
|------|--------|---------|
| Worker 定义详细度 | >200 字 | 文本长度检查 |
| 角色定义完整性 | 6 个部分齐全 | 结构检查 |
| 决策准确率 | >80% | 人工评估 |
| 用户满意度 | >4/5 分 | 用户测试 |

---

## 九、总结

### 核心优势

| 特性 | v2.0 | v3.0 企业组织模式 |
|------|------|------------------|
| **决策方式** | 固定相似度匹配 | CEO Agent 智能决策 |
| **团队规模** | 预定义范围 | 团队负责人动态决定 |
| **Agent 定义** | 简单模板 | LLM 生成详细定义（>200 字） |
| **简单任务** | 也启动集群 | CEO 决策：自己做/单 Agent/集群 |
| **资源管理** | 固定配额 | 可申请调整，CEO 审批 |
| **用户交互** | 技术视角 | 企业组织视角（董事长→CEO） |

### 企业组织类比

```
现实企业              MultiClaw
────────────────────────────────────────
投资人/董事长    →    用户
CEO              →    CEO Agent（实例管理）
项目负责人       →    团队负责人 Agent
部门员工         →    Worker Agent
公司             →    MultiClaw 实例
项目             →    项目团队
资源预算         →    Token/成本配额
```

**v3.0 方案实现了真正的智能决策和动态生成，让 AI 像企业一样高效协作！**

---

**审批状态**: 待审批  
**负责人**: 待定  
**最后更新**: 2026 年 2 月 28 日
