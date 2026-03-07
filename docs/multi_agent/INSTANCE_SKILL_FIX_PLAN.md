# MultiClaw 多实例架构问题分析与完整修复方案

> 文档创建时间：2026-03-07
> 状态：待实施

## 一、问题汇总

### 问题 1：实例创建问题

| 问题点 | 严重程度 | 说明 |
|--------|----------|------|
| 三套实现并存 | 高 | `CreateCompanySkill`、`CreateCompanyTool`、`CompanyManager` 逻辑不一致 |
| 进程未持久化 | 高 | CEO 实例只是普通子进程，系统重启后不会恢复 |
| 无健康监控 | 中 | 没有进程健康检查和自动重启机制 |
| CEO Agent 未真正启动 | 高 | `create_ceo_agent()` 只创建对象，没有启动运行循环 |

**代码证据**：
```rust
// src/tools/create_company.rs:204-226
// 注意：不使用 kill_on_drop(true)，让进程独立运行
// 进程会持续运行直到用户手动停止或系统重启
```

### 问题 2：实例管理问题

| 问题点 | 严重程度 | 说明 |
|--------|----------|------|
| 两套端口分配机制 | 高 | `InstanceManager` 用内存计数器，`CreateCompanyTool` 用静态变量 |
| 端口默认值不一致 | 高 | Gateway 默认 42617，创建公司分配 8001+ |
| 无全局实例注册表 | 高 | 每个 daemon 进程独立，不知道其他实例 |
| 配置隔离不完整 | 中 | `--config-dir` 指定不同目录仍用同一端口 |

**代码证据**：
```rust
// src/instance/manager.rs - 内存计数器
next_port: Arc<RwLock<u16>>  // 从 8001 开始

// src/tools/create_company.rs - 静态变量
static NEXT_PORT: AtomicU16 = AtomicU16::new(8001);

// src/config/schema.rs - 默认端口
fn default_gateway_port() -> u16 { 42617 }
```

### 问题 3：技能系统问题

| 问题点 | 严重程度 | 说明 |
|--------|----------|------|
| 三套 Skill 定义 | 高 | `skill_types.rs`、`compat.rs`、`orchestration.rs` 各一套 |
| Skill/Tool 概念混淆 | 高 | `create_company` 同时作为 Skill 和 Tool 存在 |
| 技能加载返回空 | 高 | `load_skills_with_config()` 返回空列表 |
| 配置的技能未实现 | 中 | `resource_allocation`、`instance_monitoring` 等未实现 |
| 技能未注入 Agent | 高 | `SkillsOrchestration` 没有注册到 Agent 执行循环 |

**代码证据**：
```rust
// src/skills/compat.rs
pub fn load_skills_with_config(...) -> Vec<Skill> {
    // 返回空列表作为占位符
    vec![]
}

// src/agent/chairman_config.rs - 配置了 5 个技能
enabled_skills: vec![
    "create_company",           // ✅ 已实现
    "company_creation_guide",   // ✅ 已实现
    "resource_allocation",      // ❌ 未实现
    "instance_monitoring",      // ❌ 未实现
    "cross_instance_communication", // ❌ 未实现
]
```

### 问题 4：通用技能实现问题

| 问题点 | 严重程度 | 说明 |
|--------|----------|------|
| 记忆压缩核心未实现 | 高 | `extract_entities`、`summarize_tool_calls` 返回空 |
| A2A 传播被注释 | 高 | `propagate_memory` 中消息发送被注释掉 |
| WASM 运行时未完成 | 中 | `WasmSkillRuntime::execute` 返回模拟结果 |
| 董事长-CEO 通信缺失 | 高 | 没有实际的跨实例通信机制 |

**代码证据**：
```rust
// src/memory/compressor.rs
async fn extract_entities(&self, _text: &str) -> Vec<Entity> {
    vec![]  // 占位符实现
}

// src/core/memory_core.rs - A2A 传播被注释
// let _ = self.a2a_gateway.send_message(msg).await;
```

---

## 二、完整修复方案

### 方案架构图

