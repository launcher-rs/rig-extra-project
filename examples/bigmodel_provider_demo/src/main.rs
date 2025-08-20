use config::Config;
use rig_extra::agent::stream_to_stdout;
use rig_extra::client::CompletionClient;
use rig_extra::completion::{Prompt, ToolDefinition};
use rig_extra::extra_providers::bigmodel;
use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_FLASH;
use rig_extra::streaming::StreamingPrompt;
use rig_extra::tool::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
struct OperationArgs {
    x: i32,
    y: i32,
}

#[derive(Debug, thiserror::Error)]
#[error("Math error")]
struct MathError;

#[derive(Deserialize, Serialize)]
struct Adder;
impl Tool for Adder {
    const NAME: &'static str = "add";

    type Error = MathError;
    type Args = OperationArgs;
    type Output = i32;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "add".to_string(),
            description: "Add x and y together".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": {
                        "type": "number",
                        "description": "The first number to add"
                    },
                    "y": {
                        "type": "number",
                        "description": "The second number to add"
                    }
                }
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        println!("[tool-call] Adding {} and {}", args.x, args.y);
        let result = args.x + args.y;
        Ok(result)
    }
}

#[derive(Debug, Deserialize, JsonSchema, Serialize)]
/// A record representing a person
struct Person {
    /// The person's first name, if provided (null otherwise)
    #[schemars(required)]
    pub first_name: Option<String>,
    /// The person's last name, if provided (null otherwise)
    #[schemars(required)]
    pub last_name: Option<String>,
    /// The person's job, if provided (null otherwise)
    #[schemars(required)]
    pub job: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    // 1. 获取配置
    let settings = Config::builder()
        .add_source(config::File::with_name("Settings"))
        .build()
        .unwrap_or_default();

    let api_key = settings
        .get_string("bigmodel_api_key")
        .expect("Missing API Key in Settings");

    let client = bigmodel::Client::new(api_key.as_str());

    let agent = client
        .agent(BIGMODEL_GLM_4_FLASH)
        .preamble("你是一个ai助手")
        .build();

    // 普通调用
    tracing::info!("同步调用:");
    let response = agent.prompt("你好").await.unwrap();
    tracing::info!("{}", response);

    // 异步调用
    tracing::info!("异步调用:");
    let mut stream = agent.stream_prompt("hello").await;
    let res = stream_to_stdout(&mut stream).await.unwrap();
    println!("Token usage response: {usage:?}", usage = res.usage());
    println!("Final text response: {message:?}", message = res.response());

    tracing::info!("工具调用==============");
    let tool_agent = client
        .agent(BIGMODEL_GLM_4_FLASH)
        .preamble("你是一个ai助手")
        .tool(Adder)
        .build();

    // 普通调用
    tracing::info!("同步工具调用调用:");
    let response = tool_agent
        .prompt("计算5+8=,然后在加12是多少")
        // 设置多轮对话的最大深度
        .multi_turn(10)
        // .prompt("计算5+8是多少")
        .await
        .unwrap();

    tracing::info!("{}", response);

    // // 异步调用
    tracing::info!("异步调用:");
    let mut stream = tool_agent.stream_prompt("8+12=").await;
    let res = stream_to_stdout(&mut stream).await.unwrap();
    println!("Token usage response: {usage:?}", usage = res.usage());
    println!("Final text response: {message:?}", message = res.response());

    // 提取
    tracing::info!("Extracting...:");
    // let data_extractor = client.extractor::<Person>(BIGMODEL_GLM_4_FLASH).build();
    let data_extractor = client.extractor::<Person>(BIGMODEL_GLM_4_FLASH).build();

    let person = data_extractor
        .extract("Hello my name is John Doe! I am a software engineer.")
        .await
        .unwrap();
    tracing::info!("person:{:?}", person);
}
