//! 全局实例注册表
//! 提供跨进程的实例状态持久化和端口分配管理

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// 实例状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum RegistryInstanceStatus {
    Running,
    Stopped,
    Crashed,
    Unknown,
}

impl Default for RegistryInstanceStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// 董事长实例信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChairmanInfo {
    /// 实例 ID
    pub instance_id: String,
    /// 监听端口
    pub port: u16,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 进程 ID
    pub pid: Option<u32>,
    /// 状态
    pub status: RegistryInstanceStatus,
    /// 启动时间
    pub started_at: DateTime<Utc>,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
}

/// 公司实例信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyInstanceInfo {
    /// 实例 ID
    pub instance_id: String,
    /// 公司名称
    pub company_name: String,
    /// 公司类型
    pub company_type: String,
    /// 监听端口
    pub port: u16,
    /// 数据目录
    pub data_dir: PathBuf,
    /// 配置文件路径
    pub config_path: PathBuf,
    /// 进程 ID
    pub pid: Option<u32>,
    /// 状态
    pub status: RegistryInstanceStatus,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 启动时间
    pub started_at: DateTime<Utc>,
    /// 最后心跳时间
    pub last_heartbeat: DateTime<Utc>,
    /// CEO 模型
    pub ceo_model: String,
    /// CEO 性格
    pub ceo_personality: String,
    /// Token 配额
    pub token_quota: u32,
    /// 最大 Agent 数
    pub max_agents: u32,
}

impl Default for CompanyInstanceInfo {
    fn default() -> Self {
        Self {
            instance_id: String::new(),
            company_name: String::new(),
            company_type: String::new(),
            port: 0,
            data_dir: PathBuf::new(),
            config_path: PathBuf::new(),
            pid: None,
            status: RegistryInstanceStatus::Unknown,
            created_at: Utc::now(),
            started_at: Utc::now(),
            last_heartbeat: Utc::now(),
            ceo_model: String::new(),
            ceo_personality: String::new(),
            token_quota: 100_000,
            max_agents: 10,
        }
    }
}

/// 实例注册表 - 全局持久化
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstanceRegistry {
    /// 注册表版本
    pub version: u32,
    /// 董事长实例信息
    pub chairman: Option<ChairmanInfo>,
    /// 公司实例列表 (instance_id -> info)
    pub companies: HashMap<String, CompanyInstanceInfo>,
    /// 端口分配表 (port -> instance_id)
    pub port_allocations: HashMap<u16, String>,
    /// 下一个可用端口
    pub next_port: u16,
}

impl InstanceRegistry {
    /// 注册表文件名
    const REGISTRY_FILE: &'static str = "instances.json";
    /// 基础端口
    pub const BASE_PORT: u16 = 8001;
    /// 董事长默认端口
    pub const CHAIRMAN_PORT: u16 = 42617;
    /// 当前版本
    const VERSION: u32 = 1;

    /// 创建新的注册表
    pub fn new() -> Self {
        Self {
            version: Self::VERSION,
            chairman: None,
            companies: HashMap::new(),
            port_allocations: HashMap::new(),
            next_port: Self::BASE_PORT,
        }
    }

    /// 获取注册表文件路径
    pub fn registry_path() -> PathBuf {
        // 使用 directories crate 获取用户主目录
        let home = directories::UserDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        
        home.join(".multiclaw")
            .join(Self::REGISTRY_FILE)
    }

