//!
//! RandAgent - 多代理随机选择器
//!
//! 该模块提供了一个 `RandAgent` 结构体，可以包装多个 AI 代理，
//! 每次调用时随机选择一个代理来执行任务。
//!
//! ## 特性
//!
//! - 支持任意数量的 AI 代理
//! - 每次调用时随机选择一个有效代理
//! - 自动记录代理失败次数，连续失败达到阈值后标记为无效
//! - 成功响应时自动重置失败计数
//! - 线程安全的随机数生成
//! - 提供构建器模式
//! - 支持失败统计和重置功能
//!
//! ## 使用示例
//!
//! ```rust
//! use rig_extra::extra_providers::{bigmodel::Client};
//! use rig::client::ProviderClient;
//! use rig::client::completion::CompletionClientDyn;
//! use rig_extra::rand_agent::RandAgentBuilder;
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // 创建多个客户端
//!     
//! let client1 = Client::from_env();
//!     let client2 = Client::from_env();
//!
//!     // 创建 agent
//!     let agent1 = client1.agent("glm-4-flash").build();
//!     let agent2 = client2.agent("glm-4-flash").build();
//!
//!     // 使用构建器创建 RandAgent，设置最大失败次数
//!     let mut rand_agent = RandAgentBuilder::new()
//!         .max_failures(3) // 连续失败3次后标记为无效
//!         .add_agent(agent1, "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .add_agent(agent2, "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .build();
//!
//!     // 发送消息，会随机选择一个有效代理
//!     let response = rand_agent.prompt("Hello!").await?;
//!     println!("Response: {}", response);
//!
//!     // 查看失败统计
//!     let stats = rand_agent.failure_stats();
//!     println!("Failure stats: {:?}", stats);
//!
//!     Ok(())
//! }
//! ```

use rand::Rng;
use rig::agent::{Agent};
use rig::client::builder::BoxAgent;
use rig::completion::Prompt;
use rig::client::completion::CompletionModelHandle;


/// Agent状态，包含agent实例和失败计数
pub struct AgentState<'a> {
    agent: BoxAgent<'a>,
    provider: String,
    model: String,
    failure_count: u32,
    max_failures: u32,
}

impl<'a> AgentState<'a> {
    fn new(agent: BoxAgent<'a>, provider: String, model: String, max_failures: u32) -> Self {
        Self {
            agent,
            provider,
            model,
            failure_count: 0,
            max_failures,
        }
    }

    fn is_valid(&self) -> bool {
        self.failure_count < self.max_failures
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
    }

    fn record_success(&mut self) {
        self.failure_count = 0;
    }
}

/// A wrapper struct that holds multiple agents and randomly selects one for each invocation
pub struct RandAgent<'a> {
    agents: Vec<AgentState<'a>>,
}

impl<'a> RandAgent<'a> {
    /// Create a new RandAgent with the given agents
    pub fn new(agents: Vec<(BoxAgent<'a>, String, String)>) -> Self {
        Self::with_max_failures(agents, 3) // 默认最大失败次数为3
    }

    /// Create a new RandAgent with custom max failure count
    pub fn with_max_failures(agents: Vec<(BoxAgent<'a>, String, String)>, max_failures: u32) -> Self {
        let agent_states = agents
            .into_iter()
            .map(|(agent, provider, model)| AgentState::new(agent, provider, model, max_failures))
            .collect();
        Self {
            agents: agent_states,
        }
    }

    
    /// Add an agent to the collection
    pub fn add_agent(&mut self, agent: BoxAgent<'a>, provider: String, model: String) {
        self.agents.push(AgentState::new(agent, provider, model, 3)); // 使用默认最大失败次数
    }

    /// Add an agent to the collection with custom max failure count
    pub fn add_agent_with_max_failures(&mut self, agent: BoxAgent<'a>, provider: String, model: String, max_failures: u32) {
        self.agents.push(AgentState::new(agent, provider, model, max_failures));
    }

    /// Get the number of valid agents
    pub fn len(&self) -> usize {
        self.agents.iter().filter(|state| state.is_valid()).count()
    }

    /// Get the total number of agents (including invalid ones)
    pub fn total_len(&self) -> usize {
        self.agents.len()
    }

    /// Check if there are any valid agents
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get a random valid agent from the collection
    async fn get_random_valid_agent(&mut self) -> Option<&mut AgentState<'a>> {
        let valid_indices: Vec<usize> = self
            .agents
            .iter()
            .enumerate()
            .filter(|(_, state)| state.is_valid())
            .map(|(i, _)| i)
            .collect();

        if valid_indices.is_empty() {
            return None;
        }

        let mut rng = rand::rng();
        let random_index = rng.random_range(0..valid_indices.len());
        let agent_index = valid_indices[random_index];
        self.agents.get_mut(agent_index)
    }

    /// Prompt a random valid agent with the given message
    pub async fn prompt(
        &mut self,
        message: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let agent_state = self
            .get_random_valid_agent()
            .await
            .ok_or("No valid agents available")?;

        // 打印使用的provider和model
        tracing::info!("Using provider: {}, model: {}", agent_state.provider, agent_state.model);
        match agent_state.agent.prompt(message).await {
            Ok(response) => {
                agent_state.record_success();
                Ok(response)
            }
            Err(e) => {
                agent_state.record_failure();
                Err(e.into())
            }
        }
    }

    
    /// Get all agents (for debugging or inspection)
    pub fn agents(&self) -> &[AgentState<'a>] {
        &self.agents
    }

    /// Get failure statistics
    pub fn failure_stats(&self) -> Vec<(usize, u32, u32)> {
        self.agents
            .iter()
            .enumerate()
            .map(|(i, state)| (i, state.failure_count, state.max_failures))
            .collect()
    }

    /// Reset failure counts for all agents
    pub fn reset_failures(&mut self) {
        for state in &mut self.agents {
            state.failure_count = 0;
        }
    }
}

// Note: RandAgent cannot implement Clone because BoxAgent<'a> may not implement Clone
// If you need to clone a RandAgent, you'll need to rebuild it from the original agents

/// Builder for creating RandAgent instances
pub struct RandAgentBuilder<'a> {
    agents: Vec<(BoxAgent<'a>, String, String)>,
    max_failures: u32,
}

impl<'a> RandAgentBuilder<'a> {
    /// Create a new RandAgentBuilder
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            max_failures: 3, // 默认最大失败次数
        }
    }

    /// Set the maximum number of consecutive failures before marking an agent as invalid
    pub fn max_failures(mut self, max_failures: u32) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// Add an agent to the builder
    ///
    /// # 参数
    /// - agent: 代理实例
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_agent(mut self, agent: BoxAgent<'a>, provider_name: String, model_name: String) -> Self {
        self.agents.push((agent, provider_name, model_name));
        self
    }

    /// Add an agent from an AgentBuilder
    ///
    /// # 参数
    /// - builder: AgentBuilder 实例
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    ///
    /// 推荐优先使用 add_agent，add_builder 适用于直接传 AgentBuilder 的场景。
    pub fn add_builder(mut self, builder: Agent<CompletionModelHandle<'a>>, provider_name: &str, model_name: &str) -> Self {
        self.agents.push((builder, provider_name.to_string(), model_name.to_string()));
        self
    }

    /// Build the RandAgent
    pub fn build(self) -> RandAgent<'a> {
        RandAgent::with_max_failures(self.agents, self.max_failures)
    }
}

impl<'a> Default for RandAgentBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}