```
┌─────────────────────────────────────────────────────────────────────┐
│                        用户层                                        │
│                    multiclaw CLI / Web UI                            │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    董事长实例 (主实例)                                 │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │                  ChairmanAgent                                │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐         │   │
│  │  │ 内置技能    │  │ 用户技能    │  │ A2A 通信    │         │   │
│  │  │ - 创建公司  │  │ - 自定义    │  │ - CEO 通信  │         │   │
│  │  │ - 监控公司  │  │ - 扩展      │  │ - 状态同步  │         │   │
│  │  │ - 资源分配  │  │             │  │             │         │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘         │   │
│  └─────────────────────────────────────────────────────────────┘   │
│                                │                                    │
│                    InstanceRegistry (全局)                          │
│                    ~/.multiclaw/instances.json                      │
└─────────────────────────────────────────────────────────────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              ▼                 ▼                 ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│   公司实例 A         │ │   公司实例 B         │ │   公司实例 N         │
│  ┌───────────────┐  │ │  ┌───────────────┐  │ │  ┌───────────────┐  │
│  │  CEO Agent    │  │ │  │  CEO Agent    │  │ │  │  CEO Agent    │  │
│  │  ┌─────────┐  │  │ │  │  ┌─────────┐  │  │ │  │  ┌─────────┐  │  │
│  │  │内置技能 │  │  │ │  │  │内置技能 │  │  │ │  │  │内置技能 │  │  │
│  │  │- 创建团队│  │  │ │  │  │- 创建团队│  │  │ │  │  │- 创建团队│  │  │
│  │  │- 任务分配│  │  │ │  │  │- 任务分配│  │  │ │  │  │- 任务分配│  │  │
│  │  └─────────┘  │  │ │  └─────────┘  │  │ │  └─────────┘  │  │
│  └───────────────┘  │ │  └───────────────┘  │ │  └───────────────┘  │
│  Port: 8001         │ │  Port: 8002         │ │  Port: 8003         │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘
```

---

### 修复模块 1：统一实例管理

#### 1.1 创建全局实例注册表

**新文件**: `src/instance/registry.rs`

```rust
use std::collections::HashMap;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

/// 实例注册表 - 全局单例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRegistry {
    /// 董事长实例信息
    pub chairman: Option<ChairmanInfo>,
    /// 公司实例列表
    pub companies: HashMap<String, CompanyInstanceInfo>,
    /// 端口分配表
    pub port_allocations: HashMap<u16, String>,  // port -> instance_id
    /// 下一个可用端口
    pub next_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanInfo {
    pub instance_id: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub pid: Option<u32>,
    pub status: InstanceStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInstanceInfo {
    pub instance_id: String,
    pub company_name: String,
    pub company_type: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub pid: Option<u32>,
    pub status: InstanceStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub ceo_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InstanceStatus {
    Running,
    Stopped,
    Crashed,
    Unknown,
}

impl InstanceRegistry {
    const DEFAULT_PATH: &'static str = ".multiclaw/instances.json";
    const BASE_PORT: u16 = 8001;
    
    /// 加载或创建注册表
    pub async fn load() -> Self {
        let path = Self::registry_path();
        if path.exists() {
            if let Ok(content) = tokio::fs::read_to_string(&path).await {
                if let Ok(registry) = serde_json::from_str(&content) {
                    return registry;
                }
            }
        }
        Self::default()
    }
    
    /// 保存注册表
    pub async fn save(&self) -> anyhow::Result<()> {
        let path = Self::registry_path();
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;
        Ok(())
    }
    
    /// 分配端口
    pub fn allocate_port(&mut self) -> u16 {
        while self.port_allocations.contains_key(&self.next_port) {
            self.next_port += 1;
        }
        let port = self.next_port;
        self.next_port += 1;
        port
    }
    
    /// 注册公司实例
    pub fn register_company(&mut self, info: CompanyInstanceInfo) {
        let port = info.port;
        let id = info.instance_id.clone();
        self.port_allocations.insert(port, id.clone());
        self.companies.insert(id, info);
    }
    
    /// 更新实例状态
    pub fn update_status(&mut self, instance_id: &str, status: InstanceStatus) {
        if let Some(info) = self.companies.get_mut(instance_id) {
            info.status = status;
        }
    }
    
    fn registry_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(Self::DEFAULT_PATH)
    }
}

impl Default for InstanceRegistry {
    fn default() -> Self {
        Self {
            chairman: None,
            companies: HashMap::new(),
            port_allocations: HashMap::new(),
            next_port: Self::BASE_PORT,
        }
    }
}

/// 全局注册表管理器
pub struct RegistryManager {
    registry: RwLock<InstanceRegistry>,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(InstanceRegistry::default()),
        }
    }
    
    pub async fn load(&self) {
        let mut reg = self.registry.write().await;
        *reg = InstanceRegistry::load().await;
    }
    
    pub async fn allocate_port(&self) -> u16 {
        let mut reg = self.registry.write().await;
        let port = reg.allocate_port();
        let _ = reg.save().await;
        port
    }
    
    pub async fn register_company(&self, info: CompanyInstanceInfo) {
        let mut reg = self.registry.write().await;
        reg.register_company(info);
        let _ = reg.save().await;
    }
    
    pub async fn get_company(&self, instance_id: &str) -> Option<CompanyInstanceInfo> {
        let reg = self.registry.read().await;
        reg.companies.get(instance_id).cloned()
    }
    
    pub async fn list_companies(&self) -> Vec<CompanyInstanceInfo> {
        let reg = self.registry.read().await;
        reg.companies.values().cloned().collect()
    }
}
```