    /// 从文件加载注册表
    pub async fn load() -> Self {
        let path = Self::registry_path();
        
        if path.exists() {
            match tokio::fs::read_to_string(&path).await {
                Ok(content) => {
                    match serde_json::from_str::<Self>(&content) {
                        Ok(registry) => {
                            tracing::info!(
                                "Loaded instance registry: {} companies, next_port={}",
                                registry.companies.len(),
                                registry.next_port
                            );
                            return registry;
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse instance registry: {}", e);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to read instance registry: {}", e);
                }
            }
        }
        
        Self::new()
    }

    /// 保存注册表到文件
    pub async fn save(&self) -> anyhow::Result<()> {
        let path = Self::registry_path();
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        
        let content = serde_json::to_string_pretty(self)?;
        tokio::fs::write(&path, content).await?;
        
        tracing::debug!("Saved instance registry to {}", path.display());
        Ok(())
    }

    /// 分配下一个可用端口
    pub fn allocate_port(&mut self) -> u16 {
        // 跳过已分配的端口
        while self.port_allocations.contains_key(&self.next_port) {
            self.next_port += 1;
        }
        
        let port = self.next_port;
        self.next_port += 1;
        
        tracing::info!("Allocated port {} for new instance", port);
        port
    }

    /// 释放端口
    pub fn release_port(&mut self, port: u16) {
        if let Some(instance_id) = self.port_allocations.remove(&port) {
            tracing::info!("Released port {} from instance {}", port, instance_id);
        }
        
        // 重置下一个可用端口（如果释放的是最小端口）
        if port < self.next_port {
            self.next_port = port;
        }
    }

    /// 注册董事长实例
    pub fn register_chairman(&mut self, info: ChairmanInfo) {
        let port = info.port;
        let id = info.instance_id.clone();
        
        self.port_allocations.insert(port, id.clone());
        self.chairman = Some(info);
        
        tracing::info!("Registered chairman instance {} on port {}", id, port);
    }

    /// 注册公司实例
    pub fn register_company(&mut self, info: CompanyInstanceInfo) {
        let port = info.port;
        let id = info.instance_id.clone();
        
        self.port_allocations.insert(port, id.clone());
        self.companies.insert(id.clone(), info);
        
        tracing::info!("Registered company instance {} ({}) on port {}", 
            id, 
            self.companies.get(&id).map(|c| c.company_name.as_str()).unwrap_or("unknown"),
            port
        );
    }

    /// 注销公司实例
    pub fn unregister_company(&mut self, instance_id: &str) -> Option<CompanyInstanceInfo> {
        if let Some(info) = self.companies.remove(instance_id) {
            self.port_allocations.remove(&info.port);
            tracing::info!("Unregistered company instance {}", instance_id);
            return Some(info);
        }
        None
    }

    /// 更新实例状态
    pub fn update_status(&mut self, instance_id: &str, status: RegistryInstanceStatus) {
        if let Some(info) = self.companies.get_mut(instance_id) {
            info.status = status;
            info.last_heartbeat = Utc::now();
        }
    }

    /// 更新实例 PID
    pub fn update_pid(&mut self, instance_id: &str, pid: Option<u32>) {
        if let Some(info) = self.companies.get_mut(instance_id) {
            info.pid = pid;
        }
    }

    /// 获取公司实例信息
    pub fn get_company(&self, instance_id: &str) -> Option<&CompanyInstanceInfo> {
        self.companies.get(instance_id)
    }

    /// 根据端口获取实例 ID
    pub fn get_instance_by_port(&self, port: u16) -> Option<&str> {
        self.port_allocations.get(&port).map(|s| s.as_str())
    }

    /// 列出所有公司实例
    pub fn list_companies(&self) -> Vec<&CompanyInstanceInfo> {
        self.companies.values().collect()
    }

    /// 获取运行中的实例数量
    pub fn running_count(&self) -> usize {
        self.companies.values()
            .filter(|c| c.status == RegistryInstanceStatus::Running)
            .count()
    }

    /// 检查端口是否已被分配
    pub fn is_port_allocated(&self, port: u16) -> bool {
        self.port_allocations.contains_key(&port)
    }

    /// 清理已停止的实例
    pub fn cleanup_stopped(&mut self) -> Vec<String> {
        let stopped: Vec<String> = self.companies.iter()
            .filter(|(_, c)| c.status == RegistryInstanceStatus::Stopped)
            .map(|(id, _)| id.clone())
            .collect();
        
        for id in &stopped {
            self.unregister_company(id);
        }
        
        if !stopped.is_empty() {
            tracing::info!("Cleaned up {} stopped instances", stopped.len());
        }
        
        stopped
    }
}

/// 全局注册表管理器（线程安全）
pub struct RegistryManager {
    registry: RwLock<InstanceRegistry>,
}

impl RegistryManager {
    /// 创建新的注册表管理器
    pub fn new() -> Self {
        Self {
            registry: RwLock::new(InstanceRegistry::new()),
        }
    }

    /// 从文件加载注册表
    pub async fn load(&self) {
        let mut reg = self.registry.write().await;
        *reg = InstanceRegistry::load().await;
    }

    /// 保存注册表到文件
    pub async fn save(&self) -> anyhow::Result<()> {
        let reg = self.registry.read().await;
        reg.save().await
    }

