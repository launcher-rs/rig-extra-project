//! Skill Tool 模块
//!
//! 提供简单的 API 来加载 skill 并转换为 rig Tool

use crate::skills::loader::SkillLoader;
use crate::skills::manifest::ToolSpec;
use anyhow::Result;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use std::pin::Pin;

/// Skill Tool 加载器
/// 
/// # 使用方式
/// 
/// ```ignore
/// use rig_extra::skills::tool::SkillTools;
/// 
/// // 一行代码完成加载 + 注册 handler
/// let tools = SkillTools::from_directory(["./skills/search"])
///     .with_handler("web_search", |args| Ok(json!({...})));
/// 
/// // 获取 rig Tool 和 preamble
/// let (rig_tools, preamble) = tools.build();
/// ```
#[derive(Clone)]
pub struct SkillTools {
    specs: Vec<ToolSpec>,
    handlers: Arc<RwLock<HashMap<String, ToolHandler>>>,
    async_handlers: Arc<RwLock<HashMap<String, AsyncToolHandler>>>,
    skill_dirs: Vec<String>,
}

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Handler not registered: {0}")]
    HandlerNotRegistered(String),
    #[error("Invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("Handler error: {0}")]
    HandlerError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Skill error: {0}")]
    SkillError(String),
}

/// 同步 handler
pub type ToolHandler = Arc<dyn Fn(Value) -> Result<Value, ToolError> + Send + Sync + 'static>;

/// 异步 handler
pub type AsyncToolHandler = Arc<dyn Fn(Value) -> Box<dyn std::future::Future<Output = Result<Value, ToolError>> + Send> + Send + Sync + 'static>;

impl SkillTools {
    /// 从目录加载多个 skills
    pub fn from_directory(dirs: impl IntoIterator<Item = impl AsRef<str>>) -> Result<Self> {
        let mut specs = Vec::new();
        let mut skill_dirs = Vec::new();
        
        for dir in dirs {
            let dir_str = dir.as_ref().to_string();
            let skill = SkillLoader::from_directory(&dir_str).load()?;
            for spec in skill.tool_specs() {
                specs.push(spec.clone());
            }
            skill_dirs.push(dir_str);
        }
        
        Ok(Self {
            specs,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            async_handlers: Arc::new(RwLock::new(HashMap::new())),
            skill_dirs,
        })
    }

    /// 获取 tool 名称列表
    pub fn tool_names(&self) -> Vec<&str> {
        self.specs.iter().map(|s| s.name.as_str()).collect()
    }

    /// 注册 handler
    pub fn with_handler<F>(self, name: &str, handler: F) -> Self 
    where
        F: Fn(Value) -> Result<Value, ToolError> + Send + Sync + 'static,
    {
        let name = name.to_string();
        let h: ToolHandler = Arc::new(handler);
        
        {
            let mut guard = futures::executor::block_on(self.handlers.write());
            guard.insert(name, h);
        }
        self
    }

    /// 链式注册多个 handlers
    pub fn with_handlers<F>(self, handlers: F) -> Self 
    where
        F: Fn(&str, Value) -> Result<Value, ToolError> + Send + Sync + 'static,
    {
        let h = Arc::new(handlers);
        for spec in &self.specs {
            let name = spec.name.clone();
            let spec_name = spec.name.clone();
            let handler_clone = h.clone();
            let handler_fn: ToolHandler = Arc::new(move |args: Value| {
                handler_clone(&spec_name, args)
            });
            {
                let mut guard = futures::executor::block_on(self.handlers.write());
                guard.insert(name, handler_fn);
            }
        }
        self
    }

