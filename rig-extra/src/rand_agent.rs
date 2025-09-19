//! ## 多线程使用示例
//!
//! ```rust
//! use rig_extra::extra_providers::{bigmodel::Client};
//! use rig_extra::rand_agent::RandAgentBuilder;
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
//!     let thread_safe_agent = RandAgentBuilder::new()
//!         .max_failures(3)
//!         .add_agent(client1.agent("glm-4-flash").build(),1, "bigmodel".to_string(), "glm-4-flash".to_string())
//!         .add_agent(client2.agent("glm-4-flash").build(),2, "bigmodel".to_string(), "glm-4-flash".to_string())
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

use crate::AgentInfo;
use crate::error::RandAgentError;
use backon::{ExponentialBuilder, Retryable};
use rand::Rng;
use rig::agent::Agent;
use rig::client::builder::BoxAgent;
use rig::client::completion::CompletionModelHandle;
use rig::completion::{Message, Prompt, PromptError};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

/// 代理失效回调类型，减少类型复杂度
pub type OnAgentInvalidCallback = Option<Arc<Box<dyn Fn(i32) + Send + Sync + 'static>>>;

/// 推荐使用 RandAgent，不推荐使用 RandAgent。
/// RandAgent 已不再维护，RandAgent 支持多线程并发访问且更安全。
/// 线程安全的 RandAgent，支持多线程并发访问
#[derive(Clone)]
pub struct RandAgent {
    agents: Arc<Mutex<Vec<AgentState>>>,
    on_agent_invalid: OnAgentInvalidCallback,
}

/// 线程安全的 Agent 状态
#[derive(Clone)]
pub struct AgentState {
    pub id: i32,
    pub agent: Arc<BoxAgent<'static>>,
    pub info: AgentInfo,
}

