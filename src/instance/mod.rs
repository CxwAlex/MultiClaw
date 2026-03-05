//! 实例管理模块
//! 提供多实例进程管理、配置管理和生命周期管理功能

pub mod manager;
pub mod config;

pub use manager::*;
pub use config::*;

// 导出必要的类型以供外部使用
pub use manager::{InstanceManager, InstanceConfig, InstanceType, ResourceQuota, CEOConfig, ChannelConfig, InstanceStatus, CreateInstanceRequest};
pub use config::{ConfigManager, GlobalConfig, InstancePaths};