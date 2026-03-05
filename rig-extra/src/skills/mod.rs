//! Skills 模块
//!
//! 参考 Claude Code Skills 设计：
//! - Skill = SKILL.md (指令) + scripts/ (可执行脚本)
//! - 模型根据指令自己执行任务

pub mod audit;
pub mod loader;
pub mod manifest;
pub mod tool;
