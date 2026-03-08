use crate::json_utils;
use crate::json_utils::merge;
use bytes::Bytes;
use rig::agent::Text;
use rig::client::{
    BearerAuth, Capabilities, Capable, DebugExt, Nothing, Provider, ProviderBuilder,
};
use rig::completion::{CompletionError, CompletionRequest};
use rig::http_client::HttpClientExt;
use rig::message::MessageError;
use rig::providers::openai;
use rig::providers::openai::send_compatible_streaming_request;
use rig::streaming::StreamingCompletionResponse;
use rig::{OneOrMany, client, completion, http_client, message};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tracing::{Instrument, info_span};

// use rig::providers::openai::{Message as OpenAIMessage};

const BIGMODEL_API_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4/";

#[derive(Debug, Default, Clone, Copy)]
pub struct BigmodelExt;

#[derive(Debug, Default, Clone, Copy)]

pub struct BigmodelBuilder;

type BigmodelApiKey = BearerAuth;

#[derive(Clone, Debug)]
pub struct CompletionModel<T = reqwest::Client> {
    client: Client<T>,
    pub model: String,
}

impl<T> CompletionModel<T> {
    pub fn new(client: Client<T>, model: impl Into<String>) -> Self {
        Self {
            client,
            model: model.into(),
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

impl Provider for BigmodelExt {
    const VERIFY_PATH: &'static str = "api/tags";
    type Builder = BigmodelBuilder;
}

/// provider有那些功能
impl<H> Capabilities<H> for BigmodelExt {
    type Completion = Capable<CompletionModel<H>>;
    type Embeddings = Nothing;
    type Transcription = Nothing;
    type ModelListing = Nothing;

    // #[cfg(feature = "image")]
    // type ImageGeneration = Nothing;
    //
    // #[cfg(feature = "audio")]
    // type AudioGeneration = Nothing;
}

impl DebugExt for BigmodelExt {}

impl ProviderBuilder for BigmodelBuilder {
    type Extension<H>
        = BigmodelExt
    where
        H: HttpClientExt;
    type ApiKey = BigmodelApiKey;
    const BASE_URL: &'static str = BIGMODEL_API_BASE_URL;

    fn build<H>(
        _builder: &client::ClientBuilder<Self, Self::ApiKey, H>,
    ) -> http_client::Result<Self::Extension<H>>
    where
        H: HttpClientExt,
    {
        Ok(BigmodelExt)
    }
}

pub type Client<H = reqwest::Client> = client::Client<BigmodelExt, H>;
pub type ClientBuilder<H = reqwest::Client> = client::ClientBuilder<BigmodelBuilder, String, H>;

/// Rust 的孤儿规则（Orphan Rule）
// impl ProviderClient for Client{
//     type Input = String;
//
//     fn from_env() -> Self {
//         let api_key = std::env::var("BIGMODEL_API_KEY").expect("BIGMODEL_API_KEY not set");
//         Self::new(&api_key).unwrap()
//     }
//
//     fn from_val(input: Self::Input) -> Self {
//         Self::new(&input).unwrap()
//     }
// }

// ---------- API Error and Response Structures ----------
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

pub const BIGMODEL_GLM_4_7_FLASH: &str = "glm-4.7-flash";

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionResponse {
    pub choices: Vec<Choice>,
    pub created: i64,
    pub id: String,
    pub model: String,
    #[serde(rename = "request_id")]
    pub request_id: String,
    pub usage: Option<Usage>,
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
                        _ => {}
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct PromptTokensDetails {
    /// Cached tokens from prompt caching
    #[serde(default)]
    pub cached_tokens: usize,
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
                    let usage = response
                        .usage
                        .as_ref()
                        .map(|usage| completion::Usage {
                            input_tokens: usage.prompt_tokens as u64,
                            output_tokens: (usage.total_tokens - usage.prompt_tokens) as u64,
                            total_tokens: usage.total_tokens as u64,
                            cached_input_tokens: usage
                                .prompt_tokens_details
                                .as_ref()
                                .map(|d| d.cached_tokens as u64)
                                .unwrap_or(0),
                        })
                        .unwrap_or_default();
                    tracing::debug!("response choices: {:?}: ", choice);
                    Ok(completion::CompletionResponse {
                        choice,
                        usage,
                        raw_response: response,
                        message_id: None,
                    })
                } else {
                    let choice = OneOrMany::one(message::AssistantContent::Text(Text {
                        text: content.clone().unwrap_or_else(|| "".to_owned()),
                    }));
                    let usage = response
                        .usage
                        .as_ref()
                        .map(|usage| completion::Usage {
                            input_tokens: usage.prompt_tokens as u64,
                            output_tokens: (usage.total_tokens - usage.prompt_tokens) as u64,
                            total_tokens: usage.total_tokens as u64,
                            cached_input_tokens: usage
                                .prompt_tokens_details
                                .as_ref()
                                .map(|d| d.cached_tokens as u64)
                                .unwrap_or(0),
                        })
                        .unwrap_or_default();
                    Ok(completion::CompletionResponse {
                        choice,
                        usage,
                        raw_response: response,
                        message_id: None,
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

/// 同步请求
impl<T> completion::CompletionModel for CompletionModel<T>
where
    T: HttpClientExt + Clone + Send + std::fmt::Debug + Default + 'static,
{
    type Response = CompletionResponse;
    type StreamingResponse = openai::StreamingCompletionResponse;
    type Client = Client<T>;

    fn make(client: &Self::Client, model: impl Into<String>) -> Self {
        Self::new(client.clone(), model.into())
    }

    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat",
                gen_ai.operation.name = "chat",
                gen_ai.provider.name = "groq",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = tracing::field::Empty,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        span.record("gen_ai.system_instructions", &completion_request.preamble);

        let request = self.create_completion_request(completion_request)?;

        if tracing::enabled!(tracing::Level::TRACE) {
            tracing::trace!(target: "rig::completions",
                "Groq completion request: {}",
                serde_json::to_string_pretty(&request)?
            );
        }

        let body = serde_json::to_vec(&request)?;
        let req = self
            .client
            .post("/chat/completions")?
            .body(body)
            .map_err(|e| http_client::Error::Instance(e.into()))?;

        let async_block = async move {
            let response = self.client.send::<_, Bytes>(req).await?;
            let status = response.status();
            let response_body = response.into_body().into_future().await?.to_vec();

            let tt = response_body.clone();
            let response = serde_json::from_slice::<serde_json::Value>(&tt)?;
            println!(
                "response:\r\n {}",
                serde_json::to_string_pretty(&response).unwrap()
            );

            if status.is_success() {
                match serde_json::from_slice::<ApiResponse<CompletionResponse>>(&response_body)? {
                    ApiResponse::Ok(response) => {
                        let span = tracing::Span::current();
                        span.record("gen_ai.response.id", response.id.clone());
                        span.record("gen_ai.response.model_name", response.model.clone());
                        if let Some(ref usage) = response.usage {
                            span.record("gen_ai.usage.input_tokens", usage.prompt_tokens);
                            span.record(
                                "gen_ai.usage.output_tokens",
                                usage.total_tokens - usage.prompt_tokens,
                            );
                        }

                        if tracing::enabled!(tracing::Level::TRACE) {
                            tracing::trace!(target: "rig::completions",
                                "Groq completion response: {}",
                                serde_json::to_string_pretty(&response)?
                            );
                        }

                        response.try_into()
                    }
                    ApiResponse::Err(err) => Err(CompletionError::ProviderError(err.message)),
                }
            } else {
                Err(CompletionError::ProviderError(
                    String::from_utf8_lossy(&response_body).to_string(),
                ))
            }
        };

        tracing::Instrument::instrument(async_block, span).await
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse<Self::StreamingResponse>, CompletionError> {
        let preamble = request.preamble.clone();

        let mut request = self.create_completion_request(request)?;

        request = merge(request, json!({"stream": true}));

        let body = serde_json::to_vec(&request)?;

        let req = self
            .client
            .post("/chat/completions")?
            .body(body)
            .map_err(|e| http_client::Error::Instance(e.into()))?;

        let span = if tracing::Span::current().is_disabled() {
            info_span!(
                target: "rig::completions",
                "chat_streaming",
                gen_ai.operation.name = "chat_streaming",
                gen_ai.provider.name = "galadriel",
                gen_ai.request.model = self.model,
                gen_ai.system_instructions = preamble,
                gen_ai.response.id = tracing::field::Empty,
                gen_ai.response.model = tracing::field::Empty,
                gen_ai.usage.output_tokens = tracing::field::Empty,
                gen_ai.usage.input_tokens = tracing::field::Empty,
                gen_ai.input.messages = serde_json::to_string(&request.get("messages").unwrap()).unwrap(),
                gen_ai.output.messages = tracing::field::Empty,
            )
        } else {
            tracing::Span::current()
        };

        send_compatible_streaming_request(self.client.clone(), req)
            .instrument(span)
            .await
    }
}
