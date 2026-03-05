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
        if !config_path.exists() {
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