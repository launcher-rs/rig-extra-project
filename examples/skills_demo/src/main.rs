//! Skills Demo - 参考 Claude Code Skills 设计
//!
//! 核心思想:
//! - Skill = SKILL.md (指令) + scripts/ (可执行脚本)
//! - 模型根据指令自己决定如何执行，不需要 handlers
//!
//! 运行: cargo run -p skills_demo

use config::Config;
use rig_extra::client::CompletionClient;
use rig_extra::completion::Prompt;
use rig_extra::extra_providers::bigmodel;
use rig_extra::extra_providers::bigmodel::BIGMODEL_GLM_4_7_FLASH;
use rig_extra::skills::loader::SkillLoader;
use rig_extra::tool::ToolDyn;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

fn get_skill_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_default()
        .join("skills")
}

/// 执行命令的工具 (模拟 Claude Code 的执行能力)
fn execute_shell(command: &str) -> String {
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", command])
        .output();
    
    #[cfg(not(target_os = "windows"))]
    let output = Command::new("sh")
        .args(["-c", command])
        .output();

    match output {
        Ok(o) => {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout).to_string()
            } else {
                format!("Error: {}", String::from_utf8_lossy(&o.stderr))
            }
        }
        Err(e) => format!("Failed to execute: {}", e),
    }
}

/// ExecuteTool 的参数
#[derive(Debug, Deserialize)]
struct ExecuteArgs {
    command: String,
}

/// ExecuteTool 错误
#[derive(Debug, Error)]
pub enum ExecuteError {
    #[error("Execute error: {0}")]
    Execute(String),
}

#[tokio::main]
async fn main() {
    // 1. 加载配置
    let api_key = Config::builder()
        .add_source(config::File::with_name("Settings"))
        .build()
        .unwrap_or_default()
        .get_string("bigmodel_api_key")
        .expect("Missing API Key");

    let skill_dir = get_skill_dir();
    println!("Skill 目录: {:?}", skill_dir);

    // 2. 加载 weather skill (不需要 handlers!)
    let weather_path = skill_dir.join("weather");
    let weather_skill = SkillLoader::from_directory(&weather_path)
        .load()
        .expect("Failed to load weather skill");

    println!("\n=== Weather Skill ===");
    println!("Name: {}", weather_skill.info.name());
    println!("Description: {}", weather_skill.info.description());
    println!("Preamble:\n{}", weather_skill.preamble());

    // 3. 加载 search-skill
    let search_path = skill_dir.join("search-skill");
    let search_skill = SkillLoader::from_directory(&search_path)
        .load()
        .expect("Failed to load search skill");

    println!("\n=== Search Skill ===");
    println!("Name: {}", search_skill.info.name());
    println!("Description: {}", search_skill.info.description());

    // 4. 创建可执行的 tool (execute 命令)
    let execute_tool = ExecuteTool;

    // 5. 创建 agent (只需要 preamble，不需要 handlers!)
    let client: bigmodel::Client = bigmodel::Client::new(&api_key).unwrap();
    
    // 组合 preamble
    let combined_preamble = format!(
        "{}\n\n你也可以使用 execute 工具执行 shell 命令。",
        weather_skill.preamble()
    );

    let agent = client
        .agent(BIGMODEL_GLM_4_7_FLASH)
        .name("skill-agent")
        .tool(execute_tool)
        .preamble(&combined_preamble)
        .default_max_turns(3)
        .build();

    // 6. 测试 - 让模型根据 skill 指令自己执行
    println!("\n=== 测试: 查询北京天气 ===");
    println!("(模型会根据 SKILL.md 中的指令自己执行 curl 命令)\n");
    
    match agent.prompt("北京今天的天气怎么样？").await {
        Ok(response) => println!("{}", response),
        Err(e) => println!("Error: {}", e),
    }
}

/// 执行 shell 命令的工具
struct ExecuteTool;

impl rig::tool::Tool for ExecuteTool {
    const NAME: &'static str = "execute";
    type Error = ExecuteError;
    type Args = ExecuteArgs;
    type Output = Value;

    fn name(&self) -> String {
        "execute".to_string()
    }

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "execute".to_string(),
            description: "Execute a shell command and return the output".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let output = execute_shell(&args.command);
        Ok(json!({ "output": output }))
    }
}
