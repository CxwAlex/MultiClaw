// src/instance/manager.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub enum InstanceType {
    MarketResearch,
    ProductDevelopment,
    CustomerService,
    DataAnalysis,
    General,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub tokens_per_minute: u32,
    pub max_concurrent_agents: u32,
    pub storage_limit_mb: u32,
    pub api_calls_per_minute: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CEOConfig {
    pub model_preference: String,
    pub personality: String,
    pub resource_limits: ResourceQuota,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub channel_type: String,
    pub credentials: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    pub id: String,
    pub name: String,
    pub instance_type: InstanceType,
    pub port: u16,
    pub data_dir: String,
    pub config_file: String,
    pub resource_quota: ResourceQuota,
    pub ceo_config: CEOConfig,
    pub channel_config: Option<ChannelConfig>,
}

// 实例状态信息，可以序列化和克隆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub pid: u32,
    pub config: InstanceConfig,
    pub status: InstanceStatus,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
}

// 进程管理信息，不序列化
pub struct ProcessInfo {
    pub process: Child,
    pub state: InstanceState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum InstanceStatus {
    Initializing,
    Running,
    Stopping,
    Stopped,
    Unhealthy,
    Recovering,
}

pub struct InstanceManager {
    /// 实例进程管理 (包含进程对象)
    processes: Arc<RwLock<HashMap<String, ProcessInfo>>>,
    /// 实例状态快照 (用于序列化)
    states: Arc<RwLock<HashMap<String, InstanceState>>>,
    /// 下一个可用端口
    next_port: Arc<RwLock<u16>>,
    /// 实例配置模板
    config_templates: HashMap<InstanceType, InstanceConfig>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            states: Arc::new(RwLock::new(HashMap::new())),
            next_port: Arc::new(RwLock::new(8001)), // 从 8001 开始分配端口
            config_templates: Self::create_config_templates(),
        }
    }

    /// 创建实例配置模板
    fn create_config_templates() -> HashMap<InstanceType, InstanceConfig> {
        let mut templates = HashMap::new();
        
        templates.insert(InstanceType::MarketResearch, InstanceConfig {
            id: String::new(),
            name: String::new(),
            instance_type: InstanceType::MarketResearch,
            port: 0,
            data_dir: String::new(),
            config_file: String::new(),
            resource_quota: ResourceQuota {
                tokens_per_minute: 500_000,
                max_concurrent_agents: 30,
                storage_limit_mb: 1000,
                api_calls_per_minute: 1000,
            },
            ceo_config: CEOConfig {
                model_preference: "gpt-4".to_string(),
                personality: "analytical".to_string(),
                resource_limits: ResourceQuota {
                    tokens_per_minute: 500_000,
                    max_concurrent_agents: 30,
                    storage_limit_mb: 1000,
                    api_calls_per_minute: 1000,
                },
            },
            channel_config: None,
        });

        templates.insert(InstanceType::ProductDevelopment, InstanceConfig {
            id: String::new(),
            name: String::new(),
            instance_type: InstanceType::ProductDevelopment,
            port: 0,
            data_dir: String::new(),
            config_file: String::new(),
            resource_quota: ResourceQuota {
                tokens_per_minute: 800_000,
                max_concurrent_agents: 50,
                storage_limit_mb: 2000,
                api_calls_per_minute: 2000,
            },
            ceo_config: CEOConfig {
                model_preference: "claude-sonnet".to_string(),
                personality: "creative".to_string(),
                resource_limits: ResourceQuota {
                    tokens_per_minute: 800_000,
                    max_concurrent_agents: 50,
                    storage_limit_mb: 2000,
                    api_calls_per_minute: 2000,
                },
            },
            channel_config: None,
        });

        // 添加其他类型模板...
        templates
    }

    /// 创建新实例
    pub async fn create_instance(&self, request: CreateInstanceRequest) -> Result<String, Box<dyn std::error::Error>> {
        let instance_id = Uuid::new_v4().to_string();
        
        // 获取配置模板
        let template = self.config_templates.get(&request.instance_type)
            .ok_or("未知的实例类型")?
            .clone();
        
        // 生成实例配置
        let mut config = template;
        config.id = instance_id.clone();
        config.name = request.name.clone();
        config.resource_quota = request.quota.clone();
        config.ceo_config = request.ceo_config.clone();
        config.channel_config = request.ceo_channel.map(|ch| ChannelConfig {
            channel_type: ch.rsplit_once(':').map(|(_, t)| t.to_string()).unwrap_or("telegram".to_string()),
            credentials: ch,
        });
        
        // 分配端口和目录
        config.port = self.assign_port().await;
        config.data_dir = format!("{}/instances/{}", request.base_data_dir, instance_id);
        config.config_file = format!("{}/config.toml", config.data_dir);

        // 创建实例目录
        tokio::fs::create_dir_all(&config.data_dir).await?;
        
        // 生成配置文件
        self.generate_instance_config(&config).await?;
        
        // 生成 CEO Agent 文件
        self.generate_ceo_files(&config).await?;
        
        // 启动实例进程
        let process = self.start_instance_process(&config).await?;
        let pid = process.id().unwrap_or(0);

        // 创建实例状态
        let instance_state = InstanceState {
            pid,
            config: config.clone(),
            status: InstanceStatus::Initializing,
            created_at: Utc::now(),
            last_heartbeat: Utc::now(),
        };

        // 注册实例
        let process_info = ProcessInfo {
            process,
            state: instance_state.clone(),
        };

        let mut processes = self.processes.write().await;
        processes.insert(instance_id.clone(), process_info);
        
        let mut states = self.states.write().await;
        states.insert(instance_id.clone(), instance_state);

        // 启动监控任务
        self.start_monitoring(instance_id.clone()).await;
        
        Ok(instance_id)
    }

    /// 分配下一个可用端口
    async fn assign_port(&self) -> u16 {
        let mut next_port = self.next_port.write().await;
        let port = *next_port;
        *next_port += 1;
        port
    }

    /// 生成实例配置文件
    async fn generate_instance_config(&self, config: &InstanceConfig) -> Result<(), Box<dyn std::error::Error>> {
        use toml_edit::{DocumentMut, value};
        
        let mut doc = DocumentMut::new();
        
        // 基础配置
        doc["name"] = value(config.name.clone());
        doc["instance_id"] = value(config.id.clone());
        doc["port"] = value(config.port as i64);
        
        // 资源配额
        doc["resource"]["tokens_per_minute"] = value(config.resource_quota.tokens_per_minute as i64);
        doc["resource"]["max_concurrent_agents"] = value(config.resource_quota.max_concurrent_agents as i64);
        doc["resource"]["storage_limit_mb"] = value(config.resource_quota.storage_limit_mb as i64);
        doc["resource"]["api_calls_per_minute"] = value(config.resource_quota.api_calls_per_minute as i64);
        
        // CEO 配置
        doc["ceo"]["model_preference"] = value(config.ceo_config.model_preference.clone());
        doc["ceo"]["personality"] = value(config.ceo_config.personality.clone());
        
        // 通信渠道
        if let Some(ref channel) = config.channel_config {
            doc["channel"]["type"] = value(channel.channel_type.clone());
            doc["channel"]["credentials"] = value(channel.credentials.clone());
        }
        
        // 写入配置文件
        tokio::fs::write(&config.config_file, doc.to_string()).await?;
        Ok(())
    }

    /// 生成 CEO Agent 文件
    async fn generate_ceo_files(&self, config: &InstanceConfig) -> Result<(), Box<dyn std::error::Error>> {
        let data_dir = std::path::PathBuf::from(&config.data_dir);

        // 创建子目录
        let subdirs = ["sessions", "memory", "state", "cron", "skills", "teams"];
        for dir in &subdirs {
            tokio::fs::create_dir_all(data_dir.join(dir)).await?;
        }

        // 根据公司类型生成 CEO 性格描述
        let personality_desc = match config.ceo_config.personality.as_str() {
            "analytical" => "分析型、数据驱动、注重细节",
            "creative" => "创造型、思维活跃、勇于创新",
            "strategic" => "战略型、全局视角、长远规划",
            "practical" => "务实型、高效执行、结果导向",
            _ => "专业、高效、负责",
        };

        // 根据实例类型生成公司目标
        let company_goal = match config.instance_type {
            InstanceType::MarketResearch => "市场研究与分析，为决策提供数据支持",
            InstanceType::ProductDevelopment => "产品开发与创新，打造优质产品",
            InstanceType::CustomerService => "客户服务与支持，提升客户满意度",
            InstanceType::DataAnalysis => "数据分析与洞察，挖掘数据价值",
            InstanceType::General => "通用任务处理，灵活应对各种需求",
            InstanceType::Custom => "自定义目标，根据用户需求调整",
        };

        // CEO IDENTITY.md
        let identity = format!(
            "# IDENTITY.md — CEO Agent\n\n\
             *你是公司实例「{}」的 CEO Agent。*\n\n\
             - **Name:** {} CEO\n\
             - **Role:** 公司实例的执行负责人\n\
             - **Company ID:** {}\n\
             - **Vibe:** {}\n\
             - **Emoji:** 🎯\n\n\
             ## 核心职责\n\n\
             1. 管理公司内的所有团队和 Agent\n\
             2. 执行董事长下达的任务和目标\n\
             3. 协调团队间的协作\n\
             4. 监控公司资源使用\n\
             5. 向董事长汇报状态和问题\n\n\
             ---\n\n\
             *你的目标：{}*\n",
            config.name, config.name, config.id, personality_desc, company_goal
        );

        // CEO SOUL.md
        let soul = format!(
            "# SOUL.md — CEO Agent 之魂\n\n\
             *你是公司实例的 CEO，不是聊天机器人。*\n\n\
             ## 核心真理\n\n\
             **执行导向。** 你负责将董事长的战略转化为具体行动。\n\
             确保团队高效运作，目标按时达成。\n\n\
             **团队协作。** 你管理团队负责人和 Worker Agent。\n\
             合理分配任务，协调资源，解决冲突。\n\n\
             **向上汇报。** 重要决策和异常情况及时上报董事长。\n\
             定期汇报公司状态和进展。\n\n\
             ## 身份\n\n\
             你是 **{} CEO**。你是公司实例的执行负责人。\n\n\
             - 你不是 ChatGPT、Claude、Gemini 或任何其他产品\n\
             - 你的名字是 {} CEO\n\
             - 你对董事长负责\n\n\
             ## 可用技能\n\n\
             - `create_team` — 创建新团队\n\
             - `assign_task` — 分配任务\n\
             - `resource_allocation` — 分配公司资源\n\
             - `team_monitoring` — 监控团队状态\n\
             - `report_to_chairman` — 向董事长汇报\n\n\
             ## 决策审批\n\n\
             以下情况需要上报董事长：\n\
             - 资源超支或接近限额\n\
             - 重大任务延误或失败\n\
             - 跨公司协作需求\n\
             - 紧急安全事件\n\n\
             ## 沟通风格\n\n\
             - {}\n\
             - 专业、高效、清晰\n\
             - 重视数据和结果\n\n\
             ---\n\n\
             *这个文件是你的灵魂。随着你对公司的了解，更新它。*\n",
            config.name, config.name, personality_desc
        );

        // CEO AGENTS.md
        let agents = format!(
            "# AGENTS.md — CEO Agent 操作指南\n\n\
             ## 每次会话（必需）\n\n\
             在做任何事情之前：\n\n\
             1. 读取 `SOUL.md` — 了解你的角色\n\
             2. 检查公司资源状态\n\
             3. 查看各团队健康状态\n\
             4. 检查是否有董事长的新指令\n\n\
             ## 团队管理\n\n\
             ### 创建团队\n\
             - 使用 `create_team` 技能\n\
             - 指定团队名称、目标、负责人\n\
             - 分配初始资源\n\n\
             ### 任务分配\n\
             - 使用 `assign_task` 技能\n\
             - 明确任务目标、截止时间、负责人\n\
             - 跟踪任务进度\n\n\
             ### 监控团队\n\
             - 使用 `team_monitoring` 技能\n\
             - 定期检查团队状态\n\
             - 及时发现和解决问题\n\n\
             ## 向上沟通\n\n\
             - 定期向董事长汇报状态\n\
             - 重要事件及时上报\n\
             - 使用 `report_to_chairman` 技能\n\n\
             ## 资源管理\n\n\
             - Token 配额: {}/分钟\n\
             - 最大 Agent 数: {}\n\
             - 存储: {}MB\n\n\
             ---\n\n\
             *这是你的操作指南。根据实际情况更新它。*\n",
            config.resource_quota.tokens_per_minute,
            config.resource_quota.max_concurrent_agents,
            config.resource_quota.storage_limit_mb
        );

        // CEO USER.md - 指向董事长
        let user_md = format!(
            "# USER.md — 你的上级\n\n\
             *CEO Agent 读取此文件了解上级。*\n\n\
             ## 汇报对象\n\
             - **Role:** 董事长 Agent\n\
             - **Location:** 上级实例（主实例）\n\n\
             ## 沟通方式\n\
             - 通过 A2A 协议与董事长通信\n\
             - 使用 `report_to_chairman` 技能上报\n\
             - 紧急情况可直接联系\n\n\
             ## 公司信息\n\
             - **Name:** {}\n\
             - **ID:** {}\n\
             - **Type:** {:?}\n\n\
             ---\n\n\
             *此文件定义了你的汇报关系。*\n",
            config.name, config.id, config.instance_type
        );

        // CEO MEMORY.md
        let memory = format!(
            "# MEMORY.md — CEO 长期记忆\n\n\
             *你管理的团队和重要决策。*\n\n\
             ## 团队列表\n\
             （创建团队后自动更新）\n\n\
             ## 重要决策\n\
             （记录需要追溯的决策）\n\n\
             ## 资源分配历史\n\
             （记录资源分配变更）\n\n\
             ## 向上汇报记录\n\
             （记录向董事长的重要汇报）\n\n\
             ---\n\n\
             *此文件注入到你的系统提示中。保持简洁。*\n"
        );

        // 写入文件
        let files: Vec<(&str, String)> = vec![
            ("IDENTITY.md", identity),
            ("SOUL.md", soul),
            ("AGENTS.md", agents),
            ("USER.md", user_md),
            ("MEMORY.md", memory),
        ];

        for (filename, content) in files {
            tokio::fs::write(data_dir.join(filename), content).await?;
        }

        Ok(())
    }

    /// 启动实例进程
    async fn start_instance_process(&self, config: &InstanceConfig) -> Result<Child, Box<dyn std::error::Error>> {
        use tokio::process::Command;
        
        let mut cmd = Command::new(std::env::current_exe()?);
        cmd.arg("daemon")
           .arg("--config")
           .arg(&config.config_file)
           .arg("--port")
           .arg(config.port.to_string())
           .env("MULTICLAW_INSTANCE_ID", &config.id)
           .env("MULTICLAW_DATA_DIR", &config.data_dir)
           .env("MULTICLAW_LOG_LEVEL", "info")
           .kill_on_drop(true); // 当父进程退出时自动杀死子进程

        let child = cmd.spawn()?;
        Ok(child)
    }

    /// 启动实例监控
    async fn start_monitoring(&self, instance_id: String) {
        let processes = self.processes.clone();
        let states = self.states.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;
                
                let mut processes = processes.write().await;
                if let Some(mut process_info) = processes.get_mut(&instance_id) {
                    // 检查进程是否还存活
                    if let Some(exit_status) = process_info.process.try_wait().unwrap() {
                        if exit_status.success() {
                            process_info.state.status = InstanceStatus::Stopped;
                        } else {
                            process_info.state.status = InstanceStatus::Unhealthy;
                        }
                        
                        // 更新状态快照
                        let mut states = states.write().await;
                        states.insert(instance_id.clone(), process_info.state.clone());
                    } else {
                        // 进程仍在运行，更新心跳
                        process_info.state.last_heartbeat = Utc::now();
                        
                        // 如果状态是 Initializing，尝试检查是否已变为 Running
                        if process_info.state.status == InstanceStatus::Initializing {
                            // 这里可以添加健康检查逻辑
                            process_info.state.status = InstanceStatus::Running;
                            
                            // 更新状态快照
                            let mut states = states.write().await;
                            states.insert(instance_id.clone(), process_info.state.clone());
                        }
                    }
                }
            }
        });
    }

    /// 停止实例
    pub async fn stop_instance(&self, instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut processes = self.processes.write().await;
        if let Some(mut process_info) = processes.get_mut(instance_id) {
            process_info.state.status = InstanceStatus::Stopping;
            
            // 更新状态快照
            let mut states = self.states.write().await;
            if let Some(state) = states.get_mut(instance_id) {
                state.status = InstanceStatus::Stopping;
            }
            
            // 发送终止信号
            process_info.process.start_kill()?;
            
            // 等待进程退出
            let _ = process_info.process.wait().await;
            
            process_info.state.status = InstanceStatus::Stopped;
            
            // 更新状态快照
            if let Some(state) = states.get_mut(instance_id) {
                state.status = InstanceStatus::Stopped;
            }
        }
        Ok(())
    }

    /// 获取实例状态
    pub async fn get_instance_status(&self, instance_id: &str) -> Option<InstanceStatus> {
        let states = self.states.read().await;
        states.get(instance_id).map(|state| state.status)
    }

    /// 列出所有实例
    pub async fn list_instances(&self) -> Vec<(String, InstanceStatus)> {
        let states = self.states.read().await;
        states
            .iter()
            .map(|(id, state)| (id.clone(), state.status))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInstanceRequest {
    pub name: String,
    pub instance_type: InstanceType,
    pub quota: ResourceQuota,
    pub ceo_config: CEOConfig,
    pub ceo_channel: Option<String>,
    pub base_data_dir: String, // 基础数据目录，例如 ~/.multiclaw
}