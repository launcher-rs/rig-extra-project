//! 获取openrouter中的模型列表

use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub slug: String,
    #[serde(rename = "updated_at")]
    pub updated_at: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub name: String,
    #[serde(rename = "short_name")]
    pub short_name: String,
    pub author: String,
    pub description: String,
    #[serde(rename = "context_length")]
    pub context_length: i64,
    #[serde(rename = "input_modalities")]
    pub input_modalities: Vec<String>,
    #[serde(rename = "output_modalities")]
    pub output_modalities: Vec<String>,
    #[serde(rename = "has_text_output")]
    pub has_text_output: bool,
    pub group: String,
    pub permaslug: String,
    #[serde(rename = "endpoint")]
    #[serde(default)]
    pub endpoint: Option<Endpoint>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Endpoint {
    pub name: String,
    #[serde(rename = "context_length")]
    pub context_length: i64,
    #[serde(rename = "model_variant_slug")]
    pub model_variant_slug: String,
    #[serde(rename = "model_variant_permaslug")]
    pub model_variant_permaslug: String,
    #[serde(rename = "is_free")]
    pub is_free: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelsResponse {
    pub data: Vec<Model>,
}

/// 获取 openrouter 模型列表
pub async fn fetch_openrouter_model_list() -> Result<Vec<Model>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://openrouter.ai/api/frontend/models")
        .header(
            reqwest::header::USER_AGENT,
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/139.0.0.0 Safari/537.36",
        )
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .await?;

    if !resp.status().is_success() {
        return Err(format!("request failed: {}", resp.status()).into());
    }

    let parsed: ModelsResponse = resp.json().await?;
    Ok(parsed.data)
}