    /// 注册异步 handler
    pub fn with_async_handler<F, Fut>(self, name: &str, handler: F) -> Self 
    where
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value, ToolError>> + Send + 'static,
    {
        let name = name.to_string();
        let h: AsyncToolHandler = Arc::new(move |args: Value| {
            let fut = handler(args);
            Box::new(fut) as Box<dyn std::future::Future<Output = Result<Value, ToolError>> + Send>
        });
        
        {
            let mut guard = futures::executor::block_on(self.async_handlers.write());
            guard.insert(name, h);
        }
        self
    }

    /// 批量注册异步 handlers
    pub fn with_async_handlers<F, Fut>(self, handlers: F) -> Self 
    where
        F: Fn(&str, Value) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<Value, ToolError>> + Send + 'static,
    {
        let h = Arc::new(handlers);
        for spec in &self.specs {
            let name = spec.name.clone();
            let spec_name = spec.name.clone();
            let handler_clone = h.clone();
            let handler_fn: AsyncToolHandler = Arc::new(move |args: Value| {
                let fut = handler_clone(&spec_name, args);
                Box::new(fut) as Box<dyn std::future::Future<Output = Result<Value, ToolError>> + Send>
            });
            {
                let mut guard = futures::executor::block_on(self.async_handlers.write());
                guard.insert(name, handler_fn);
            }
        }
        self
    }

    /// 构建 rig Tools 和 preamble
    pub fn build(self) -> (Vec<SkillTool>, String) {
        let handlers = self.handlers.clone();
        let async_handlers = self.async_handlers.clone();
        
        // 获取第一个 skill 的 preamble
        let preamble = self.skill_dirs.first()
            .and_then(|dir| {
                SkillLoader::from_directory(dir).load().ok()
            })
            .map(|s| s.preamble())
            .unwrap_or_default();

        let tools = self.specs
            .into_iter()
            .map(|spec| SkillTool {
                spec,
                handlers: handlers.clone(),
                async_handlers: async_handlers.clone(),
            })
            .collect();

        (tools, preamble)
    }

    /// 直接转换为 rig Tools (不含 preamble)
    pub fn to_tools(&self) -> Vec<SkillTool> {
        let handlers = self.handlers.clone();
        let async_handlers = self.async_handlers.clone();
        self.specs
            .iter()
            .map(|spec| SkillTool {
                spec: spec.clone(),
                handlers: handlers.clone(),
                async_handlers: async_handlers.clone(),
            })
            .collect()
    }

    /// 获取 preamble
    pub fn preamble(&self) -> String {
        self.skill_dirs.first()
            .and_then(|dir| {
                SkillLoader::from_directory(dir).load().ok()
            })
            .map(|s| s.preamble())
            .unwrap_or_default()
    }
}

/// 单个 Skill Tool (实现了 rig 的 Tool trait)
pub struct SkillTool {
    spec: ToolSpec,
    handlers: Arc<RwLock<HashMap<String, ToolHandler>>>,
    async_handlers: Arc<RwLock<HashMap<String, AsyncToolHandler>>>,
}

impl SkillTool {
    fn parameters(&self) -> Value {
        if self.spec.parameters.is_object() {
            self.spec.parameters.clone()
        } else {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": true
            })
        }
    }

    async fn get_handler(&self) -> Option<ToolHandler> {
        let handlers = self.handlers.read().await;
        handlers.get(&self.spec.name).cloned()
    }

    async fn get_async_handler(&self) -> Option<AsyncToolHandler> {
        let handlers = self.async_handlers.read().await;
        handlers.get(&self.spec.name).cloned()
    }
}

impl Tool for SkillTool {
    const NAME: &'static str = "";
    type Error = ToolError;
    type Args = Value;
    type Output = Value;

    fn name(&self) -> String {
        self.spec.name.clone()
    }

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: self.spec.name.clone(),
            description: self.spec.description.clone(),
            parameters: self.parameters(),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let handler = self.get_handler().await;
        match handler {
            Some(h) => h(args),
            None => Ok(serde_json::json!({
                "error": format!("Handler not registered for tool: {}", self.spec.name)
            })),
        }
    }
}
