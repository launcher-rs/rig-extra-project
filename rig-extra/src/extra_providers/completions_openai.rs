use crate::HttpClient;
use rig::agent::AgentBuilder;
use rig::client::CompletionClient;
use rig::extractor::ExtractorBuilder;
use rig::providers;
use rig::providers::openai::{Client, CompletionModel};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 获取openai client
pub fn get_completions_openai_client(base_url: &str, api_key: &str) -> Client<HttpClient> {
    let client = providers::openai::Client::builder()
        .base_url(base_url)
        .api_key(api_key)
        .build()
        .unwrap();

    client
}

/// 获取 openai agent builder
pub fn get_completions_openai_agent_builder(
    base_url: &str,
    api_key: &str,
    model_name: &str,
) -> AgentBuilder<CompletionModel> {
    let client = get_completions_openai_client(base_url, api_key);
    let agent_builder = client
        .completion_model(model_name)
        .completions_api()
        .into_agent_builder();
    agent_builder
}

/// 获取 openai extractor builder
pub fn get_completions_openai_extractor_builder<U>(
    base_url: &str,
    api_key: &str,
    model_name: &str,
) -> ExtractorBuilder<CompletionModel, U>
where
    U: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync,
{
    let client = get_completions_openai_client(base_url, api_key);
    let extractor_builder = client.completions_api().extractor::<U>(model_name);
    extractor_builder
}
