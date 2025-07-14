use rig::completion::{CompletionError, CompletionRequest};
use rig::extractor::ExtractorBuilder;
use rig::message::{MessageError, Text};
use rig::providers::openai;
use rig::{OneOrMany, completion, message};
use rig::client::{AsEmbeddings, AsTranscription, CompletionClient, ProviderClient};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use rig::providers::openai::send_compatible_streaming_request;
use rig::streaming::StreamingCompletionResponse;

use crate::json_utils;

// ================================================================
// BIGMODEL 客户端
// ================================================================
const BIGMODEL_API_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4/";

#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    http_client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: &str) -> Self {
        Self::from_url(api_key, BIGMODEL_API_BASE_URL)
    }

    pub fn from_url(api_key: &str, base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: reqwest::Client::builder()
                .default_headers({
                    let mut headers = reqwest::header::HeaderMap::new();
                    headers.insert(
                        "Authorization",
                        format!("Bearer {api_key}")
                            .parse()
                            .expect("Bearer token should parse"),
                    );
                    headers
                })
                .build()
                .expect("bigmodel reqwest client should build"),
        }
    }



    fn post(&self, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}/{}", self.base_url, path).replace("//", "/");
        self.http_client.post(url)
    }

    pub fn completion_model(&self, model: &str) -> CompletionModel {
        CompletionModel::new(self.clone(), model)
    }



    /// 为completion模型创建提取构建器
    pub fn extractor<T: JsonSchema + for<'a> Deserialize<'a> + Serialize + Send + Sync>(
        &self,
        model: &str,
    ) -> ExtractorBuilder<T, CompletionModel> {
        ExtractorBuilder::new(self.completion_model(model))
    }
}

impl ProviderClient for Client {
    fn from_env() -> Self
    where
        Self: Sized
    {
        let api_key = std::env::var("BIGMODEL_API_KEY").expect("BIGMODEL_KEY not set");
        Self::new(&api_key)
    }
}

impl AsTranscription for Client {}

impl AsEmbeddings for Client {}



impl CompletionClient for Client {
    type CompletionModel = CompletionModel;