    /// 分配端口
    pub async fn allocate_port(&self) -> u16 {
        let mut reg = self.registry.write().await;
        let port = reg.allocate_port();
        let _ = reg.save().await;
        port
    }

    /// 注册董事长实例
    pub async fn register_chairman(&self, info: ChairmanInfo) {
        let mut reg = self.registry.write().await;
        reg.register_chairman(info);
        let _ = reg.save().await;
    }

    /// 注册公司实例
    pub async fn register_company(&self, info: CompanyInstanceInfo) {
        let mut reg = self.registry.write().await;
        reg.register_company(info);
        let _ = reg.save().await;
    }

    /// 注销公司实例
    pub async fn unregister_company(&self, instance_id: &str) -> Option<CompanyInstanceInfo> {
        let mut reg = self.registry.write().await;
        let result = reg.unregister_company(instance_id);
        let _ = reg.save().await;
        result
    }

    /// 更新实例状态
    pub async fn update_status(&self, instance_id: &str, status: RegistryInstanceStatus) {
        let mut reg = self.registry.write().await;
        reg.update_status(instance_id, status);
        let _ = reg.save().await;
    }

    /// 更新实例 PID
    pub async fn update_pid(&self, instance_id: &str, pid: Option<u32>) {
        let mut reg = self.registry.write().await;
        reg.update_pid(instance_id, pid);
        let _ = reg.save().await;
    }

    /// 获取公司实例信息
    pub async fn get_company(&self, instance_id: &str) -> Option<CompanyInstanceInfo> {
        let reg = self.registry.read().await;
        reg.get_company(instance_id).cloned()
    }

    /// 根据端口获取实例 ID
    pub async fn get_instance_by_port(&self, port: u16) -> Option<String> {
        let reg = self.registry.read().await;
        reg.get_instance_by_port(port).map(|s| s.to_string())
    }

    /// 列出所有公司实例
    pub async fn list_companies(&self) -> Vec<CompanyInstanceInfo> {
        let reg = self.registry.read().await;
        reg.list_companies().into_iter().cloned().collect()
    }

    /// 获取运行中的实例数量
    pub async fn running_count(&self) -> usize {
        let reg = self.registry.read().await;
        reg.running_count()
    }

    /// 检查端口是否已被分配
    pub async fn is_port_allocated(&self, port: u16) -> bool {
        let reg = self.registry.read().await;
        reg.is_port_allocated(port)
    }

    /// 清理已停止的实例
    pub async fn cleanup_stopped(&self) -> Vec<String> {
        let mut reg = self.registry.write().await;
        let result = reg.cleanup_stopped();
        let _ = reg.save().await;
        result
    }

    /// 获取注册表快照
    pub async fn snapshot(&self) -> InstanceRegistry {
        let reg = self.registry.read().await;
        reg.clone()
    }
}

impl Default for RegistryManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_registry_port_allocation() {
        let mut registry = InstanceRegistry::new();
        
        // 分配端口
        let port1 = registry.allocate_port();
        let port2 = registry.allocate_port();
        let port3 = registry.allocate_port();
        
        assert_eq!(port1, 8001);
        assert_eq!(port2, 8002);
        assert_eq!(port3, 8003);
        
        // 端口应该被标记为已分配
        assert!(registry.is_port_allocated(8001));
        assert!(registry.is_port_allocated(8002));
        assert!(registry.is_port_allocated(8003));
    }

    #[tokio::test]
    async fn test_registry_company_registration() {
        let mut registry = InstanceRegistry::new();
        
        let info = CompanyInstanceInfo {
            instance_id: "test-123".to_string(),
            company_name: "Test Company".to_string(),
            company_type: "MarketResearch".to_string(),
            port: 8001,
            ..Default::default()
        };
        
        registry.register_company(info);
        
        assert!(registry.get_company("test-123").is_some());
        assert!(registry.is_port_allocated(8001));
        assert_eq!(registry.get_instance_by_port(8001), Some("test-123"));
    }

    #[tokio::test]
    async fn test_registry_manager() {
        let manager = RegistryManager::new();
        
        let info = CompanyInstanceInfo {
            instance_id: "test-456".to_string(),
            company_name: "Test Company 2".to_string(),
            company_type: "ProductDevelopment".to_string(),
            port: 8005,
            ..Default::default()
        };
        
        manager.register_company(info).await;
        
        let retrieved = manager.get_company("test-456").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().company_name, "Test Company 2");
    }
}