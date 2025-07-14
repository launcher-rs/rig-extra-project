//! ## 多线程使用示例
//!
//! ```rust
//! use rig_extra::extra_providers::{bigmodel::Client};
//! use rig_extra::thread_safe_rand_agent::ThreadSafeRandAgentBuilder;
//! use std::sync::Arc;
//! use tokio::task;
//! use rig::client::ProviderClient;
//! use rig_extra::error::RandAgentError;
//! #[tokio::main]
//! async fn main() -> Result<(), RandAgentError> {
//!     // 创建线程安全的 RandAgent
//!
//!     //创建多个客户端 
//!     let client1 = Client::from_env();
//!     let client2 = Client::from_env();
//!     use rig::client::completion::CompletionClientDyn;
//!     use rig::completion::Prompt;
//!
//!
//!     let thread_safe_agent = ThreadSafeRandAgentBuilder::new()
//!         .max_failures(3)
//!         .add_agent(client1.agent("glm-4-flash").build(), "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .add_agent(client2.agent("glm-4-flash").build(), "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .build();
//!
//!     let agent_arc = Arc::new(thread_safe_agent);
//!
//!     // 创建多个并发任务
//!     let mut handles = vec![];
//!     for i in 0..5 {
//!         let agent_clone = Arc::clone(&agent_arc);
//!         let handle = task::spawn(async move {
//!             let response = agent_clone.prompt(&format!("Hello from task {}", i)).await?;
//!             println!("Task {} response: {}", i, response);
//!             Ok::<(), RandAgentError>(())
//!         });
//!         handles.push(handle);
//!     }
//!
//!     // 等待所有任务完成
//!     for handle in handles {
//!         handle.await??;
//!     }
//!
//!     Ok(())
//! }
//! ```

use std::sync::Arc;
use rand::Rng;
use rig::agent::Agent;
use rig::client::builder::BoxAgent;
use rig::client::completion::CompletionModelHandle;
use rig::completion::{Message, Prompt, PromptError};
use tokio::sync::Mutex;

/// 推荐使用 ThreadSafeRandAgent，不推荐使用 RandAgent。
/// RandAgent 已不再维护，ThreadSafeRandAgent 支持多线程并发访问且更安全。
/// 线程安全的 RandAgent，支持多线程并发访问
pub struct ThreadSafeRandAgent {
    agents: Arc<Mutex<Vec<ThreadSafeAgentState>>>,
}

/// 线程安全的 Agent 状态
#[derive(Clone)]
pub struct ThreadSafeAgentState {
    pub agent: Arc<BoxAgent<'static>>,
    pub provider: String,
    pub model: String,
    pub failure_count: u32,
    pub max_failures: u32,
}

impl Prompt for ThreadSafeRandAgent {
    #[allow(refining_impl_trait)]
    async fn prompt(&self, prompt: impl Into<Message> + Send) -> Result<String, PromptError> {
        // 第一步：选择代理并获取其信息
        let mut agent_state = self.get_random_valid_agent_state()
            .await
            .ok_or(PromptError::MaxDepthError {
                max_depth: 0,
                chat_history: vec![],
                prompt: "没有有效agent".into(),
            })?;

        // 第二步：执行异步操作
        tracing::info!("Using provider: {}, model: {}", agent_state.provider, agent_state.model);
        match agent_state.agent.prompt(prompt).await {
            Ok(content) => {
                agent_state.record_success();
                Ok(content)
            }
            Err(e) => {
                agent_state.record_failure();
                Err(e)
            }
        }
    }
}

