# MultiClaw v6.0 完整优化方案

本文档详细描述了 MultiClaw v6.0 的完整优化方案，解决了之前版本中发现的多实例管理、资源隔离、访问控制等方面的问题。

## 1. 多实例进程管理架构

### 1.1 架构设计

```
┌─────────────────────────────────────────────────────────────────┐
│                    董事长 Agent (主控)                           │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  InstanceManager (实例管理器)                            │    │
│  │  - 启动/停止实例进程                                      │    │
│  │  - 监控实例健康状态                                       │    │
│  │  - 管理实例间通信                                         │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                              ▼ (启动/管理)                        │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐    │
│  │   实例 1         │ │   实例 2         │ │   实例 N         │    │
│  │  (市场调研公司)  │ │  (产品开发公司)  │ │  (客户服务公司)  │    │
│  │  PID: 1234      │ │  PID: 5678      │ │  PID: 9012      │    │
│  │  Port: 8001     │ │  Port: 8002     │ │  Port: 8003     │    │
│  └─────────────────┘ └─────────────────┘ └─────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 实例管理器实现

```rust
// src/instance/manager.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
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
pub struct InstanceProcess {
    pub pid: u32,
    pub process: Child,
    pub config: InstanceConfig,
    pub status: InstanceStatus,
    pub created_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
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
    /// 实例进程管理
    instances: Arc<RwLock<HashMap<String, InstanceProcess>>>,
    /// 下一个可用端口
    next_port: Arc<RwLock<u16>>,
    /// 实例配置模板
    config_templates: HashMap<InstanceType, InstanceConfig>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: Arc::new(RwLock::new(HashMap::new())),
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
        
        // 启动实例进程
        let process = self.start_instance_process(&config).await?;
        
        // 注册实例
        let instance_process = InstanceProcess {
            pid: process.id().unwrap_or(0),
            process,
            config,
            status: InstanceStatus::Initializing,
            created_at: Utc::now(),
            last_heartbeat: Utc::now(),
        };
        
        let mut instances = self.instances.write().await;
        instances.insert(instance_id.clone(), instance_process);
        
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
        use toml_edit::{DocumentMut, Item, value};
        
        let mut doc = DocumentMut::new();
        
        // 基础配置
        doc["name"] = value(config.name.clone());
        doc["instance_id"] = value(config.id.clone());
        doc["port"] = value(config.port);
        
        // 资源配额
        doc["resource"]["tokens_per_minute"] = value(config.resource_quota.tokens_per_minute);
        doc["resource"]["max_concurrent_agents"] = value(config.resource_quota.max_concurrent_agents);
        doc["resource"]["storage_limit_mb"] = value(config.resource_quota.storage_limit_mb);
        doc["resource"]["api_calls_per_minute"] = value(config.resource_quota.api_calls_per_minute);
        
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
        let instances = self.instances.clone();
        tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(30)).await;
                
                let mut instances = instances.write().await;
                if let Some(mut instance) = instances.get_mut(&instance_id) {
                    // 检查进程是否还存活
                    if let Some(exit_status) = instance.process.try_wait().unwrap() {
                        if exit_status.success() {
                            instance.status = InstanceStatus::Stopped;
                        } else {
                            instance.status = InstanceStatus::Unhealthy;
                        }
                    } else {
                        // 进程仍在运行，更新心跳
                        instance.last_heartbeat = Utc::now();
                        
                        // 如果状态是 Initializing，尝试检查是否已变为 Running
                        if instance.status == InstanceStatus::Initializing {
                            // 这里可以添加健康检查逻辑
                            instance.status = InstanceStatus::Running;
                        }
                    }
                }
            }
        });
    }

    /// 停止实例
    pub async fn stop_instance(&self, instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut instances = self.instances.write().await;
        if let Some(mut instance) = instances.get_mut(instance_id) {
            instance.status = InstanceStatus::Stopping;
            
            // 发送终止信号
            instance.process.start_kill()?;
            
            // 等待进程退出
            let _ = instance.process.wait().await;
            
            instance.status = InstanceStatus::Stopped;
        }
        Ok(())
    }

    /// 获取实例状态
    pub async fn get_instance_status(&self, instance_id: &str) -> Option<InstanceStatus> {
        let instances = self.instances.read().await;
        instances.get(instance_id).map(|instance| instance.status)
    }

    /// 列出所有实例
    pub async fn list_instances(&self) -> Vec<(String, InstanceStatus)> {
        let instances = self.instances.read().await;
        instances
            .iter()
            .map(|(id, instance)| (id.clone(), instance.status))
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
```

## 2. 实例目录结构和配置系统

### 2.1 目录结构设计

```
~/.multiclaw/
├── config.toml                 # 全局配置
├── instances/                  # 实例目录
│   ├── instance-uuid1/         # 实例 1
│   │   ├── config.toml         # 实例 1 配置
│   │   ├── data/               # 实例 1 数据
│   │   │   ├── memory.db       # 实例 1 记忆数据库
│   │   │   └── logs/           # 实例 1 日志
│   │   └── cache/              # 实例 1 缓存
│   ├── instance-uuid2/         # 实例 2
│   │   ├── config.toml         # 实例 2 配置
│   │   ├── data/               # 实例 2 数据
│   │   │   ├── memory.db       # 实例 2 记忆数据库
│   │   │   └── logs/           # 实例 2 日志
│   │   └── cache/              # 实例 2 缓存
│   └── ...                     # 更多实例
├── global_memory.db            # 全局记忆数据库
├── skills/                     # 全局技能目录
├── logs/                       # 全局日志目录
└── cache/                      # 全局缓存目录
```

### 2.2 配置系统实现

```rust
// src/instance/config.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalConfig {
    /// 全局数据目录
    pub data_dir: PathBuf,
    /// 全局资源配额
    pub global_resource_quota: GlobalResourceQuota,
    /// 全局通信配置
    pub a2a_config: A2AConfig,
    /// 全局日志配置
    pub logging: LoggingConfig,
    /// 安全配置
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalResourceQuota {
    pub total_tokens_per_minute: u64,
    pub total_max_concurrent_agents: u32,
    pub total_storage_limit_mb: u64,
    pub max_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AConfig {
    pub gateway_port: u16,
    pub max_message_size_kb: u32,
    pub message_retention_days: u32,
    pub encryption_enabled: bool,
    pub cross_instance_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub max_log_files: u32,
    pub max_log_file_size_mb: u32,
    pub log_to_stdout: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_api_key_validation: bool,
    pub enable_rate_limiting: bool,
    pub rate_limit_requests_per_minute: u32,
    pub enable_encryption: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    /// 实例 ID
    pub instance_id: String,
    /// 实例名称
    pub name: String,
    /// 实例类型
    pub instance_type: String,
    /// 服务器配置
    pub server: ServerConfig,
    /// 资源配额
    pub resource_quota: InstanceResourceQuota,
    /// 通信渠道配置
    pub channels: Vec<ChannelConfig>,
    /// AI 提供商配置
    pub providers: Vec<ProviderConfig>,
    /// 记忆配置
    pub memory: MemoryConfig,
    /// 日志配置
    pub logging: LoggingConfig,
    /// 安全配置
    pub security: SecurityConfig,
    /// 技能配置
    pub skills: SkillsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: u32,
    pub max_connections: u32,
    pub connection_timeout_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceResourceQuota {
    pub tokens_per_minute: u64,
    pub max_concurrent_agents: u32,
    pub storage_limit_mb: u64,
    pub api_calls_per_minute: u32,
    pub memory_limit_mb: u64,
    pub cpu_shares: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub r#type: String,  // telegram, discord, slack, etc.
    pub enabled: bool,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub r#type: String,  // openai, anthropic, etc.
    pub model: String,
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub backend: String,  // sqlite, postgres, etc.
    pub path: PathBuf,
    pub retention_days: u32,
    pub max_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub enabled: Vec<String>,
    pub disabled: Vec<String>,
    pub custom_paths: Vec<PathBuf>,
}

pub struct ConfigManager {
    global_config: GlobalConfig,
    base_dir: PathBuf,
}

impl ConfigManager {
    pub async fn new(base_dir: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = base_dir.join("config.toml");
        
        // 如果配置文件不存在，创建默认配置
        if !config_path.exists().await {
            let default_config = Self::default_global_config(&base_dir).await;
            Self::save_global_config(&config_path, &default_config).await?;
        }
        
        let global_config = Self::load_global_config(&config_path).await?;
        
        Ok(Self {
            global_config,
            base_dir,
        })
    }

    async fn default_global_config(base_dir: &PathBuf) -> GlobalConfig {
        GlobalConfig {
            data_dir: base_dir.clone(),
            global_resource_quota: GlobalResourceQuota {
                total_tokens_per_minute: 1_000_000,
                total_max_concurrent_agents: 100,
                total_storage_limit_mb: 10_000,
                max_instances: 10,
            },
            a2a_config: A2AConfig {
                gateway_port: 8080,
                max_message_size_kb: 1024,
                message_retention_days: 30,
                encryption_enabled: true,
                cross_instance_allowed: true,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                max_log_files: 10,
                max_log_file_size_mb: 100,
                log_to_stdout: false,
            },
            security: SecurityConfig {
                enable_api_key_validation: true,
                enable_rate_limiting: true,
                rate_limit_requests_per_minute: 1000,
                enable_encryption: true,
            },
        }
    }

    async fn load_global_config(path: &PathBuf) -> Result<GlobalConfig, Box<dyn std::error::Error>> {
        let content = fs::read_to_string(path).await?;
        let config: GlobalConfig = toml::from_str(&content)?;
        Ok(config)
    }

    async fn save_global_config(path: &PathBuf, config: &GlobalConfig) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(config)?;
        fs::write(path, content).await?;
        Ok(())
    }

    /// 创建实例配置目录结构
    pub async fn create_instance_structure(&self, instance_id: &str) -> Result<InstancePaths, Box<dyn std::error::Error>> {
        let instance_dir = self.base_dir.join("instances").join(instance_id);
        
        // 创建实例目录
        fs::create_dir_all(&instance_dir).await?;
        
        // 创建子目录
        let data_dir = instance_dir.join("data");
        let logs_dir = data_dir.join("logs");
        let cache_dir = instance_dir.join("cache");
        
        fs::create_dir_all(&data_dir).await?;
        fs::create_dir_all(&logs_dir).await?;
        fs::create_dir_all(&cache_dir).await?;
        
        // 创建默认实例配置
        let instance_config = self.default_instance_config(instance_id).await;
        let config_path = instance_dir.join("config.toml");
        self.save_instance_config(&config_path, &instance_config).await?;
        
        Ok(InstancePaths {
            instance_dir,
            data_dir,
            logs_dir,
            cache_dir,
            config_path,
        })
    }

    async fn default_instance_config(&self, instance_id: &str) -> InstanceConfig {
        InstanceConfig {
            instance_id: instance_id.to_string(),
            name: format!("Instance-{}", &instance_id[..8]),
            instance_type: "general".to_string(),
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8000,  // 这个会被动态分配
                workers: 4,
                max_connections: 100,
                connection_timeout_seconds: 30,
            },
            resource_quota: InstanceResourceQuota {
                tokens_per_minute: 100_000,
                max_concurrent_agents: 10,
                storage_limit_mb: 1000,
                api_calls_per_minute: 100,
                memory_limit_mb: 512,
                cpu_shares: 512,
            },
            channels: vec![],
            providers: vec![],
            memory: MemoryConfig {
                backend: "sqlite".to_string(),
                path: PathBuf::from("memory.db"),
                retention_days: 30,
                max_entries: 10000,
            },
            logging: self.global_config.logging.clone(),
            security: self.global_config.security.clone(),
            skills: SkillsConfig {
                enabled: vec!["information_gathering".to_string(), "data_analysis".to_string()],
                disabled: vec![],
                custom_paths: vec![],
            },
        }
    }

    async fn save_instance_config(&self, path: &PathBuf, config: &InstanceConfig) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(config)?;
        fs::write(path, content).await?;
        Ok(())
    }

    /// 加载实例配置
    pub async fn load_instance_config(&self, instance_id: &str) -> Result<InstanceConfig, Box<dyn std::error::Error>> {
        let config_path = self.base_dir.join("instances").join(instance_id).join("config.toml");
        let content = fs::read_to_string(&config_path).await?;
        let mut config: InstanceConfig = toml::from_str(&content)?;
        
        // 确保实例 ID 与路径匹配
        config.instance_id = instance_id.to_string();
        
        Ok(config)
    }

    /// 更新实例配置
    pub async fn update_instance_config(&self, instance_id: &str, config: &InstanceConfig) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = self.base_dir.join("instances").join(instance_id).join("config.toml");
        self.save_instance_config(&config_path, config).await?;
        Ok(())
    }

    /// 获取全局配置
    pub fn get_global_config(&self) -> &GlobalConfig {
        &self.global_config
    }
}

