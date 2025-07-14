use thiserror::Error;

#[derive(Debug, Error)]
pub enum RandAgentError {
    #[error("No valid agents available")] 
    NoValidAgents,
    #[error("Agent error: {0}")]
    AgentError(#[from] Box<dyn std::error::Error + Send + Sync>),
} 