impl ThreadSafeAgentState {
    fn new(agent: BoxAgent<'static>, provider: String, model: String, max_failures: u32) -> Self {
        Self {
            agent: Arc::new(agent),
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

impl ThreadSafeRandAgent {
    /// 创建新的线程安全 RandAgent
    pub fn new(agents: Vec<(BoxAgent<'static>, String, String)>) -> Self {
        Self::with_max_failures(agents, 3)
    }

    /// 使用自定义最大失败次数创建线程安全 RandAgent
    pub fn with_max_failures(agents: Vec<(BoxAgent<'static>, String, String)>, max_failures: u32) -> Self {
        let agent_states = agents
            .into_iter()
            .map(|(agent, provider, model)| ThreadSafeAgentState::new(agent, provider, model, max_failures))
            .collect();
        Self {
            agents: Arc::new(Mutex::new(agent_states)),
        }
    }

    /// 添加代理到集合中
    pub async fn add_agent(&self, agent: BoxAgent<'static>, provider: String, model: String) {
        let mut agents = self.agents.lock().await;
        agents.push(ThreadSafeAgentState::new(agent, provider, model, 3));
    }

    /// 使用自定义最大失败次数添加代理
    pub async fn add_agent_with_max_failures(&self, agent: BoxAgent<'static>, provider: String, model: String, max_failures: u32) {
        let mut agents = self.agents.lock().await;
        agents.push(ThreadSafeAgentState::new(agent, provider, model, max_failures));
    }

    /// 获取有效代理数量
    pub async fn len(&self) -> usize {
        let agents = self.agents.lock().await;
        agents.iter().filter(|state| state.is_valid()).count()
    }
    
    /// 从集合中获取一个随机有效代理
    pub async fn get_random_valid_agent_state(&self) -> Option<ThreadSafeAgentState> {
        let mut agents = self.agents.lock().await;

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
        agents.get_mut(agent_index).cloned()
    }

    /// 获取总代理数量（包括无效的）
    pub async fn total_len(&self) -> usize {
        let agents = self.agents.lock().await;
        agents.len()
    }

    /// 检查是否有有效代理
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }
    
    /// 获取所有代理（用于调试或检查）
    pub async fn agents(&self) -> Vec<(String, String, u32, u32)> {
        let agents = self.agents.lock().await;
        agents
            .iter()
            .map(|state| (
                state.provider.clone(),
                state.model.clone(),
                state.failure_count,
                state.max_failures
            ))
            .collect()
    }

    /// 获取失败统计
    pub async fn failure_stats(&self) -> Vec<(usize, u32, u32)> {
        let agents = self.agents.lock().await;
        agents
            .iter()
            .enumerate()
            .map(|(i, state)| (i, state.failure_count, state.max_failures))
            .collect()
    }

    /// 重置所有代理的失败计数
    pub async fn reset_failures(&self) {
        let mut agents = self.agents.lock().await;
        for state in agents.iter_mut() {
            state.failure_count = 0;
        }
    }
}

/// 线程安全 RandAgent 的构建器
pub struct ThreadSafeRandAgentBuilder {
    agents: Vec<(BoxAgent<'static>, String, String)>,
    max_failures: u32,
}

impl ThreadSafeRandAgentBuilder {
    /// 创建新的 ThreadSafeRandAgentBuilder
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            max_failures: 3, // 默认最大失败次数
        }
    }

    /// 设置连续失败的最大次数，超过后标记代理为无效
    pub fn max_failures(mut self, max_failures: u32) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// 添加代理到构建器
    ///
    /// # 参数
    /// - agent: 代理实例（需要是 'static 生命周期）
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_agent(mut self, agent: BoxAgent<'static>, provider_name: String, model_name: String) -> Self {
        self.agents.push((agent, provider_name, model_name));
        self
    }

    /// 从 AgentBuilder 添加代理
    ///
    /// # 参数
    /// - builder: AgentBuilder 实例（需要是 'static 生命周期）
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_builder(mut self, builder: Agent<CompletionModelHandle<'static>>, provider_name: &str, model_name: &str) -> Self {
        self.agents.push((builder, provider_name.to_string(), model_name.to_string()));
        self
    }

    /// 构建 ThreadSafeRandAgent
    pub fn build(self) -> ThreadSafeRandAgent {
        ThreadSafeRandAgent::with_max_failures(self.agents, self.max_failures)
    }
}

impl Default for ThreadSafeRandAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}