use config::Config;
use rig_extra::client::CompletionClient;
use rig_extra::completion::Prompt;
use rig_extra::extra_providers::bigmodel;
use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_7_FLASH;
use rig_extra::tools::serpapi_tool::SerpapiTool;
use rig_extra::tools::tavily_tool::TavilyTool;

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

    let serpapi_api_keys = settings
        .get::<Vec<String>>("serpapi_api_keys")
        .expect("Missing Serpapi API Key in Settings");

    let tavily_api_keys = settings
        .get::<Vec<String>>("tavily_api_keys")
        .expect("Missing Tavily API Key in Settings");

    println!("serpapi_api_keys: {:?}", serpapi_api_keys);
    println!("tavily_api_keys: {:?}", tavily_api_keys);

    let client:bigmodel::Client = bigmodel::Client::new(api_key.as_str()).unwrap();

    let agent = client
        .agent(BIGMODEL_GLM_4_7_FLASH)
        .name("ai agent")
        .tool(SerpapiTool::new_with_keys(serpapi_api_keys))
        .preamble("你是一个ai助手")
        .build();

    let result = agent.prompt("获取一周内AI最新动态").await.unwrap();
    println!("{}", result);

    println!("====================================================");
    let agent2 = client
        .agent(BIGMODEL_GLM_4_7_FLASH)
        .name("ai agent")
        .tool(TavilyTool::new_with_keys(tavily_api_keys))
        .preamble("你是一个ai助手")
        .build();

    let result = agent2.prompt("获取一周内AI最新动态").await.unwrap();
    println!("{}", result);
}
