//! A2A (Agent-to-Agent) 通信模块
//! 提供 MultiClaw 中 Agent 间通信的标准协议和网关实现

pub mod protocol;
pub mod gateway;

pub use protocol::{
    A2AMessage, A2AMessageType, MessagePriority, MessageValidator, A2AMessageBuilder
};
pub use gateway::A2AGateway;