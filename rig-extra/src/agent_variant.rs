use crate::extra_providers::bigmodel;
use rig::agent::{Agent};
use rig::completion::{Message, Prompt, PromptError};
use rig::providers::{
    anthropic, azure, cohere, deepseek, galadriel, gemini, groq, huggingface, hyperbolic, mira,
    mistral, moonshot, ollama, openai, openrouter, perplexity, together, xai,
};

/// Agent 变体枚举，支持不同的 provider
#[derive(Clone)]
pub enum AgentVariant {
    OpenAI(Agent<openai::completion::CompletionModel>),
    ResponsesOpenAI(Agent<openai::responses_api::ResponsesCompletionModel>),
    Ollama(Agent<ollama::CompletionModel>),
    Bigmodel(Agent<bigmodel::CompletionModel>),
    OpenRouter(Agent<openrouter::completion::CompletionModel>),
    Anthropic(Agent<anthropic::completion::CompletionModel>),
    Cohere(Agent<cohere::completion::CompletionModel>),
    Gemini(Agent<gemini::completion::CompletionModel>),
    Huggingface(Agent<huggingface::completion::CompletionModel>),
    Mistral(Agent<mistral::completion::CompletionModel>),
    Together(Agent<together::completion::CompletionModel>),
    XAI(Agent<xai::completion::CompletionModel>),
    Azure(Agent<azure::CompletionModel>),
    DeepSeek(Agent<deepseek::CompletionModel>),
    Galadriel(Agent<galadriel::CompletionModel>),
    Groq(Agent<groq::CompletionModel>),
    Hyperbolic(Agent<hyperbolic::CompletionModel>),
    Mira(Agent<mira::CompletionModel>),
    Mooshot(Agent<moonshot::CompletionModel>),
    Perplexity(Agent<perplexity::CompletionModel>),
}

/// 异步调用未完成...
impl AgentVariant {
    /// 同步调用
    pub async fn prompt(
        &self,
        prompt: impl Into<Message> + Send,
    ) -> Result<String, PromptError> {
        match self {
            AgentVariant::OpenAI(agent) => agent.prompt(prompt).await,
            AgentVariant::ResponsesOpenAI(agent) => agent.prompt(prompt).await,
            AgentVariant::Ollama(agent) => agent.prompt(prompt).await,
            AgentVariant::Bigmodel(agent) => agent.prompt(prompt).await,
            AgentVariant::OpenRouter(agent) => agent.prompt(prompt).await,
            AgentVariant::Anthropic(agent) => agent.prompt(prompt).await,
            AgentVariant::Cohere(agent) => agent.prompt(prompt).await,
            AgentVariant::Gemini(agent) => agent.prompt(prompt).await,
            AgentVariant::Huggingface(agent) => agent.prompt(prompt).await,
            AgentVariant::Mistral(agent) => agent.prompt(prompt).await,
            AgentVariant::Together(agent) => agent.prompt(prompt).await,
            AgentVariant::XAI(agent) => agent.prompt(prompt).await,
            AgentVariant::Azure(agent) => agent.prompt(prompt).await,
            AgentVariant::DeepSeek(agent) => agent.prompt(prompt).await,
            AgentVariant::Galadriel(agent) => agent.prompt(prompt).await,
            AgentVariant::Groq(agent) => agent.prompt(prompt).await,
            AgentVariant::Hyperbolic(agent) => agent.prompt(prompt).await,
            AgentVariant::Mira(agent) => agent.prompt(prompt).await,
            AgentVariant::Mooshot(agent) => agent.prompt(prompt).await,
            AgentVariant::Perplexity(agent) => agent.prompt(prompt).await,
        }
    }
}