    fn completion_model(&self, model: &str) -> Self::CompletionModel {
        CompletionModel::new(self.clone(), model)
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ApiResponse<T> {
    Ok(T),
    Err(ApiErrorResponse),
}

// ================================================================
// Bigmodel Completion API
// ================================================================
pub const BIGMODEL_GLM_4_FLASH: &str = "glm-4-flash";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
    pub created: i64,
    pub id: String,
    pub model: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
    pub usage: Usage,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
#[serde(tag = "role", rename_all = "lowercase")]
pub enum Message {
    User {
        content: String,
    },
    Assistant {
        content: Option<String>,
        #[serde(default, deserialize_with = "json_utils::null_or_vec")]
        tool_calls: Vec<ToolCall>,
    },
    System {
        content: String,
    },
    #[serde(rename = "tool")]
    ToolResult {
        tool_call_id: String,
        content: String,
    },
}

impl Message {
    pub fn system(content: &str) -> Message {
        Message::System {
            content: content.to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ToolResultContent {
    text: String,
}
impl TryFrom<message::ToolResultContent> for ToolResultContent {
    type Error = MessageError;
    fn try_from(value: message::ToolResultContent) -> Result<Self, Self::Error> {
        let message::ToolResultContent::Text(Text { text }) = value else {
            return Err(MessageError::ConversionError(
                "Non-text tool results not supported".into(),
            ));
        };

        Ok(Self { text })
    }
}

impl TryFrom<message::Message> for Message {
    type Error = MessageError;

    fn try_from(message: message::Message) -> Result<Self, Self::Error> {
        Ok(match message {
            message::Message::User { content } => {
                let mut texts = Vec::new();
                let mut images = Vec::new();

                for uc in content.into_iter() {
                    match uc {
                        message::UserContent::Text(message::Text { text }) => texts.push(text),
                        message::UserContent::Image(img) => images.push(img.data),
                        message::UserContent::ToolResult(result) => {
                            let content = result
                                .content
                                .into_iter()
                                .map(ToolResultContent::try_from)
                                .collect::<Result<Vec<ToolResultContent>, MessageError>>()?;

                            let content = OneOrMany::many(content).map_err(|x| {
                                MessageError::ConversionError(format!(
                                    "Couldn't make a OneOrMany from a list of tool results: {x}"
                                ))
                            })?;

                            return Ok(Message::ToolResult {
                                tool_call_id: result.id,
                                content: content.first().text,
                            });
                        }
                        _ => {}
                    }
                }

                let collapsed_content = texts.join(" ");

                Message::User {
                    content: collapsed_content,
                }
            }
            message::Message::Assistant { content, .. } => {
                let mut texts = Vec::new();
                let mut tool_calls = Vec::new();

                for ac in content.into_iter() {
                    match ac {
                        message::AssistantContent::Text(message::Text { text }) => texts.push(text),
                        message::AssistantContent::ToolCall(tc) => tool_calls.push(tc.into()),
                    }
                }

                let collapsed_content = texts.join(" ");

                Message::Assistant {
                    content: Some(collapsed_content),
                    tool_calls,
                }
            }
        })
    }
}

impl From<message::ToolResult> for Message {
    fn from(tool_result: message::ToolResult) -> Self {
        let content = match tool_result.content.first() {
            message::ToolResultContent::Text(text) => text.text,
            message::ToolResultContent::Image(_) => String::from("[Image]"),
        };

        Message::ToolResult {
            tool_call_id: tool_result.id,
            content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCall {
    pub function: CallFunction,
    pub id: String,
    pub index: usize,
    #[serde(default)]
    pub r#type: ToolType,
}

impl From<message::ToolCall> for ToolCall {
    fn from(tool_call: message::ToolCall) -> Self {
        Self {
            id: tool_call.id,
            index: 0,
            r#type: ToolType::Function,
            function: CallFunction {
                name: tool_call.function.name,
                arguments: tool_call.function.arguments,
            },
        }
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    #[default]
    Function,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CallFunction {
    pub name: String,
    #[serde(with = "json_utils::stringified_json")]
    pub arguments: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    #[serde(rename = "finish_reason")]
    pub finish_reason: String,
    pub index: i64,
    pub message: Message,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: i64,
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: i64,
    #[serde(rename = "total_tokens")]
    pub total_tokens: i64,
}

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(response: CompletionResponse) -> Result<Self, Self::Error> {
        let choice = response.choices.first().ok_or_else(|| {
            CompletionError::ResponseError("Response contained no choices".to_owned())
        })?;

        match &choice.message {
            Message::Assistant {
                tool_calls,
                content,
            } => {
                if !tool_calls.is_empty() {
                    let tool_result = tool_calls
                        .iter()
                        .map(|call| {
                            completion::AssistantContent::tool_call(
                                &call.function.name,
                                &call.function.name,
                                call.function.arguments.clone(),
                            )
                        })
                        .collect::<Vec<_>>();

                    let choice = OneOrMany::many(tool_result).map_err(|_| {
                        CompletionError::ResponseError(
                            "Response contained no message or tool call (empty)".to_owned(),
                        )
                    })?;
                    // let usage = completion::Usage {
                    //     input_tokens: response.usage.prompt_tokens as u64,
                    //     output_tokens: (response.usage.total_tokens - response.usage.prompt_tokens)
                    //         as u64,
                    //     total_tokens: response.usage.total_tokens as u64,
                    // };
                    tracing::debug!("response choices: {:?}: ", choice);
                    Ok(completion::CompletionResponse {
                        choice,
                        // usage,
                        raw_response: response,
                    })
                } else {
                    let choice = OneOrMany::one(message::AssistantContent::Text(Text {
                        text: content.clone().unwrap_or_else(|| "".to_owned()),
                    }));
                    // let usage = completion::Usage {
                    //     input_tokens: response.usage.prompt_tokens as u64,
                    //     output_tokens: (response.usage.total_tokens - response.usage.prompt_tokens)
                    //         as u64,
                    //     total_tokens: response.usage.total_tokens as u64,
                    // };
                    Ok(completion::CompletionResponse {
                        choice,
                        // usage,
                        raw_response: response,
                    })
                }
            }
            // Message::Assistant { tool_calls } => {}
            _ => Err(CompletionError::ResponseError(
                "Chat response does not include an assistant message".into(),
            )),
        }
    }
}

#[derive(Clone)]
pub struct CompletionModel {
    client: Client,
    pub model: String,
}



// 函数定义
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomFunctionDefinition {
    #[serde(rename = "type")]
    pub type_field: String,
    pub function: Function,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

impl CompletionModel {
    pub fn new(client: Client, model: &str) -> Self {
        Self {
            client,
            model: model.to_string(),
        }
    }

    fn create_completion_request(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<Value, CompletionError> {
        // 构建消息顺序（上下文、聊天历史、提示）
        let mut partial_history = vec![];
        if let Some(docs) = completion_request.normalized_documents() {
            partial_history.push(docs);
        }
        partial_history.extend(completion_request.chat_history);

        // 使用前言初始化完整历史（如果不存在则为空）
        let mut full_history: Vec<Message> = completion_request
            .preamble
            .map_or_else(Vec::new, |preamble| vec![Message::system(&preamble)]);

        // 转换并扩展其余历史
        full_history.extend(
            partial_history
                .into_iter()
                .map(message::Message::try_into)
                .collect::<Result<Vec<Message>, _>>()?,
        );

        let request = if completion_request.tools.is_empty() {
            json!({
                "model": self.model,
                "messages": full_history,
                "temperature": completion_request.temperature,
            })
        } else {
            // tools
            let tools = completion_request
                .tools
                .into_iter()
                .map(|item| {
                    let custom_function = Function {
                        name: item.name,
                        description: item.description,
                        parameters: item.parameters,
                    };
                    CustomFunctionDefinition {
                        type_field: "function".to_string(),
                        function: custom_function,
                    }
                })
                .collect::<Vec<_>>();

            tracing::debug!("tools: {:?}", tools);

            json!({
                "model": self.model,
                "messages": full_history,
                "temperature": completion_request.temperature,
                "tools": tools,
                "tool_choice": "auto",
            })
        };

        let request = if let Some(params) = completion_request.additional_params {
            json_utils::merge(request, params)
        } else {
            request
        };

        Ok(request)
    }
}

/// 同步请求
impl completion::CompletionModel for CompletionModel {
    type Response = CompletionResponse;
    type StreamingResponse = openai::StreamingCompletionResponse;

    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<CompletionResponse>, CompletionError> {
        tracing::debug!("create_completion_request========");
        let request = self.create_completion_request(completion_request)?;

        tracing::debug!(
            "request: \r\n {}",
            serde_json::to_string_pretty(&request).unwrap()
        );

        let response = self
            .client
            .post("/chat/completions")
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let data: Value = response.json().await.expect("api error");
            tracing::debug!("response: {}", serde_json::to_string_pretty(&data).unwrap());
            let data: ApiResponse<CompletionResponse> =
                serde_json::from_value(data).expect("deserialize completion response");
            match data {
                ApiResponse::Ok(response) => {
                    tracing::info!(target: "rig",
                        "bigmodel completion token usage: {:?}",
                        response.usage
                    );
                    response.try_into()
                }
                ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
            }
        } else {
            Err(CompletionError::ProviderError(response.text().await?))
        }
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let mut request = self.create_completion_request(request)?;

        request = json_utils::merge(request, json!({"stream": true}));

        let builder = self.client.post("/chat/completions").json(&request);

        send_compatible_streaming_request(builder).await
    }
}


