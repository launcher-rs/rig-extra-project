use futures::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::pin::Pin;

use rig::agent::{Agent, MultiTurnStreamItem};
use rig::completion::{Message, Prompt, PromptError};
use rig::providers::{
    anthropic, azure, cohere, deepseek, galadriel, gemini, groq, huggingface, hyperbolic, mira,
    mistral, moonshot, ollama, openai, openrouter, perplexity, together, xai,
};
use rig::streaming::StreamingPrompt;
// 你自己的 provider
use crate::extra_providers::bigmodel;

/// 所有 Provider 的统一枚举
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

/// 统一的返回流类型
pub type AnyMultiStream<R> =
    Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<R>, PromptError>> + Send>>;

/// 将 Provider 原始流转换成统一的 Event 流
fn map_stream<SR, R, E>(
    stream: Pin<Box<dyn Stream<Item = Result<MultiTurnStreamItem<SR>, E>> + Send>>,
) -> AnyMultiStream<R>
where
    SR: serde::Serialize + Send + 'static,
    R: DeserializeOwned + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    Box::pin(stream.map(move |item| {
        item.map_err(|e| {
            // Convert error to string and create PromptError
            let error_msg = format!("stream error: {}", e);
            PromptError::MaxDepthError {
                max_depth: 0,
                chat_history: Box::new(vec![]),
                prompt: error_msg.into(),
            }
        })
        .and_then(|multi_item| {
            // Convert MultiTurnStreamItem<SR> to MultiTurnStreamItem<R>
            // by serializing and deserializing the whole item
            serde_json::to_value(&multi_item)
                .map_err(|e| PromptError::MaxDepthError {
                    max_depth: 0,
                    chat_history: Box::new(vec![]),
                    prompt: format!("serialization error: {}", e).into(),
                })
                .and_then(|val| {
                    serde_json::from_value(val).map_err(|e| PromptError::MaxDepthError {
                        max_depth: 0,
                        chat_history: Box::new(vec![]),
                        prompt: format!("deserialization error: {}", e).into(),
                    })
                })
        })
    }))
}

// ======================
// 同步 Prompt 调用
// ======================
impl AgentVariant {
    pub async fn prompt(&self, prompt: impl Into<Message> + Send) -> Result<String, PromptError> {
        match self {
            AgentVariant::OpenAI(a) => a.prompt(prompt).await,
            AgentVariant::ResponsesOpenAI(a) => a.prompt(prompt).await,
            AgentVariant::Ollama(a) => a.prompt(prompt).await,
            AgentVariant::Bigmodel(a) => a.prompt(prompt).await,
            AgentVariant::OpenRouter(a) => a.prompt(prompt).await,
            AgentVariant::Anthropic(a) => a.prompt(prompt).await,
            AgentVariant::Cohere(a) => a.prompt(prompt).await,
            AgentVariant::Gemini(a) => a.prompt(prompt).await,
            AgentVariant::Huggingface(a) => a.prompt(prompt).await,
            AgentVariant::Mistral(a) => a.prompt(prompt).await,
            AgentVariant::Together(a) => a.prompt(prompt).await,
            AgentVariant::XAI(a) => a.prompt(prompt).await,
            AgentVariant::Azure(a) => a.prompt(prompt).await,
            AgentVariant::DeepSeek(a) => a.prompt(prompt).await,
            AgentVariant::Galadriel(a) => a.prompt(prompt).await,
            AgentVariant::Groq(a) => a.prompt(prompt).await,
            AgentVariant::Hyperbolic(a) => a.prompt(prompt).await,
            AgentVariant::Mira(a) => a.prompt(prompt).await,
            AgentVariant::Mooshot(a) => a.prompt(prompt).await,
            AgentVariant::Perplexity(a) => a.prompt(prompt).await,
        }
    }
}

// ======================
// Streaming Prompt 调用（重点）
// ======================
impl AgentVariant {
    pub async fn stream_prompt<R>(
        &self,
        prompt: impl Into<Message> + Send,
    ) -> Result<AnyMultiStream<R>, PromptError>
    where
        R: DeserializeOwned + Send + 'static,
    {
        async fn handle<M, R>(
            agent: &Agent<M>,
            prompt: impl Into<Message> + Send,
        ) -> Result<AnyMultiStream<R>, PromptError>
        where
            M: rig::completion::CompletionModel + Send + Sync + 'static,
            <M as rig::completion::CompletionModel>::StreamingResponse:
                serde::Serialize + Send + 'static,
            R: DeserializeOwned + Send + 'static,
        {
            // Provider 层的错误类型通常是 StreamingError
            let raw_stream = agent.stream_prompt(prompt).await;

            Ok(map_stream(raw_stream))
        }

        match self {
            AgentVariant::OpenAI(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::ResponsesOpenAI(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Ollama(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Bigmodel(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::OpenRouter(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Anthropic(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Cohere(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Gemini(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Huggingface(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Mistral(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Together(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::XAI(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Azure(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::DeepSeek(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Galadriel(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Groq(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Hyperbolic(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Mira(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Mooshot(a) => handle::<_, R>(a, prompt).await,
            AgentVariant::Perplexity(a) => handle::<_, R>(a, prompt).await,
        }
    }
}
