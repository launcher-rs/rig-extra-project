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
//! use rig_extra::error::RandAgentError;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), RandAgentError> {
//!     use rig::completion::Prompt;
//! // 创建多个客户端
//!     
//! use rig_extra::error::RandAgentError;
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
//!         .add_agent(agent1,1, "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .add_agent(agent2, 2,"bigmodel".to_string(), "glm-4-flash".to_string())
//!         .build();
//!
//!     // 发送消息，会随机选择一个有效代理
//!     let response = rand_agent.prompt("Hello!").await?;
//!     println!("Response: {}", response);
//!
//!     // 查看失败统计
//!     let stats = rand_agent.failure_stats().await;
//!     println!("Failure stats: {:?}", stats);
//!
//!     Ok(())
//! }
//! ```

use rand::Rng;
use rig::agent::{Agent};
use rig::client::builder::BoxAgent;
use rig::completion::{Message, Prompt, PromptError};
use rig::client::completion::CompletionModelHandle;
use tokio::sync::Mutex;


/// Agent状态，包含agent实例和失败计数
pub struct AgentState<'a> {
    id: i32,
    agent: BoxAgent<'a>,
    provider: String,
    model: String,
    failure_count: u32,
    max_failures: u32,
}

impl<'a> AgentState<'a> {
    fn new(agent: BoxAgent<'a>,id:i32, provider: String, model: String, max_failures: u32) -> Self {
        Self {
            id,
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

/// 代理失效回调类型，减少类型复杂度
pub type OnRandAgentInvalidCallback = Option<Box<dyn Fn(i32) + Send + Sync + 'static>>;

/// 包装多个代理的结构体，每次调用时随机选择一个代理
pub struct RandAgent<'a> {
    agents: Mutex<Vec<AgentState<'a>>>,
    on_agent_invalid: OnRandAgentInvalidCallback,
}

impl Prompt for RandAgent<'_> {
    #[allow(refining_impl_trait)]
    async fn prompt(&self, prompt: impl Into<Message> + Send) -> Result<String, PromptError> {
        let mut agents = self.agents.lock().await;
        let agent_state = Self::get_random_valid_agent(&mut agents)
            .await
            .ok_or(PromptError::MaxDepthError {
                max_depth: 0,
                chat_history: vec![],
                prompt: "没有有效agent".into(),
            })?;

        tracing::info!("Using provider: {}, model: {}", agent_state.provider, agent_state.model);
        match agent_state.agent.prompt(prompt).await {
            Ok(content) => {
                agent_state.record_success();
                Ok(content)
            }
            Err(e) => {
                agent_state.record_failure();
                if !agent_state.is_valid() {
                    if let Some(cb) = &self.on_agent_invalid {
                        cb(agent_state.id);
                    }
                }
                Err(e)
            }
        }
    }
}

impl<'a> RandAgent<'a> {
    /// 使用给定的代理创建新的 RandAgent
    pub fn new(agents: Vec<(BoxAgent<'a>, i32, String, String)>) -> Self {
        Self::with_max_failures_and_callback(agents, 3, None)
    }

    /// 使用自定义最大失败次数和回调创建新的 RandAgent
    pub fn with_max_failures_and_callback(
        agents: Vec<(BoxAgent<'a>, i32, String, String)>,
        max_failures: u32,
        on_agent_invalid: OnRandAgentInvalidCallback,
    ) -> Self {
        let agent_states = agents
            .into_iter()
            .map(|(agent, id, provider, model)| AgentState::new(agent, id, provider, model, max_failures))
            .collect();
        Self {
            agents: Mutex::new(agent_states),
            on_agent_invalid,
        }
    }

    /// 使用自定义最大失败次数创建新的 RandAgent
    pub fn with_max_failures(agents: Vec<(BoxAgent<'a>, i32,String, String)>, max_failures: u32) -> Self {
        Self::with_max_failures_and_callback(agents, max_failures, None)
    }

    /// 设置 agent 失效时的回调
    pub fn set_on_agent_invalid<F>(&mut self, callback: F)
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        self.on_agent_invalid = Some(Box::new(callback));
    }

    
    /// 向集合中添加代理
    pub async fn add_agent(&self, agent: BoxAgent<'a>, id: i32, provider: String, model: String) {
        self.agents.lock().await.push(AgentState::new(agent, id, provider, model, 3)); // 使用默认最大失败次数
    }

    /// 使用自定义最大失败次数向集合中添加代理
    pub async fn add_agent_with_max_failures(&self, agent: BoxAgent<'a>, id: i32, provider: String, model: String, max_failures: u32) {
        self.agents.lock().await.push(AgentState::new(agent, id, provider, model, max_failures));
    }

    /// 获取有效代理的数量
    pub async fn len(&self) -> usize {
        self.agents.lock().await.iter().filter(|state| state.is_valid()).count()
    }

    /// 获取代理总数（包括无效的）
    pub async fn total_len(&self) -> usize {
        self.agents.lock().await.len()
    }

    /// 检查是否有有效代理
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// 从集合中获取一个随机有效代理
    async fn get_random_valid_agent<'b>(agents: &'b mut Vec<AgentState<'a>>) -> Option<&'b mut AgentState<'a>> {
        let valid_indices: Vec<usize> = agents
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
        agents.get_mut(agent_index)
    }
    

    /// 获取失败统计信息
    pub async fn failure_stats(&self) -> Vec<(usize, u32, u32)> {
        self.agents
            .lock()
            .await
            .iter()
            .enumerate()
            .map(|(i, state)| (i, state.failure_count, state.max_failures))
            .collect()
    }

    /// 重置所有代理的失败计数
    pub async fn reset_failures(&self) {
        for state in self.agents.lock().await.iter_mut() {
            state.failure_count = 0;
        }
    }
}



/// 用于创建 RandAgent 实例的构建器
pub struct RandAgentBuilder<'a> {
    pub(crate) agents: Vec<(BoxAgent<'a>, i32, String, String)>,
    max_failures: u32,
    on_agent_invalid: Option<Box<dyn Fn(i32) + Send + Sync + 'static>>,
}

impl<'a> RandAgentBuilder<'a> {
    /// 创建新的 RandAgentBuilder
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            max_failures: 3, // 默认最大失败次数
            on_agent_invalid: None,
        }
    }

    /// 设置标记代理为无效前的最大连续失败次数
    pub fn max_failures(mut self, max_failures: u32) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// 设置 agent 失效时的回调
    pub fn on_agent_invalid<F>(mut self, callback: F) -> Self
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        self.on_agent_invalid = Some(Box::new(callback));
        self
    }

    /// 向构建器添加代理
    ///
    /// # 参数
    /// - agent: 代理实例
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_agent(mut self, agent: BoxAgent<'a>, id: i32, provider_name: String, model_name: String) -> Self {
        self.agents.push((agent, id, provider_name, model_name));
        self
    }

    /// 从 AgentBuilder 添加代理
    ///
    /// # 参数
    /// - builder: AgentBuilder 实例
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    ///
    /// 推荐优先使用 add_agent，add_builder 适用于直接传 AgentBuilder 的场景。
    pub fn add_builder(mut self, builder: Agent<CompletionModelHandle<'a>>, id: i32, provider_name: &str, model_name: &str) -> Self {
        self.agents.push((builder, id, provider_name.to_string(), model_name.to_string()));
        self
    }

    /// 构建 RandAgent
    pub fn build(self) -> RandAgent<'a> {
        RandAgent::with_max_failures_and_callback(self.agents, self.max_failures, self.on_agent_invalid)
    }
}

impl<'a> Default for RandAgentBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}