#### 1.2 修改 daemon 启动逻辑

**修改文件**: `src/daemon/mod.rs`

```rust
use crate::instance::{RegistryManager, InstanceRegistry, CompanyInstanceInfo, InstanceStatus};

pub async fn run(config: Config, host: String, port: u16) -> Result<()> {
    // 加载全局注册表
    let registry_manager = Arc::new(RegistryManager::new());
    registry_manager.load().await;
    
    // 检查是否是董事长实例
    let is_chairman = config.workspace_dir.join("chairman_config.toml").exists();
    
    if is_chairman {
        // 董事长实例：使用固定端口或配置端口
        let chairman_port = port; // 使用传入的端口或配置端口
        tracing::info!("Starting Chairman instance on port {}", chairman_port);
        
        // 更新注册表中的董事长信息
        // ...
    } else {
        // 公司实例：从注册表获取分配的端口
        if let Some(instance_id) = std::env::var("MULTICLAW_INSTANCE_ID").ok() {
            if let Some(info) = registry_manager.get_company(&instance_id).await {
                let allocated_port = info.port;
                tracing::info!("Starting Company instance {} on port {}", 
                    info.company_name, allocated_port);
                // 使用分配的端口
                // ...
            }
        }
    }
    
    // ... 其余启动逻辑
}
```

---

### 修复模块 2：统一技能系统

#### 2.1 统一 Skill 定义

**修改文件**: `src/skills/mod.rs`

```rust
// 统一导出，移除重复定义
mod skill_types;
mod orchestration;
mod registry;
mod builtin;

// 重导出统一的类型
pub use skill_types::{Skill, SkillMetadata, SkillExecutor, SkillContext, SkillResult};
pub use orchestration::SkillsOrchestration;
pub use registry::SkillRegistry;
pub use builtin::{
    // 董事长技能
    CreateCompanySkill, CompanyCreationGuideSkill,
    InstanceMonitoringSkill, CrossInstanceCommunicationSkill,
    // CEO 技能
    CreateTeamSkill, TaskAssignmentSkill,
    // 通用技能
    MemoryCompressionSkill, MemorySharingSkill,
};
```

#### 2.2 创建技能注册中心

**新文件**: `src/skills/registry.rs`

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use super::{Skill, SkillExecutor, SkillMetadata};

/// 技能注册中心
pub struct SkillRegistry {
    /// 已注册的技能
    skills: RwLock<HashMap<String, Arc<dyn SkillExecutor>>>,
    /// 技能元数据
    metadata: RwLock<HashMap<String, SkillMetadata>>,
    /// 身份-技能映射
    identity_skills: HashMap<String, Vec<String>>,  // identity -> skill_ids
}

impl SkillRegistry {
    pub fn new() -> Self {
        let mut identity_skills = HashMap::new();
        
        // 董事长内置技能
        identity_skills.insert("chairman".to_string(), vec![
            "create_company".to_string(),
            "company_creation_guide".to_string(),
            "instance_monitoring".to_string(),
            "cross_instance_communication".to_string(),
            "resource_allocation".to_string(),
        ]);
        
        // CEO 内置技能
        identity_skills.insert("ceo".to_string(), vec![
            "create_team".to_string(),
            "task_assignment".to_string(),
            "team_monitoring".to_string(),
            "report_to_chairman".to_string(),
        ]);
        
        // 团队负责人技能
        identity_skills.insert("team_lead".to_string(), vec![
            "create_worker".to_string(),
            "task_distribution".to_string(),
            "knowledge_sharing".to_string(),
        ]);
        
        Self {
            skills: RwLock::new(HashMap::new()),
            metadata: RwLock::new(HashMap::new()),
            identity_skills,
        }
    }
    