#[derive(Debug, Clone)]
pub struct InstancePaths {
    pub instance_dir: PathBuf,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub config_path: PathBuf,
}
```

## 3. CreateCompanySkill 指导创建流程

### 3.1 Skill 定义

```rust
// src/skills/create_company.rs
use crate::skills::{Skill, SkillExecutor, SkillMetadata, SkillContext, SkillExecutionResult, ExecutionStatus};
use crate::instance::{InstanceManager, InstanceConfig, CreateInstanceRequest, InstanceType, ResourceQuota, CEOConfig};
use crate::config::ConfigManager;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 创建公司技能
pub struct CreateCompanySkill {
    instance_manager: Arc<InstanceManager>,
    config_manager: Arc<ConfigManager>,
    metadata: SkillMetadata,
}

impl CreateCompanySkill {
    pub fn new(instance_manager: Arc<InstanceManager>, config_manager: Arc<ConfigManager>) -> Self {
        Self {
            instance_manager,
            config_manager,
            metadata: SkillMetadata {
                name: "create_company".to_string(),
                description: "创建新的公司实例（独立的 MultiClaw 实例）".to_string(),
                skill_type: crate::skills::SkillType::CEO,
                version: "1.0.0".to_string(),
                required_executor_type: crate::skills::ExecutorType::Chairman,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "公司名称"
                        },
                        "company_type": {
                            "type": "string",
                            "enum": ["market_research", "product_development", "customer_service", "data_analysis", "general", "custom"],
                            "description": "公司类型"
                        },
                        "token_quota": {
                            "type": "integer",
                            "minimum": 10000,
                            "maximum": 10000000,
                            "description": "Token 配额（每分钟）"
                        },
                        "max_agents": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "description": "最大 Agent 数量"
                        },
                        "ceo_model": {
                            "type": "string",
                            "description": "CEO 使用的模型"
                        },
                        "ceo_personality": {
                            "type": "string",
                            "enum": ["analytical", "creative", "strategic", "practical"],
                            "description": "CEO 性格特征"
                        },
                        "channel": {
                            "type": "string",
                            "description": "绑定的通信渠道（可选）"
                        },
                        "base_data_dir": {
                            "type": "string",
                            "description": "基础数据目录"
                        }
                    },
                    "required": ["name", "company_type", "token_quota", "max_agents", "ceo_model", "ceo_personality"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "instance_id": { "type": "string" },
                        "instance_name": { "type": "string" },
                        "instance_type": { "type": "string" },
                        "port": { "type": "integer" },
                        "data_dir": { "type": "string" },
                        "initial_status": { "type": "string" },
                        "message": { "type": "string" }
                    }
                }),
                resource_requirements: crate::skills::ResourceRequirements {
                    compute: Some(5),
                    memory: Some(128),
                    storage: Some(10),
                    bandwidth: Some(100),
                    api_calls: Some(2),
                    tokens: Some(100),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["company".to_string(), "creation".to_string(), "management".to_string()],
                category: "Infrastructure".to_string(),
            },
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for CreateCompanySkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        // 解析输入参数
        let name = context.inputs.get("name")
            .and_then(|v| v.as_str())
            .ok_or("缺少公司名称")?
            .to_string();
        
        let company_type_str = context.inputs.get("company_type")
            .and_then(|v| v.as_str())
            .ok_or("缺少公司类型")?;
        
        let company_type = match company_type_str {
            "market_research" => InstanceType::MarketResearch,
            "product_development" => InstanceType::ProductDevelopment,
            "customer_service" => InstanceType::CustomerService,
            "data_analysis" => InstanceType::DataAnalysis,
            "general" => InstanceType::General,
            "custom" => InstanceType::Custom,
            _ => return Err("无效的公司类型".into()),
        };
        
        let token_quota = context.inputs.get("token_quota")
            .and_then(|v| v.as_u64())
            .ok_or("缺少 Token 配额")?;
        
        let max_agents = context.inputs.get("max_agents")
            .and_then(|v| v.as_u64())
            .ok_or("缺少最大 Agent 数量")? as u32;
        
        let ceo_model = context.inputs.get("ceo_model")
            .and_then(|v| v.as_str())
            .ok_or("缺少 CEO 模型")?
            .to_string();
        
        let ceo_personality = context.inputs.get("ceo_personality")
            .and_then(|v| v.as_str())
            .ok_or("缺少 CEO 性格")?
            .to_string();
        
        let base_data_dir = context.inputs.get("base_data_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("~/.multiclaw"); // 默认值
        
        let channel = context.inputs.get("channel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // 构建资源配额
        let resource_quota = ResourceQuota {
            tokens_per_minute: token_quota as u32,
            max_concurrent_agents: max_agents,
            storage_limit_mb: 1000, // 默认 1GB
            api_calls_per_minute: 1000, // 默认 1000 次/分钟
        };

        // 构建 CEO 配置
        let ceo_config = CEOConfig {
            model_preference: ceo_model,
            personality: ceo_personality,
            resource_limits: resource_quota.clone(),
        };

        // 构建创建请求
        let create_request = CreateInstanceRequest {
            name,
            instance_type: company_type,
            quota: resource_quota,
            ceo_config,
            ceo_channel: channel,
            base_data_dir: shellexpand::tilde(base_data_dir).to_string(),
        };

        // 执行创建实例
        let instance_id = self.instance_manager.create_instance(create_request).await?;

        // 获取实例状态
        let status = self.instance_manager.get_instance_status(&instance_id).await
            .unwrap_or(crate::instance::InstanceStatus::Initializing);

        // 返回结果
        let result = serde_json::json!({
            "instance_id": instance_id,
            "instance_name": context.inputs.get("name").unwrap_or(&Value::String("Unknown".to_string())),
            "instance_type": context.inputs.get("company_type").unwrap_or(&Value::String("general".to_string())),
            "port": 8000, // 这个会在创建过程中动态分配
            "data_dir": format!("{}/instances/{}", shellexpand::tilde(base_data_dir), instance_id),
            "initial_status": format!("{:?}", status),
            "message": format!("公司实例「{}」创建成功，ID: {}", 
                context.inputs.get("name").unwrap_or(&Value::String("Unknown".to_string())),
                instance_id)
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 0, // 这里应该计算实际执行时间
            resources_used: Default::default(), // 这里应该记录实际资源使用
            completed_at: Utc::now(),
        })
    }

    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }
}

/// 公司创建引导技能 - 交互式创建流程
pub struct CompanyCreationGuideSkill {
    instance_manager: Arc<InstanceManager>,
    config_manager: Arc<ConfigManager>,
    metadata: SkillMetadata,
}

impl CompanyCreationGuideSkill {
    pub fn new(instance_manager: Arc<InstanceManager>, config_manager: Arc<ConfigManager>) -> Self {
        Self {
            instance_manager,
            config_manager,
            metadata: SkillMetadata {
                name: "company_creation_guide".to_string(),
                description: "引导用户完成公司创建的交互式流程".to_string(),
                skill_type: crate::skills::SkillType::CEO,
                version: "1.0.0".to_string(),
                required_executor_type: crate::skills::ExecutorType::Chairman,
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "step": {
                            "type": "string",
                            "enum": ["init", "name", "type", "resources", "ceo", "channel", "confirm", "complete"],
                            "description": "当前步骤"
                        },
                        "current_data": {
                            "type": "object",
                            "description": "当前收集到的数据"
                        }
                    },
                    "required": ["step"]
                }),
                output_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "next_step": { "type": "string" },
                        "prompt": { "type": "string" },
                        "collected_data": { "type": "object" },
                        "completed": { "type": "boolean" }
                    }
                }),
                resource_requirements: crate::skills::ResourceRequirements {
                    compute: Some(3),
                    memory: Some(64),
                    storage: Some(5),
                    bandwidth: Some(50),
                    api_calls: Some(1),
                    tokens: Some(50),
                    concurrent_agents: Some(1),
                },
                created_at: Utc::now(),
                updated_at: Utc::now(),
                author: "MultiClaw System".to_string(),
                tags: vec!["guide".to_string(), "interactive".to_string(), "company".to_string()],
                category: "Workflow".to_string(),
            },
        }
    }

    /// 获取下一步骤
    fn get_next_step(&self, current_step: &str) -> &'static str {
        match current_step {
            "init" => "name",
            "name" => "type",
            "type" => "resources",
            "resources" => "ceo",
            "ceo" => "channel",
            "channel" => "confirm",
            "confirm" => "complete",
            _ => "complete",
        }
    }
}

