pub mod error;
pub mod extra_providers;
mod get_openai_agent;
mod get_openrouter_model_list;
mod json_utils;
pub mod rand_agent;
pub mod simple_rand_builder;
#[cfg(feature = "rig-extra-tools")]
pub mod tools;

pub use get_openrouter_model_list::*;

/// 导出 backon 实现失败重试
pub use backon::*;
pub use reqwest::Client as HttpClient;
pub use rig::*;

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: i32,
    /// 提供者
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// 失败次数
    pub failure_count: u32,
    /// 最大失败次数
    pub max_failures: u32,
}
