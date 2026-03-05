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