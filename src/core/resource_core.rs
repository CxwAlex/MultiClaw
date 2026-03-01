//! ResourceCore - 资源管理核心模块
//! 负责管理 MultiClaw 实例的计算、存储、网络等资源

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// 资源类型枚举
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// 计算资源 - CPU 时间片
    Compute,
    /// 内存资源 - RAM
    Memory,
    /// 存储资源 - 磁盘空间
    Storage,
    /// 网络带宽资源
    NetworkBandwidth,
    /// API 调用次数
    ApiCalls,
    /// Token 使用量
    Tokens,
    /// 并发 Agent 数量
    ConcurrentAgents,
    /// 自定义资源类型
    Custom(String),
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Compute
    }
}

/// 资源配额结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// 资源类型
    pub resource_type: ResourceType,
    /// 限制值
    pub limit: u64,
    /// 重置周期（秒）
    pub reset_period_seconds: u64,
    /// 描述信息
    pub description: String,
}

/// 资源使用情况
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceUsage {
    /// 资源类型
    pub resource_type: ResourceType,
    /// 已使用量
    pub used: u64,
    /// 限制值
    pub limit: u64,
    /// 余量
    pub remaining: u64,
    /// 使用百分比 (0-100)
    pub usage_percentage: u8,
    /// 最后更新时间
    pub last_updated: DateTime<Utc>,
}

/// 资源请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequest {
    /// 请求唯一标识
    pub id: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 请求量
    pub amount: u64,
    /// 优先级 (0-100)
    pub priority: u8,
    /// 请求方标识
    pub requester_id: String,
    /// 请求时间
    pub request_time: DateTime<Utc>,
    /// 超时时间
    pub timeout: Option<DateTime<Utc>>,
}

/// 资源分配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationResult {
    /// 请求 ID
    pub request_id: String,
    /// 是否分配成功
    pub success: bool,
    /// 分配的量
    pub allocated_amount: u64,
    /// 拒绝原因 (如果失败)
    pub rejection_reason: Option<String>,
    /// 分配时间
    pub allocation_time: DateTime<Utc>,
}

/// 资源策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcePolicy {
    /// 策略名称
    pub name: String,
    /// 资源类型
    pub resource_type: ResourceType,
    /// 分配策略类型
    pub policy_type: AllocationPolicyType,
    /// 优先级权重
    pub priority_weights: HashMap<u8, f64>,
    /// 是否允许超额订阅
    pub allow_overcommit: bool,
    /// 预留比例 (0.0-1.0)
    pub reserved_ratio: f64,
}

/// 分配策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AllocationPolicyType {
    /// 先到先得
    FirstComeFirstServed,
    /// 基于优先级
    PriorityBased,
    /// 公平共享
    FairShare,
    /// 预留优先
    ReservationFirst,
}

/// 资源配额配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfiguration {
    /// 用户/实例 ID
    pub entity_id: String,
    /// 配置的配额
    pub quotas: Vec<ResourceQuota>,
    /// 生效时间
    pub effective_from: DateTime<Utc>,
    /// 过期时间
    pub effective_until: Option<DateTime<Utc>>,
}

/// ResourceCore - 资源管理核心
pub struct ResourceCore {
    /// 各种资源的总量
    total_resources: DashMap<ResourceType, AtomicUsize>,
    /// 各种资源的已用量
    used_resources: DashMap<ResourceType, AtomicUsize>,
    /// 资源配额配置
    quotas: DashMap<String, Vec<ResourceQuota>>, // entity_id -> quotas
    /// 当前活动的资源请求
    active_requests: DashMap<String, ResourceRequest>,
    /// 资源策略
    policies: DashMap<ResourceType, ResourcePolicy>,
    /// 资源使用历史
    usage_history: DashMap<ResourceType, Vec<ResourceUsage>>,
    /// 最大历史记录数
    max_history_size: usize,
    /// 全局锁，防止资源竞争
    resource_lock: Arc<RwLock<()>>,
    /// 预留资源
    reserved_resources: DashMap<ResourceType, AtomicUsize>,
}

