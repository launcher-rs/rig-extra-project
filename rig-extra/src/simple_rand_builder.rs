use crate::extra_providers::bigmodel;
use crate::rand_agent::RandAgentBuilder;
use rig::client::completion::CompletionClientDyn;
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
                    let mut client_builder = anthropic::Client::builder(&agent_conf.api_key);
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                agent,
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
                ProviderEnum::Cohere => {
                    let client = cohere::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Gemini => {
                    let mut client_builder = gemini::Client::builder(&agent_conf.api_key);
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }
                    match client_builder.build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                agent,
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
                ProviderEnum::Huggingface => {
                    let client = huggingface::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Mistral => {
                    let client = mistral::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::OpenAi => {
                    let mut client_builder = openai::Client::builder(&agent_conf.api_key);
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url)
                    }

                    match client_builder.build() {
                        Ok(client) => {
                            // 不支持 completions_api,至少ollama使用这个会报错
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                agent,
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
                    let mut client_builder = openrouter::Client::builder(&agent_conf.api_key);
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url)
                    }

                    match client_builder.build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                agent,
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
                ProviderEnum::Together => {
                    let client = together::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::XAI => {
                    let client = xai::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Azure => {
                    tracing::info!("Azure simple_builder暂不支持,参数有点多，可以自行添加........ ")
                }
                ProviderEnum::DeepSeek => {
                    let client = deepseek::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Galadriel => {
                    let client = galadriel::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Groq => {
                    let client = groq::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Hyperbolic => {
                    let client = hyperbolic::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Mira => {
                    let client = mira::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Mooshot => {
                    let client = moonshot::Client::new(&agent_conf.api_key);
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
                ProviderEnum::Ollama => {
                    let mut client_builder = ollama::Client::builder();
                    if let Some(api_base_url) = &agent_conf.api_base_url {
                        client_builder = client_builder.base_url(api_base_url);
                    }

                    match client_builder.build() {
                        Ok(client) => {
                            let agent = client
                                .agent(&agent_conf.model_name)
                                .name(agent_name.as_str())
                                .preamble(&system_prompt)
                                .build();
                            self.agents.push((
                                agent,
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
                ProviderEnum::Perplexity => {
                    // let client = perplexity::Client::new(&agent_conf.api_key);
                    // let agent = client
                    //     .agent(&agent_conf.model_name)
                    //     .name(agent_name.as_str())
                    //     .preamble(&system_prompt)
                    //     .build();
                    // self.agents.push((
                    //     agent,
                    //     agent_conf.id,
                    //     agent_conf.provider.to_string(),
                    //     agent_conf.model_name,
                    // ));
                    tracing::info!("Perplexity 暂不支持,没有实现BoxAgent........ ")
                }
                ProviderEnum::Bigmodel => {
                    let client = if let Some(api_base_url) = agent_conf.api_base_url {
                        bigmodel::Client::from_url(&agent_conf.api_key, &api_base_url)
                    } else {
                        bigmodel::Client::new(&agent_conf.api_key)
                    };
                    let agent = client
                        .agent(&agent_conf.model_name)
                        .name(agent_name.as_str())
                        .preamble(&system_prompt)
                        .build();
                    self.agents.push((
                        agent,
                        agent_conf.id,
                        agent_conf.provider.to_string(),
                        agent_conf.model_name,
                    ));
                }
            }
        }
        self
    }
}
