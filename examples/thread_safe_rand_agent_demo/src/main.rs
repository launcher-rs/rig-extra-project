use rig_extra::extra_providers::{bigmodel};
use std::sync::Arc;
use config::Config;
use serde::Deserialize;
use tokio::task;
use rig_extra::client::completion::CompletionClientDyn;
use rig_extra::providers::{ollama, openai};
use rig_extra::thread_safe_rand_agent::ThreadSafeRandAgentBuilder;

#[derive(Debug, Deserialize)]
struct AgentConfig {
    provider: String,
    model_name: String,
    api_key: String,
    api_base_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 设置日志
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

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

    // 创建线程安全的 RandAgent
    let mut rand_agent_builder = ThreadSafeRandAgentBuilder::new().max_failures(5);
    for agent_conf in agent_configs {
        match agent_conf.provider.as_str() {
            "bigmodel" => {
                let client = bigmodel::Client::new(&agent_conf.api_key);
                let agent = client
                    .agent(&agent_conf.model_name)
                    .build();

                rand_agent_builder = rand_agent_builder.add_builder(agent,"bigmodel",&agent_conf.model_name);
            },
            "openai" => {
                let client = if let Some(api_base_url) = agent_conf.api_base_url {
                    openai::Client::from_url(&agent_conf.api_key,&api_base_url)
                }else {
                    openai::Client::new(&agent_conf.api_key)
                };
                let agent_builder = client.agent(&agent_conf.model_name).build();
                rand_agent_builder = rand_agent_builder.add_builder(agent_builder,"openai",&agent_conf.model_name);
            },
            "ollama" => {
                let client = if let Some(api_base_url) = agent_conf.api_base_url {
                    ollama::Client::from_url(&api_base_url)
                }else {
                    ollama::Client::new()
                };
                let agent_builder = client.agent(&agent_conf.model_name).build();
                rand_agent_builder = rand_agent_builder.add_builder(agent_builder,"ollama",&agent_conf.model_name);
            }
            other => {
                println!("[WARN] provider '{other}' 暂未支持, 跳过该agent");
            }
        }
    }
    let thread_safe_agent = rand_agent_builder.build();



    println!("创建了线程安全的 RandAgent，总代理数量: {}", thread_safe_agent.total_len());
    println!("有效代理数量: {}", thread_safe_agent.len());

    // 将线程安全代理包装在 Arc 中以支持多线程共享
    let agent_arc = Arc::new(thread_safe_agent);

    // 创建多个并发任务
    let mut handles = vec![];
    let num_tasks = 5;

    println!("\n开始并发执行 {num_tasks} 个任务...");

    for i in 0..num_tasks {
        let agent_clone = Arc::clone(&agent_arc);
        let handle = task::spawn(async move {
            let prompt = format!("请简单介绍一下你自己，并告诉我你是第{}个任务", i + 1);
            
            match agent_clone.prompt(&prompt).await {
                Ok(response) => {
                    println!("任务 {} 成功: {}", i + 1, response.lines().next().unwrap_or("无响应"));
                    Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
                }
                Err(e) => {
                    println!("任务 {} 失败: {}", i + 1, e);
                    Err(e)
                }
            }
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(())) => {
                success_count += 1;
                println!("任务 {} 完成", i + 1);
            }
            Ok(Err(e)) => {
                error_count += 1;
                println!("任务 {} 执行失败: {}", i + 1, e);
            }
            Err(e) => {
                error_count += 1;
                println!("任务 {} 任务本身失败: {}", i + 1, e);
            }
        }
    }

    println!("\n=== 执行结果统计 ===");
    println!("成功任务数: {success_count}");
    println!("失败任务数: {error_count}");
    println!("成功率: {:.1}%", (success_count as f64 / num_tasks as f64) * 100.0);

    // 显示最终状态
    println!("\n=== 最终状态 ===");
    println!("总代理数量: {}", agent_arc.total_len());
    println!("有效代理数量: {}", agent_arc.len());
    
    let stats = agent_arc.failure_stats();
    println!("失败统计:");
    for (index, failures, max_failures) in stats {
        let status = if failures >= max_failures { "无效" } else { "有效" };
        println!("  Agent {index}: {failures}/{max_failures} 失败 - {status}");
    }

    // 重置失败计数
    agent_arc.reset_failures();
    println!("已重置所有代理的失败计数");

    Ok(())
} 