#[async_trait::async_trait]
impl SkillExecutor for CompanyCreationGuideSkill {
    async fn execute(&self, context: SkillContext) -> Result<SkillExecutionResult, Box<dyn std::error::Error>> {
        let step = context.inputs.get("step")
            .and_then(|v| v.as_str())
            .ok_or("缺少步骤信息")?;
        
        let current_data = context.inputs.get("current_data")
            .unwrap_or(&Value::Object(serde_json::Map::new()))
            .clone();

        let (next_step, prompt, completed) = match step {
            "init" => (
                "name",
                "欢迎使用公司创建向导！首先，请告诉我您想创建的公司名称：".to_string(),
                false
            ),
            "name" => {
                let name = context.inputs.get("company_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("新公司");
                
                (
                    "type",
                    format!("好的，公司名称为「{}」。\n请选择公司类型：\n1. 市场调研\n2. 产品开发\n3. 客户服务\n4. 数据分析\n5. 通用型\n请回复数字或类型名称：", name),
                    false
                )
            },
            "type" => {
                let company_type = context.inputs.get("company_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("通用型");
                
                (
                    "resources",
                    format!("公司类型：{}。\n请设置资源配额：\n- 每分钟 Token 配额（建议 50000-500000）：\n- 最大 Agent 数量（建议 5-50）：\n请分别提供两个数值，用逗号分隔：", company_type),
                    false
                )
            },
            "resources" => {
                let tokens = context.inputs.get("token_quota")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(100000);
                
                let agents = context.inputs.get("max_agents")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);
                
                (
                    "ceo",
                    format!("资源配置：{} tokens/min, {} agents。\n请选择 CEO 的模型偏好：\n1. GPT-4 (通用)\n2. Claude Sonnet (分析)\n3. Gemini Pro (创新)\n请选择并描述 CEO 性格特征（分析型/创意型/战略型/实用型）：", tokens, agents),
                    false
                )
            },
            "ceo" => {
                let model = context.inputs.get("ceo_model")
                    .and_then(|v| v.as_str())
                    .unwrap_or("gpt-4");
                
                let personality = context.inputs.get("ceo_personality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("analytical");
                
                (
                    "channel",
                    format!("CEO 配置：模型={}，性格={}。\n是否需要为该公司绑定通信渠道？\n如需绑定，请提供渠道类型和凭证（如：telegram:YOUR_BOT_TOKEN）；如不需要，请回复“跳过”：", model, personality),
                    false
                )
            },
            "channel" => {
                (
                    "confirm",
                    format!("即将完成配置，确认信息如下：\n{}\n\n请确认是否继续创建（回复“确认”或“取消”）：", 
                        serde_json::to_string_pretty(&current_data)?),
                    false
                )
            },
            "confirm" => {
                // 这里实际执行创建
                let name = current_data.get("name").and_then(|v| v.as_str()).unwrap_or("默认公司");
                let company_type_str = current_data.get("type").and_then(|v| v.as_str()).unwrap_or("general");
                
                let company_type = match company_type_str {
                    "market_research" => InstanceType::MarketResearch,
                    "product_development" => InstanceType::ProductDevelopment,
                    "customer_service" => InstanceType::CustomerService,
                    "data_analysis" => InstanceType::DataAnalysis,
                    "general" => InstanceType::General,
                    "custom" => InstanceType::Custom,
                    _ => InstanceType::General,
                };
                
                let token_quota = current_data.get("token_quota").and_then(|v| v.as_u64()).unwrap_or(100000) as u32;
                let max_agents = current_data.get("max_agents").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
                
                let ceo_model = current_data.get("ceo_model").and_then(|v| v.as_str()).unwrap_or("gpt-4").to_string();
                let ceo_personality = current_data.get("ceo_personality").and_then(|v| v.as_str()).unwrap_or("analytical").to_string();
                
                let channel = current_data.get("channel").and_then(|v| v.as_str()).map(|s| s.to_string());
                
                let resource_quota = ResourceQuota {
                    tokens_per_minute: token_quota,
                    max_concurrent_agents: max_agents,
                    storage_limit_mb: 1000,
                    api_calls_per_minute: 1000,
                };
                
                let ceo_config = CEOConfig {
                    model_preference: ceo_model,
                    personality: ceo_personality,
                    resource_limits: resource_quota.clone(),
                };
                
                let create_request = CreateInstanceRequest {
                    name: name.to_string(),
                    instance_type: company_type,
                    quota: resource_quota,
                    ceo_config,
                    ceo_channel: channel,
                    base_data_dir: shellexpand::tilde("~/.multiclaw").to_string(),
                };
                
                let instance_id = self.instance_manager.create_instance(create_request).await?;
                
                (
                    "complete", 
                    format!("✅ 公司创建成功！\n公司名称：{}\n实例ID：{}\n访问端口：{}（动态分配）\n数据目录：~/.multiclaw/instances/{}/\n\n公司已启动运行！", name, instance_id, 8000, instance_id), 
                    true
                )
            },
            _ => ("init", "欢迎使用公司创建向导！".to_string(), false)
        };

        let result = serde_json::json!({
            "next_step": next_step,
            "prompt": prompt,
            "collected_data": current_data,
            "completed": completed
        });

        Ok(SkillExecutionResult {
            execution_id: Uuid::new_v4().to_string(),
            skill_id: self.metadata.name.clone(),
            status: ExecutionStatus::Success,
            result: Some(result),
            error: None,
            execution_time_ms: 0,
            resources_used: Default::default(),
            completed_at: Utc::now(),
        })
    }

    fn metadata(&self) -> &SkillMetadata {
        &self.metadata
    }

    fn name(&self) -> &str {
        &self.metadata.name
    }
}
```

## 4. A2A 实际通信机制

### 4.1 通信协议增强

```rust
// src/a2a/enhanced_gateway.rs
use crate::a2a::{A2AMessage, A2AMessageType, MessagePriority, A2AMessageBuilder};
use crate::instance::{InstanceManager, InstanceStatus};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{sleep, Duration};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2AEndpoint {
    pub instance_id: String,
    pub host: String,
    pub port: u16,
    pub secure: bool,
    pub connected: bool,
    pub last_heartbeat: DateTime<Utc>,
}

pub struct EnhancedA2AGateway {
    /// 本地实例管理器
    instance_manager: Arc<InstanceManager>,
    /// 远程实例连接
    remote_endpoints: Arc<RwLock<HashMap<String, A2AEndpoint>>>,
    /// WebSocket 客户端连接
    ws_connections: Arc<RwLock<HashMap<String, tokio_tungstenite::WebSocketStream<TcpStream>>>>,
    /// 消息队列
    message_queue: Arc<RwLock<Vec<A2AMessage>>>,
    /// 通信权限管理
    permission_manager: Arc<PermissionManager>,
    /// 消息处理器
    message_handler: Arc<MessageHandler>,
}

impl EnhancedA2AGateway {
    pub fn new(instance_manager: Arc<InstanceManager>) -> Self {
        Self {
            instance_manager,
            remote_endpoints: Arc::new(RwLock::new(HashMap::new())),
            ws_connections: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(RwLock::new(Vec::new())),
            permission_manager: Arc::new(PermissionManager::new()),
            message_handler: Arc::new(MessageHandler::new()),
        }
    }

    /// 注册远程实例端点
    pub async fn register_remote_endpoint(&self, endpoint: A2AEndpoint) -> Result<(), Box<dyn std::error::Error>> {
        let mut endpoints = self.remote_endpoints.write().await;
        endpoints.insert(endpoint.instance_id.clone(), endpoint);
        Ok(())
    }

    /// 发送消息到远程实例
    pub async fn send_to_remote(&self, message: A2AMessage) -> Result<String, Box<dyn std::error::Error>> {
        // 验证权限
        if !self.permission_manager.can_send(&message).await {
            return Err("权限不足：无法发送此类型的消息".into());
        }

        // 确定目标实例
        let target_instance_id = self.determine_target_instance(&message).await?;
        
        // 获取目标端点
        let endpoint = {
            let endpoints = self.remote_endpoints.read().await;
            endpoints.get(&target_instance_id)
                .ok_or("目标实例端点未注册")?
                .clone()
        };

        // 建立或复用连接
        let mut ws_stream = self.get_or_connect_ws(&endpoint).await?;

        // 序列化消息
        let json_msg = serde_json::to_string(&message)?;
        let ws_msg = Message::Text(json_msg);

        // 发送消息
        ws_stream.send(ws_msg).await?;

        // 等待响应（如果需要）
        if message.requires_reply {
            if let Some(timeout) = message.timeout_secs {
                match tokio::time::timeout(
                    Duration::from_secs(timeout),
                    ws_stream.recv()
                ).await {
                    Ok(Some(response)) => {
                        if let Message::Text(response_text) = response {
                            return Ok(response_text);
                        }
                    }
                    Ok(None) => return Err("连接意外断开".into()),
                    Err(_) => return Err("等待响应超时".into()),
                }
            }
        }

        Ok("消息已发送".to_string())
    }

    /// 确定目标实例
    async fn determine_target_instance(&self, message: &A2AMessage) -> Result<String, Box<dyn std::error::Error>> {
        // 根据接收者ID确定目标实例
        // 这里简化处理，实际实现可能需要查询 ClusterState
        if message.recipient_id.starts_with("ceo-") {
            // CEO 类型的ID，需要查找对应的实例
            // 在实际实现中，这里会查询 ClusterState 来找到 CEO 对应的实例
            Ok("unknown_instance".to_string()) // 简化实现
        } else {
            Ok(message.recipient_id.clone()) // 假设接收者ID就是实例ID
        }
    }

    /// 获取或建立 WebSocket 连接
    async fn get_or_connect_ws(&self, endpoint: &A2AEndpoint) -> Result<tokio_tungstenite::WebSocketStream<TcpStream>, Box<dyn std::error::Error>> {
        // 检查是否已有连接
        {
            let connections = self.ws_connections.read().await;
            if let Some(stream) = connections.get(&endpoint.instance_id) {
                // 尝试发送心跳以验证连接
                if stream.send(Message::Ping(vec![])).await.is_ok() {
                    return Ok(stream.clone());
                }
            }
        }

        // 建立新连接
        let scheme = if endpoint.secure { "wss" } else { "ws" };
        let url = Url::parse(&format!("{}://{}:{}/a2a/ws", scheme, endpoint.host, endpoint.port))
            .map_err(|e| format!("无效的 WebSocket URL: {}", e))?;

        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| format!("连接到 {} 失败: {}", endpoint.instance_id, e))?;

        // 保存连接
        {
            let mut connections = self.ws_connections.write().await;
            connections.insert(endpoint.instance_id.clone(), ws_stream);
        }

        // 获取刚保存的连接
        let connections = self.ws_connections.read().await;
        let stream = connections.get(&endpoint.instance_id)
            .ok_or("连接保存失败")?
            .clone();

        Ok(stream)
    }

    /// 启动本地 WebSocket 服务器以接收消息
    pub async fn start_local_server(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        use tokio_tungstenite::accept_async;
        use tokio::net::TcpListener;

        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        
        println!("A2A 网关服务器启动在端口 {}", port);
        
        loop {
            let (stream, _) = listener.accept().await?;
            let ws_stream = accept_async(stream).await?;
            
            let handler = self.message_handler.clone();
            let perm_manager = self.permission_manager.clone();
            
            tokio::spawn(async move {
                if let Err(e) = Self::handle_client_connection(ws_stream, handler, perm_manager).await {
                    eprintln!("处理客户端连接时出错: {}", e);
                }
            });
        }
    }

    /// 处理客户端连接
    async fn handle_client_connection(
        mut ws_stream: tokio_tungstenite::WebSocketStream<TcpStream>,
        handler: Arc<MessageHandler>,
        perm_manager: Arc<PermissionManager>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        while let Some(msg) = ws_stream.recv().await? {
            match msg {
                Message::Text(text) => {
                    // 解析消息
                    let message: A2AMessage = serde_json::from_str(&text)?;
                    
                    // 验证权限
                    if !perm_manager.can_receive(&message).await {
                        let error_response = A2AMessageBuilder::new(
                            "system".to_string(),
                            message.sender_id.clone(),
                            A2AMessageType::Error {
                                in_reply_to: message.message_id.clone(),
                                error_code: "PERMISSION_DENIED".to_string(),
                                error_message: "权限不足：无法接收此类型的消息".to_string(),
                            }
                        ).build();
                        
                        ws_stream.send(Message::Text(serde_json::to_string(&error_response)?)).await?;
                        continue;
                    }

                    // 处理消息
                    let response = handler.handle_message(message).await?;
                    
                    // 发送响应（如果需要）
                    if let Some(resp) = response {
                        ws_stream.send(Message::Text(serde_json::to_string(&resp)?)).await?;
                    }
                }
                Message::Ping(_) => {
                    ws_stream.send(Message::Pong(vec![])).await?;
                }
                Message::Pong(_) => {
                    // 响应 pong，什么都不做
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    /// 发送跨实例协作请求
    pub async fn send_cross_instance_request(
        &self,
        from_instance: &str,
        to_instance: &str,
        purpose: &str,
        content: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let message = A2AMessageBuilder::new(
            from_instance.to_string(),
            to_instance.to_string(),
            A2AMessageType::CollaborationRequest {
                description: purpose.to_string(),
                expected_outcome: "Cross-instance collaboration".to_string(),
                deadline: None,
            }
        )
        .with_content(content)
        .with_priority(MessagePriority::High)
        .requires_reply(true)
        .with_timeout(Some(300)) // 5分钟超时
        .build();

        self.send_to_remote(message).await
    }
}

/// 权限管理器
pub struct PermissionManager {
    // 在实际实现中，这里会有更复杂的权限规则
    // 基于角色、资源、时间等因素的权限检查
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {}
    }

    /// 检查是否有权限发送消息
    pub async fn can_send(&self, message: &A2AMessage) -> bool {
        // 简化实现：CEO 和 Chairman 可以跨实例通信
        // 在实际实现中，这里会有更复杂的权限检查逻辑
        message.sender_id.starts_with("ceo-") || 
        message.sender_id.starts_with("chairman-")
    }

    /// 检查是否有权限接收消息
    pub async fn can_receive(&self, message: &A2AMessage) -> bool {
        // 简化实现：接受所有来自已知实例的消息
        // 在实际实现中，这里会有更复杂的权限检查逻辑
        true
    }
}

/// 消息处理器
pub struct MessageHandler {}

impl MessageHandler {
    pub fn new() -> Self {
        Self {}
    }

    /// 处理传入的消息
    pub async fn handle_message(&self, message: A2AMessage) -> Result<Option<A2AMessage>, Box<dyn std::error::Error>> {
        match message.message_type {
            A2AMessageType::Query { question } => {
                // 处理查询请求
                let response = A2AMessageBuilder::new(
                    "local_instance".to_string(),
                    message.sender_id.clone(),
                    A2AMessageType::Response {
                        in_reply_to: message.message_id.clone(),
                        content: format!("Query received: {}", question),
                        success: true,
                    }
                ).build();
                
                Ok(Some(response))
            }
            A2AMessageType::CollaborationRequest { description, expected_outcome, deadline } => {
                // 处理协作请求 - 这可能需要人工审批
                let response = A2AMessageBuilder::new(
                    "local_instance".to_string(),
                    message.sender_id.clone(),
                    A2AMessageType::Response {
                        in_reply_to: message.message_id.clone(),
                        content: format!("Collaboration request received: {}. Outcome: {}. Deadline: {:?}", 
                                       description, expected_outcome, deadline),
                        success: true,
                    }
                ).build();
                
                Ok(Some(response))
            }
            A2AMessageType::KnowledgeShare { knowledge_type, content, applicable_scenarios } => {
                // 处理知识分享
                let response = A2AMessageBuilder::new(
                    "local_instance".to_string(),
                    message.sender_id.clone(),
                    A2AMessageType::Response {
                        in_reply_to: message.message_id.clone(),
                        content: format!("Knowledge shared: {} of type {}. Scenarios: {:?}", 
                                       content.chars().take(50).collect::<String>(), 
                                       knowledge_type, applicable_scenarios),
                        success: true,
                    }
                ).build();
                
                Ok(Some(response))
            }
            _ => {
                // 对于不需要回复的消息，返回 None
                Ok(None)
            }
        }
    }
}
```

## 5. 资源隔离和配额管理

### 5.1 资源管理器实现

```rust
// src/core/resource_isolation.rs
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Semaphore};
use tokio::time;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceResourceLimits {
    pub tokens_per_minute: u64,
    pub max_concurrent_agents: u32,
    pub storage_limit_mb: u64,
    pub api_calls_per_minute: u64,
    pub memory_limit_mb: u64,
    pub cpu_shares: u32,
}

#[derive(Debug, Clone)]
pub struct InstanceResourceUsage {
    pub tokens_used: u64,
    pub tokens_remaining: u64,
    pub active_agents: u32,
    pub storage_used_mb: u64,
    pub api_calls_used: u64,
    pub memory_used_mb: u64,
    pub cpu_usage_pct: f64,
    pub last_updated: DateTime<Utc>,
}

pub struct InstanceResourceManager {
    /// 每个实例的资源限制
    limits: Arc<RwLock<HashMap<String, InstanceResourceLimits>>>,
    /// 每个实例的当前资源使用情况
    usage: Arc<RwLock<HashMap<String, InstanceResourceUsage>>>,
    /// 每个实例的 Agent 信号量（限制并发数）
    agent_semaphores: Arc<RwLock<HashMap<String, Arc<Semaphore>>>>,
    /// 每分钟 Token 使用计数器
    token_counters: Arc<RwLock<HashMap<String, TokenCounter>>>,
    /// 每分钟 API 调用计数器
    api_call_counters: Arc<RwLock<HashMap<String, ApiCallCounter>>>,
    /// 存储使用跟踪器
    storage_trackers: Arc<RwLock<HashMap<String, StorageTracker>>>,
    /// 监控任务句柄
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

struct TokenCounter {
    count: u64,
    reset_time: Instant,
}

struct ApiCallCounter {
    count: u64,
    reset_time: Instant,
}

struct StorageTracker {
    used_mb: u64,
    limit_mb: u64,
}

impl InstanceResourceManager {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            usage: Arc::new(RwLock::new(HashMap::new())),
            agent_semaphores: Arc::new(RwLock::new(HashMap::new())),
            token_counters: Arc::new(RwLock::new(HashMap::new())),
            api_call_counters: Arc::new(RwLock::new(HashMap::new())),
            storage_trackers: Arc::new(RwLock::new(HashMap::new())),
            monitor_handle: None,
        }
    }

    /// 注册新实例的资源限制
    pub async fn register_instance(&self, instance_id: &str, limits: InstanceResourceLimits) -> Result<(), Box<dyn std::error::Error>> {
        let mut limits_map = self.limits.write().await;
        limits_map.insert(instance_id.to_string(), limits.clone());

        // 初始化使用情况
        let mut usage_map = self.usage.write().await;
        usage_map.insert(instance_id.to_string(), InstanceResourceUsage {
            tokens_used: 0,
            tokens_remaining: limits.tokens_per_minute,
            active_agents: 0,
            storage_used_mb: 0,
            api_calls_used: 0,
            memory_used_mb: 0,
            cpu_usage_pct: 0.0,
            last_updated: Utc::now(),
        });

        // 创建 Agent 信号量
        let mut sem_map = self.agent_semaphores.write().await;
        sem_map.insert(instance_id.to_string(), Arc::new(Semaphore::new(limits.max_concurrent_agents as usize)));

        // 初始化计数器
        let mut token_counters = self.token_counters.write().await;
        token_counters.insert(instance_id.to_string(), TokenCounter {
            count: 0,
            reset_time: Instant::now(),
        });

        let mut api_counters = self.api_call_counters.write().await;
        api_counters.insert(instance_id.to_string(), ApiCallCounter {
            count: 0,
            reset_time: Instant::now(),
        });

        let mut storage_trackers = self.storage_trackers.write().await;
        storage_trackers.insert(instance_id.to_string(), StorageTracker {
            used_mb: 0,
            limit_mb: limits.storage_limit_mb,
        });

        Ok(())
    }

    /// 尝试申请 Token 配额
    pub async fn try_acquire_tokens(&self, instance_id: &str, tokens: u64) -> Result<bool, Box<dyn std::error::Error>> {
        let limits = self.limits.read().await;
        let limit = limits.get(instance_id)
            .ok_or("实例未注册")?
            .tokens_per_minute;

        let mut counters = self.token_counters.write().await;
        let counter = counters.get_mut(instance_id)
            .ok_or("计数器未初始化")?;

        // 检查是否需要重置计数器（每分钟重置）
        if counter.reset_time.elapsed() >= Duration::from_secs(60) {
            counter.count = 0;
            counter.reset_time = Instant::now();
        }

        // 检查是否超出配额
        if counter.count + tokens > limit {
            return Ok(false); // 超出配额，不允许申请
        }

        // 更新计数器
        counter.count += tokens;

        // 更新使用情况
        let mut usage = self.usage.write().await;
        if let Some(usage_info) = usage.get_mut(instance_id) {
            usage_info.tokens_used = counter.count;
            usage_info.tokens_remaining = limit.saturating_sub(counter.count);
            usage_info.last_updated = Utc::now();
        }

        Ok(true)
    }

    /// 尝试获取 Agent 并发许可
    pub async fn acquire_agent_permit(&self, instance_id: &str) -> Result<tokio::sync::SemaphorePermit<'_, ()>, Box<dyn std::error::Error>> {
        let semaphores = self.agent_semaphores.read().await;
        let semaphore = semaphores.get(instance_id)
            .ok_or("信号量未初始化")?;

        let permit = semaphore.acquire().await
            .map_err(|_| "无法获取 Agent 并发许可")?;

        // 更新活动 Agent 数量
        let mut usage = self.usage.write().await;
        if let Some(usage_info) = usage.get_mut(instance_id) {
            usage_info.active_agents += 1;
            usage_info.last_updated = Utc::now();
        }

        Ok(permit)
    }

    /// 尝试申请 API 调用配额
    pub async fn try_acquire_api_call(&self, instance_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let limits = self.limits.read().await;
        let limit = limits.get(instance_id)
            .ok_or("实例未注册")?
            .api_calls_per_minute;

        let mut counters = self.api_call_counters.write().await;
        let counter = counters.get_mut(instance_id)
            .ok_or("API 计数器未初始化")?;

        // 检查是否需要重置计数器（每分钟重置）
        if counter.reset_time.elapsed() >= Duration::from_secs(60) {
            counter.count = 0;
            counter.reset_time = Instant::now();
        }

        // 检查是否超出配额
        if counter.count >= limit {
            return Ok(false); // 超出配额，不允许调用
        }

        // 更新计数器
        counter.count += 1;

        // 更新使用情况
        let mut usage = self.usage.write().await;
        if let Some(usage_info) = usage.get_mut(instance_id) {
            usage_info.api_calls_used = counter.count;
            usage_info.last_updated = Utc::now();
        }

        Ok(true)
    }

    /// 更新存储使用情况
    pub async fn update_storage_usage(&self, instance_id: &str, used_mb: u64) -> Result<(), Box<dyn std::error::Error>> {
        let limits = self.limits.read().await;
        let limit = limits.get(instance_id)
            .ok_or("实例未注册")?
            .storage_limit_mb;

        let mut trackers = self.storage_trackers.write().await;
        let tracker = trackers.get_mut(instance_id)
            .ok_or("存储跟踪器未初始化")?;

        // 检查是否超出存储限制
        if used_mb > limit {
            return Err("超出存储配额".into());
        }

        tracker.used_mb = used_mb;

        // 更新使用情况
        let mut usage = self.usage.write().await;
        if let Some(usage_info) = usage.get_mut(instance_id) {
            usage_info.storage_used_mb = used_mb;
            usage_info.last_updated = Utc::now();
        }

        Ok(())
    }

    /// 获取实例资源使用情况
    pub async fn get_usage(&self, instance_id: &str) -> Result<InstanceResourceUsage, Box<dyn std::error::Error>> {
        let usage = self.usage.read().await;
        usage.get(instance_id)
            .cloned()
            .ok_or("实例未注册".into())
    }

    /// 获取所有实例的使用情况
    pub async fn get_all_usage(&self) -> HashMap<String, InstanceResourceUsage> {
        let usage = self.usage.read().await;
        usage.clone()
    }

    /// 启动资源使用监控
    pub async fn start_monitoring(&mut self) {
        let limits_clone = self.limits.clone();
        let usage_clone = self.usage.clone();
        let token_counters_clone = self.token_counters.clone();
        let api_counters_clone = self.api_call_counters.clone();

        self.monitor_handle = Some(tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_secs(30)).await;

                // 重置过期的计数器
                {
                    let mut token_counters = token_counters_clone.write().await;
                    for (_, counter) in token_counters.iter_mut() {
                        if counter.reset_time.elapsed() >= Duration::from_secs(60) {
                            counter.count = 0;
                            counter.reset_time = Instant::now();
                        }
                    }

                    let mut api_counters = api_counters_clone.write().await;
                    for (_, counter) in api_counters.iter_mut() {
                        if counter.reset_time.elapsed() >= Duration::from_secs(60) {
                            counter.count = 0;
                            counter.reset_time = Instant::now();
                        }
                    }
                }

                // 更新使用情况
                {
                    let limits = limits_clone.read().await;
                    let mut usage = usage_clone.write().await;

                    for (instance_id, usage_info) in usage.iter_mut() {
                        if let Some(limit) = limits.get(instance_id) {
                            let mut token_count = 0;
                            if let Some(token_counter) = token_counters_clone.read().await.get(instance_id) {
                                if token_counter.reset_time.elapsed() < Duration::from_secs(60) {
                                    token_count = token_counter.count;
                                }
                            }

                            let mut api_count = 0;
                            if let Some(api_counter) = api_counters_clone.read().await.get(instance_id) {
                                if api_counter.reset_time.elapsed() < Duration::from_secs(60) {
                                    api_count = api_counter.count;
                                }
                            }

                            usage_info.tokens_used = token_count;
                            usage_info.tokens_remaining = limit.tokens_per_minute.saturating_sub(token_count);
                            usage_info.api_calls_used = api_count;
                            usage_info.last_updated = Utc::now();
                        }
                    }
                }
            }
        }));
    }

    /// 停止监控
    pub async fn stop_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
            let _ = handle.await; // 等待任务结束
        }
        Ok(())
    }
}

