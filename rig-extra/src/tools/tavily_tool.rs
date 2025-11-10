//! [tavily](https://www.tavily.com/) 注册需要邮箱
//! tavily 免费版: 免费计划: 每月 1,000 API 积分

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::time::Duration;
use tavily2::{SearchResponse, Tavily, TavilyError};

/// tavily 获取谷歌搜索
/// 支持多个apikey,随机选择一个api key
pub struct TavilyTool {
    /// api key
    pub api_keys: Vec<String>,
}

impl TavilyTool {
    /// 单个api
    pub fn new<S: Into<String>>(api_key: S) -> Self {
        Self {
            api_keys: vec![api_key.into()],
        }
    }

    /// 多个api
    pub fn new_with_keys<S: Into<String>>(api_key: Vec<S>) -> Self {
        if api_key.len() == 0 {
            panic!("Api key should be greater than 0");
        }
        Self {
            api_keys: api_key.into_iter().map(|k| k.into()).collect(),
        }
    }
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
/// Tavily
pub struct TavilyArgs {
    /// 搜索关键词
    pub query: String,
}

impl Tool for TavilyTool {
    const NAME: &'static str = "Serpapi Tool";
    type Error = TavilyError;
    type Args = TavilyArgs;
    type Output = SearchResponse;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "使用Tavily进行联网搜索".to_string(),
            parameters: serde_json::to_value(schema_for!(Self::Args)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let tavily = Tavily::builder_with_keys(self.api_keys.clone())
            .timeout(Duration::from_secs(60))
            .max_retries(5)
            .multi_retry(true)
            .build()?;

        let result = tavily.search(args.query).await?;

        Ok(result)
    }
}
