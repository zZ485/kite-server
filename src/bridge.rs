use chrono::{DateTime, Local};

pub use host::AgentManager;
pub use model::*;
pub use protocol::{ErrorResponse, RequestFrame, RequestPayload, ResponsePayload, ResponseResult};

mod host;
mod model;
mod protocol;

pub type Result<T> = anyhow::Result<T>;

#[derive(Debug, ToPrimitive, thiserror::Error)]
#[error("代理节点错误: {}")]
/// Business error of web socket host
pub enum HostError {
    #[error("无可用的代理节点, 无法连接到校园网")]
    NoAgentAvailable = 120,
    #[error("Agent 节点请求超时或异常, 请重试")]
    Timeout = 121,
    #[error("Agent 节点丢失, 请重试")]
    Disconnected = 122,
    #[error("Agent 端响应不匹配")]
    Mismatched = 123,
}

/// Agent state
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentStatus {
    pub seq: u16,
    /// Agent name
    pub name: String,
    /// Intranet network address
    pub intranet_addr: String,
    /// External network address
    pub external_addr: String,
    /// Processed requests' count
    pub requests: u32,
    /// Last use.
    pub last_use: DateTime<Local>,
}
