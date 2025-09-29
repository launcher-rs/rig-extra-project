//! 获取github趋势榜: https://github.com/trending

use rig::completion::ToolDefinition;
use rig::tool::Tool;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Serialize)]
pub struct GithubTrendingTool;

#[derive(Deserialize, Serialize, Default)]
pub struct EmptyArgs {}

#[derive(Debug, thiserror::Error)]
pub enum GithubTrendingToolError {
    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Selector parse failed: {0}")]
    Selector(String),
}

#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
/// github趋势榜
pub struct GithubTrendingData {
    /// 代码仓库标题
    pub title: String,
    /// 代码仓库描述
    pub description: String,
    /// 代码仓库链接
    pub url: String,
    /// 编程语言
    pub language: String,
    /// 代码仓库star数量
    pub stars: String,
    /// 代码仓库fork数量
    pub forks: String,
    /// 代码仓库今天star数量
    pub today_stars: String,
}
impl GithubTrendingTool {
    async fn get_github_trending(
        &self,
    ) -> Result<Vec<GithubTrendingData>, GithubTrendingToolError> {
        let resp = reqwest::get("https://github.com/trending").await?;
        let content = resp.text().await?;

        let document = Html::parse_document(&content);
        let selector = Selector::parse(".Box-row")
            .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;

        let mut results = Vec::new();
        for element in document.select(&selector) {
            let title_selector = Selector::parse("h2 a")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let title = element
                .select(&title_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            let title = title.replace("\n", "").trim().to_string();
            let title = title.replace(" ", "").trim().to_string();
            // println!("title: {}", title);

            let desc_selector = Selector::parse("p.col-9")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let description = element
                .select(&desc_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            // println!("description: {}", description);

            // 提取链接
            let link_selector = Selector::parse("h2 a")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let link = element
                .select(&link_selector)
                .next()
                .and_then(|element| element.value().attr("href"))
                .map(|href| format!("https://github.com{href}"))
                .unwrap_or_default();
            // println!("link: {}", link);

            let language_selector = Selector::parse("span[itemprop='programmingLanguage']")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let language = element
                .select(&language_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            // println!("language: {}", language);

            // stars 数量
            let stars_selector = Selector::parse("a[href$='/stargazers']")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let stars = element
                .select(&stars_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            // println!("stars: {}", stars);
            // forks数量
            let forks_selector = Selector::parse("a[href$='/forks']")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let forks = element
                .select(&forks_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            // println!("forks: {}", forks);

            // 今日star数
            let stars_today_selector = Selector::parse("span.d-inline-block.float-sm-right")
                .map_err(|e| GithubTrendingToolError::Selector(e.to_string()))?;
            let stars_today = element
                .select(&stars_today_selector)
                .next()
                .map(|element| element.text().collect::<Vec<_>>().join(""))
                .unwrap_or_default()
                .trim()
                .to_string();
            // println!("stars: {}", stars_today);

            let data = GithubTrendingData {
                title,
                description,
                url: link,
                language,
                stars,
                forks,
                today_stars: stars_today,
            };
            results.push(data);
        }

        Ok(results)
    }
}

impl Tool for GithubTrendingTool {
    const NAME: &'static str = "GithubTrendingTool";
    type Error = GithubTrendingToolError;
    type Args = EmptyArgs;
    type Output = Vec<GithubTrendingData>;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取github趋势榜单".to_string(),
            parameters: json!({
                "type": "object",
                "title": "No parameters",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, _args: Self::Args) -> Result<Self::Output, Self::Error> {
        let data = self.get_github_trending().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extra_providers::bigmodel;
    use crate::extra_providers::bigmodel::BIGMODEL_GLM_4_FLASH;
    use config::Config;
    use rig::client::CompletionClient;
    use rig::completion::Prompt;
    #[tokio::test]
    async fn test_github_trending() {
        let current_dir = format!("{}\\..\\Settings", env!("CARGO_MANIFEST_DIR"));

        let settings = Config::builder()
            .add_source(config::File::with_name(current_dir.as_str()))
            .build()
            .unwrap_or_default();

        let api_key = settings
            .get_string("bigmodel_api_key")
            .expect("Missing API Key in Settings");

        let client = bigmodel::Client::new(api_key.as_str());
        let agent = client
            .agent(BIGMODEL_GLM_4_FLASH)
            .tool(GithubTrendingTool)
            .name("ai agent")
            .preamble("你是一个ai助手")
            .build();
        let result = agent
            .prompt("获取GitHub趋势榜")
            .multi_turn(1)
            .await
            .unwrap();
        println!("{}", result);
    }
}
