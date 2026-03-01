//! HealthCore - 健康检查核心模块
//! 负责监控 MultiClaw 系统各组件的健康状况

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 健康状态枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// 健康 - 所有功能正常
    Healthy,
    /// 警告 - 部分功能受限
    Warning,
    /// 不健康 - 功能严重受损
    Unhealthy,
    /// 未知 - 状态未确定
    Unknown,
    /// 维护中 - 系统正在维护
    Maintenance,
}

/// 健康检查类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HealthCheckType {
    /// 系统资源检查
    SystemResources,
    /// 内存使用情况
    MemoryUsage,
    /// CPU 使用情况
    CpuUsage,
    /// 磁盘空间
    DiskSpace,
    /// 网络连接
    NetworkConnectivity,
    /// 数据库连接
    DatabaseConnection,
    /// 外部 API 连接
    ExternalApiConnection,
    /// Agent 连接状态
    AgentConnection,
    /// 服务可用性
    ServiceAvailability,
    /// 自定义检查
    Custom,
}

/// 健康检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// 检查 ID
    pub id: String,
    /// 检查类型
    pub check_type: HealthCheckType,
    /// 健康状态
    pub status: HealthStatus,
    /// 消息描述
    pub message: String,
    /// 详细信息
    pub details: HashMap<String, String>,
    /// 检查时间
    pub timestamp: DateTime<Utc>,
    /// 延迟毫秒数
    pub latency_ms: Option<u128>,
}

/// 健康指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// CPU 使用率 (%)
    pub cpu_usage_percent: f64,
    /// 内存使用率 (%)
    pub memory_usage_percent: f64,
    /// 可用内存 (MB)
    pub available_memory_mb: u64,
    /// 磁盘使用率 (%)
    pub disk_usage_percent: f64,
    /// 可用磁盘 (GB)
    pub available_disk_gb: f64,
    /// 网络延迟 (ms)
    pub network_latency_ms: f64,
    /// 活跃 Agent 数量
    pub active_agents: usize,
    /// 挂起任务数量
    pub pending_tasks: usize,
    /// 错误计数
    pub error_count: usize,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 健康检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// 检查类型
    pub check_type: HealthCheckType,
    /// 检查间隔 (秒)
    pub interval_seconds: u64,
    /// 超时时间 (秒)
    pub timeout_seconds: u64,
    /// 重试次数
    pub retry_count: u8,
    /// 阈值配置
    pub thresholds: HashMap<String, f64>,
    /// 是否启用
    pub enabled: bool,
}

/// 健康检查处理器 trait
#[async_trait::async_trait]
pub trait HealthCheckHandler: Send + Sync {
    /// 执行健康检查
    async fn check_health(&self) -> HealthCheckResult;
    /// 获取处理器名称
    fn name(&self) -> &str;
}

/// HealthCore - 健康检查核心
pub struct HealthCore {
    /// 健康检查结果存储
    check_results: DashMap<HealthCheckType, HealthCheckResult>,
    /// 健康指标
    metrics: Arc<RwLock<HealthMetrics>>,
    /// 健康检查配置
    configurations: DashMap<HealthCheckType, HealthCheckConfig>,
    /// 健康检查处理器
    handlers: DashMap<HealthCheckType, Arc<dyn HealthCheckHandler>>,
    /// 健康状态
    overall_status: Arc<AtomicBool>,
    /// 组件健康状态
    component_status: DashMap<String, HealthStatus>,
    /// 检查计数器
    check_counter: DashMap<HealthCheckType, AtomicUsize>,
    /// 是否正在运行健康检查
    is_running: Arc<AtomicBool>,
    /// 任务取消令牌
    cancel_token: Arc<AtomicBool>,
}

