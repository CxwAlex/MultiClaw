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
    monitor_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
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
            monitor_handle: Arc::new(RwLock::new(None)),
        }
    }

    /// 注册新实例的资源限制
    pub async fn register_instance(&self, instance_id: &str, limits: InstanceResourceLimits) -> Result<(), Box<dyn std::error::Error>> {
        let limits_clone = limits.clone();
        let mut limits_map = self.limits.write().await;
        limits_map.insert(instance_id.to_string(), limits_clone);

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
    pub async fn acquire_agent_permit(&self, instance_id: &str) -> Result<tokio::sync::OwnedSemaphorePermit, Box<dyn std::error::Error>> {
        let semaphore = {
            let semaphores = self.agent_semaphores.read().await;
            semaphores.get(instance_id)
                .ok_or("信号量未初始化")?
                .clone()
        };

        let permit = semaphore.acquire_owned().await
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
    pub async fn start_monitoring(&self) {
        let limits_clone = self.limits.clone();
        let usage_clone = self.usage.clone();
        let token_counters_clone = self.token_counters.clone();
        let api_counters_clone = self.api_call_counters.clone();
        let monitor_handle = self.monitor_handle.clone();

        let handle = tokio::spawn(async move {
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
        });

        // 将句柄存储到 RwLock 中
        let mut handle_guard = monitor_handle.write().await;
        *handle_guard = Some(handle);
    }

    /// 停止监控
    pub async fn stop_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        let handle_option = {
            let mut handle_guard = self.monitor_handle.write().await;
            handle_guard.take()
        };
        
        if let Some(handle) = handle_option {
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
    monitor_handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
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
        let tokens_remaining = global_limits.total_tokens_per_minute;
        let storage_remaining_mb = global_limits.total_storage_limit_mb;
        
        Self {
            global_limits,
            global_usage: Arc::new(RwLock::new(GlobalResourceUsage {
                tokens_used: 0,
                tokens_remaining,
                active_agents: 0,
                active_instances: 0,
                storage_used_mb: 0,
                storage_remaining_mb,
                last_updated: Utc::now(),
            })),
            instance_manager: Arc::new(InstanceResourceManager::new()),
            monitor_handle: Arc::new(RwLock::new(None)),
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
        self.instance_manager.register_instance(instance_id, limits.clone()).await?;

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
    pub async fn start_monitoring(&self) {
        self.instance_manager.start_monitoring().await;

        let global_usage = self.global_usage.clone();
        let instance_manager = self.instance_manager.clone();
        let monitor_handle = self.monitor_handle.clone();

        let handle = tokio::spawn(async move {
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
        });

        // 将句柄存储到 RwLock 中
        let mut handle_guard = monitor_handle.write().await;
        *handle_guard = Some(handle);
    }

    /// 停止监控
    pub async fn stop_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        let handle = {
            let mut handle_lock = self.monitor_handle.write().await;
            handle_lock.take()
        };

        if let Some(handle) = handle {
            handle.abort();
            let _ = handle.await;
        }

        self.instance_manager.clone().stop_monitoring().await?;
        Ok(())
    }
}