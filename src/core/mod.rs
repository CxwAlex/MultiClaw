//! Core - 核心功能模块
//! 包含 MultiClaw 的核心组件：记忆、资源、健康检查等

pub mod memory_core;
pub mod resource_core;
pub mod health_core;

pub use memory_core::{MemoryCore, MemoryLevel, MemoryEntry, MemoryQuery, MemorySearchResult, AccessRole, AccessPermissions};
pub use resource_core::{ResourceCore, ResourceType, ResourceQuota, ResourceUsage, ResourceRequest, AllocationResult};
pub use health_core::{HealthCore, HealthStatus, HealthCheckType, HealthCheckResult, HealthMetrics};