impl HealthCore {
    /// 创建新的 HealthCore 实例
    pub fn new() -> Self {
        Self {
            check_results: DashMap::new(),
            metrics: Arc::new(RwLock::new(HealthMetrics::default())),
            configurations: DashMap::new(),
            handlers: DashMap::new(),
            overall_status: Arc::new(AtomicBool::new(false)),
            component_status: DashMap::new(),
            check_counter: DashMap::new(),
            is_running: Arc::new(AtomicBool::new(false)),
            cancel_token: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 注册健康检查处理器
    pub fn register_handler(&self, check_type: HealthCheckType, handler: Arc<dyn HealthCheckHandler>) {
        self.handlers.insert(check_type, handler);
        
        // 设置默认配置
        let default_config = HealthCheckConfig {
            check_type,
            interval_seconds: 30, // 默认每30秒检查一次
            timeout_seconds: 10,  // 默认10秒超时
            retry_count: 2,       // 默认重试2次
            thresholds: HashMap::new(),
            enabled: true,
        };
        
        self.configurations.insert(check_type, default_config);
    }

    /// 配置健康检查
    pub fn configure_check(&self, config: HealthCheckConfig) {
        self.configurations.insert(config.check_type, config);
    }

    /// 执行单个健康检查
    pub async fn perform_check(&self, check_type: HealthCheckType) -> Result<HealthCheckResult, Box<dyn std::error::Error>> {
        if let Some(handler) = self.handlers.get(&check_type) {
            let start_time = std::time::Instant::now();
            let result = handler.check_health().await;
            let latency = start_time.elapsed().as_millis();
            
            // 更新结果
            let mut result = result;
            result.latency_ms = Some(latency);
            result.timestamp = Utc::now();
            
            self.check_results.insert(check_type, result.clone());
            
            // 更新计数器
            let counter = self.check_counter
                .entry(check_type)
                .or_insert(AtomicUsize::new(0));
            counter.fetch_add(1, Ordering::Relaxed);
            
            Ok(result)
        } else {
            Err(format!("No handler registered for check type {:?}", check_type).into())
        }
    }

    /// 执行所有注册的健康检查
    pub async fn perform_all_checks(&self) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();
        
        for entry in self.handlers.iter() {
            let check_type = *entry.key();
            
            // 检查配置是否启用
            if let Some(config) = self.configurations.get(&check_type) {
                if !config.enabled {
                    continue;
                }
            }
            
            if let Ok(result) = self.perform_check(check_type).await {
                results.push(result);
            }
        }
        
        results
    }

    /// 获取特定类型检查的最新结果
    pub fn get_latest_check_result(&self, check_type: HealthCheckType) -> Option<HealthCheckResult> {
        self.check_results.get(&check_type).map(|r| r.value().clone())
    }

    /// 获取所有检查的最新结果
    pub fn get_all_check_results(&self) -> Vec<HealthCheckResult> {
        self.check_results
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 获取健康指标
    pub async fn get_metrics(&self) -> HealthMetrics {
        self.metrics.read().await.clone()
    }

    /// 更新健康指标
    pub async fn update_metrics(&self, metrics: HealthMetrics) {
        let mut metrics_guard = self.metrics.write().await;
        *metrics_guard = metrics;
    }

    /// 获取组件健康状态
    pub fn get_component_status(&self, component_name: &str) -> Option<HealthStatus> {
        self.component_status.get(component_name).map(|s| *s.value())
    }

    /// 设置组件健康状态
    pub fn set_component_status(&self, component_name: String, status: HealthStatus) {
        self.component_status.insert(component_name, status);
        
        // 更新整体状态
        self.update_overall_status();
    }

    /// 更新整体健康状态
    fn update_overall_status(&self) {
        let mut overall_healthy = true;
        
        for entry in self.component_status.iter() {
            match entry.value() {
                HealthStatus::Unhealthy | HealthStatus::Unknown => {
                    overall_healthy = false;
                    break;
                }
                HealthStatus::Warning => {
                    // 警告状态，整体仍视为健康但需要注意
                }
                _ => {}
            }
        }
        
        self.overall_status.store(overall_healthy, Ordering::Relaxed);
    }

    /// 获取整体健康状态
    pub fn get_overall_status(&self) -> HealthStatus {
        let has_any_issues = self.component_status.iter()
            .any(|entry| matches!(entry.value(), HealthStatus::Unhealthy | HealthStatus::Unknown));
        
        if has_any_issues {
            HealthStatus::Unhealthy
        } else {
            let has_warnings = self.component_status.iter()
                .any(|entry| matches!(entry.value(), HealthStatus::Warning));
            
            if has_warnings {
                HealthStatus::Warning
            } else {
                HealthStatus::Healthy
            }
        }
    }

    /// 启动定期健康检查
    pub async fn start_periodic_checks(&self) {
        if self.is_running.load(Ordering::Relaxed) {
            return;
        }

        self.is_running.store(true, Ordering::Relaxed);
        self.cancel_token.store(false, Ordering::Relaxed);

        let cancel_token = self.cancel_token.clone();
        let check_results = self.check_results.clone();
        let configurations = self.configurations.clone();
        let handlers = self.handlers.clone();
        let metrics = self.metrics.clone();
        let component_status = self.component_status.clone();

        tokio::spawn(async move {
            loop {
                if cancel_token.load(Ordering::Relaxed) {
                    break;
                }

                // 执行所有检查
                for entry in configurations.iter() {
                    let config = entry.value();
                    if !config.enabled {
                        continue;
                    }

                    // 检查是否到达下次检查时间
                    if let Some(last_result) = check_results.get(&config.check_type) {
                        let elapsed = (Utc::now() - last_result.timestamp).num_seconds();
                        if elapsed < config.interval_seconds as i64 {
                            continue;
                        }
                    }

                    // 执行检查
                    if let Some(handler) = handlers.get(&config.check_type) {
                        let start_time = std::time::Instant::now();
                        let result = handler.check_health().await;
                        let latency = start_time.elapsed().as_millis();

                        let mut result = result;
                        result.latency_ms = Some(latency);
                        result.timestamp = Utc::now();

                        check_results.insert(config.check_type, result.clone());

                        // 更新组件状态
                        component_status.insert(handler.name().to_string(), result.status);
                    }

                    // 短暂休眠，避免过于频繁的检查
                    sleep(Duration::from_millis(100)).await;
                }

                // 更新指标
                if let Ok(mut metrics_guard) = metrics.try_write() {
                    metrics_guard.last_updated = Utc::now();
                }

                // 每次轮询后休眠
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    /// 停止定期健康检查
    pub fn stop_periodic_checks(&self) {
        self.cancel_token.store(true, Ordering::Relaxed);
        self.is_running.store(false, Ordering::Relaxed);
    }

    /// 获取检查统计信息
    pub fn get_check_statistics(&self) -> HashMap<HealthCheckType, usize> {
        let mut stats = HashMap::new();
        
        for entry in self.check_counter.iter() {
            let check_type = *entry.key();
            let count = entry.value().load(Ordering::Relaxed);
            stats.insert(check_type, count);
        }
        
        stats
    }

    /// 重置检查计数器
    pub fn reset_check_counters(&self) {
        for entry in self.check_counter.iter() {
            entry.value().store(0, Ordering::Relaxed);
        }
    }

    /// 获取健康报告
    pub async fn get_health_report(&self) -> HealthReport {
        let results = self.get_all_check_results();
        let metrics = self.get_metrics().await;
        let overall_status = self.get_overall_status();
        let statistics = self.get_check_statistics();

        HealthReport {
            timestamp: Utc::now(),
            overall_status,
            check_results: results,
            metrics,
            statistics,
        }
    }
}

/// 健康报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// 报告生成时间
    pub timestamp: DateTime<Utc>,
    /// 整体健康状态
    pub overall_status: HealthStatus,
    /// 各项检查结果
    pub check_results: Vec<HealthCheckResult>,
    /// 健康指标
    pub metrics: HealthMetrics,
    /// 检查统计信息
    pub statistics: HashMap<HealthCheckType, usize>,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
            available_memory_mb: 0,
            disk_usage_percent: 0.0,
            available_disk_gb: 0.0,
            network_latency_ms: 0.0,
            active_agents: 0,
            pending_tasks: 0,
            error_count: 0,
            last_updated: Utc::now(),
        }
    }
}

// 实现内置的健康检查处理器

/// 系统资源健康检查处理器
pub struct SystemResourcesCheck {
    name: String,
}

impl SystemResourcesCheck {
    pub fn new() -> Self {
        Self {
            name: "SystemResourcesCheck".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheckHandler for SystemResourcesCheck {
    async fn check_health(&self) -> HealthCheckResult {
        // 这里应该实现真正的系统资源检查逻辑
        // 为了示例，我们模拟检查结果
        HealthCheckResult {
            id: Uuid::new_v4().to_string(),
            check_type: HealthCheckType::SystemResources,
            status: HealthStatus::Healthy,
            message: "System resources are within acceptable limits".to_string(),
            details: HashMap::from([
                ("cpu_usage".to_string(), "45%".to_string()),
                ("memory_usage".to_string(), "60%".to_string()),
                ("disk_usage".to_string(), "30%".to_string()),
            ]),
            timestamp: Utc::now(),
            latency_ms: Some(5),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Agent 连接健康检查处理器
pub struct AgentConnectionCheck {
    name: String,
}

impl AgentConnectionCheck {
    pub fn new() -> Self {
        Self {
            name: "AgentConnectionCheck".to_string(),
        }
    }
}

#[async_trait::async_trait]
impl HealthCheckHandler for AgentConnectionCheck {
    async fn check_health(&self) -> HealthCheckResult {
        // 这里应该实现真正的 Agent 连接检查逻辑
        HealthCheckResult {
            id: Uuid::new_v4().to_string(),
            check_type: HealthCheckType::AgentConnection,
            status: HealthStatus::Healthy,
            message: "All agents are connected and responsive".to_string(),
            details: HashMap::from([
                ("connected_agents".to_string(), "15".to_string()),
                ("disconnected_agents".to_string(), "0".to_string()),
                ("avg_response_time_ms".to_string(), "120".to_string()),
            ]),
            timestamp: Utc::now(),
            latency_ms: Some(10),
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_health_core_basic_operations() {
        let health_core = HealthCore::new();

        // 注册处理器
        health_core.register_handler(HealthCheckType::SystemResources, Arc::new(SystemResourcesCheck::new()));
        health_core.register_handler(HealthCheckType::AgentConnection, Arc::new(AgentConnectionCheck::new()));

        // 执行单个检查
        let result = health_core.perform_check(HealthCheckType::SystemResources).await;
        assert!(result.is_ok());

        let result = result.unwrap();
        assert_eq!(result.check_type, HealthCheckType::SystemResources);
        assert_eq!(result.status, HealthStatus::Healthy);

        // 执行所有检查
        let results = health_core.perform_all_checks().await;
        assert!(results.len() >= 1);

        // 设置组件状态
        health_core.set_component_status("system_resources".to_string(), HealthStatus::Healthy);
        health_core.set_component_status("agent_connection".to_string(), HealthStatus::Warning);

        // 获取整体状态（应该因为警告而返回警告）
        let overall_status = health_core.get_overall_status();
        assert_eq!(overall_status, HealthStatus::Warning);

        // 获取最新检查结果
        let latest_result = health_core.get_latest_check_result(HealthCheckType::SystemResources);
        assert!(latest_result.is_some());

        // 获取健康指标
        let metrics = health_core.get_metrics().await;
        assert_eq!(metrics.active_agents, 0);

        // 更新健康指标
        let new_metrics = HealthMetrics {
            cpu_usage_percent: 45.0,
            memory_usage_percent: 60.0,
            available_memory_mb: 2048,
            disk_usage_percent: 30.0,
            available_disk_gb: 500.0,
            network_latency_ms: 20.0,
            active_agents: 10,
            pending_tasks: 5,
            error_count: 2,
            last_updated: Utc::now(),
        };
        
        health_core.update_metrics(new_metrics).await;

        // 获取更新后的指标
        let updated_metrics = health_core.get_metrics().await;
        assert_eq!(updated_metrics.cpu_usage_percent, 45.0);
        assert_eq!(updated_metrics.active_agents, 10);

        // 获取检查统计
        let stats = health_core.get_check_statistics();
        assert!(stats.contains_key(&HealthCheckType::SystemResources));

        // 获取健康报告
        let report = health_core.get_health_report().await;
        assert_eq!(report.check_results.len(), 2); // 我们注册了两个检查
    }

    #[tokio::test]
    async fn test_health_core_with_failure() {
        let health_core = HealthCore::new();

        // 创建一个总是返回不健康状态的处理器
        struct FailingCheck;
        #[async_trait::async_trait]
        impl HealthCheckHandler for FailingCheck {
            async fn check_health(&self) -> HealthCheckResult {
                HealthCheckResult {
                    id: Uuid::new_v4().to_string(),
                    check_type: HealthCheckType::Custom,
                    status: HealthStatus::Unhealthy,
                    message: "This check always fails".to_string(),
                    details: HashMap::new(),
                    timestamp: Utc::now(),
                    latency_ms: Some(5),
                }
            }

            fn name(&self) -> &str {
                "FailingCheck"
            }
        }

        health_core.register_handler(HealthCheckType::Custom, Arc::new(FailingCheck));

        // 设置组件状态为不健康
        health_core.set_component_status("failing_component".to_string(), HealthStatus::Unhealthy);

        // 获取整体状态（应该为不健康）
        let overall_status = health_core.get_overall_status();
        assert_eq!(overall_status, HealthStatus::Unhealthy);

        // 执行检查
        let result = health_core.perform_check(HealthCheckType::Custom).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, HealthStatus::Unhealthy);
    }
}