//! 实例管理模块
//! 提供多实例进程管理、配置管理和生命周期管理功能

pub mod manager;
pub mod config;
pub mod registry;
pub mod service;

pub use manager::*;
pub use config::*;
pub use registry::*;
pub use service::*;

// 导出必要的类型以供外部使用
pub use manager::{InstanceManager, InstanceConfig, InstanceType, ResourceQuota, CEOConfig, ChannelConfig, InstanceStatus, CreateInstanceRequest};
pub use config::{ConfigManager, GlobalConfig, InstancePaths};
pub use registry::{InstanceRegistry, RegistryManager, ChairmanInfo, CompanyInstanceInfo, RegistryInstanceStatus};
pub use service::{InstanceService, list_services};