use std::sync::Arc;
use std::time::Duration;
use rig_extra::client::completion::CompletionClientDyn;
use rig_extra::completion::{Prompt, PromptError};
use rig_extra::providers::ollama;
use rig_extra::{ExponentialBuilder, Retryable};
use rig_extra::rand_agent::RandAgentBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 设置日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();
    
    let ollama_client = ollama::Client::new();
    
    // 普通调用
    let agent = ollama_client.agent("qwen2.5:14b").build();
    
    match agent.prompt("讲个笑话").await{
        Ok(result) => {
            println!("result: {result}");
        }
        Err(err) => {
            println!("Error: {err}");
        }
    }
    
    // retry 调用
    let agent = Arc::new(agent);
    let result = (|| {
        let agent_clone = agent.clone();
        async move {
            agent_clone.prompt("你好").await
        }
    }).retry(ExponentialBuilder::default())
        .sleep(tokio::time::sleep)
        .notify(|err: &PromptError, dur: Duration| {
            println!("retrying {err:?} after {dur:?}");
        })
        .await;
    
    match result {
        Ok(result) => {
            println!("result: {result}");
        }
        Err(err) => {
            println!("Error: {err}");
        }
    }
    
    
    // rand agent
    let rand_agent_builder = RandAgentBuilder::new()
        .max_failures(5)
        .on_agent_invalid(|id|{
            println!("Invalid agent id: {id}");
        });

    let agent1 = ollama_client.agent("qwen2.5:14b").build();
    let agent2 = ollama_client.agent("qwen2.5:14b").build();
    let rand_agent_builder = rand_agent_builder.add_agent(agent1,1,"ollama".to_string(),"qwen2.5:14b".to_string());
    let rand_agent_builder = rand_agent_builder.add_agent(agent2,2,"ollama".to_string(),"qwen2.5:14b".to_string());
    let thread_safe_agent = rand_agent_builder.build();
    println!("rand_agent 请求........");
    let result = thread_safe_agent.try_invoke_with_retry("讲个笑话".into(),Some(3)).await?;
    println!("result: {result}");

    Ok(())
}
