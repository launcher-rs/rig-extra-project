//! [serpapi](https://serpapi.com/) 注册需要邮箱，需要验证手机号
//! serpapi 免费版: 每个月可以免费使用250次

use backon::{ExponentialBuilder, Retryable};
use reqwest::Client;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use std::collections::HashMap;
// use rand::prelude::IndexedRandom; // removed to avoid non-Send RNG in async

/// serpaapi 获取谷歌搜索
/// 支持多个apikey,随机选择一个api key
pub struct SerpapiTool {
    /// api key
    pub api_keys: Vec<String>,
}

impl SerpapiTool {
    /// 单个api
    pub fn new<S: Into<String>>(api_key: S) -> Self {
        Self {
            api_keys: vec![api_key.into()],
        }
    }

    /// 多个api
    pub fn new_with_keys<S: Into<String>>(api_key: Vec<S>) -> Self {
        if api_key.is_empty() {
            panic!("Api key should be greater than 0");
        }
        Self {
            api_keys: api_key.into_iter().map(|k| k.into()).collect(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Serpapi Error")]
pub enum SerpapiError {
    #[error("Json Error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Request Error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("Custom Error")]
    CustomError(String),
}

#[derive(Deserialize, JsonSchema, Debug, Clone)]
/// Serpapi搜索参数
pub struct SerpapiArgs {
    /// 搜索时间范围,
    /// 参数: `qdr:h`: 最近一小时, `qdr:d`: 最近一天, `qdr:w`: 最近一周,`qdr:m`: 最近一月,`qdr:y`: 最近一年
    pub tbs: Option<String>,
    /// 搜索国家： `us`: 美国,`uk`: 英国,`cn`: 中国,`ru`: 俄罗斯,...
    pub gl: Option<String>,
    /// 搜索语言: `en`: 英文,`zh-cn`: 简体中文, `zh-tw`: 繁体中文,ru: 俄文
    pub hl: Option<String>,
    /// 搜索关键词
    pub query: String,
}
impl Tool for SerpapiTool {
    const NAME: &'static str = "Serpapi Tool";
    type Error = SerpapiError;
    type Args = SerpapiArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "使用 Serpapi进行谷歌内容搜索".to_string(),
            parameters: serde_json::to_value(schema_for!(Self::Args)).unwrap(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let config = ExponentialBuilder::default();
        let api_keys = self.api_keys.clone();
        let args_clone = args.clone();

        let result = (move || {
            let args = args_clone.clone();
            let api_keys = api_keys.clone();
            async move {
                tracing::debug!("args: {:?}", args);

                // 构建搜索参数
                let mut params = HashMap::new();
                params.insert("engine".to_string(), "google".to_string());
                params.insert("q".to_string(), args.query);
                if let Some(tbs) = args.tbs {
                    params.insert("tbs".to_string(), tbs);
                }
                if let Some(gl) = args.gl {
                    params.insert("gl".to_string(), gl);
                }
                if let Some(hl) = args.hl {
                    params.insert("hl".to_string(), hl);
                }

                // 选择一个 api key（基于时间的简单随机）
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let idx = (now.subsec_nanos() as usize) % api_keys.len();
                let api_key = api_keys[idx].clone();
                params.insert("api_key".to_string(), api_key);

                // 执行搜索
                let client = Client::new();
                let response = client
                    .get("https://serpapi.com/search")
                    .query(&params)
                    .send()
                    .await?;
                let search_result: serde_json::Value = response.json().await?;
                tracing::info!("search result: {:?}", search_result);
                let organic_results = search_result
                    .get("organic_results")
                    .ok_or(SerpapiError::CustomError("没有organic_results".to_string()))?;
                let result = serde_json::to_string(organic_results)?;
                tracing::debug!("result: {}", result);
                Ok(result)
            }
        })
        .retry(config)
        .sleep(tokio::time::sleep)
        .notify(|err: &SerpapiError, _dur: std::time::Duration| {
            tracing::error!("重试错误: {:?}", err);
        })
        .await?;

        Ok(result)
    }
}
