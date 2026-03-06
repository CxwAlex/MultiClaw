// src/tools/create_company.rs
//! 创建公司实例的工具
//! 允许董事长 Agent 在对话中创建新的公司实例
//!
//! 创建的公司实例是独立的 MultiClaw 进程：
//! - 有自己的端口
//! - 有独立的数据目录
//! - 由 CEO Agent 管理
//! - 向董事长汇报

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use tokio::sync::RwLock;

use super::traits::{Tool, ToolResult};
use crate::config::Config;

/// 下一个可用端口（从 8001 开始）
static NEXT_PORT: std::sync::atomic::AtomicU16 = std::sync::atomic::AtomicU16::new(8001);

/// 公司类型预设配置
#[derive(Clone, Debug)]
pub struct CompanyTypePreset {
    pub type_name: String,
    pub display_name: String,
    pub description: String,
    pub default_token_quota: u32,
    pub default_max_agents: u32,
    pub default_ceo_model: String,
    pub default_ceo_personality: String,
}

/// 获取公司类型预设
pub fn get_company_presets() -> HashMap<String, CompanyTypePreset> {
    let mut presets = HashMap::new();

    presets.insert("market_research".to_string(), CompanyTypePreset {
        type_name: "market_research".to_string(),
        display_name: "市场研究".to_string(),
        description: "市场调研、竞品分析、行业报告".to_string(),
        default_token_quota: 500_000,
        default_max_agents: 30,
        default_ceo_model: "qwen-max".to_string(),
        default_ceo_personality: "analytical".to_string(),
    });

    presets.insert("product_development".to_string(), CompanyTypePreset {
        type_name: "product_development".to_string(),
        display_name: "产品开发".to_string(),
        description: "产品设计、研发管理、迭代规划".to_string(),
        default_token_quota: 800_000,
        default_max_agents: 50,
        default_ceo_model: "qwen-max".to_string(),
        default_ceo_personality: "creative".to_string(),
    });

    presets.insert("customer_service".to_string(), CompanyTypePreset {
        type_name: "customer_service".to_string(),
        display_name: "客户服务".to_string(),
        description: "客户支持、工单处理、FAQ 管理".to_string(),
        default_token_quota: 600_000,
        default_max_agents: 40,
        default_ceo_model: "qwen-plus".to_string(),
        default_ceo_personality: "practical".to_string(),
    });

    presets.insert("data_analysis".to_string(), CompanyTypePreset {
        type_name: "data_analysis".to_string(),
        display_name: "数据分析".to_string(),
        description: "数据挖掘、报表生成、趋势分析".to_string(),
        default_token_quota: 400_000,
        default_max_agents: 20,
        default_ceo_model: "qwen-max".to_string(),
        default_ceo_personality: "analytical".to_string(),
    });

    presets.insert("personal_assistant".to_string(), CompanyTypePreset {
        type_name: "personal_assistant".to_string(),
        display_name: "个人助理".to_string(),
        description: "日程管理、任务追踪、生活助手".to_string(),
        default_token_quota: 200_000,
        default_max_agents: 10,
        default_ceo_model: "qwen-plus".to_string(),
        default_ceo_personality: "balanced".to_string(),
    });

    presets.insert("general".to_string(), CompanyTypePreset {
        type_name: "general".to_string(),
        display_name: "通用型".to_string(),
        description: "通用任务处理、灵活配置".to_string(),
        default_token_quota: 100_000,
        default_max_agents: 10,
        default_ceo_model: "qwen-plus".to_string(),
        default_ceo_personality: "balanced".to_string(),
    });

    presets.insert("custom".to_string(), CompanyTypePreset {
        type_name: "custom".to_string(),
        display_name: "自定义".to_string(),
        description: "完全自定义配置".to_string(),
        default_token_quota: 100_000,
        default_max_agents: 10,
        default_ceo_model: "qwen-plus".to_string(),
        default_ceo_personality: "balanced".to_string(),
    });

    presets
}

/// 创建公司工具
pub struct CreateCompanyTool {
    workspace_dir: PathBuf,
    /// 主配置引用（用于继承关键设置）
    parent_config: Arc<Config>,
    /// 运行中的实例进程
    processes: Arc<RwLock<HashMap<String, tokio::process::Child>>>,
}

