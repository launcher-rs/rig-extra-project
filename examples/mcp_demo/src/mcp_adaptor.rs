// 将 MCP 转换为 rig tool

use rig_extra::completion::ToolDefinition;
use rig_extra::tool::{ToolDyn, ToolError};
use rig_extra::{completion, tool};
use rmcp::model::{CallToolRequestParam, CallToolResult};
use rmcp::serde_json;
use rmcp::serde_json::json;
use rmcp::service::ServerSink;
use std::future::Future;
use std::pin::Pin;

pub struct McpToolAdaptor {
    pub tool: rmcp::model::Tool,
    pub server: ServerSink,
}

impl ToolDyn for McpToolAdaptor {
    fn name(&self) -> String {
        self.tool.name.to_string()
    }

    fn definition(
        &self,
        _prompt: String,
    ) -> Pin<Box<dyn Future<Output = ToolDefinition> + Send + Sync + '_>> {
        Box::pin(std::future::ready(completion::ToolDefinition {
            name: self.name(),
            description: self
                .tool
                .description
                .clone()
                .unwrap_or_default()
                .to_string(),
            // 参数：self.tool.schema_as_json_value(),
            parameters: json!({
                "type": "object",
                "properties": self.tool.schema_as_json_value(),
            }),
        }))
    }

    fn call(
        &self,
        args: String,
    ) -> Pin<Box<dyn Future<Output = Result<String, ToolError>> + Send + Sync + '_>> {
        let server = self.server.clone();

        Box::pin(async move {
            println!("工具调用{}, 参数:{}", self.tool.name.clone(), args);

            let args = if args.is_empty() {
                None
            } else {
                serde_json::from_str(&args).map_err(tool::ToolError::JsonError)?
            };
            let call_mcp_tool_result = server
                .call_tool(CallToolRequestParam {
                    name: self.tool.name.clone(),
                    arguments: args,
                })
                .await
                .map_err(|e| tool::ToolError::ToolCallError(Box::new(e)))?;
            println!("call_mcp_tool_result {call_mcp_tool_result:?}");

            Ok(convert_mcp_call_tool_result_to_string(call_mcp_tool_result))
        })
    }
}

pub fn convert_mcp_call_tool_result_to_string(result: CallToolResult) -> String {
    serde_json::to_string(&result).unwrap()
}
