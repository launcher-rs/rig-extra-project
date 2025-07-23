use std::sync::Arc;
use config::Config;
use tokio::task;
use rig_extra::completion::{Prompt, PromptError};
use rig_extra::simple_rand_builder::AgentConfig;
use rig_extra::streaming::{stream_to_stdout, StreamingPrompt};
use rig_extra::thread_safe_rand_agent::ThreadSafeRandAgentBuilder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    let rand_agent_builder = ThreadSafeRandAgentBuilder::new()
        .max_failures(5)
        .on_agent_invalid(|id|{
            println!("Invalid agent id: {id}");
        });
    let rand_agent_builder = rand_agent_builder.simple_builder(agent_configs,"You are a helpful assistant".to_string());
    let thread_safe_agent = rand_agent_builder.build();



    println!("创建了线程安全的 RandAgent，总代理数量: {}", thread_safe_agent.total_len().await);
    println!("有效代理数量: {}", thread_safe_agent.len().await);

    // 将线程安全代理包装在 Arc 中以支持多线程共享
    let agent_arc = Arc::new(thread_safe_agent);

    // 创建多个并发任务
    let mut handles = vec![];
    let num_tasks = 2;

    println!("\n开始并发执行 {num_tasks} 个任务...");

    for i in 0..num_tasks {
        let agent_clone = Arc::clone(&agent_arc);
        let handle: task::JoinHandle<Result<String, PromptError>> = task::spawn(async move {
            let prompt = format!("请简单介绍一下你自己，并告诉我你是第{}个任务", i + 1);
            // let prompt = "将一个笑话".to_string();
            let result = agent_clone.prompt(&prompt).await?;
            Ok(result)
        });
        handles.push(handle);
    }

    // 等待所有任务完成
    let mut success_count = 0;
    let mut error_count = 0;
    
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(Ok(response)) => {
                success_count += 1;
                println!("任务 {} 完成: {}", i + 1,response);
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
    
    // 同步调用
    for i in 0..10{
        let prompt = format!("请简单介绍一下你自己，并告诉我你是第{}个任务", i + 1);
        // let prompt = "将一个笑话".to_string();
        match  agent_arc.prompt(&prompt).await{
            Ok(result) => {
                println!("result: {result}");
            }
            Err(err) => {
                println!("error: {err}");
            }
        }
        
    }
    
    

    println!("\n=== 执行结果统计 ===");
    println!("成功任务数: {success_count}");
    println!("失败任务数: {error_count}");
    println!("成功率: {:.1}%", (success_count as f64 / num_tasks as f64) * 100.0);

    // 显示最终状态
    println!("\n=== 最终状态 ===");
    println!("总代理数量: {}", agent_arc.total_len().await);
    println!("有效代理数量: {}", agent_arc.len().await);
    
    let stats = agent_arc.failure_stats().await;
    println!("失败统计:");
    for (index, failures, max_failures) in stats {
        let status = if failures >= max_failures { "无效" } else { "有效" };
        println!("  Agent {index}: {failures}/{max_failures} 失败 - {status}");
    }

    // 重置失败计数
    agent_arc.reset_failures().await;
    println!("已重置所有代理的失败计数");
    
    // 异步调用
    if let Some(agent) = agent_arc.get_random_valid_agent_state().await{
        let agent = agent.agent.clone();
        match  agent.stream_prompt("写一个故事").await{
            Ok(mut stream) => {
                stream_to_stdout(&agent, &mut stream).await?;
            }
            Err(err) => {
                println!("error: {err}");
            }
        }
    }

    // 获取agents info
    let agents_info = agent_arc.get_agents_info().await;
    for info in agents_info {
        println!("agent info : {:?}",info);
    }

    if let Some(bigmodel_agent) = agent_arc.get_agent_by_name("Bigmodel","glm-4-flash").await{
        let result = bigmodel_agent.agent.prompt("将一个笑话").await?;
        println!("bigmodel_agent result: {}",result);
    }else {
        println!("bigmodel_agent not found");
    }
    

    Ok(())
} 