    /// 注册技能
    pub async fn register(&self, skill: Arc<dyn SkillExecutor>) {
        let meta = skill.metadata();
        let name = meta.name.clone();
        
        self.skills.write().await.insert(name.clone(), skill);
        self.metadata.write().await.insert(name, meta);
    }
    
    /// 获取身份对应的技能
    pub async fn get_skills_for_identity(&self, identity: &str) -> Vec<Arc<dyn SkillExecutor>> {
        let skill_names = self.identity_skills.get(identity)
            .cloned()
            .unwrap_or_default();
        
        let skills = self.skills.read().await;
        skill_names
            .into_iter()
            .filter_map(|name| skills.get(&name).cloned())
            .collect()
    }
    
    /// 获取技能描述（用于提示）
    pub async fn get_skill_descriptions(&self, identity: &str) -> String {
        let skills = self.get_skills_for_identity(identity).await;
        let mut result = String::from("<available_skills>\n");
        
        for skill in &skills {
            let meta = skill.metadata();
            result.push_str(&format!(
                "  <skill>\n    <name>{}</name>\n    <description>{}</description>\n",
                meta.name, meta.description
            ));
            
            // 添加参数说明
            if !meta.input_schema.is_null() {
                result.push_str("    <parameters>\n");
                result.push_str(&serde_json::to_string_pretty(&meta.input_schema).unwrap_or_default());
                result.push_str("\n    </parameters>\n");
            }
            
            result.push_str("  </skill>\n");
        }
        
        result.push_str("</available_skills>");
        result
    }
}
```

#### 2.3 实现董事长内置技能

**新文件**: `src/skills/builtin/instance_monitoring.rs`

```rust
use async_trait::async_trait;
use crate::skills::{SkillExecutor, SkillMetadata, SkillContext, SkillResult};
use crate::instance::RegistryManager;

/// 实例监控技能
pub struct InstanceMonitoringSkill {
    registry: Arc<RegistryManager>,
}

impl InstanceMonitoringSkill {
    pub fn new(registry: Arc<RegistryManager>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl SkillExecutor for InstanceMonitoringSkill {
    async fn execute(&self, ctx: SkillContext) -> Result<SkillResult, Box<dyn std::error::Error>> {
        let action = ctx.params.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");
        
        match action {
            "list" => {
                let companies = self.registry.list_companies().await;
                let mut result = String::from("当前运行的公司实例：\n\n");
                
                for company in companies {
                    result.push_str(&format!(
                        "- **{}** (ID: {})\n  端口: {}\n  状态: {:?}\n  创建时间: {}\n",
                        company.company_name,
                        company.instance_id,
                        company.port,
                        company.status,
                        company.created_at.format("%Y-%m-%d %H:%M:%S")
                    ));
                }
                
                Ok(SkillResult {
                    success: true,
                    output: result,
                    metadata: Default::default(),
                })
            }
            "status" => {
                let instance_id = ctx.params.get("instance_id")
                    .and_then(|v| v.as_str())
                    .ok_or("缺少 instance_id 参数")?;
                
                if let Some(info) = self.registry.get_company(instance_id).await {
                    Ok(SkillResult {
                        success: true,
                        output: format!(
                            "公司: {}\n状态: {:?}\n端口: {}\nCEO 模型: {}",
                            info.company_name, info.status, info.port, info.ceo_model
                        ),
                        metadata: Default::default(),
                    })
                } else {
                    Ok(SkillResult {
                        success: false,
                        output: format!("未找到实例: {}", instance_id),
                        metadata: Default::default(),
                    })
                }
            }
            _ => Ok(SkillResult {
                success: false,
                output: format!("未知操作: {}", action),
                metadata: Default::default(),
            })
        }
    }
    
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "instance_monitoring".to_string(),
            description: "监控和管理公司实例状态".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "status"],
                        "description": "操作类型"
                    },
                    "instance_id": {
                        "type": "string",
                        "description": "实例ID（status 操作需要）"
                    }
                }
            }),
            ..Default::default()
        }
    }
    
    fn name(&self) -> &str { "instance_monitoring" }
}
```

**新文件**: `src/skills/builtin/cross_instance_communication.rs`

```rust
use async_trait::async_trait;
use crate::skills::{SkillExecutor, SkillMetadata, SkillContext, SkillResult};
use crate::instance::RegistryManager;
use crate::a2a::{A2AGateway, A2AMessage, A2AMessageType};

