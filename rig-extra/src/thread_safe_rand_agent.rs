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


use std::sync::{Arc, Mutex};
use rand::Rng;
use rig::agent::Agent;
use rig::client::builder::BoxAgent;
use rig::client::completion::CompletionModelHandle;
use rig::completion::Prompt;

use crate::error::RandAgentError;

/// 线程安全的 RandAgent，支持多线程并发访问
pub struct ThreadSafeRandAgent {
    agents: Arc<Mutex<Vec<ThreadSafeAgentState>>>,
}

/// 线程安全的 Agent 状态
pub struct ThreadSafeAgentState {
    agent: Arc<BoxAgent<'static>>,
    provider: String,
    model: String,
    failure_count: u32,
    max_failures: u32,
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
    pub fn add_agent(&self, agent: BoxAgent<'static>, provider: String, model: String) {
        let mut agents = self.agents.lock().unwrap();
        agents.push(ThreadSafeAgentState::new(agent, provider, model, 3));
    }

    /// 使用自定义最大失败次数添加代理
    pub fn add_agent_with_max_failures(&self, agent: BoxAgent<'static>, provider: String, model: String, max_failures: u32) {
        let mut agents = self.agents.lock().unwrap();
        agents.push(ThreadSafeAgentState::new(agent, provider, model, max_failures));
    }

    /// 获取有效代理数量
    pub fn len(&self) -> usize {
        let agents = self.agents.lock().unwrap();
        agents.iter().filter(|state| state.is_valid()).count()
    }

    /// 获取总代理数量（包括无效的）
    pub fn total_len(&self) -> usize {
        let agents = self.agents.lock().unwrap();
        agents.len()
    }

    /// 检查是否有有效代理
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }



    /// 向随机有效代理发送消息
    pub async fn prompt(
        &self,
        message: &str,
    ) -> Result<String, RandAgentError> {
        // 第一步：选择代理并获取其信息
        let (agent_index, provider, model) = {
            let agents = self.agents.lock().unwrap();

            // 找到所有有效代理的索引
            let valid_indices: Vec<usize> = agents
                .iter()
                .enumerate()
                .filter(|(_, state)| state.is_valid())
                .map(|(i, _)| i)
                .collect();

            if valid_indices.is_empty() {
                return Err(RandAgentError::NoValidAgents);
            }

            // 随机选择一个有效代理
            let mut rng = rand::rng();
            let random_index = rng.random_range(0..valid_indices.len());
            let agent_index = valid_indices[random_index];

            // 获取代理信息
            let agent_state = &agents[agent_index];
            let provider = agent_state.provider.clone();
            let model = agent_state.model.clone();

            (agent_index, provider, model)
        };

        // 打印使用的 provider 和 model
        tracing::info!("Using provider: {}, model: {}", provider, model);

        // 第二步：执行异步操作（在锁外执行）
        let result = {
            // 获取代理的 Arc 克隆以避免在异步操作中持有锁
            let agent = {
                let agents = self.agents.lock().unwrap();
                Arc::clone(&agents[agent_index].agent)
            };

            // 在锁外执行异步操作
            agent.prompt(message).await.map_err(|e| RandAgentError::AgentError(Box::new(e)))
        };

        // 第三步：根据结果更新失败计数
        match &result {
            Ok(_) => {
                let mut agents = self.agents.lock().unwrap();
                agents[agent_index].record_success();
            }
            Err(_) => {
                let mut agents = self.agents.lock().unwrap();
                agents[agent_index].record_failure();
            }
        }

        result
    }

    /// 获取所有代理（用于调试或检查）
    pub fn agents(&self) -> Vec<(String, String, u32, u32)> {
        let agents = self.agents.lock().unwrap();
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
    pub fn failure_stats(&self) -> Vec<(usize, u32, u32)> {
        let agents = self.agents.lock().unwrap();
        agents
            .iter()
            .enumerate()
            .map(|(i, state)| (i, state.failure_count, state.max_failures))
            .collect()
    }

    /// 重置所有代理的失败计数
    pub fn reset_failures(&self) {
        let mut agents = self.agents.lock().unwrap();
        for state in agents.iter_mut() {
            state.failure_count = 0;
        }
    }
}

// 实现 Send + Sync trait
unsafe impl Send for ThreadSafeRandAgent {}
unsafe impl Sync for ThreadSafeRandAgent {}


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