impl ResourceCore {
    /// 创建新的 ResourceCore 实例
    pub fn new() -> Self {
        Self {
            total_resources: DashMap::new(),
            used_resources: DashMap::new(),
            quotas: DashMap::new(),
            active_requests: DashMap::new(),
            policies: DashMap::new(),
            usage_history: DashMap::new(),
            max_history_size: 1000,
            resource_lock: Arc::new(RwLock::new(())),
            reserved_resources: DashMap::new(),
        }
    }

    /// 设置资源总量
    pub fn set_total_resource(&self, resource_type: ResourceType, total: u64) {
        let rt = resource_type.clone();
        self.total_resources
            .insert(rt.clone(), AtomicUsize::new(total as usize));
        self.used_resources
            .insert(rt.clone(), AtomicUsize::new(0));
        self.usage_history.insert(rt, Vec::new());
    }

    /// 配置实体的资源配额
    pub async fn configure_quota(&self, config: QuotaConfiguration) -> Result<(), Box<dyn std::error::Error>> {
        // 检查配额是否超出总量
        for quota in &config.quotas {
            if let Some(total_resource) = self.total_resources.get(&quota.resource_type) {
                if quota.limit > total_resource.load(Ordering::Relaxed) as u64 {
                    return Err(format!(
                        "Quota limit {} exceeds total available {} for resource {:?}",
                        quota.limit,
                        total_resource.load(Ordering::Relaxed),
                        quota.resource_type
                    ).into());
                }
            }
        }

        self.quotas.insert(config.entity_id, config.quotas);
        Ok(())
    }

    /// 检查是否有足够的资源
    pub fn has_sufficient_resources(&self, resource_type: ResourceType, amount: u64) -> bool {
        if let (Some(total), Some(used), Some(reserved)) = (
            self.total_resources.get(&resource_type),
            self.used_resources.get(&resource_type),
            self.reserved_resources.get(&resource_type),
        ) {
            let total_val = total.load(Ordering::Relaxed) as u64;
            let used_val = used.load(Ordering::Relaxed) as u64;
            let reserved_val = reserved.load(Ordering::Relaxed) as u64;
            
            // 检查是否超过总量
            if used_val + amount > total_val {
                return false;
            }
            
            // 检查是否超过可用量（扣除预留）
            if used_val + amount > total_val - reserved_val {
                return false;
            }
            
            true
        } else {
            false
        }
    }

    /// 申请资源
    pub async fn request_resources(&self, request: ResourceRequest) -> Result<AllocationResult, Box<dyn std::error::Error>> {
        let _lock = self.resource_lock.read().await;

        // 检查请求是否超时
        if let Some(timeout) = request.timeout {
            if Utc::now() > timeout {
                return Ok(AllocationResult {
                    request_id: request.id,
                    success: false,
                    allocated_amount: 0,
                    rejection_reason: Some("Request timed out".to_string()),
                    allocation_time: Utc::now(),
                });
            }
        }

        let resource_type = request.resource_type.clone(); // 保存一份副本
        let requester_id = request.requester_id.clone();   // 保存一份副本
        
        // 获取请求方的配额限制
        let quota_limit = self.get_entity_quota_limit(&requester_id, resource_type.clone());

        // 检查配额限制
        if let Some(limit) = quota_limit {
            let used = self.get_entity_resource_usage(&requester_id, resource_type.clone());
            if used + request.amount > limit {
                return Ok(AllocationResult {
                    request_id: request.id,
                    success: false,
                    allocated_amount: 0,
                    rejection_reason: Some(format!("Quota exceeded: used {} + requested {} > limit {}", used, request.amount, limit)),
                    allocation_time: Utc::now(),
                });
            }
        }

        // 检查系统总资源是否足够
        if !self.has_sufficient_resources(resource_type.clone(), request.amount) {
            return Ok(AllocationResult {
                request_id: request.id,
                success: false,
                allocated_amount: 0,
                rejection_reason: Some("Insufficient system resources".to_string()),
                allocation_time: Utc::now(),
            });
        }

        // 尝分配资源
        if let Some(used_resource) = self.used_resources.get(&resource_type) {
            let old_value = used_resource.fetch_add(request.amount as usize, Ordering::Relaxed);
            let new_value = old_value + request.amount as usize;

            // 更新历史记录
            self.update_usage_history(resource_type, new_value as u64).await;

            // 添加到活动请求
            self.active_requests.insert(request.id.clone(), request.clone());

            Ok(AllocationResult {
                request_id: request.id,
                success: true,
                allocated_amount: request.amount,
                rejection_reason: None,
                allocation_time: Utc::now(),
            })
        } else {
            Ok(AllocationResult {
                request_id: request.id,
                success: false,
                allocated_amount: 0,
                rejection_reason: Some("Resource type not registered".to_string()),
                allocation_time: Utc::now(),
            })
        }
    }

