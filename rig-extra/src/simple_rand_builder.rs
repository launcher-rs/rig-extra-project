use crate::agent_variant::AgentVariant;
use crate::extra_providers::bigmodel;
use crate::rand_agent::RandAgentBuilder;
use rig::client::CompletionClient;
use rig::providers::*;
use serde::{Deserialize, Serialize};
use strum_macros::Display;

#[derive(Debug, Display, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderEnum {
    Anthropic,
    Cohere,
    Gemini,
    Huggingface,
    Mistral,
    OpenAi,
    ResponsesOpenAi,
    OpenRouter,
    Together,
    XAI,
    Azure,
    DeepSeek,
    Galadriel,
    Groq,
    Hyperbolic,
    Mira,
    Mooshot,
    Ollama,
    Perplexity,
    // embedding模型
    // Voyageai,
    Bigmodel,
}

#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub id: i32,
    pub provider: ProviderEnum,
    pub model_name: String,
    pub api_key: String,
    pub api_base_url: Option<String>,
    pub system_prompt: Option<String>,
    pub agent_name: Option<String>,
}

impl RandAgentBuilder {
    /// 简单构建器
    pub fn simple_builder(
        mut self,
        agent_configs: Vec<AgentConfig>,
        global_system_prompt: String,
    ) -> Self {
        for agent_conf in agent_configs {
            let agent_name = agent_conf.agent_name.unwrap_or("rand agent".to_string());
            let system_prompt = agent_conf
                .system_prompt
                .unwrap_or(global_system_prompt.clone());

            match agent_conf.provider {
                ProviderEnum::Anthropic => {
                    let mut client_builder = anthropic::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.api_key(&agent_conf.api_key).build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                AgentVariant::Anthropic(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::Cohere => match cohere::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Cohere(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Gemini => {
                    let mut client_builder = gemini::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.api_key(&agent_conf.api_key).build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                AgentVariant::Gemini(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::Huggingface => match huggingface::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Huggingface(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Mistral => match mistral::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Mistral(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::OpenAi => {
                    let mut client_builder = openai::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.api_key(&agent_conf.api_key).build() {
                        Ok(client) => {
                            let agent = client
                                .completions_api()
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();

                            self.agents.push((
                                AgentVariant::OpenAI(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::ResponsesOpenAi => {
                    let mut client_builder = openai::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.api_key(&agent_conf.api_key).build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                AgentVariant::ResponsesOpenAI(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::OpenRouter => {
                    let mut client_builder = openrouter::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.api_key(&agent_conf.api_key).build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                AgentVariant::OpenRouter(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::Together => match together::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Together(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::XAI => match xai::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::XAI(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Azure => {
                    tracing::info!("Azure simple_builder暂不支持,参数有点多，可以自行添加........ ")
                }
                ProviderEnum::DeepSeek => match deepseek::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::DeepSeek(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Galadriel => match galadriel::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Galadriel(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Groq => match groq::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Groq(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Hyperbolic => match hyperbolic::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Hyperbolic(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Mira => match mira::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Mira(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Mooshot => match moonshot::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Mooshot(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Ollama => {
                    // Ollama uses Nothing as API key type, which means it doesn't need an API key
                    // In rig-core 0.25, we need to use builder pattern with no API key
                    use rig::client::Nothing;
                    let client_result = if let Some(api_base_url) = &agent_conf.api_base_url {
                        ollama::Client::builder()
                            .base_url(api_base_url)
                            .api_key(Nothing)
                            .build()
                    } else {
                        ollama::Client::builder().api_key(Nothing).build()
                    };
                    match client_result {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                AgentVariant::Ollama(agent),
                                agent_conf.id,
                                agent_conf.provider.to_string(),
                                agent_conf.model_name,
                            ));
                        }
                        Err(err) => {
                            tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                        }
                    }
                }
                ProviderEnum::Perplexity => match perplexity::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Perplexity(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
                ProviderEnum::Bigmodel => match bigmodel::Client::new(&agent_conf.api_key) {
                    Ok(client) => {
                        let agent = client
                            .agent(&agent_conf.model_name)
                            .name(agent_name.as_str())
                            .preamble(&system_prompt)
                            .build();
                        self.agents.push((
                            AgentVariant::Bigmodel(agent),
                            agent_conf.id,
                            agent_conf.provider.to_string(),
                            agent_conf.model_name,
                        ));
                    }
                    Err(err) => {
                        tracing::error!("添加 {} 错误: {}", agent_conf.provider, err);
                    }
                },
            }
        }
        self
    }
}
