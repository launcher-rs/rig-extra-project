//! Skill 清单定义
//!
//! 支持 SKILL.md 和 SKILL.toml 两种格式

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Skill 名称
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillName(pub String);

/// Skill 描述
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillDescription(pub String);

/// Skill 清单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Skill 名称
    pub name: SkillName,
    /// Skill 描述
    #[serde(default)]
    pub description: SkillDescription,
    /// Tool 定义
    #[serde(default)]
    pub tools: Vec<ToolSpec>,
    /// Prompt 模板
    #[serde(default)]
    pub prompts: Vec<PromptSpec>,
}

impl SkillManifest {
    /// 从 TOML 内容解析
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// 从 Markdown 文件解析 (提取 frontmatter 或解析特殊格式)
    pub fn from_markdown(content: &str) -> Option<Self> {
        // 尝试解析 YAML frontmatter
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let frontmatter = &content[3..end];
                if let Ok(manifest) = serde_yaml::from_str::<SkillManifest>(frontmatter) {
                    return Some(manifest);
                }
            }
        }
        None
    }
}

/// Tool 规范
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Tool 名称
    pub name: String,
    /// Tool 描述
    #[serde(default)]
    pub description: String,
    /// Tool 类型: script, shell, function
    #[serde(default = "default_tool_kind")]
    pub kind: String,
    /// 命令 (对于 script/shell 类型)
    #[serde(default)]
    pub command: String,
    /// 参数模式 (对于 function 类型)
    #[serde(default)]
    pub parameters: serde_json::Value,
}

fn default_tool_kind() -> String {
    "function".to_string()
}

/// Prompt 规范
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSpec {
    /// Prompt 名称
    pub name: String,
    /// Prompt 内容
    pub content: String,
}

/// 已加载的 Skill 相关信息
#[derive(Debug, Clone)]
pub struct LoadedSkillInfo {
    /// Skill 路径
    pub path: PathBuf,
    /// 清单
    pub manifest: SkillManifest,
    /// 原始内容 (如果是 Markdown)
    pub markdown_content: Option<String>,
}

impl LoadedSkillInfo {
    /// 获取 skill 名称
    pub fn name(&self) -> &str {
        &self.manifest.name.0
    }

    /// 获取 description
    pub fn description(&self) -> &str {
        &self.manifest.description.0
    }

    /// 获取所有 tool 名称
    pub fn tool_names(&self) -> Vec<&str> {
        self.manifest.tools.iter().map(|t| t.name.as_str()).collect()
    }

    /// 获取所有 prompts
    pub fn prompts(&self) -> Vec<&PromptSpec> {
        self.manifest.prompts.iter().collect()
    }
}