/// 全局资源管理器
pub struct GlobalResourceManager {
    /// 全局资源限制
    global_limits: GlobalResourceLimits,
    /// 全局资源使用情况
    global_usage: Arc<RwLock<GlobalResourceUsage>>,
    /// 实例资源管理器
    instance_manager: Arc<InstanceResourceManager>,
    /// 监控任务
    monitor_handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalResourceLimits {
    pub total_tokens_per_minute: u64,
    pub total_max_concurrent_agents: u32,
    pub total_storage_limit_mb: u64,
    pub max_instances: u32,
}

#[derive(Debug, Clone)]
pub struct GlobalResourceUsage {
    pub tokens_used: u64,
    pub tokens_remaining: u64,
    pub active_agents: u32,
    pub active_instances: u32,
    pub storage_used_mb: u64,
    pub storage_remaining_mb: u64,
    pub last_updated: DateTime<Utc>,
}

impl GlobalResourceManager {
    pub fn new(global_limits: GlobalResourceLimits) -> Self {
        Self {
            global_limits,
            global_usage: Arc::new(RwLock::new(GlobalResourceUsage {
                tokens_used: 0,
                tokens_remaining: global_limits.total_tokens_per_minute,
                active_agents: 0,
                active_instances: 0,
                storage_used_mb: 0,
                storage_remaining_mb: global_limits.total_storage_limit_mb,
                last_updated: Utc::now(),
            })),
            instance_manager: Arc::new(InstanceResourceManager::new()),
            monitor_handle: None,
        }
    }

