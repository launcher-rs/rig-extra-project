use crate::simple_rand_builder::AgentConfig;
use rig::agent::{Agent, AgentBuilder};
use rig::client::CompletionClient;
use rig::client::completion::CompletionModelHandle;
use rig::providers::openai::Client;
use std::sync::Arc;

/// 由于和 CompletionClientDyn trait冲突，所有另起一页
pub fn get_openai_agent(
    client: Client,
    model_name: &str,
    agent_name: String,
    system_prompt: String,
) -> Agent<CompletionModelHandle<'static>> {
    let model = client.completion_model(model_name).completions_api();
    let handle = CompletionModelHandle {
        inner: Arc::new(model),
    };

    let agent = AgentBuilder::new(handle)
        .name(agent_name.as_str())
        .preamble(&system_prompt)
        .build();

    agent
}
