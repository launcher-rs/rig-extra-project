use config::Config;
use rig_extra::completion::Prompt;
use rig_extra::rand_agent::RandAgentBuilder;
use rig_extra::error::RandAgentError;
use rig_extra::simple_rand_builder::AgentConfig;

#[tokio::main]
async fn main() -> Result<(), RandAgentError> {
    // 1. 读取配置
    let settings = Config::builder()
        .add_source(config::File::with_name("Settings"))
        .build()
        .unwrap_or_default();

    // 解析 agent 配置
    let agent_configs: Vec<AgentConfig> = settings.get_array("agents")
        .ok()
        .map(|arr| {
            arr.into_iter().filter_map(|v| {
                v.clone().try_deserialize().ok()
            }).collect()
        })
        .unwrap_or_default();
    
    let rand_agent_builder = RandAgentBuilder::new()
        .max_failures(5)
        .on_agent_invalid(|id|{
        println!("Invalid agent id: {id}");
    });
    let rand_agent_builder = rand_agent_builder.simple_builder(agent_configs,"You are a helpful assistant".to_string());
    let rand_agent = rand_agent_builder.build();

    println!("Created RandAgent with {} total agents", rand_agent.total_len().await);
    println!("Valid agents: {}", rand_agent.len().await);

    // 多次调用，每次都会随机选择一个有效代理
    for i in 1..=20 {
        println!("\n--- 调用 #{i} ---");

        match rand_agent.prompt("请将一个笑话").await {
            Ok(response) => {
                println!("Agent response: {response}");
            }
            Err(e) => {
                println!("Error: {e}");
            }
        }

        // 显示失败统计
        let stats = rand_agent.failure_stats().await;
        println!("失败统计:");
        for (index, failures, max_failures) in stats {
            let status = if failures >= max_failures { "无效" } else { "有效" };
            println!("  Agent {index}: {failures}/{max_failures} 失败 - {status}");
        }
        println!("有效代理数量: {}/{}", rand_agent.len().await, rand_agent.total_len().await);
    }

    // 演示重置失败计数
    println!("\n--- 重置所有代理的失败计数 ---");
    rand_agent.reset_failures().await;
    println!("重置后有效代理数量: {}/{}", rand_agent.len().await, rand_agent.total_len().await);

    Ok(())
} 