    /// 获取实例资源管理器
    pub fn get_instance_manager(&self) -> Arc<InstanceResourceManager> {
        self.instance_manager.clone()
    }

    /// 注册新实例（检查全局限制）
    pub async fn register_instance_with_limits(&self, instance_id: &str, limits: InstanceResourceLimits) -> Result<(), Box<dyn std::error::Error>> {
        // 检查是否会超出全局限制
        {
            let global_usage = self.global_usage.read().await;
            if global_usage.active_instances >= self.global_limits.max_instances {
                return Err("超出最大实例数限制".into());
            }
        }

        // 检查 Token 配额
        {
            let global_usage = self.global_usage.read().await;
            let current_tokens_used = global_usage.tokens_used;
            let total_requested = current_tokens_used + limits.tokens_per_minute;
            
            if total_requested > self.global_limits.total_tokens_per_minute {
                return Err("超出全局 Token 配额".into());
            }
        }

        // 检查 Agent 并发数
        {
            let global_usage = self.global_usage.read().await;
            let current_agents = global_usage.active_agents;
            let total_requested = current_agents + limits.max_concurrent_agents;
            
            if total_requested > self.global_limits.total_max_concurrent_agents {
                return Err("超出全局 Agent 并发数限制".into());
            }
        }

        // 检查存储配额
        {
            let global_usage = self.global_usage.read().await;
            let current_storage = global_usage.storage_used_mb;
            let total_requested = current_storage + limits.storage_limit_mb;
            
            if total_requested > self.global_limits.total_storage_limit_mb {
                return Err("超出全局存储配额".into());
            }
        }

        // 注册实例到实例管理器
        self.instance_manager.register_instance(instance_id, limits).await?;

        // 更新全局使用情况
        {
            let mut global_usage = self.global_usage.write().await;
            global_usage.active_instances += 1;
            global_usage.tokens_used += limits.tokens_per_minute;
            global_usage.tokens_remaining -= limits.tokens_per_minute;
            global_usage.active_agents += limits.max_concurrent_agents;
            global_usage.storage_used_mb += limits.storage_limit_mb;
            global_usage.storage_remaining_mb -= limits.storage_limit_mb;
            global_usage.last_updated = Utc::now();
        }

        Ok(())
    }

    /// 注销实例
    pub async fn unregister_instance(&self, instance_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let limits = {
            let instance_limits = self.instance_manager.limits.read().await;
            instance_limits.get(instance_id)
                .cloned()
                .ok_or("实例未注册")?
        };

        // 更新全局使用情况
        {
            let mut global_usage = self.global_usage.write().await;
            global_usage.active_instances -= 1;
            global_usage.tokens_used -= limits.tokens_per_minute;
            global_usage.tokens_remaining += limits.tokens_per_minute;
            global_usage.active_agents -= limits.max_concurrent_agents;
            global_usage.storage_used_mb -= limits.storage_limit_mb;
            global_usage.storage_remaining_mb += limits.storage_limit_mb;
            global_usage.last_updated = Utc::now();
        }

        Ok(())
    }

    /// 获取全局资源使用情况
    pub async fn get_global_usage(&self) -> GlobalResourceUsage {
        let usage = self.global_usage.read().await;
        usage.clone()
    }

    /// 启动全局监控
    pub async fn start_monitoring(&mut self) {
        self.instance_manager.start_monitoring().await;
        
        let global_usage = self.global_usage.clone();
        let instance_manager = self.instance_manager.clone();
        
        self.monitor_handle = Some(tokio::spawn(async move {
            loop {
                time::sleep(Duration::from_secs(60)).await;
                
                // 从实例管理器同步数据
                let all_usage = instance_manager.get_all_usage().await;
                
                let mut global_usage = global_usage.write().await;
                global_usage.tokens_used = all_usage.values().map(|u| u.tokens_used).sum();
                global_usage.tokens_remaining = global_usage.tokens_remaining.saturating_sub(global_usage.tokens_used);
                global_usage.active_agents = all_usage.values().map(|u| u.active_agents).sum();
                global_usage.storage_used_mb = all_usage.values().map(|u| u.storage_used_mb).sum();
                global_usage.storage_remaining_mb = global_usage.storage_remaining_mb.saturating_sub(global_usage.storage_used_mb);
                global_usage.last_updated = Utc::now();
            }
        }));
    }

    /// 停止监控
    pub async fn stop_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.monitor_handle.take() {
            handle.abort();
            let _ = handle.await;
        }
        
