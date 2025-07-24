pub mod extra_providers;
mod json_utils;
pub mod error;
pub mod rand_agent;
pub mod simple_rand_builder;

pub use rig::*;
/// 导出 backon 实现失败重试
pub use backon::*;

#[derive(Debug,Clone)]
pub struct AgentInfo{
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