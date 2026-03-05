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
    monitoring_task: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
}

impl RecoverySystem {
    pub fn new(checkpoint_dir: PathBuf) -> Self {
        Self {
            health_status: Arc::new(RwLock::new(HashMap::new())),
            checkpoint_manager: Arc::new(CheckpointManager::new(checkpoint_dir)),
            health_checker: Arc::new(HealthChecker::new()),
            recovery_policy: RecoveryPolicy::default(),
            recovery_task: None,
            monitoring_task: Arc::new(RwLock::new(None)),
        }
    }

    /// 启动健康监控
    pub async fn start_monitoring(&self) {
        let health_status = self.health_status.clone();
        let checkpoint_manager = self.checkpoint_manager.clone();
        let health_checker = self.health_checker.clone();
        let policy = self.recovery_policy.clone();
        let monitoring_task = self.monitoring_task.clone();

        let handle = tokio::spawn(async move {
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
                        status.insert(instance_id.clone(), health.clone());
                    }

                    // 创建健康状态的副本用于后续使用
                    let health_status_val = health.status;
                    let health_recovery_attempts_val = health.recovery_attempts;
                    let health_instance_id_val = instance_id.clone();

                    // 如果实例不健康，根据策略决定是否恢复
                    if health_status_val == ComponentStatus::Critical || health_status_val == ComponentStatus::Unhealthy {
                        if health_recovery_attempts_val < policy.max_recovery_attempts {
                            // 创建检查点 - 使用一个安全的包装函数
                            if let Some(cp) = Self::safe_create_checkpoint(&checkpoint_manager, &instance_id).await {
                                println!("Created checkpoint for unhealthy instance {}: {}", instance_id, cp.id);

                                // 尝试恢复实例
                                if Self::attempt_recovery(&health_instance_id_val, &cp, &policy).await {
                                    // 更新恢复尝试次数
                                    let mut status = health_status.write().await;
                                    if let Some(hs) = status.get_mut(&health_instance_id_val) {
                                        hs.recovery_attempts += 1;
                                        hs.status = ComponentStatus::Healthy;
                                    }
                                }
                            } else {
                                eprintln!("Failed to create checkpoint for instance {}", instance_id);
                            }
                        } else {
                            println!("Max recovery attempts reached for instance {}, marking as unrecoverable", instance_id);
                        }
                    }
                }
            }
        });

        // 将句柄存储到 RwLock 中
        let mut task_guard = monitoring_task.write().await;
        *task_guard = Some(handle);
    }

    /// 安全地创建检查点，处理错误而不暴露非 Send 类型
    async fn safe_create_checkpoint(checkpoint_manager: &CheckpointManager, instance_id: &str) -> Option<Checkpoint> {
        match checkpoint_manager.create_checkpoint(instance_id).await {
            Ok(cp) => Some(cp),
            Err(e) => {
                eprintln!("Error creating checkpoint for {}: {}", instance_id, e);
                None
            }
        }
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
        self.checkpoint_manager.create_checkpoint(instance_id).await.map_err(|e| e.into())
    }

    /// 从检查点恢复
    pub async fn restore_from_checkpoint(&self, checkpoint_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.checkpoint_manager.restore_from_checkpoint(checkpoint_id).await.map_err(|e| e.into())
    }

    /// 停止监控
    pub async fn stop_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        let handle = {
            let mut task_guard = self.monitoring_task.write().await;
            task_guard.take()
        };
        
        if let Some(handle) = handle {
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