    /// 释放资源
    pub async fn release_resources(&self, requester_id: &str, resource_type: ResourceType, amount: u64) -> Result<(), Box<dyn std::error::Error>> {
        let _lock = self.resource_lock.write().await;

        if let Some(used_resource) = self.used_resources.get(&resource_type) {
            let old_value = used_resource.fetch_sub(amount as usize, Ordering::Relaxed);
            let new_value = if old_value >= amount as usize {
                old_value - amount as usize
            } else {
                0 // 防止下溢
            };

            used_resource.store(new_value, Ordering::Relaxed);

            // 更新历史记录
            self.update_usage_history(resource_type, new_value as u64).await;

            // 移除相关的活动请求
            self.cleanup_requester_requests(requester_id).await;

            Ok(())
        } else {
            Err("Resource type not registered".into())
        }
    }

    /// 更新资源使用历史
    async fn update_usage_history(&self, resource_type: ResourceType, current_usage: u64) {
        if let Some(mut history) = self.usage_history.get_mut(&resource_type) {
            let total = self.total_resources.get(&resource_type)
                .map(|r| r.load(Ordering::Relaxed) as u64)
                .unwrap_or(1); // 遌溉除零错误

            let usage = ResourceUsage {
                resource_type,
                used: current_usage,
                limit: total,
                remaining: total.saturating_sub(current_usage),
                usage_percentage: ((current_usage as f64 / total as f64) * 100.0) as u8,
                last_updated: Utc::now(),
            };

            history.value_mut().push(usage);

            // 限制历史记录大小
            if history.len() > self.max_history_size {
                let len = history.value().len();
            if len > self.max_history_size {
                history.value_mut().drain(0..len - self.max_history_size);
            }
            }
        }
    }

    /// 清理特定请求者的活动请求
    async fn cleanup_requester_requests(&self, requester_id: &str) {
        let ids_to_remove: Vec<String> = self.active_requests
            .iter()
            .filter(|entry| entry.value().requester_id == requester_id)
            .map(|entry| entry.key().clone())
            .collect();

        for id in ids_to_remove {
            self.active_requests.remove(&id);
        }
    }

    /// 获取实体的配额限制
    fn get_entity_quota_limit(&self, entity_id: &str, resource_type: ResourceType) -> Option<u64> {
        if let Some(quotas) = self.quotas.get(entity_id) {
            for quota in quotas.value() {
                if quota.resource_type == resource_type {
                    return Some(quota.limit);
                }
            }
        }
        None
    }

    /// 获取实体的资源使用量
    fn get_entity_resource_usage(&self, _entity_id: &str, _resource_type: ResourceType) -> u64 {
        // 这里可以实现更复杂的逻辑来跟踪每个实体的资源使用
        // 目前简化处理，返回全局使用量
        self.used_resources
            .get(&_resource_type)
            .map(|r| r.load(Ordering::Relaxed) as u64)
            .unwrap_or(0)
    }