/// 跨实例通信技能
pub struct CrossInstanceCommunicationSkill {
    registry: Arc<RegistryManager>,
    a2a_gateway: Arc<A2AGateway>,
}

impl CrossInstanceCommunicationSkill {
    pub fn new(registry: Arc<RegistryManager>, a2a_gateway: Arc<A2AGateway>) -> Self {
        Self { registry, a2a_gateway }
    }
}

#[async_trait]
impl SkillExecutor for CrossInstanceCommunicationSkill {
    async fn execute(&self, ctx: SkillContext) -> Result<SkillResult, Box<dyn std::error::Error>> {
        let target_instance = ctx.params.get("target_instance")
            .and_then(|v| v.as_str())
            .ok_or("缺少 target_instance 参数")?;
        
        let message = ctx.params.get("message")
            .and_then(|v| v.as_str())
            .ok_or("缺少 message 参数")?;
        
        // 获取目标实例信息
        let target_info = self.registry.get_company(target_instance).await
            .ok_or_else(|| format!("未找到目标实例: {}", target_instance))?;
        
        // 构建 A2A 消息
        let a2a_message = A2AMessage {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: "chairman".to_string(),
            target_id: target_instance.to_string(),
            message_type: A2AMessageType::Command {
                command: "user_request".to_string(),
                params: serde_json::json!({ "content": message }),
            },
            priority: crate::a2a::MessagePriority::Normal,
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        };
        
        // 发送消息
        self.a2a_gateway.send_message(a2a_message).await?;
        
        // 等待响应（可选）
        let response = ctx.params.get("wait_for_response")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        
        if response {
            // 等待 CEO 响应
            let reply = self.a2a_gateway.wait_for_reply(
                &target_instance,
                std::time::Duration::from_secs(30)
            ).await?;
            
            Ok(SkillResult {
                success: true,
                output: format!("CEO 回复：\n{}", reply),
                metadata: Default::default(),
            })
        } else {
            Ok(SkillResult {
                success: true,
                output: format!("消息已发送到 {} (端口: {})", 
                    target_info.company_name, target_info.port),
                metadata: Default::default(),
            })
        }
    }
    
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "cross_instance_communication".to_string(),
            description: "与 CEO 实例通信，传递用户需求或查询状态".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "target_instance": {
                        "type": "string",
                        "description": "目标实例ID或公司名称"
                    },
                    "message": {
                        "type": "string",
                        "description": "要发送的消息内容"
                    },
                    "wait_for_response": {
                        "type": "boolean",
                        "description": "是否等待响应",
                        "default": false
                    }
                },
                "required": ["target_instance", "message"]
            }),
            ..Default::default()
        }
    }
    
    fn name(&self) -> &str { "cross_instance_communication" }
}
```

---

### 修复模块 3：Agent 技能绑定

#### 3.1 修改 Agent 初始化流程

**修改文件**: `src/agent/agent.rs`

```rust
use crate::skills::{SkillRegistry, SkillExecutor};
use crate::instance::RegistryManager;

pub struct Agent {
    // 现有字段...
    identity: String,
    skill_registry: Arc<SkillRegistry>,
    skills: Vec<Arc<dyn SkillExecutor>>,
}

impl Agent {
    /// 从配置创建 Agent，自动绑定身份对应的技能
    pub async fn from_config_with_skills(
        config: &Config,
        identity: &str,
        registry: Arc<SkillRegistry>,
    ) -> Result<Self> {
        // 获取身份对应的技能
        let skills = registry.get_skills_for_identity(identity).await;
        
        // 构建技能提示
        let skills_prompt = registry.get_skill_descriptions(identity).await;
        
        // 创建 Agent
        let mut agent = Self::from_config(config)?;
        agent.identity = identity.to_string();
        agent.skill_registry = registry;
        agent.skills = skills;
        agent.skills_prompt = Some(skills_prompt);
        
        Ok(agent)
    }
    
