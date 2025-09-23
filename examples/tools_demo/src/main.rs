use config::Config;
use rig_extra::client::CompletionClient;
use rig_extra::completion::Prompt;
use rig_extra::extra_providers::bigmodel;
use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_FLASH;
use rig_extra::tools::serpapi::SerpapiTools;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .with_target(true)
        .init();
    // 1. 获取配置
    let settings = Config::builder()
        .add_source(config::File::with_name("Settings"))
        .build()
        .unwrap_or_default();

    let api_key = settings
        .get_string("bigmodel_api_key")
        .expect("Missing API Key in Settings");

    let serpapi_api_key = settings
        .get_string("serpapi_api_key")
        .expect("Missing Serpapi API Key in Settings");
    let client = bigmodel::Client::new(api_key.as_str());

    let agent = client
        .agent(BIGMODEL_GLM_4_FLASH)
        .name("ai agent")
        .tool(SerpapiTools::new(serpapi_api_key))
        .preamble("你是一个ai助手")
        .build();

    let result = agent.prompt("获取一周内AI最新动态").await.unwrap();
    println!("{}", result);
}