    /// 设置资源预留
    pub fn reserve_resources(&self, resource_type: ResourceType, amount: u64) {
        let entry = self.reserved_resources
            .entry(resource_type)
            .or_insert(AtomicUsize::new(0));
        entry.store(amount as usize, Ordering::Relaxed);
    }

    /// 获取资源使用情况
    pub async fn get_resource_usage(&self, resource_type: ResourceType) -> Option<ResourceUsage> {
        if let (Some(total_res), Some(used_res)) = (
            self.total_resources.get(&resource_type),
            self.used_resources.get(&resource_type),
        ) {
            let total = total_res.load(Ordering::Relaxed) as u64;
            let used = used_res.load(Ordering::Relaxed) as u64;

            Some(ResourceUsage {
                resource_type,
                used,
                limit: total,
                remaining: total.saturating_sub(used),
                usage_percentage: if total > 0 {
                    ((used as f64 / total as f64) * 100.0) as u8
                } else {
                    0
                },
                last_updated: Utc::now(),
            })
        } else {
            None
        }
    }

    /// 获取所有资源的使用情况
    pub async fn get_all_resource_usage(&self) -> Vec<ResourceUsage> {
        let mut usages = Vec::new();

        for entry in self.total_resources.iter() {
            let resource_type = entry.key().clone();
            if let Some(usage) = self.get_resource_usage(resource_type).await {
                usages.push(usage);
            }
        }

        usages
    }

    /// 获取资源使用历史
    pub async fn get_resource_history(&self, resource_type: ResourceType) -> Vec<ResourceUsage> {
        self.usage_history
            .get(&resource_type)
            .map(|history| history.value().clone())
            .unwrap_or_default()
    }

    /// 设置资源分配策略
    pub fn set_allocation_policy(&self, policy: ResourcePolicy) {
        let resource_type = policy.resource_type.clone();
        self.policies.insert(resource_type, policy);
    }

