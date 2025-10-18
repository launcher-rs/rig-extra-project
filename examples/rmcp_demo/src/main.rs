use config::Config;
use rig_extra::completion::Prompt;
use rig_extra::extra_providers;

use rig_extra::client::CompletionClient;
use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_5_FLASH;
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::SseClientTransport,
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // 1. 获取配置
    let settings = Config::builder()
        .add_source(config::File::with_name("Settings"))
        .build()
        .unwrap_or_default();

    let api_key = settings
        .get_string("bigmodel_api_key")
        .expect("Missing API Key in Settings");

    let mcp_addr = settings
        .get_string("mcp_addr")
        .expect("Missing mcp_addr in Settings");

    // 传输层
    // start需要 transport-sse-client-reqwest features
    let transport = SseClientTransport::start(mcp_addr)
        .await
        .expect("不能连接MCP服务器");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            title: None,
            version: "0.0.1".to_string(),
            icons: None,
            website_url: None,
        },
    };

    let client = client_info
        .serve(transport)
        .await
        .inspect_err(|e| {
            tracing::error!("client error: {:#?}", e);
        })
        .unwrap();

    let server_info = client.peer_info();
    tracing::debug!("server info: {:#?}", server_info);

    let tools = client.list_tools(Default::default()).await.unwrap();
    tracing::debug!("tools: {:#?}", tools);

    // 索取所有工具
    let all_tools = client.list_all_tools().await.unwrap();

    let llm_client = extra_providers::bigmodel::Client::new(api_key.as_str());

    let agent = llm_client
        .agent(BIGMODEL_GLM_4_5_FLASH)
        .rmcp_tools(all_tools, client.peer().to_owned())
        .preamble("你是一个ai助手")
        .build();

    let result = agent.prompt("获取github趋势榜").await.unwrap();

    tracing::info!("结果: {}", result);

    let result = agent.prompt("今天几号了").await.unwrap();
    tracing::info!("结果: {}", result);
}
