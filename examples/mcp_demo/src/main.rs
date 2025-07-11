mod mcp_adaptor;

use crate::mcp_adaptor::McpToolAdaptor;
use config::Config;
use rig_extra::completion::Prompt;
use rig_extra::extra_providers;

use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_FLASH;
use rig_extra::tool::ToolDyn;
use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    transport::SseClientTransport,
};
use rig_extra::client::CompletionClient;

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
    let transport = SseClientTransport::start(mcp_addr)
        .await
        .expect("不能连接MCP服务区");

    let client_info = ClientInfo {
        protocol_version: Default::default(),
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: "test sse client".to_string(),
            version: "0.0.1".to_string(),
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

    let all_tools = client.list_all_tools().await.unwrap();

    let llm_client = extra_providers::bigmodel::Client::new(api_key.as_str());

    let mut agent = llm_client
        .agent(BIGMODEL_GLM_4_FLASH)
        .preamble("你是一个ai助手")
        .build();
    // dynamic_tools 需要配合 向量数据库配合使用

    for tool in all_tools {
        let server = client.peer().clone();
        let rig_tool = McpToolAdaptor {
            tool: tool.clone(),
            server,
        };
        agent.static_tools.push(rig_tool.name());
        agent.tools.add_tool(rig_tool);
    }

    let result = agent.prompt("获取github趋势榜").await.unwrap();

    tracing::info!("结果: {}", result);

    let result = agent.prompt("今天几号了").await.unwrap();
    tracing::info!("结果: {}", result);
}
