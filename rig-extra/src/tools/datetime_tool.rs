//! 获取时间日期

use chrono::{Datelike, Local};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tyme4rs::tyme::solar::SolarDay;

#[derive(Deserialize, Serialize)]
pub struct DatetimeTool;

#[derive(Deserialize, Serialize, Default)]
pub struct EmptyArgs {}

#[derive(Debug, thiserror::Error)]
#[error("DatetimeTool error")]
pub struct DatetimeToolError;

impl DatetimeTool {
    /// 获取时间信息
    pub fn get_time_info(&self) -> String {
        let now = Local::now();
        let mut info = Vec::new();
        let time_info = format!("当前时间: {}", now.format("%Y-%m-%d %H:%M:%S"));
        info.push(time_info);

        let solar: SolarDay = SolarDay::from_ymd(
            now.year() as isize,
            now.month() as usize,
            now.day() as usize,
        );
        info.push(solar.get_lunar_day().to_string());

        info.push(format!(
            "生肖:{}",
            solar
                .get_lunar_day()
                .get_lunar_month()
                .get_lunar_year()
                .get_sixty_cycle()
                .get_earth_branch()
                .get_zodiac()
        ));
        info.push(format!("星期{}", solar.get_week()));
        info.push(format!("星座:{}", solar.get_constellation()));
        // 农历节气第几天
        info.push(solar.get_term_day().to_string());

        // 公历现代节日
        if let Some(festival) = solar.get_festival() {
            info.push(format!("节日: {festival}"));
        }

        // 法定假日（自2001-12-29起）
        if let Some(legal_holiday) = solar.get_legal_holiday() {
            info.push(legal_holiday.to_string());
        }

        info.join(",")
    }
}
impl Tool for DatetimeTool {
    const NAME: &'static str = "DatetimeTool";
    type Error = DatetimeToolError;
    type Args = EmptyArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "获取当前时间日期的工具,包括获取农历、公历，法定假期、生肖的信息"
                .to_string(),
            parameters: json!({
                "type": "object",
                "title": "No parameters",
                "properties": {},
                "additionalProperties": false
            }),
        }
    }

    async fn call(&self, _args: EmptyArgs) -> Result<Self::Output, Self::Error> {
        let date_info = self.get_time_info();
        Ok(date_info)
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
    async fn test_datetime_tool() {
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
            .tool(DatetimeTool)
            .name("ai agent")
            .preamble("你是一个ai助手")
            .build();

        let result = agent
            .prompt("今天几号了,距离下一个节日还有几天")
            .multi_turn(1)
            .await
            .unwrap();
        println!("{}", result);
    }
}