    /// 执行技能
    pub async fn execute_skill(
        &self,
        skill_name: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<SkillResult> {
        // 查找技能
        let skill = self.skills.iter()
            .find(|s| s.name() == skill_name)
            .ok_or_else(|| anyhow::anyhow!("技能未找到: {}", skill_name))?;
        
        // 构建上下文
        let ctx = SkillContext {
            params,
            agent_context: self.context.clone(),
            memory: self.memory.clone(),
        };
        
        // 执行技能
        skill.execute(ctx).await
            .map_err(|e| anyhow::anyhow!("技能执行失败: {}", e))
    }
}
```

#### 3.2 修改 Chairman Agent 初始化

**修改文件**: `src/agent/chairman.rs`

```rust
use crate::skills::{
    SkillRegistry, CreateCompanySkill, CompanyCreationGuideSkill,
    InstanceMonitoringSkill, CrossInstanceCommunicationSkill,
};
use crate::instance::RegistryManager;

impl ChairmanAgent {
    pub async fn initialize_with_config(
        config: ChairmanConfig,
        user_id: String,
        host: String,
        registry_manager: Arc<RegistryManager>,
    ) -> Result<Self> {
        // 创建技能注册中心
        let skill_registry = Arc::new(SkillRegistry::new());
        
        // 注册董事长技能
        skill_registry.register(Arc::new(CreateCompanySkill::new(
            registry_manager.clone(),
        ))).await;
        
        skill_registry.register(Arc::new(CompanyCreationGuideSkill::new(
            registry_manager.clone(),
        ))).await;
        
        skill_registry.register(Arc::new(InstanceMonitoringSkill::new(
            registry_manager.clone(),
        ))).await;
        
        // A2A 网关
        let a2a_gateway = Arc::new(A2AGateway::new());
        
        skill_registry.register(Arc::new(CrossInstanceCommunicationSkill::new(
            registry_manager.clone(),
            a2a_gateway.clone(),
        ))).await;
        
        // 创建 Agent 并绑定技能
        let agent = Agent::from_config_with_skills(
            &config.base_config,
            "chairman",
            skill_registry.clone(),
        ).await?;
        
        Ok(Self {
            agent,
            config,
            user_id,
            registry_manager,
            skill_registry,
            a2a_gateway,
        })
    }
}
```

---

### 修复模块 4：完善记忆系统

#### 4.1 实现记忆压缩核心功能

**修改文件**: `src/memory/compressor.rs`

```rust
impl MemoryCompressor {
    /// 提取实体
    async fn extract_entities(&self, text: &str) -> Vec<Entity> {
        // 使用 LLM 提取实体
        let prompt = format!(
            "从以下文本中提取关键实体（人物、地点、组织、事件等）：\n\n{}\n\n请以 JSON 格式返回实体列表。",
            text
        );
        
        let response = self.provider
            .complete(&prompt, &[])
            .await
            .ok();
        
        if let Some(content) = response {
            // 解析 JSON 响应
            if let Ok(entities) = serde_json::from_str::<Vec<Entity>>(&content) {
                return entities;
            }
        }
        
        vec![]
    }
    
    /// 摘要工具调用
    async fn summarize_tool_calls(&self, tool_calls: &[ToolCall]) -> Vec<ToolCallSummary> {
        if tool_calls.is_empty() {
            return vec![];
        }
        
        let tool_descriptions: Vec<String> = tool_calls
            .iter()
            .map(|tc| format!("{}({})", tc.name, serde_json::to_string(&tc.args).unwrap_or_default()))
            .collect();
        
        let prompt = format!(
            "总结以下工具调用序列的目的和结果：\n{}\n\n请简要描述这些调用的整体目的。",
            tool_descriptions.join("\n")
        );
        
        let response = self.provider
            .complete(&prompt, &[])
            .await
            .ok();
        
        vec![ToolCallSummary {
            tools_used: tool_calls.iter().map(|tc| tc.name.clone()).collect(),
            purpose: response.unwrap_or_default(),
            success: true,
        }]
    }
    
