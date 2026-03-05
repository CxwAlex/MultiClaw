//! A2A (Agent-to-Agent) 通信模块
//! 提供 MultiClaw 中 Agent 间通信的标准协议和网关实现

pub mod experience;
pub mod experience_pool;
pub mod protocol;
pub mod gateway;
pub mod enhanced_gateway;

pub use experience::{
    ExperienceCapsule, ExperienceExtractor, ExecutionTrace, Outcome, StrategyTemplate,
    ExperienceExtractorConfig
};
pub use experience_pool::{ExperiencePool, ExperienceStore, InMemoryExperienceStore, ExperienceQuery};
pub use protocol::{
    A2AMessage, A2AMessageType, MessagePriority, MessageValidator, A2AMessageBuilder
};
pub use gateway::A2AGateway;
pub use enhanced_gateway::{EnhancedA2AGateway, A2AEndpoint};