impl Prompt for RandAgent {
    #[allow(refining_impl_trait)]
    async fn prompt(&self, prompt: impl Into<Message> + Send) -> Result<String, PromptError> {
        // 第一步：选择代理并获取其索引
        let agent_index =
            self.get_random_valid_agent_index()
                .await
                .ok_or(PromptError::MaxDepthError {
                    max_depth: 0,
                    chat_history: Box::new(vec![]),
                    prompt: "没有有效agent".into(),
                })?;

        // 第二步：加锁并获取可变引用
        let mut agents = self.agents.lock().await;
        let agent_state = &mut agents[agent_index];

        tracing::info!(
            "Using provider: {}, model: {},id: {}",
            agent_state.info.provider,
            agent_state.info.model,
            agent_state.info.id
        );
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

impl AgentState {
    fn new(
        agent: BoxAgent<'static>,
        id: i32,
        provider: String,
        model: String,
        max_failures: u32,
    ) -> Self {
        Self {
            id,
            agent: Arc::new(agent),
            info: AgentInfo {
                id,
                provider,
                model,
                failure_count: 0,
                max_failures,
            },
        }
    }

    fn is_valid(&self) -> bool {
        self.info.failure_count < self.info.max_failures
    }

    fn record_failure(&mut self) {
        self.info.failure_count += 1;
    }

    fn record_success(&mut self) {
        self.info.failure_count = 0;
    }
}

impl RandAgent {
    /// 创建新的线程安全 RandAgent
    pub fn new(agents: Vec<(BoxAgent<'static>, i32, String, String)>) -> Self {
        Self::with_max_failures_and_callback(agents, 3, None)
    }

    /// 使用自定义最大失败次数和回调创建线程安全 RandAgent
    pub fn with_max_failures_and_callback(
        agents: Vec<(BoxAgent<'static>, i32, String, String)>,
        max_failures: u32,
        on_agent_invalid: OnAgentInvalidCallback,
    ) -> Self {
        let agent_states = agents
            .into_iter()
            .map(|(agent, id, provider, model)| {
                AgentState::new(agent, id, provider, model, max_failures)
            })
            .collect();
        Self {
            agents: Arc::new(Mutex::new(agent_states)),
            on_agent_invalid,
        }
    }

    /// 使用自定义最大失败次数创建线程安全 RandAgent
    pub fn with_max_failures(
        agents: Vec<(BoxAgent<'static>, i32, String, String)>,
        max_failures: u32,
    ) -> Self {
        Self::with_max_failures_and_callback(agents, max_failures, None)
    }

    /// 设置 agent 失效时的回调
    pub fn set_on_agent_invalid<F>(&mut self, callback: F)
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        self.on_agent_invalid = Some(Arc::new(Box::new(callback)));
    }

    /// 添加代理到集合中
    pub async fn add_agent(
        &self,
        agent: BoxAgent<'static>,
        id: i32,
        provider: String,
        model: String,
    ) {
        let mut agents = self.agents.lock().await;
        agents.push(AgentState::new(agent, id, provider, model, 3));
    }

    /// 使用自定义最大失败次数添加代理
    pub async fn add_agent_with_max_failures(
        &self,
        agent: BoxAgent<'static>,
        id: i32,
        provider: String,
        model: String,
        max_failures: u32,
    ) {
        let mut agents = self.agents.lock().await;
        agents.push(AgentState::new(agent, id, provider, model, max_failures));
    }

    /// 获取有效代理数量
    pub async fn len(&self) -> usize {
        let agents = self.agents.lock().await;
        agents.iter().filter(|state| state.is_valid()).count()
    }

    /// 从集合中获取一个随机有效代理的索引
    pub async fn get_random_valid_agent_index(&self) -> Option<usize> {
        let agents = self.agents.lock().await;
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
        Some(valid_indices[random_index])
    }

    /// 从集合中获取一个随机有效代理
    /// 注意: 并不会增加失败计数
    pub async fn get_random_valid_agent_state(&self) -> Option<AgentState> {
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

    /// 获取agent info
    pub async fn get_agents_info(&self) -> Vec<AgentInfo> {
        let agents = self.agents.lock().await;
        let agent_infos = agents.iter().map(|agent| agent.info.clone()).collect::<_>();
        tracing::info!("agents info: {:?}", agent_infos);
        agent_infos
    }

    /// 获取失败统计
    pub async fn failure_stats(&self) -> Vec<(usize, u32, u32)> {
        let agents = self.agents.lock().await;
        agents
            .iter()
            .enumerate()
            .map(|(i, state)| (i, state.info.failure_count, state.info.max_failures))
            .collect()
    }

    /// 重置所有代理的失败计数
    pub async fn reset_failures(&self) {
        let mut agents = self.agents.lock().await;
        for state in agents.iter_mut() {
            state.info.failure_count = 0;
        }
    }

    /// 通过名称获取 agent
    pub async fn get_agent_by_name(
        &self,
        provider_name: &str,
        model_name: &str,
    ) -> Option<AgentState> {
        let mut agents = self.agents.lock().await;

        for agent in agents.iter_mut() {
            if agent.info.provider == provider_name && agent.info.model == model_name {
                return Some(agent.clone());
            }
        }

        None
    }

    /// 通过id获取 agent
    pub async fn get_agent_by_id(&self, id: i32) -> Option<AgentState> {
        let mut agents = self.agents.lock().await;

        for agent in agents.iter_mut() {
            if agent.info.id == id {
                return Some(agent.clone());
            }
        }

        None
    }

    /// 添加失败重试
    pub async fn try_invoke_with_retry(
        &self,
        info: Message,
        retry_num: Option<usize>,
    ) -> Result<String, RandAgentError> {
        let mut config = ExponentialBuilder::default();
        if let Some(retry_num) = retry_num {
            config = config.with_max_times(retry_num)
        }

        let info = Arc::new(info);

        let content = (|| {
            let agent = self.clone();
            let prompt = info.clone();
            async move { agent.prompt((*prompt).clone()).await }
        })
        .retry(config)
        .sleep(tokio::time::sleep)
        .notify(|err: &PromptError, dur: Duration| {
            println!("retrying {err:?} after {dur:?}");
        })
        .await?;
        Ok(content)
    }

    #[allow(refining_impl_trait)]
    pub async fn prompt_with_info(
        &self,
        prompt: impl Into<Message> + Send,
    ) -> Result<(String, AgentInfo), PromptError> {
        // 第一步：选择代理并获取其索引
        let agent_index =
            self.get_random_valid_agent_index()
                .await
                .ok_or(PromptError::MaxDepthError {
                    max_depth: 0,
                    chat_history: Box::new(vec![]),
                    prompt: "没有有效agent".into(),
                })?;

        // 第二步：加锁并获取可变引用
        let mut agents = self.agents.lock().await;
        let agent_state = &mut agents[agent_index];

        let agent_info = agent_state.info.clone();

        tracing::info!(
            "prompt_with_info Using provider: {}, model: {},id: {}",
            agent_state.info.provider,
            agent_state.info.model,
            agent_state.info.id
        );
        match agent_state.agent.prompt(prompt).await {
            Ok(content) => {
                agent_state.record_success();
                Ok((content, agent_info))
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

    /// 添加失败重试
    pub async fn try_invoke_with_info_retry(
        &self,
        info: Message,
        retry_num: Option<usize>,
    ) -> Result<(String, AgentInfo), RandAgentError> {
        let mut config = ExponentialBuilder::default();
        if let Some(retry_num) = retry_num {
            config = config.with_max_times(retry_num)
        }

        let info = Arc::new(info);

        let content = (|| {
            let agent = self.clone();
            let prompt = info.clone();
            async move { agent.prompt_with_info((*prompt).clone()).await }
        })
        .retry(config)
        .sleep(tokio::time::sleep)
        .notify(|err: &PromptError, dur: Duration| {
            println!("retrying {err:?} after {dur:?}");
        })
        .await?;
        Ok(content)
    }
}

/// 线程安全 RandAgent 的构建器
pub struct RandAgentBuilder {
    pub(crate) agents: Vec<(BoxAgent<'static>, i32, String, String)>,
    max_failures: u32,
    on_agent_invalid: OnAgentInvalidCallback,
}

impl RandAgentBuilder {
    /// 创建新的 RandAgentBuilder
    pub fn new() -> Self {
        Self {
            agents: Vec::new(),
            max_failures: 3, // 默认最大失败次数
            on_agent_invalid: None,
        }
    }

    /// 设置连续失败的最大次数，超过后标记代理为无效
    pub fn max_failures(mut self, max_failures: u32) -> Self {
        self.max_failures = max_failures;
        self
    }

    /// 设置 agent 失效时的回调
    pub fn on_agent_invalid<F>(mut self, callback: F) -> Self
    where
        F: Fn(i32) + Send + Sync + 'static,
    {
        self.on_agent_invalid = Some(Arc::new(Box::new(callback)));
        self
    }

    /// 添加代理到构建器
    ///
    /// # 参数
    /// - agent: 代理实例（需要是 'static 生命周期）
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_agent(
        mut self,
        agent: BoxAgent<'static>,
        id: i32,
        provider_name: String,
        model_name: String,
    ) -> Self {
        self.agents.push((agent, id, provider_name, model_name));
        self
    }

    /// 从 AgentBuilder 添加代理
    ///
    /// # 参数
    /// - builder: AgentBuilder 实例（需要是 'static 生命周期）
    /// - provider_name: 提供方名称（如 openai、bigmodel 等）
    /// - model_name: 模型名称（如 gpt-3.5、glm-4-flash 等）
    pub fn add_builder(
        mut self,
        builder: Agent<CompletionModelHandle<'static>>,
        id: i32,
        provider_name: &str,
        model_name: &str,
    ) -> Self {
        self.agents.push((
            builder,
            id,
            provider_name.to_string(),
            model_name.to_string(),
        ));
        self
    }

    /// 构建 RandAgent
    pub fn build(self) -> RandAgent {
        RandAgent::with_max_failures_and_callback(
            self.agents,
            self.max_failures,
            self.on_agent_invalid,
        )
    }
}

impl Default for RandAgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