    /// 获取当前活动的资源请求
    pub async fn get_active_requests(&self) -> Vec<ResourceRequest> {
        self.active_requests
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// 获取资源统计信息
    pub async fn get_statistics(&self) -> ResourceStatistics {
        let mut stats = ResourceStatistics::default();

        for entry in self.total_resources.iter() {
            let resource_type = entry.key().clone();
            if let Some(usage) = self.get_resource_usage(resource_type).await {
                stats.add_usage(&usage);
            }
        }

        stats
    }
}

/// 资源统计信息
#[derive(Debug, Clone, Default)]
pub struct ResourceStatistics {
    pub total_compute: u64,
    pub used_compute: u64,
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_storage: u64,
    pub used_storage: u64,
    pub total_network: u64,
    pub used_network: u64,
    pub total_tokens: u64,
    pub used_tokens: u64,
    pub total_agents: u64,
    pub used_agents: u64,
}

impl ResourceStatistics {
    pub fn add_usage(&mut self, usage: &ResourceUsage) {
        match &usage.resource_type {
            ResourceType::Compute => {
                self.total_compute = usage.limit;
                self.used_compute = usage.used;
            }
            ResourceType::Memory => {
                self.total_memory = usage.limit;
                self.used_memory = usage.used;
            }
            ResourceType::Storage => {
                self.total_storage = usage.limit;
                self.used_storage = usage.used;
            }
            ResourceType::NetworkBandwidth => {
                self.total_network = usage.limit;
                self.used_network = usage.used;
            }
            ResourceType::Tokens => {
                self.total_tokens = usage.limit;
                self.used_tokens = usage.used;
            }
            ResourceType::ConcurrentAgents => {
                self.total_agents = usage.limit;
                self.used_agents = usage.used;
            }
            ResourceType::ApiCalls => {
                // API 调用资源统计
            }
            ResourceType::Custom(_) => {
                // 自定义资源类型暂不统计
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_resource_core_basic_operations() {
        let resource_core = ResourceCore::new();

        // 设置资源总量
        resource_core.set_total_resource(ResourceType::Compute, 1000);
        resource_core.set_total_resource(ResourceType::Memory, 8192); // 8GB

        // 配置实体配额
        let mut quotas = Vec::new();
        quotas.push(ResourceQuota {
            resource_type: ResourceType::Compute,
            limit: 500,
            reset_period_seconds: 3600,
            description: "Compute quota for test entity".to_string(),
        });

        let config = QuotaConfiguration {
            entity_id: "test_entity".to_string(),
            quotas,
            effective_from: Utc::now(),
            effective_until: None,
        };

        resource_core.configure_quota(config).await.expect("Failed to configure quota");

        // 申请资源
        let request = ResourceRequest {
            id: Uuid::new_v4().to_string(),
            resource_type: ResourceType::Compute,
            amount: 100,
            priority: 50,
            requester_id: "test_entity".to_string(),
            request_time: Utc::now(),
            timeout: Some(Utc::now() + chrono::Duration::seconds(30)),
        };

        let result = resource_core.request_resources(request).await.expect("Failed to request resources");
        assert!(result.success);
        assert_eq!(result.allocated_amount, 100);

        // 检查资源使用情况
        let usage = resource_core.get_resource_usage(ResourceType::Compute).await.unwrap();
        assert_eq!(usage.used, 100);
        assert_eq!(usage.remaining, 900);

        // 再次申请，应该成功
        let request2 = ResourceRequest {
            id: Uuid::new_v4().to_string(),
            resource_type: ResourceType::Compute,
            amount: 300,
            priority: 50,
            requester_id: "test_entity".to_string(),
            request_time: Utc::now(),
            timeout: Some(Utc::now() + chrono::Duration::seconds(30)),
        };

        let result2 = resource_core.request_resources(request2).await.expect("Failed to request resources");
        assert!(result2.success);
        assert_eq!(result2.allocated_amount, 300);

        // 检查资源使用情况
        let usage2 = resource_core.get_resource_usage(ResourceType::Compute).await.unwrap();
        assert_eq!(usage2.used, 400); // 100 + 300

        // 尝试申请超出配额的资源，应该失败
        let request3 = ResourceRequest {
            id: Uuid::new_v4().to_string(),
            resource_type: ResourceType::Compute,
            amount: 200, // 总共将使用 600，超过配额 500
            priority: 50,
            requester_id: "test_entity".to_string(),
            request_time: Utc::now(),
            timeout: Some(Utc::now() + chrono::Duration::seconds(30)),
        };

        let result3 = resource_core.request_resources(request3).await.expect("Failed to request resources");
        assert!(!result3.success);
        assert!(result3.rejection_reason.is_some());

        // 释放资源
        resource_core.release_resources("test_entity", ResourceType::Compute, 300).await.expect("Failed to release resources");

        // 检查资源使用情况
        let usage3 = resource_core.get_resource_usage(ResourceType::Compute).await.unwrap();
        assert_eq!(usage3.used, 100); // 400 - 300
    }

    #[tokio::test]
    async fn test_resource_core_statistics() {
        let resource_core = ResourceCore::new();

        // 设置资源总量
        resource_core.set_total_resource(ResourceType::Compute, 1000);
        resource_core.set_total_resource(ResourceType::Memory, 8192);

        // 申请一些资源
        let request = ResourceRequest {
            id: Uuid::new_v4().to_string(),
            resource_type: ResourceType::Compute,
            amount: 200,
            priority: 50,
            requester_id: "test_entity".to_string(),
            request_time: Utc::now(),
            timeout: Some(Utc::now() + chrono::Duration::seconds(30)),
        };

        resource_core.request_resources(request).await.expect("Failed to request resources");

        let stats = resource_core.get_statistics().await;
        assert_eq!(stats.total_compute, 1000);
        assert_eq!(stats.used_compute, 200);
    }
}