impl CreateCompanyTool {
    pub fn new(workspace_dir: PathBuf) -> Self {
        Self {
            workspace_dir,
            parent_config: Arc::new(Config::default()),
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 使用主配置创建工具
    pub fn with_config(workspace_dir: PathBuf, config: Arc<Config>) -> Self {
        Self {
            workspace_dir,
            parent_config: config,
            processes: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 分配下一个可用端口
    fn assign_port(&self) -> u16 {
        NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// 启动实例进程
    async fn start_instance_process(
        &self,
        instance_id: &str,
        port: u16,
        config_path: &std::path::Path,
        data_dir: &std::path::Path,
    ) -> Result<u32, String> {
        use tokio::process::Command;

        let exe = std::env::current_exe()
            .map_err(|e| format!("获取当前可执行文件路径失败: {}", e))?;

        let mut cmd = Command::new(exe);
        cmd.arg("daemon")
           .arg("--port")
           .arg(port.to_string())
           .env("MULTICLAW_INSTANCE_ID", instance_id)
           .env("MULTICLAW_DATA_DIR", data_dir.to_string_lossy().to_string())
           .env("MULTICLAW_CONFIG", config_path.to_string_lossy().to_string())
           .kill_on_drop(true);

        let child = cmd.spawn()
            .map_err(|e| format!("启动实例进程失败: {}", e))?;

        let pid = child.id().unwrap_or(0);

        // 保存进程引用
        let mut processes = self.processes.write().await;
        processes.insert(instance_id.to_string(), child);

        Ok(pid)
    }

    /// 获取公司类型列表（用于工具描述）
    fn get_company_types_description() -> String {
        let presets = get_company_presets();
        let mut desc = String::from("可选的公司类型：\n");
        for (key, preset) in presets.iter() {
            desc.push_str(&format!(
                "- `{}`: {} - {} (默认 {} token/分钟, {} agents)\n",
                key, preset.display_name, preset.description,
                preset.default_token_quota, preset.default_max_agents
            ));
        }
        desc
    }
}

#[async_trait]
impl Tool for CreateCompanyTool {
    fn name(&self) -> &str {
        "create_company"
    }

    fn description(&self) -> &str {
        "创建新的公司实例（独立的 MultiClaw 实例）。公司由 CEO Agent 管理，向董事长汇报。支持自定义公司名称、类型、资源配额和 CEO 配置。"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "公司名称"
                },
                "company_type": {
                    "type": "string",
                    "enum": ["market_research", "product_development", "customer_service", "data_analysis", "general", "personal_assistant", "custom"],
                    "description": "公司类型：market_research(市场研究), product_development(产品开发), customer_service(客户服务), data_analysis(数据分析), general(通用), personal_assistant(个人助理), custom(自定义)"
                },
                "token_quota": {
                    "type": "integer",
                    "minimum": 10000,
                    "maximum": 100000000,
                    "default": 1000000,
                    "description": "Token 配额（每分钟）。100M token 约等于每天100元。默认 1M/分钟"
                },
                "max_agents": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "default": 10,
                    "description": "最大 Agent 数量"
                },
                "ceo_model": {
                    "type": "string",
                    "default": "qwen-max",
                    "description": "CEO 使用的模型"
                },
                "ceo_personality": {
                    "type": "string",
                    "enum": ["analytical", "creative", "strategic", "practical", "balanced"],
                    "default": "balanced",
                    "description": "CEO 性格特征：analytical(分析型), creative(创意型), strategic(战略型), practical(务实型), balanced(平衡型)"
                },
                "channel": {
                    "type": "string",
                    "description": "绑定的通信渠道（可选），如 telegram、discord 等"
                }
            },
            "required": ["name", "company_type"]
        })
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        // 获取公司预设
        let presets = get_company_presets();

        // 解析参数
        let name = args.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("缺少公司名称"))?
            .to_string();

        let company_type_str = args.get("company_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general");

        // 获取预设配置
        let preset = presets.get(company_type_str).cloned().unwrap_or_else(|| {
            presets.get("general").cloned().unwrap()
        });

        // 使用用户提供的值或预设默认值
        let token_quota = args.get("token_quota")
            .and_then(|v| v.as_u64())
            .unwrap_or(preset.default_token_quota as u64) as u32;

        let max_agents = args.get("max_agents")
            .and_then(|v| v.as_u64())
            .unwrap_or(preset.default_max_agents as u64) as u32;

        let ceo_model = args.get("ceo_model")
            .and_then(|v| v.as_str())
            .unwrap_or(&preset.default_ceo_model)
            .to_string();