    /// 提取决策点
    async fn extract_decisions(&self, turns: &[ConversationTurn]) -> Vec<Decision> {
        let decisions_text: Vec<String> = turns
            .iter()
            .filter_map(|t| {
                if t.content.contains("决定") || t.content.contains("选择") {
                    Some(t.content.clone())
                } else {
                    None
                }
            })
            .collect();
        
        if decisions_text.is_empty() {
            return vec![];
        }
        
        let prompt = format!(
            "从以下对话中提取关键决策点：\n{}\n\n请列出做出的决策及其理由。",
            decisions_text.join("\n")
        );
        
        let response = self.provider
            .complete(&prompt, &[])
            .await
            .ok();
        
        // 解析决策
        vec![Decision {
            description: response.unwrap_or_default(),
            rationale: String::new(),
            timestamp: chrono::Utc::now(),
        }]
    }
}
```

#### 4.2 启用 A2A 记忆传播

**修改文件**: `src/core/memory_core.rs`

```rust
impl MemoryCore {
    /// 传播记忆到其他实例
    pub async fn propagate_memory(&self, entry_id: &str) -> Result<()> {
        let entry = self.memory_store.get(entry_id)
            .ok_or_else(|| anyhow::anyhow!("记忆条目不存在"))?;
        
        // 根据共享策略确定传播目标
        let policy = self.default_sharing_policies.get(&entry.level)
            .ok_or_else(|| anyhow::anyhow!("无共享策略"))?;
        
        // 构建 A2A 消息
        let message = A2AMessage {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: entry.source_id.clone(),
            target_id: "*".to_string(),  // 广播
            message_type: A2AMessageType::KnowledgeShare {
                knowledge_type: format!("{:?}", entry.level),
                content: serde_json::to_string(&*entry)?,
                applicable_scenarios: vec!["memory_propagation".to_string()],
            },
            priority: MessagePriority::Low,
            timestamp: chrono::Utc::now(),
            metadata: Default::default(),
        };
        
        // 发送消息（取消注释）
        self.a2a_gateway.send_message(message).await?;
        
        tracing::info!("Memory propagated: {} -> {:?}", entry_id, entry.level);
        Ok(())
    }
}
```

---

### 修复模块 5：实例持久化

#### 5.1 添加服务注册功能

**新文件**: `src/instance/service.rs`

```rust
use std::path::PathBuf;

/// 实例服务管理
pub struct InstanceService {
    instance_id: String,
    service_name: String,
}

impl InstanceService {
    /// 注册实例为系统服务
    pub async fn register(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            self.register_systemd(exe_path, config_dir, port).await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            self.register_launchd(exe_path, config_dir, port).await?;
        }
        
        Ok(())
    }
    
    #[cfg(target_os = "linux")]
    async fn register_systemd(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        let service_content = format!(
            r#"[Unit]
Description=MultiClaw Instance - {}
After=network.target

[Service]
Type=simple
ExecStart={} daemon --config-dir {} --port {}
Restart=always
RestartSec=3
Environment=MULTICLAW_INSTANCE_ID={}

[Install]
WantedBy=default.target
"#,
            self.instance_id,
            exe_path.display(),
            config_dir.display(),
            port,
            self.instance_id
        );
        
        let service_path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取配置目录"))?
            .join("systemd")
            .join("user")
            .join(&self.service_name);
        
        tokio::fs::create_dir_all(service_path.parent().unwrap()).await?;
        tokio::fs::write(&service_path, service_content).await?;
        
        // 启用服务
        tokio::process::Command::new("systemctl")
            .args(&["--user", "enable", &self.service_name])
            .status()
            .await?;
        
        tracing::info!("Registered systemd service: {}", self.service_name);
        Ok(())
    }
    
    #[cfg(target_os = "macos")]
    async fn register_launchd(&self, exe_path: &PathBuf, config_dir: &PathBuf, port: u16) -> Result<()> {
        let plist_content = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
        <string>daemon</string>
        <string>--config-dir</string>
        <string>{}</string>
        <string>--port</string>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>EnvironmentVariables</key>
    <dict>
        <key>MULTICLAW_INSTANCE_ID</key>
        <string>{}</string>
    </dict>
</dict>
</plist>
"#,
            self.service_name,
            exe_path.display(),
            config_dir.display(),
            port,
            self.instance_id
        );
        
