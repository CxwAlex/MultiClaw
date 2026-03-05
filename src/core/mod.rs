//! Core - 核心功能模块
//! 包含 MultiClaw 的核心组件：记忆、资源、健康检查、故障恢复等

pub mod memory_core;
pub mod resource_core;
pub mod health_core;
pub mod recovery_core;
pub mod checkpoint_mgr;
pub mod resource_isolation;
pub mod recovery_system;

pub use memory_core::{MemoryCore, MemoryLevel, MemoryEntry, MemoryQuery, MemorySearchResult, AccessRole, AccessPermissions};
pub use resource_core::{ResourceCore, ResourceType, ResourceQuota, ResourceUsage, ResourceRequest, AllocationResult};
pub use health_core::{HealthCore, HealthStatus, HealthCheckType, HealthCheckResult, HealthMetrics};
pub use recovery_core::{RecoveryCore, RecoveryStatus, RecoveryConfig, FailureType, RecoveryPlan};
pub use checkpoint_mgr::{CheckpointManager, CheckpointConfig, TaskCheckpoint, CheckpointStatus};
pub use resource_isolation::{InstanceResourceManager, GlobalResourceManager, InstanceResourceLimits, InstanceResourceUsage};
pub use recovery_system::{RecoverySystem, HealthStatus as RecoveryHealthStatus, ComponentStatus, InstanceHealth, Checkpoint, StateSnapshot, RecoveryPolicy, RecoveryStrategy};