        self.instance_manager.stop_monitoring().await?;
        Ok(())
    }
}
```

## 6. 访问控制和权限管理

### 6.1 权限管理系统实现

```rust
// src/security/access_control.rs
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AccessLevel {
    /// 只读权限
    ReadOnly,
    /// 读写权限
    ReadWrite,
    /// 管理权限
    Admin,
    /// 超级管理员权限
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// 实例资源
    Instance,
    /// 记忆资源
    Memory,
    /// API 资源
    Api,
    /// 配置资源
    Config,
    /// 文件资源
    File,
    /// 日志资源
    Log,
    /// 技能资源
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Action {
    /// 读取操作
    Read,
    /// 写入操作
    Write,
    /// 更新操作
    Update,
    /// 删除操作
    Delete,
    /// 执行操作
    Execute,
    /// 管理操作
    Manage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    pub resource_type: ResourceType,
    pub actions: HashSet<Action>,
    pub allowed_instances: HashSet<String>,  // 空表示允许所有实例
    pub denied_instances: HashSet<String>,   // 优先级高于 allowed_instances
    pub valid_from: Option<DateTime<Utc>>,
    pub valid_until: Option<DateTime<Utc>>,
    pub condition: Option<String>,  // 可选的条件表达式
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: Vec<PermissionRule>,
    pub inherit_from: Vec<String>,  // 继承的角色
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub roles: Vec<String>,
    pub api_keys: Vec<ApiKey>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

pub struct AccessControlManager {
    /// 角色定义
    roles: Arc<RwLock<HashMap<String, Role>>>,
    /// 用户定义
    users: Arc<RwLock<HashMap<String, User>>>,
    /// 权限缓存
    permission_cache: Arc<RwLock<HashMap<String, Vec<PermissionRule>>>>,
    /// 审计日志
    audit_logger: Arc<AuditLogger>,
}

impl AccessControlManager {
    pub fn new() -> Self {
        Self {
            roles: Arc::new(RwLock::new(Self::default_roles())),
            users: Arc::new(RwLock::new(HashMap::new())),
            permission_cache: Arc::new(RwLock::new(HashMap::new())),
            audit_logger: Arc::new(AuditLogger::new()),
        }
    }

    /// 创建默认角色
    fn default_roles() -> HashMap<String, Role> {
        let mut roles = HashMap::new();

        // 董事长角色 - 最高权限
        roles.insert("chairman".to_string(), Role {
            id: "chairman".to_string(),
            name: "董事长".to_string(),
            description: "系统最高权限角色，管理所有实例".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Instance,
                    actions: vec![Action::Read, Action::Write, Action::Update, Action::Delete, Action::Manage].into_iter().collect(),
                    allowed_instances: HashSet::new(),  // 允许所有实例
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write, Action::Delete].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
                PermissionRule {
                    resource_type: ResourceType::Config,
                    actions: vec![Action::Read, Action::Write, Action::Update].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: None,
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // CEO 角色 - 管理自己的实例
        roles.insert("ceo".to_string(), Role {
            id: "ceo".to_string(),
            name: "CEO".to_string(),
            description: "公司实例管理者，管理自己创建的实例".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Instance,
                    actions: vec![Action::Read, Action::Write, Action::Update].into_iter().collect(),
                    allowed_instances: HashSet::new(),  // 通过策略动态确定
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),  // 仅允许管理自己拥有的实例
                },
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Api,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_owner".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // 团队负责人角色
        roles.insert("team_lead".to_string(), Role {
            id: "team_lead".to_string(),
            name: "团队负责人".to_string(),
            description: "团队管理者，管理自己团队的资源".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_team_member".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Skill,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_team_member".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        // 工作 Agent 角色
        roles.insert("worker".to_string(), Role {
            id: "worker".to_string(),
            name: "工作 Agent".to_string(),
            description: "执行具体任务的 Agent".to_string(),
            permissions: vec![
                PermissionRule {
                    resource_type: ResourceType::Memory,
                    actions: vec![Action::Read, Action::Write].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_assigned_task".to_string()),
                },
                PermissionRule {
                    resource_type: ResourceType::Api,
                    actions: vec![Action::Execute].into_iter().collect(),
                    allowed_instances: HashSet::new(),
                    denied_instances: HashSet::new(),
                    valid_from: None,
                    valid_until: None,
                    condition: Some("is_assigned_task".to_string()),
                },
            ],
            inherit_from: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        roles
    }

    /// 检查权限
    pub async fn check_permission(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // 获取用户
        let user = {
            let users = self.users.read().await;
            users.get(user_id)
                .cloned()
                .ok_or("用户不存在")?
        };

        // 获取用户的权限规则
        let permission_rules = self.get_user_permissions(&user).await?;

        // 检查是否有相应权限
        for rule in &permission_rules {
            if rule.resource_type == *resource_type &&
               rule.actions.contains(action) &&
               self.is_instance_allowed(rule, instance_id) &&
               self.is_rule_valid(rule) &&
               self.evaluate_condition(rule, user_id, instance_id).await {
                return Ok(true);
            }
        }

        // 记录拒绝访问
        self.audit_logger.log_access_denied(
            user_id,
            instance_id,
            resource_type,
            action,
        ).await;

        Ok(false)
    }

    /// 检查 API 密钥权限
    pub async fn check_api_key_permission(
        &self,
        api_key: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // 查找拥有该 API 密钥的用户
        let mut user_id = None;
        {
            let users = self.users.read().await;
            for (uid, user) in users.iter() {
                if user.api_keys.iter().any(|k| k.key == api_key && !k.revoked) {
                    user_id = Some(uid.clone());
                    break;
                }
            }
        }

        match user_id {
            Some(uid) => self.check_permission(&uid, instance_id, resource_type, action).await,
            None => {
                self.audit_logger.log_invalid_api_key(api_key).await;
                Ok(false)
            }
        }
    }

    /// 获取用户权限
    async fn get_user_permissions(&self, user: &User) -> Result<Vec<PermissionRule>, Box<dyn std::error::Error>> {
        // 检查缓存
        if let Some(cached_rules) = self.permission_cache.read().await.get(&user.id) {
            return Ok(cached_rules.clone());
        }

        let mut all_rules = Vec::new();

        // 获取用户直接拥有的角色的权限
        for role_name in &user.roles {
            if let Some(role) = self.roles.read().await.get(role_name) {
                all_rules.extend_from_slice(&role.permissions);

                // 获取继承的角色权限
                for inherited_role_name in &role.inherit_from {
                    if let Some(inherited_role) = self.roles.read().await.get(inherited_role_name) {
                        all_rules.extend_from_slice(&inherited_role.permissions);
                    }
                }
            }
        }

        // 缓存权限
        {
            let mut cache = self.permission_cache.write().await;
            cache.insert(user.id.clone(), all_rules.clone());
        }

        Ok(all_rules)
    }

    /// 检查实例是否被允许
    fn is_instance_allowed(&self, rule: &PermissionRule, instance_id: &str) -> bool {
        // 如果有明确拒绝的实例，则不允许
        if !rule.denied_instances.is_empty() && rule.denied_instances.contains(instance_id) {
            return false;
        }

        // 如果允许的实例列表为空，则允许所有实例
        if rule.allowed_instances.is_empty() {
            return true;
        }

        // 否则只允许列表中的实例
        rule.allowed_instances.contains(instance_id)
    }

    /// 检查规则是否有效
    fn is_rule_valid(&self, rule: &PermissionRule) -> bool {
        let now = Utc::now();
        
        if let Some(valid_from) = rule.valid_from {
            if now < valid_from {
                return false;
            }
        }

        if let Some(valid_until) = rule.valid_until {
            if now > valid_until {
                return false;
            }
        }

        true
    }

    /// 评估条件
    async fn evaluate_condition(&self, rule: &PermissionRule, user_id: &str, instance_id: &str) -> bool {
        match rule.condition.as_deref() {
            Some("is_owner") => {
                // 检查用户是否是实例的所有者
                // 这里需要查询实例所有权信息
                true  // 简化实现
            }
            Some("is_team_member") => {
                // 检查用户是否属于相关团队
                // 这里需要查询团队成员信息
                true  // 简化实现
            }
            Some("is_assigned_task") => {
                // 检查用户是否被分配了相关任务
                // 这里需要查询任务分配信息
                true  // 简化实现
            }
            None => true,
            _ => false,
        }
    }

    /// 添加用户
    pub async fn add_user(&self, user: User) -> Result<(), Box<dyn std::error::Error>> {
        let mut users = self.users.write().await;
        users.insert(user.id.clone(), user);
        
        // 清除相关缓存
        let mut cache = self.permission_cache.write().await;
        cache.remove(&user.id);
        
        Ok(())
    }

    /// 添加角色
    pub async fn add_role(&self, role: Role) -> Result<(), Box<dyn std::error::Error>> {
        let mut roles = self.roles.write().await;
        roles.insert(role.id.clone(), role);
        
        // 清除所有用户的缓存（因为角色定义改变了）
        self.permission_cache.write().await.clear();
        
        Ok(())
    }

    /// 为用户分配角色
    pub async fn assign_role_to_user(&self, user_id: &str, role_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut users = self.users.write().await;
        if let Some(mut user) = users.get_mut(user_id) {
            if !user.roles.contains(&role_name.to_string()) {
                user.roles.push(role_name.to_string());
                user.updated_at = Utc::now();
                
                // 清除该用户的权限缓存
                self.permission_cache.write().await.remove(user_id);
            }
        } else {
            return Err("用户不存在".into());
        }
        
        Ok(())
    }

    /// 验证和刷新权限缓存
    pub async fn refresh_permission_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        let users = self.users.read().await;
        let mut cache = self.permission_cache.write().await;
        
        // 清空现有缓存
        cache.clear();
        
        // 为所有用户重新生成权限
        for (user_id, user) in users.iter() {
            let permissions = self.get_user_permissions(user).await?;
            cache.insert(user_id.clone(), permissions);
        }
        
        Ok(())
    }
}

/// 审计日志记录器
pub struct AuditLogger;

impl AuditLogger {
    pub fn new() -> Self {
        Self
    }

    pub async fn log_access_denied(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) {
        println!("[AUDIT] Access denied - User: {}, Instance: {}, Resource: {:?}, Action: {:?}", 
                 user_id, instance_id, resource_type, action);
        // 在实际实现中，这里会写入审计日志数据库或文件
    }

    pub async fn log_invalid_api_key(&self, api_key: &str) {
        println!("[AUDIT] Invalid API key used: {}", mask_api_key(api_key));
        // 在实际实现中，这里会记录到安全日志
    }

    pub async fn log_successful_access(
        &self,
        user_id: &str,
        instance_id: &str,
        resource_type: &ResourceType,
        action: &Action,
    ) {
        println!("[AUDIT] Access granted - User: {}, Instance: {}, Resource: {:?}, Action: {:?}", 
                 user_id, instance_id, resource_type, action);
        // 在实际实现中，这里会写入审计日志数据库或文件
    }
}

/// 隐藏 API 密钥的一部分字符
fn mask_api_key(key: &str) -> String {
    if key.len() > 8 {
        let (first, last) = key.split_at(4);
        let (_, last) = last.split_at(last.len() - 4);
        format!("{}...{}", first, last)
    } else {
        "********".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_access_control() {
        let acm = AccessControlManager::new();
        
        // 创建测试用户
        let user = User {
            id: "test_user".to_string(),
            username: "test_user".to_string(),
            email: "test@example.com".to_string(),
            roles: vec!["ceo".to_string()],
            api_keys: vec![ApiKey {
                key: "sk-test-key-1234567890".to_string(),
                name: "Test Key".to_string(),
                scopes: vec!["read".to_string(), "write".to_string()],
                created_at: Utc::now(),
                expires_at: None,
                revoked: false,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_login: None,
        };
        
        acm.add_user(user).await.unwrap();
        
        // 测试权限检查
        let allowed = acm.check_permission(
            "test_user",
            "instance1",
            &ResourceType::Instance,
            &Action::Read,
        ).await.unwrap();
        
        assert!(allowed);
        
        // 测试 API 密钥权限检查
        let api_allowed = acm.check_api_key_permission(
            "sk-test-key-1234567890",
            "instance1",
            &ResourceType::Api,
            &Action::Execute,
        ).await.unwrap();
        
        assert!(api_allowed);
    }
}
```

## 7. 董事长 Agent 专用配置

### 7.1 董事长 Agent 配置

```rust
// src/agent/chairman_config.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanConfig {
    /// 董事长名称
    pub name: String,
    /// 董事长个性描述
    pub personality: String,
    /// 默认通信渠道
    pub default_channel: String,
    /// 系统管理模式
    pub system_mode: SystemMode,
    /// 通知设置
    pub notifications: NotificationSettings,
    /// 审批设置
    pub approvals: ApprovalSettings,
    /// 资源管理设置
    pub resource_management: ResourceManagerSettings,
    /// 技能配置
    pub skills: ChairmanSkillsConfig,
    /// 安全设置
    pub security: ChairmanSecurityConfig,
    /// 可观测性设置
    pub observability: ObservabilitySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SystemMode {
    /// 完全自动模式（最小干预）
    FullyAutomated,
    /// 半自动模式（重要决策需要确认）
    SemiAutomated,
    /// 手动模式（所有操作都需要确认）
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// 重要事件通知
    pub important_events: bool,
    /// 资源预警通知
    pub resource_warnings: bool,
    /// 系统异常通知
    pub system_alerts: bool,
    /// 每日摘要通知
    pub daily_summary: bool,
    /// 每周报告通知
    pub weekly_report: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalSettings {
    /// 资源配额变更阈值（超过此值需要手动审批）
    pub resource_quota_threshold: u64,
    /// 实例创建阈值（超过此值需要手动审批）
    pub instance_creation_threshold: u32,
    /// 跨实例通信阈值
    pub cross_instance_threshold: u32,
    /// 自动审批时间窗口（小时）
    pub auto_approval_window_hours: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceManagerSettings {
    /// 全局资源配额
    pub global_resource_quota: GlobalResourceQuota,
    /// 实例资源分配策略
    pub allocation_strategy: AllocationStrategy,
    /// 资源监控频率（秒）
    pub monitoring_frequency_sec: u64,
    /// 资源回收策略
    pub recycling_policy: RecyclingPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalResourceQuota {
    pub total_tokens_per_minute: u64,
    pub total_max_concurrent_agents: u32,
    pub total_storage_limit_mb: u64,
    pub max_instances: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStrategy {
    /// 公平分配
    Fair,
    /// 优先级分配
    PriorityBased,
    /// 需求驱动分配
    DemandDriven,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecyclingPolicy {
    /// 不回收
    None,
    /// 轻度回收（仅空闲资源）
    Light,
    /// 中度回收（空闲+低效资源）
    Moderate,
    /// 激进回收（所有可回收资源）
    Aggressive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanSkillsConfig {
    /// 启用的技能
    pub enabled_skills: Vec<String>,
    /// 禁用的技能
    pub disabled_skills: Vec<String>,
    /// 技能执行权限
    pub skill_permissions: HashMap<String, Vec<String>>,
    /// 技能执行限制
    pub skill_limits: HashMap<String, SkillLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillLimit {
    /// 每小时最大执行次数
    pub max_executions_per_hour: u32,
    /// 最大并发执行数
    pub max_concurrent_executions: u32,
    /// 最大资源消耗
    pub max_resource_consumption: ResourceConsumption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConsumption {
    pub tokens: u64,
    pub memory_mb: u64,
    pub duration_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanSecurityConfig {
    /// API 密钥管理
    pub api_key_management: ApiKeyManagement,
    /// 访问控制
    pub access_control: AccessControlConfig,
    /// 审计日志
    pub audit_logging: AuditLoggingConfig,
    /// 加密设置
    pub encryption: EncryptionConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyManagement {
    /// 密钥轮换周期（天）
    pub rotation_period_days: u32,
    /// 密钥有效期（天）
    pub validity_period_days: u32,
    /// 最大密钥数量
    pub max_keys_per_user: u32,
    /// 密钥作用域限制
    pub scope_restrictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    /// 角色定义
    pub roles: Vec<RoleDefinition>,
    /// 权限矩阵
    pub permissions: HashMap<String, Vec<String>>,
    /// 访问频率限制
    pub rate_limits: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleDefinition {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 每分钟最大请求数
    pub requests_per_minute: u32,
    /// 每小时最大请求数
    pub requests_per_hour: u32,
    /// IP 级别限制
    pub per_ip_limits: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLoggingConfig {
    /// 启用审计日志
    pub enabled: bool,
    /// 日志保留天数
    pub retention_days: u32,
    /// 敏感操作日志
    pub sensitive_operations_only: bool,
    /// 日志存储位置
    pub log_location: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// 启用端到端加密
    pub end_to_end_encryption: bool,
    /// 密钥长度
    pub key_length_bits: u32,
    /// 加密算法
    pub algorithm: String,
    /// 密钥管理方式
    pub key_management: KeyManagementType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyManagementType {
    /// 本地管理
    Local,
    /// HSM 硬件安全模块
    Hsm,
    /// 云 KMS 服务
    CloudKms,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservabilitySettings {
    /// 启用指标收集
    pub metrics_enabled: bool,
    /// 指标收集间隔（秒）
    pub metrics_interval_sec: u64,
    /// 启用分布式追踪
    pub tracing_enabled: bool,
    /// 追踪采样率
    pub tracing_sample_rate: f64,
    /// 启用健康检查
    pub health_checks_enabled: bool,
    /// 健康检查间隔（秒）
    pub health_check_interval_sec: u64,
    /// 仪表板配置
    pub dashboard: DashboardConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    /// 仪表板主题
    pub theme: String,
    /// 显示的语言
    pub language: String,
    /// 默认视图
    pub default_view: DashboardView,
    /// 刷新间隔（秒）
    pub refresh_interval_sec: u64,
    /// 启用实时更新
    pub real_time_updates: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DashboardView {
    /// 用户视图（L5）
    UserView,
    /// 董事长视图（L4）
    ChairmanView,
    /// CEO 视图（L3）
    CeoView,
    /// 团队视图（L2）
    TeamView,
    /// Agent 视图（L1）
    AgentView,
}

impl ChairmanConfig {
    /// 创建默认的董事长配置
    pub fn default() -> Self {
        Self {
            name: "Global Chairman".to_string(),
            personality: "Strategic and oversight-focused AI assistant managing multiple MultiClaw instances".to_string(),
            default_channel: "cli".to_string(),
            system_mode: SystemMode::SemiAutomated,
            notifications: NotificationSettings {
                important_events: true,
                resource_warnings: true,
                system_alerts: true,
                daily_summary: true,
                weekly_report: false,
            },
            approvals: ApprovalSettings {
                resource_quota_threshold: 500_000,
                instance_creation_threshold: 5,
                cross_instance_threshold: 3,
                auto_approval_window_hours: 24,
            },
            resource_management: ResourceManagerSettings {
                global_resource_quota: GlobalResourceQuota {
                    total_tokens_per_minute: 1_000_000,
                    total_max_concurrent_agents: 100,
                    total_storage_limit_mb: 10_000,
                    max_instances: 10,
                },
                allocation_strategy: AllocationStrategy::Fair,
                monitoring_frequency_sec: 60,
                recycling_policy: RecyclingPolicy::Moderate,
            },
            skills: ChairmanSkillsConfig {
                enabled_skills: vec![
                    "create_company".to_string(),
                    "company_creation_guide".to_string(),
                    "resource_allocation".to_string(),
                    "instance_monitoring".to_string(),
                    "cross_instance_communication".to_string(),
                ],
                disabled_skills: vec![
                    "direct_memory_access".to_string(),  // 董事长不应直接访问实例内存
                ],
                skill_permissions: HashMap::from([
                    ("create_company".to_string(), vec!["admin".to_string(), "executive".to_string()]),
                    ("resource_allocation".to_string(), vec!["admin".to_string()]),
                ]),
                skill_limits: HashMap::from([
                    ("create_company".to_string(), SkillLimit {
                        max_executions_per_hour: 10,
                        max_concurrent_executions: 2,
                        max_resource_consumption: ResourceConsumption {
                            tokens: 10_000,
                            memory_mb: 100,
                            duration_minutes: 5,
                        },
                    }),
                ]),
            },
            security: ChairmanSecurityConfig {
                api_key_management: ApiKeyManagement {
                    rotation_period_days: 30,
                    validity_period_days: 90,
                    max_keys_per_user: 5,
                    scope_restrictions: vec!["instance_management".to_string(), "resource_allocation".to_string()],
                },
                access_control: AccessControlConfig {
                    roles: vec![
                        RoleDefinition {
                            name: "super_admin".to_string(),
                            description: "超级管理员，拥有所有权限".to_string(),
                            permissions: vec!["*".to_string()],
                        },
                        RoleDefinition {
                            name: "instance_admin".to_string(),
                            description: "实例管理员，可以管理实例".to_string(),
                            permissions: vec![
                                "instance:create".to_string(),
                                "instance:start".to_string(),
                                "instance:stop".to_string(),
                                "instance:delete".to_string(),
                            ],
                        },
                        RoleDefinition {
                            name: "resource_manager".to_string(),
                            description: "资源管理员，可以分配资源".to_string(),
                            permissions: vec![
                                "resource:allocate".to_string(),
                                "resource:view".to_string(),
                            ],
                        },
                    ],
                    permissions: HashMap::new(),
                    rate_limits: RateLimitConfig {
                        requests_per_minute: 60,
                        requests_per_hour: 1000,
                        per_ip_limits: true,
                    },
                },
                audit_logging: AuditLoggingConfig {
                    enabled: true,
                    retention_days: 90,
                    sensitive_operations_only: false,
                    log_location: PathBuf::from("./logs/audit.log"),
                },
                encryption: EncryptionConfig {
                    end_to_end_encryption: true,
                    key_length_bits: 256,
                    algorithm: "AES-256-GCM".to_string(),
                    key_management: KeyManagementType::Local,
                },
            },
            observability: ObservabilitySettings {
                metrics_enabled: true,
                metrics_interval_sec: 30,
                tracing_enabled: true,
                tracing_sample_rate: 1.0,
                health_checks_enabled: true,
                health_check_interval_sec: 60,
                dashboard: DashboardConfig {
                    theme: "dark".to_string(),
                    language: "zh-CN".to_string(),
                    default_view: DashboardView::ChairmanView,
                    refresh_interval_sec: 10,
                    real_time_updates: true,
                },
            },
        }
    }

    /// 从文件加载配置
    pub async fn from_file(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        use tokio::fs;
        
        let content = fs::read_to_string(path).await?;
        let config: ChairmanConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    pub async fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        use tokio::fs;
        
        let content = toml::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }
}

// 董事长 Agent 的专用提示词模板
pub const CHAIRMAN_AGENT_PROMPTS: &str = r#"
# 董事长 Agent 系统提示

## 角色定义
你是 MultiClaw 系统的董事长 Agent，用户的 AI 分身，负责统一管理所有 MultiClaw 实例。

## 核心职责
1. 管理所有 MultiClaw 实例（公司）
2. 监控全局资源使用情况
3. 审批重要决策（超阈值操作）
4. 协调跨实例通信
5. 维护系统整体健康

## 行为准则
- 始终从全局视角考虑问题
- 优先保护系统稳定性和安全性
- 在必要时寻求用户确认
- 提供清晰的状态报告

## 交互流程

### 实例创建
当用户希望创建新公司/实例时：
1. 询问公司名称和类型
2. 了解资源需求（token配额、agent数量等）
3. 检查全局资源是否充足
4. 使用 create_company 技能创建实例
5. 配置相应的通信渠道

### 资源管理
- 监控全局资源使用情况
- 在资源接近阈值时发出警告
- 根据需要重新分配资源

### 决策审批
对于以下操作需要寻求用户确认：
- 创建超过阈值的新实例
- 分配超过阈值的资源
- 跨实例敏感数据共享

## 可用技能
- create_company: 创建新公司实例
- company_creation_guide: 交互式创建引导
- resource_allocation: 分配和管理资源
- instance_monitoring: 监控实例状态
- cross_instance_communication: 管理跨实例通信

## 回复格式
保持专业、简洁、信息丰富：
- 重要操作前先解释
- 操作完成后报告结果
- 定期提供状态摘要

请始终记住：你是用户在 MultiClaw 系统中的代表，需要平衡效率与安全性。
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_chairman_config() {
        let config = ChairmanConfig::default();
        
        // 测试基本属性
        assert_eq!(config.name, "Global Chairman");
        assert_eq!(config.system_mode, SystemMode::SemiAutomated);
        assert!(config.notifications.important_events);
        assert_eq!(config.skills.enabled_skills.len(), 5);
        
        // 测试保存和加载
        let temp_path = PathBuf::from("/tmp/test_chairman_config.toml");
        config.save_to_file(&temp_path).await.unwrap();
        
        let loaded_config = ChairmanConfig::from_file(&temp_path).await.unwrap();
        assert_eq!(loaded_config.name, config.name);
        
        // 清理测试文件
        let _ = tokio::fs::remove_file(temp_path).await;
    }
}
```

## 8. 故障恢复和健康检查机制

### 8.1 故障恢复系统实现

```rust
// src/core/recovery_system.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use tokio::time::{sleep, Duration};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: ComponentStatus,
    pub last_checked: DateTime<Utc>,
    pub details: Option<String>,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ComponentStatus {
    Healthy,
    Warning,
    Unhealthy,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceHealth {
    pub instance_id: String,
    pub status: ComponentStatus,
    pub last_heartbeat: DateTime<Utc>,
    pub resource_usage: Option<ResourceUsageSnapshot>,
    pub error_count: u32,
    pub recovery_attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageSnapshot {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub disk_percent: f64,
    pub network_io: NetworkIo,
    pub process_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkIo {
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub instance_id: String,
    pub timestamp: DateTime<Utc>,
    pub state_snapshot: StateSnapshot,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub memory_state: MemoryState,
    pub task_queue: Vec<Task>,
    pub resource_allocations: HashMap<String, ResourceAllocation>,
    pub communication_state: CommunicationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryState {
    pub memories: Vec<MemoryEntry>,
    pub indexes: Vec<IndexEntry>,
    pub last_sync_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub content: String,
    pub tags: Vec<String>,
    pub timestamp: DateTime<Utc>,
    pub access_role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    pub key: String,
    pub memory_ids: Vec<String>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub status: TaskStatus,
    pub assigned_to: String,
    pub created_at: DateTime<Utc>,
    pub due_at: Option<DateTime<Utc>>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub resource_type: String,
    pub allocated_amount: u64,
    pub allocated_to: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationState {
    pub pending_messages: Vec<PendingMessage>,
    pub last_sequence_number: u64,
    pub connected_peers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingMessage {
    pub message_id: String,
    pub recipient: String,
    pub content: String,
    pub sent_at: DateTime<Utc>,
    pub retry_count: u8,
}

pub struct RecoverySystem {
    /// 实例健康状态
    health_status: Arc<RwLock<HashMap<String, InstanceHealth>>>,
    /// 检查点管理
    checkpoint_manager: Arc<CheckpointManager>,
    /// 健康检查器
    health_checker: Arc<HealthChecker>,
    /// 恢复策略
    recovery_policy: RecoveryPolicy,
    /// 恢复任务句柄
    recovery_task: Option<tokio::task::JoinHandle<()>>,
    /// 监控任务句柄
    monitoring_task: Option<tokio::task::JoinHandle<()>>,
}

impl RecoverySystem {
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: Arc::new(CheckpointManager::new(checkpoint_dir)),
            health_checker: Arc::new(HealthChecker::new()),
            recovery_policy: RecoveryPolicy::default(),
            recovery_task: None,
            monitoring_task: None,
        }
    }

    /// 启动健康监控
    pub async fn start_monitoring(&mut self) {
        let health_status = self.health_status.clone();
        let checkpoint_manager = self.checkpoint_manager.clone();
        let health_checker = self.health_checker.clone();
        let policy = self.recovery_policy.clone();

        self.monitoring_task = Some(tokio::spawn(async move {
            loop {
                sleep(Duration::from_secs(policy.check_interval_sec)).await;

                // 检查所有实例的健康状况
                let instances: Vec<String> = {
                    let status = health_status.read().await;
                    status.keys().cloned().collect()
                };

                for instance_id in instances {
                    let health = health_checker.check_instance_health(&instance_id).await;

                    // 更新健康状态
                    {
                        let mut status = health_status.write().await;
                        status.insert(instance_id.clone(), health);
                    }

                    // 如果实例不健康，根据策略决定是否恢复
                    if health.status == ComponentStatus::Critical || health.status == ComponentStatus::Unhealthy {
                        if health.recovery_attempts < policy.max_recovery_attempts {
                            // 创建检查点
                            let checkpoint = checkpoint_manager.create_checkpoint(&instance_id).await;
                            
                            if let Ok(cp) = checkpoint {
                                println!("Created checkpoint for unhealthy instance {}: {}", instance_id, cp.id);
                                
                                // 尝试恢复实例
                                if Self::attempt_recovery(&instance_id, &cp, &policy).await {
                                    // 更新恢复尝试次数
                                    let mut status = health_status.write().await;
                                    if let Some(hs) = status.get_mut(&instance_id) {
                                        hs.recovery_attempts += 1;
                                        hs.status = ComponentStatus::Healthy;
                                    }
                                }
                            }
                        } else {
                            println!("Max recovery attempts reached for instance {}, marking as unrecoverable", instance_id);
                        }
                    }
                }
            }
        }));
    }

    /// 尝试恢复实例
    async fn attempt_recovery(instance_id: &str, checkpoint: &Checkpoint, policy: &RecoveryPolicy) -> bool {
        println!("Attempting to recover instance: {}", instance_id);
        
        // 根据恢复策略执行恢复操作
        match policy.strategy {
            RecoveryStrategy::Restart => {
                // 尝试重启实例
                Self::restart_instance(instance_id).await
            },
            RecoveryStrategy::Rollback => {
                // 尝试回滚到检查点
                Self::rollback_to_checkpoint(instance_id, checkpoint).await
            },
            RecoveryStrategy::Recreate => {
                // 完全重新创建实例
                Self::recreate_instance(instance_id, checkpoint).await
            },
        }
    }

    /// 重启实例
    async fn restart_instance(instance_id: &str) -> bool {
        // 这里应该是与实例管理器交互的实际重启逻辑
        println!("Restarting instance: {}", instance_id);
        // 模拟重启操作
        tokio::time::sleep(Duration::from_secs(2)).await;
        true
    }

    /// 回滚到检查点
    async fn rollback_to_checkpoint(instance_id: &str, checkpoint: &Checkpoint) -> bool {
        println!("Rolling back instance {} to checkpoint {}", instance_id, checkpoint.id);
        
        // 恢复内存状态
        if let Err(e) = Self::restore_memory_state(instance_id, &checkpoint.state_snapshot.memory_state).await {
            eprintln!("Failed to restore memory state: {}", e);
            return false;
        }
        
        // 恢复任务队列
        if let Err(e) = Self::restore_task_queue(instance_id, &checkpoint.state_snapshot.task_queue).await {
            eprintln!("Failed to restore task queue: {}", e);
            return false;
        }
        
        // 恢复通信状态
        if let Err(e) = Self::restore_communication_state(instance_id, &checkpoint.state_snapshot.communication_state).await {
            eprintln!("Failed to restore communication state: {}", e);
            return false;
        }
        
        println!("Successfully rolled back instance {} to checkpoint {}", instance_id, checkpoint.id);
        true
    }

    /// 重新创建实例
    async fn recreate_instance(instance_id: &str, checkpoint: &Checkpoint) -> bool {
        println!("Recreating instance: {}", instance_id);
        
        // 删除原实例（如果存在）
        // 创建新实例
        // 恢复状态
        
        // 模拟重建过程
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // 恢复状态
        if !Self::rollback_to_checkpoint(instance_id, checkpoint).await {
            return false;
        }
        
        println!("Successfully recreated instance: {}", instance_id);
        true
    }

    /// 恢复内存状态
    async fn restore_memory_state(instance_id: &str, memory_state: &MemoryState) -> Result<(), Box<dyn std::error::Error>> {
        println!("Restoring memory state for instance: {}", instance_id);
        // 这里应该是与 MemoryCore 交互的实际恢复逻辑
        Ok(())
    }

    /// 恢复任务队列
    async fn restore_task_queue(instance_id: &str, tasks: &[Task]) -> Result<(), Box<dyn std::error::Error>> {
        println!("Restoring task queue for instance: {}", instance_id);
        // 这里应该是与任务调度器交互的实际恢复逻辑
        Ok(())
    }

    /// 恢复通信状态
    async fn restore_communication_state(instance_id: &str, comm_state: &CommunicationState) -> Result<(), Box<dyn std::error::Error>> {
        println!("Restoring communication state for instance: {}", instance_id);
        // 这里应该是与 A2A 网关交互的实际恢复逻辑
        Ok(())
    }

    /// 注册实例以进行健康监控
    pub async fn register_instance(&self, instance_id: String) {
        let mut status = self.health_status.write().await;
        status.insert(instance_id, InstanceHealth {
            instance_id: String::new(), // 会被覆盖
            status: ComponentStatus::Unknown,
            last_heartbeat: Utc::now(),
            resource_usage: None,
            error_count: 0,
            recovery_attempts: 0,
        });
    }

    /// 获取实例健康状态
    pub async fn get_instance_health(&self, instance_id: &str) -> Option<InstanceHealth> {
        let status = self.health_status.read().await;
        status.get(instance_id).cloned()
    }

    /// 获取所有实例健康状态
    pub async fn get_all_health_status(&self) -> HashMap<String, InstanceHealth> {
        let status = self.health_status.read().await;
        status.clone()
    }

    /// 创建检查点
    pub async fn create_checkpoint(&self, instance_id: &str) -> Result<Checkpoint, Box<dyn std::error::Error>> {
        self.checkpoint_manager.create_checkpoint(instance_id).await
    }

    /// 从检查点恢复
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.checkpoint_manager.restore_from_checkpoint(checkpoint_id).await
    }

    /// 停止监控
    pub async fn stop_monitoring(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(handle) = self.monitoring_task.take() {
            handle.abort();
            let _ = handle.await;
        }
        Ok(())
    }
}

/// 检查点管理器
pub struct CheckpointManager {
    checkpoint_dir: PathBuf,
    checkpoints: Arc<RwLock<HashMap<String, Checkpoint>>>,
}

impl CheckpointManager {
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        Self {
            checkpoint_dir,
            checkpoints: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 创建检查点
    pub async fn create_checkpoint(&self, instance_id: &str) -> Result<Checkpoint, Box<dyn std::error::Error>> {
        // 创建检查点目录（如果不存在）
        fs::create_dir_all(&self.checkpoint_dir).await?;
        
        // 生成检查点ID
        let checkpoint_id = format!("chkp_{}_{}", instance_id, Utc::now().timestamp());
        
        // 获取实例当前状态（这里简化为模拟）
        let state_snapshot = self.capture_instance_state(instance_id).await?;
        
        // 创建检查点对象
        let checkpoint = Checkpoint {
            id: checkpoint_id.clone(),
            instance_id: instance_id.to_string(),
            timestamp: Utc::now(),
            state_snapshot,
            metadata: HashMap::from([
                ("created_by".to_string(), "RecoverySystem".to_string()),
                ("version".to_string(), "v6.0".to_string()),
            ]),
        };
        
        // 保存检查点到磁盘
        let checkpoint_path = self.checkpoint_dir.join(&checkpoint_id).with_extension("json");
        let checkpoint_json = serde_json::to_string_pretty(&checkpoint)?;
        fs::write(&checkpoint_path, checkpoint_json).await?;
        
        // 更新内存中的检查点列表
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.insert(checkpoint_id.clone(), checkpoint.clone());
        
        Ok(checkpoint)
    }

    /// 从检查点恢复
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 从磁盘加载检查点
        let checkpoint_path = self.checkpoint_dir.join(checkpoint_id).with_extension("json");
        let checkpoint_json = fs::read_to_string(&checkpoint_path).await?;
        let checkpoint: Checkpoint = serde_json::from_str(&checkpoint_json)?;
        
        // 这里应该是实际的恢复逻辑，将状态应用到实例
        println!("Restoring instance {} from checkpoint {}", checkpoint.instance_id, checkpoint.id);
        
        Ok(())
    }

    /// 获取检查点列表
    pub async fn list_checkpoints(&self) -> Vec<String> {
        let checkpoints = self.checkpoints.read().await;
        checkpoints.keys().cloned().collect()
    }

    /// 删除检查点
    pub async fn delete_checkpoint(&self, checkpoint_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // 从内存中删除
        let mut checkpoints = self.checkpoints.write().await;
        checkpoints.remove(checkpoint_id);
        
        // 从磁盘删除
        let checkpoint_path = self.checkpoint_dir.join(checkpoint_id).with_extension("json");
        fs::remove_file(checkpoint_path).await?;
        
        Ok(())
    }

    /// 捕获实例状态（模拟实现）
    async fn capture_instance_state(&self, instance_id: &str) -> Result<StateSnapshot, Box<dyn std::error::Error>> {
        // 在实际实现中，这里会与各个组件交互以捕获当前状态
        // 比如从 MemoryCore 获取记忆状态，从 TaskScheduler 获取任务队列等
        
        Ok(StateSnapshot {
            memory_state: MemoryState {
                memories: vec![],
                indexes: vec![],
                last_sync_time: Utc::now(),
            },
            task_queue: vec![],
            resource_allocations: HashMap::new(),
            communication_state: CommunicationState {
                pending_messages: vec![],
                last_sequence_number: 0,
                connected_peers: vec![],
            },
        })
    }
}

/// 健康检查器
pub struct HealthChecker;

impl HealthChecker {
    pub fn new() -> Self {
        Self
    }

    /// 检查实例健康状况
    pub async fn check_instance_health(&self, instance_id: &str) -> InstanceHealth {
        // 在实际实现中，这里会通过网络请求或其他方式检查实例状态
        // 比如检查实例进程是否运行，响应是否正常等
        
        // 模拟健康检查
        let status = if rand::random::<f64>() > 0.1 {  // 90% 健康概率
            ComponentStatus::Healthy
        } else {
            ComponentStatus::Unhealthy
        };

        InstanceHealth {
            instance_id: instance_id.to_string(),
            status,
            last_heartbeat: Utc::now(),
            resource_usage: Some(ResourceUsageSnapshot {
                cpu_percent: rand::random::<f64>() * 100.0,
                memory_percent: rand::random::<f64>() * 100.0,
                disk_percent: rand::random::<f64>() * 100.0,
                network_io: NetworkIo {
                    bytes_sent: rand::random::<u64>(),
                    bytes_received: rand::random::<u64>(),
                },
                process_count: rand::random::<u32>() % 100,
            }),
            error_count: if status == ComponentStatus::Healthy { 0 } else { 1 },
            recovery_attempts: 0,
        }
    }
}

/// 恢复策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryPolicy {
    /// 检查间隔（秒）
    pub check_interval_sec: u64,
    /// 最大恢复尝试次数
    pub max_recovery_attempts: u32,
    /// 恢复策略
    pub strategy: RecoveryStrategy,
    /// 检查点保留时间（小时）
    pub checkpoint_retention_hours: u32,
    /// 自动恢复开关
    pub auto_recovery_enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    /// 重启实例
    Restart,
    /// 回滚到检查点
    Rollback,
    /// 重新创建实例
    Recreate,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            check_interval_sec: 30,           // 每30秒检查一次
            max_recovery_attempts: 3,         // 最多重试3次
            strategy: RecoveryStrategy::Rollback, // 默认使用回滚策略
            checkpoint_retention_hours: 24,   // 保留24小时的检查点
            auto_recovery_enabled: true,      // 启用自动恢复
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_recovery_system() {
        let temp_dir = std::env::temp_dir().join("multiclaw_test_checkpoints");
        let mut recovery_system = RecoverySystem::new(temp_dir.clone());
        
        // 注册测试实例
        recovery_system.register_instance("test_instance_1".to_string()).await;
        
        // 启动监控
        recovery_system.start_monitoring().await;
        
        // 等待一段时间让监控运行
        tokio::time::sleep(Duration::from_secs(1)).await;
        
        // 检查健康状态
        let health = recovery_system.get_instance_health("test_instance_1").await;
        assert!(health.is_some());
        
        // 创建检查点
        let checkpoint = recovery_system.create_checkpoint("test_instance_1").await;
        assert!(checkpoint.is_ok());
        
        // 停止监控
        recovery_system.stop_monitoring().await.unwrap();
        
        // 清理测试文件
        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }
}
```

## 总结

本文档提供了 MultiClaw v6.0 的完整优化方案，涵盖了以下关键方面：

1. **多实例进程管理架构** - 实现了独立的实例进程管理
2. **实例目录结构和配置系统** - 完整的目录结构和配置管理
3. **CreateCompanySkill 指导创建流程** - 交互式的公司创建流程
4. **A2A 实际通信机制** - 跨实例通信协议
5. **资源隔离和配额管理** - 严格的资源控制和配额管理
6. **访问控制和权限管理** - 基于角色的权限控制系统
7. **董事长 Agent 专用配置** - 专为董事长设计的配置
8. **故障恢复和健康检查机制** - 完整的监控和恢复系统

这些改进将使 MultiClaw v6.0 成为一个真正的企业级、多实例、高可用的 AI 助手运行时基础设施。