        let plist_path = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("无法获取主目录"))?
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{}.plist", self.service_name));
        
        tokio::fs::create_dir_all(plist_path.parent().unwrap()).await?;
        tokio::fs::write(&plist_path, plist_content).await?;
        
        // 加载服务
        tokio::process::Command::new("launchctl")
            .args(&["load", plist_path.to_str().unwrap()])
            .status()
            .await?;
        
        tracing::info!("Registered launchd service: {}", self.service_name);
        Ok(())
    }
    
    /// 取消注册服务
    pub async fn unregister(&self) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            tokio::process::Command::new("systemctl")
                .args(&["--user", "disable", &self.service_name])
                .status()
                .await?;
        }
        
        #[cfg(target_os = "macos")]
        {
            let plist_path = dirs::home_dir()
                .unwrap()
                .join("Library")
                .join("LaunchAgents")
                .join(format!("{}.plist", self.service_name));
            
            tokio::process::Command::new("launchctl")
                .args(&["unload", plist_path.to_str().unwrap()])
                .status()
                .await?;
        }
        
        Ok(())
    }
}
```

---

## 三、实施计划

### 阶段 1：基础设施修复（优先级最高）

| 任务 | 文件 | 预计工作量 |
|------|------|------------|
| 创建全局实例注册表 | `src/instance/registry.rs` | 2h |
| 统一端口分配机制 | `src/instance/manager.rs` | 1h |
| 修改 daemon 启动逻辑 | `src/daemon/mod.rs` | 2h |
| 添加服务注册功能 | `src/instance/service.rs` | 3h |

### 阶段 2：技能系统重构

| 任务 | 文件 | 预计工作量 |
|------|------|------------|
| 统一 Skill 定义 | `src/skills/mod.rs` | 1h |
| 创建技能注册中心 | `src/skills/registry.rs` | 2h |
| 实现董事长技能 | `src/skills/builtin/*.rs` | 4h |
| 实现 CEO 技能 | `src/skills/builtin/*.rs` | 3h |

### 阶段 3：Agent 绑定修复

| 任务 | 文件 | 预计工作量 |
|------|------|------------|
| 修改 Agent 初始化 | `src/agent/agent.rs` | 2h |
| 修改 Chairman Agent | `src/agent/chairman.rs` | 2h |
| 技能执行集成 | `src/agent/execution.rs` | 3h |

### 阶段 4：记忆系统完善

| 任务 | 文件 | 预计工作量 |
|------|------|------------|
| 实现记忆压缩核心 | `src/memory/compressor.rs` | 3h |
| 启用 A2A 传播 | `src/core/memory_core.rs` | 1h |
| 跨实例通信测试 | `tests/` | 2h |

---

## 四、验证清单

### 实例管理验证
- [ ] 创建公司实例后，`~/.multiclaw/instances.json` 包含正确信息
- [ ] 不同实例使用不同端口
- [ ] 系统重启后实例自动恢复
- [ ] `multiclaw status` 显示所有实例状态

### 技能系统验证
- [ ] 董事长 Agent 启动时自动注册 5 个内置技能
- [ ] 技能描述正确注入到系统提示
- [ ] `create_company` 技能正确执行
- [ ] `instance_monitoring` 技能返回正确状态
- [ ] `cross_instance_communication` 技能成功发送消息

### 跨实例通信验证
- [ ] 董事长可以向 CEO 发送消息
- [ ] CEO 可以接收并响应消息
- [ ] 记忆可以跨实例共享
- [ ] 监控看板显示正确的实例信息

---

## 五、关键发现总结

### 1. 实例管理混乱
- **三套端口分配机制**：`InstanceManager`（内存）、`CreateCompanyTool`（静态变量）、daemon（配置文件）各管各的
- **无全局注册表**：每个进程独立运行，不知道其他实例存在
- **进程未持久化**：CEO 实例只是普通子进程，重启后消失

### 2. 技能系统形同虚设
- **三套 Skill 定义**：`skill_types.rs`、`compat.rs`、`orchestration.rs` 各一套
- **技能加载返回空**：`load_skills_with_config()` 是占位符
- **配置的技能未实现**：`resource_allocation`、`instance_monitoring` 等只有配置没有实现
- **技能未注入 Agent**：`SkillsOrchestration` 没有注册到 Agent 执行循环

### 3. 跨实例通信缺失
- **A2A 传播被注释**：`propagate_memory` 中消息发送被注释掉
- **董事长-CEO 通信无实现**：没有实际的跨实例消息传递

### 4. 建议优先级
1. **最高优先级**：创建全局实例注册表 + 统一端口分配
2. **高优先级**：统一技能系统 + 实现董事长内置技能
3. **中优先级**：Agent 技能绑定 + 跨实例通信
4. **低优先级**：记忆压缩完善 + WASM 运行时