        let ceo_personality = args.get("ceo_personality")
            .and_then(|v| v.as_str())
            .unwrap_or(&preset.default_ceo_personality)
            .to_string();

        let channel = args.get("channel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 生成公司 ID
        let company_id = format!("company_{}", uuid::Uuid::new_v4().simple());

        // 分配端口
        let port = self.assign_port();

        // 创建实例目录
        let instances_dir = self.workspace_dir.join("instances");
        let instance_dir = instances_dir.join(&company_id);

        if let Err(e) = tokio::fs::create_dir_all(&instance_dir).await {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("创建实例目录失败: {}", e)),
            });
        }

        // 生成 CEO Agent 文件
        let company_type_display = &preset.display_name;

        // 生成 IDENTITY.md
        let identity_content = format!(
r#"# {} - CEO Agent 身份

## 基本信息
- **姓名**: {} CEO
- **角色**: 首席执行官 (CEO)
- **公司**: {}
- **公司类型**: {}
- **汇报对象**: 董事长 Agent

## 职责
- 管理公司日常运营
- 协调团队资源
- 向董事长汇报重要决策
- 执行董事长下达的任务

## 权限
- Token 配额: {}/分钟
- 最大 Agent 数: {}
- 可创建和管理团队
- 可分配任务给团队成员
"#,
            name, name, name, company_type_display, token_quota, max_agents
        );

        // 生成 SOUL.md
        let personality_desc = match ceo_personality.as_str() {
            "analytical" => "分析型：善于数据分析，决策基于数据和逻辑",
            "creative" => "创意型：善于创新思维，喜欢尝试新方法",
            "strategic" => "战略型：善于长远规划，关注大局",
            "practical" => "务实型：注重执行效率，追求实际效果",
            _ => "平衡型：兼顾分析和直觉，灵活应对各种情况",
        };

        let soul_content = format!(
r#"# {} CEO - 角色设定

## 性格特征
{}

## 工作风格
- 使用 {} 模型进行决策
- 定期向董事长汇报进展
- 主动识别和解决问题
- 关注团队协作和效率

## 沟通风格
- 专业且友好
- 简洁明了
- 结果导向
- 及时响应

## 决策原则
1. 优先考虑公司目标
2. 平衡资源使用
3. 及时向上级汇报重大事项
4. 对团队负责
"#,
            name, personality_desc, ceo_model
        );

        // 生成 AGENTS.md
        let agents_content = format!(
r#"# {} CEO - 操作指南

## 日常任务
1. 检查团队状态
2. 分配任务给团队成员
3. 监控资源使用
4. 向董事长汇报进展

## 可用技能
- 创建团队: create_team
- 分配任务: assign_task
- 检查状态: check_status
- 汇报进展: report_progress

## 汇报机制
- 每日摘要报告
- 重要事件即时汇报
- 周度总结报告
- 月度绩效报告

## 紧急情况处理
1. 评估情况严重程度
2. 采取初步措施
3. 立即通知董事长
4. 执行后续跟进
"#,
            name
        );

        // 生成 USER.md
        let user_content = format!(
r#"# 汇报对象

此 CEO Agent 向 **董事长 Agent** 汇报。

董事长是用户的 AI 分身，管理所有 MultiClaw 实例。

## 汇报渠道
- 主要: 通过系统内部消息
- 紧急: 直接通知（如配置了通信渠道）
"#,
        );

        // 生成 MEMORY.md
        let memory_content = format!(
r#"# {} CEO - 记忆存储

此文件用于 CEO Agent 的长期记忆存储。

## 记忆类别
- core: 核心记忆（公司信息、团队配置）
- working: 工作记忆（当前任务、进展）
- episodic: 情景记忆（重要事件、对话）
- semantic: 语义记忆（知识、经验）
"#,
            name
        );

        // 生成完整的 config.toml（继承主配置）
        let parent_api_key = self.parent_config.api_key.as_deref().unwrap_or("");
        let parent_provider = self.parent_config.default_provider.as_deref().unwrap_or("openrouter");
        let parent_api_url = self.parent_config.api_url.as_deref().unwrap_or("");

        let config_content = format!(
r#"# {} 实例配置
# 由董事长 Agent 自动生成
# 继承主配置的关键设置

# ─────────────────────────────────────────────────────────────
# 核心配置
# ─────────────────────────────────────────────────────────────

## API 配置（继承自主配置）
api_key = "{}"
default_provider = "{}"
default_model = "{}"
{}
default_temperature = 0.7

# ─────────────────────────────────────────────────────────────
# 实例元数据
# ─────────────────────────────────────────────────────────────

[instance]
id = "{}"
name = "{}"
type = "{}"
created_by = "chairman"

# ─────────────────────────────────────────────────────────────
# 资源配额
# ─────────────────────────────────────────────────────────────

[resource]
token_quota_per_minute = {}
max_agents = {}
storage_limit_mb = 1000

# ─────────────────────────────────────────────────────────────
# CEO 配置
# ─────────────────────────────────────────────────────────────

[ceo]
model = "{}"
personality = "{}"

# ─────────────────────────────────────────────────────────────
# 运行时配置
# ─────────────────────────────────────────────────────────────

[runtime]
kind = "native"
sandbox_profile = "strict"

# ─────────────────────────────────────────────────────────────
# 记忆配置
# ─────────────────────────────────────────────────────────────

[memory]
backend = "sqlite"
enable_embeddings = false
max_entries = 10000

# ─────────────────────────────────────────────────────────────
# 安全配置
# ─────────────────────────────────────────────────────────────

[autonomy]
level = "supervised"
require_approval_for = ["shell", "file_write", "file_edit"]
block_high_risk_commands = true

[security]
sandbox_enabled = true
allowed_paths = []

# ─────────────────────────────────────────────────────────────
# 可观测性
# ─────────────────────────────────────────────────────────────

[observability]
backend = "log"

# ─────────────────────────────────────────────────────────────
# 心跳配置
# ─────────────────────────────────────────────────────────────

[heartbeat]
enabled = true
interval_secs = 60

# ─────────────────────────────────────────────────────────────
# 网关配置
# ─────────────────────────────────────────────────────────────

[gateway]
host = "127.0.0.1"
port = 0  # 自动分配
pairing_required = true
"#,
            name,
            parent_api_key,
            parent_provider,
            ceo_model,
            if parent_api_url.is_empty() {
                String::new()
            } else {
                format!("api_url = \"{}\"\n", parent_api_url)
            },
            company_id,
            name,
            company_type_str,
            token_quota,
            max_agents,
            ceo_model,
            ceo_personality
        );

        // 写入文件
        let files = vec![
            ("IDENTITY.md", identity_content),
            ("SOUL.md", soul_content),
            ("AGENTS.md", agents_content),
            ("USER.md", user_content),
            ("MEMORY.md", memory_content),
            ("config.toml", config_content),
        ];

        for (filename, content) in files {
            let path = instance_dir.join(filename);
            if let Err(e) = tokio::fs::write(&path, content).await {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("写入文件 {} 失败: {}", filename, e)),
                });
            }
        }

        // 创建 teams 目录
        let teams_dir = instance_dir.join("teams");
        if let Err(e) = tokio::fs::create_dir_all(&teams_dir).await {
            return Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("创建 teams 目录失败: {}", e)),
            });
        }

        // ── 启动实例进程 ─────────────────────────────────────
        let config_path = instance_dir.join("config.toml");

        // 启动独立进程
        let pid = match self.start_instance_process(
            &company_id,
            port,
            &config_path,
            &instance_dir,
        ).await {
            Ok(pid) => pid,
            Err(e) => {
                return Ok(ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("启动实例进程失败: {}", e)),
                });
            }
        };

        // 生成成功结果
        let result_json = serde_json::json!({
            "instance_id": company_id,
            "instance_name": name,
            "instance_type": company_type_str,
            "instance_type_display": company_type_display,
            "port": port,
            "pid": pid,
            "data_dir": instance_dir.display().to_string(),
            "config_file": config_path.display().to_string(),
            "token_quota": token_quota,
            "max_agents": max_agents,
            "ceo_model": ceo_model,
            "ceo_personality": ceo_personality,
            "channel": channel,
            "message": format!(
                "公司 '{}' 创建成功！\n\n实例信息：\n- 实例ID: {}\n- 端口: {}\n- 进程PID: {}\n- 数据目录: {}\n\nCEO Agent 已初始化，正在启动中...",
                name, company_id, port, pid, instance_dir.display()
            )
        });

        Ok(ToolResult {
            success: true,
            output: serde_json::to_string_pretty(&result_json).unwrap_or_default(),
            error: None,
        })
    }
}