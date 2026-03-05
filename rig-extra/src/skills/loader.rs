//! Skill 加载器

use crate::skills::manifest::{LoadedSkillInfo, SkillManifest, ToolSpec};
use anyhow::{bail, Context, Result};
use std::path::PathBuf;

/// Skill 加载器
pub struct SkillLoader {
    path: PathBuf,
}

impl SkillLoader {
    /// 从目录创建加载器
    pub fn from_directory(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
        }
    }

    /// 加载 skill
    pub fn load(&self) -> Result<LoadedSkill> {
        let path = self.path.clone();

        if !path.exists() {
            bail!("Skill directory does not exist: {}", path.display());
        }
        if !path.is_dir() {
            bail!("Skill path must be a directory: {}", path.display());
        }

        // 尝试加载 SKILL.toml
        let toml_path = path.join("SKILL.toml");
        if toml_path.exists() {
            let content = std::fs::read_to_string(&toml_path)
                .with_context(|| format!("failed to read {}", toml_path.display()))?;
            let manifest = toml::from_str::<SkillManifest>(&content)
                .with_context(|| format!("failed to parse SKILL.toml"))?;

            return Ok(LoadedSkill {
                info: LoadedSkillInfo {
                    path,
                    manifest,
                    markdown_content: None,
                },
            });
        }

        // 尝试加载 SKILL.md
        let md_path = path.join("SKILL.md");
        if md_path.exists() {
            let content =
                std::fs::read_to_string(&md_path).with_context(|| format!("failed to read SKILL.md"))?;

            // 尝试从 frontmatter 解析
            if let Some(manifest) = SkillManifest::from_markdown(&content) {
                return Ok(LoadedSkill {
                    info: LoadedSkillInfo {
                        path,
                        manifest,
                        markdown_content: Some(content),
                    },
                });
            }

            // 如果没有 frontmatter，创建基本的 manifest
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            return Ok(LoadedSkill {
                info: LoadedSkillInfo {
                    path,
                    manifest: SkillManifest {
                        name: crate::skills::manifest::SkillName(name),
                        description: crate::skills::manifest::SkillDescription::default(),
                        tools: vec![],
                        prompts: vec![crate::skills::manifest::PromptSpec {
                            name: "default".to_string(),
                            content: content.clone(),
                        }],
                    },
                    markdown_content: Some(content),
                },
            });
        }

        bail!("Skill must include SKILL.md or SKILL.toml");
    }

    /// 加载并审计 skill
    #[cfg(feature = "rig-extra-skills")]
    pub fn load_with_audit(&self) -> Result<(LoadedSkill, crate::skills::audit::SkillAuditReport)> {
        use crate::skills::audit;

        let skill_dir = &self.path;
        let report = audit::audit_skill_directory(skill_dir)?;

        if !report.is_clean() {
            bail!("Skill audit failed: {}", report.summary());
        }

        let skill = self.load()?;
        Ok((skill, report))
    }
}

/// 已加载的 Skill
pub struct LoadedSkill {
    pub info: LoadedSkillInfo,
}

impl LoadedSkill {
    /// 获取 preamble (用于设置 agent 的初始指令)
    pub fn preamble(&self) -> String {
        // 从 prompts 中获取 default 或第一个 prompt
        self.info
            .prompts()
            .first()
            .map(|p| p.content.clone())
            .unwrap_or_else(|| {
                self.info.description().to_string()
            })
    }

    /// 获取所有 tools 的定义
    pub fn tool_specs(&self) -> &[ToolSpec] {
        &self.info.manifest.tools
    }

    /// 检查是否包含 tools
    pub fn has_tools(&self) -> bool {
        !self.info.manifest.tools.is_empty()
    }
}

/// 从目录批量加载多个 skills
pub fn load_skills_from_directory(dir: impl Into<PathBuf>) -> Result<Vec<LoadedSkill>> {
    let dir = dir.into();
    if !dir.is_dir() {
        bail!("Expected directory: {}", dir.display());
    }

    let mut skills = Vec::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            match SkillLoader::from_directory(&path).load() {
                Ok(skill) => skills.push(skill),
                Err(e) => {
                    tracing::warn!("Failed to load skill from {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(skills)
}
