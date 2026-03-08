## rig-core学习



#### rig 简单同步调用流程

* 调用 `.prompt` 方法

  ```
   let response = comedian_agent.prompt("Entertain me!").await?;
  ```



* 第二步: 调用 `PromptRequest::from_agent` ，PromptRequest

  ```rust
  #[allow(refining_impl_trait)]
  impl<M, P> Prompt for Agent<M, P>
  where
      M: CompletionModel,
      P: PromptHook<M> + 'static,
  {
      fn prompt(
          &self,
          prompt: impl Into<Message> + WasmCompatSend,
      ) -> PromptRequest<'_, prompt_request::Standard, M, P> {
          PromptRequest::from_agent(self, prompt)
      }
  }		
  ```



* PromptRequest 是一个结构体，如何转成异步方法，进行异步调用，实现 `IntoFuture `方法

    * `IntoFuture` 是 Rust 中用于 **将某个类型转换成 `Future` 的 trait**，允许一个类型在 `.await` 时 **自动转换为 `Future`** 。换句话说：**`.await` 实际上调用的是 `IntoFuture::into_future()`**。

    * 最简单示例

      ```rust
      use std::future::{Future, IntoFuture};
      use std::pin::Pin;
      use std::task::{Context, Poll};
      
      struct MyTask;
      
      impl IntoFuture for MyTask {
          type Output = i32;
          type IntoFuture = MyFuture;
      
          fn into_future(self) -> Self::IntoFuture {
              MyFuture
          }
      }
      
      struct MyFuture;
      
      impl Future for MyFuture {
          type Output = i32;
      
          fn poll(
              self: Pin<&mut Self>,
              _cx: &mut Context<'_>,
          ) -> Poll<Self::Output> {
              Poll::Ready(42)
          }
      }
      
      #[tokio::main]
      async fn main() {
          let v = MyTask.await;
          println!("{}", v);
      }
      ```

    * 代码: 实现了2次 ，返回结果不同，一个为String ,一个为 PromptResponse

        * 在这个文件里 `PromptRequest` **实现了两次 `IntoFuture`**，看起来几乎一样，struct中泛型参数一个为 Extended，一个为Standard，另外  `Output` 不同。这其实是 Rust **利用类型参数做“类型级别分支”**的一种常见模式。  【prompt 默认返回 Standard，目前没有发现Extended】

          ```
          struct A<T>(T);
          
          A<u32>
          A<String>
          ```

          Rust 允许为不同泛型实例 **分别实现 trait**。

          ```
          let result: String = agent.prompt("hello").await?;
          let result: PromptResponse = agent.prompt("hello").await?;
          ```

          根据返回值，调用不同的方法



    ```rust
    impl<'a, M, P> IntoFuture for PromptRequest<'a, Standard, M, P>
    where
        M: CompletionModel + 'a,
        P: PromptHook<M> + 'static,
    {
        type Output = Result<String, PromptError>;
        type IntoFuture = WasmBoxedFuture<'a, Self::Output>; // This future should not outlive the agent
    
        fn into_future(self) -> Self::IntoFuture {
            Box::pin(self.send())
        }
    }
    
    impl<'a, M, P> IntoFuture for PromptRequest<'a, Extended, M, P>
    where
        M: CompletionModel + 'a,
        P: PromptHook<M> + 'static,
    {
        type Output = Result<PromptResponse, PromptError>;
        type IntoFuture = WasmBoxedFuture<'a, Self::Output>; // This future should not outlive the agent
    
        fn into_future(self) -> Self::IntoFuture {
            Box::pin(self.send())
        }
    }
    ```



* 调用 `PromptRequest<'a, Standard, M, P>`的 `send`方法

  ```rust
  impl<M, P> PromptRequest<'_, Standard, M, P>
  where
      M: CompletionModel,
      P: PromptHook<M>,
  {
      async fn send(self) -> Result<String, PromptError> {
          self.extended_details().send().await.map(|resp| resp.output)
      }
  }
  ```

* 调用 `PromptRequest<'_, Extended, M, P>` 的`send`方法

    * 带注释，该方法为执行的核心逻辑

      ```rust
      async fn send(mut self) -> Result<PromptResponse, PromptError> {
              let agent_span = if tracing::Span::current().is_disabled() {
                  info_span!(
                      "invoke_agent",
                      gen_ai.operation.name = "invoke_agent",
                      gen_ai.agent.name = self.agent_name(),
                      gen_ai.system_instructions = self.preamble,
                      gen_ai.prompt = tracing::field::Empty,
                      gen_ai.completion = tracing::field::Empty,
                      gen_ai.usage.input_tokens = tracing::field::Empty,
                      gen_ai.usage.output_tokens = tracing::field::Empty,
                  )
              } else {
                  tracing::Span::current()
              };
      
              if let Some(text) = self.prompt.rag_text() {
                  agent_span.record("gen_ai.prompt", text);
              }
      
              // Capture agent_name before borrowing chat_history
              let agent_name_for_span = self.agent_name.clone();
      
              let chat_history = if let Some(history) = self.chat_history.as_mut() {
                  history.push(self.prompt.to_owned());
                  history
              } else {
                  &mut vec![self.prompt.to_owned()]
              };
      
              let mut current_max_turns = 0;
              let mut usage = Usage::new();
              let current_span_id: AtomicU64 = AtomicU64::new(0);
      
              // We need to do at least 2 loops for 1 roundtrip (user expects normal message)
              // 进行循环
              let last_prompt = loop {
                  let prompt = chat_history
                      .last()
                      .cloned()
                      .expect("there should always be at least one message in the chat history");
      
                  // 如果达到最大循环次数，返回最后一条prompt, 结束循环,last prompt 将作为错误信息
                  if current_max_turns > self.max_turns + 1 {
                      // 返回最后一条prompt。llm的最好回答
                      break prompt;
                  }
      
                  current_max_turns += 1;
      
                  if self.max_turns > 1 {
                      tracing::info!(
                          "Current conversation depth: {}/{}",
                          current_max_turns,
                          self.max_turns
                      );
                  }
      
                  // hook on_completion_call 执行，模型请求前 hook执行
                  if let Some(ref hook) = self.hook
                      && let HookAction::Terminate { reason } = hook
                          .on_completion_call(&prompt, &chat_history[..chat_history.len() - 1])
                          .await
                  {
                      return Err(PromptError::prompt_cancelled(chat_history.to_vec(), reason));
                  }
      
                  let span = tracing::Span::current();
                  let chat_span = info_span!(
                      target: "rig::agent_chat",
                      parent: &span,
                      "chat",
                      gen_ai.operation.name = "chat",
                      gen_ai.agent.name = agent_name_for_span.as_deref().unwrap_or(UNKNOWN_AGENT_NAME),
                      gen_ai.system_instructions = self.preamble,
                      gen_ai.provider.name = tracing::field::Empty,
                      gen_ai.request.model = tracing::field::Empty,
                      gen_ai.response.id = tracing::field::Empty,
                      gen_ai.response.model = tracing::field::Empty,
                      gen_ai.usage.output_tokens = tracing::field::Empty,
                      gen_ai.usage.input_tokens = tracing::field::Empty,
                      gen_ai.input.messages = tracing::field::Empty,
                      gen_ai.output.messages = tracing::field::Empty,
                  );
      
                  let chat_span = if current_span_id.load(Ordering::SeqCst) != 0 {
                      let id = Id::from_u64(current_span_id.load(Ordering::SeqCst));
                      chat_span.follows_from(id).to_owned()
                  } else {
                      chat_span
                  };
      
                  if let Some(id) = chat_span.id() {
                      current_span_id.store(id.into_u64(), Ordering::SeqCst);
                  };
      
                  // 进行http 请求
                  let resp = build_completion_request(
                      &self.model,
                      prompt.clone(),
                      chat_history[..chat_history.len() - 1].to_vec(),
                      self.preamble.as_deref(),
                      &self.static_context,
                      self.temperature,
                      self.max_tokens,
                      self.additional_params.as_ref(),
                      self.tool_choice.as_ref(),
                      &self.tool_server_handle,
                      &self.dynamic_context,
                      self.output_schema.as_ref(),
                  )
                  .await?
                  .send()
                  .instrument(chat_span.clone())
                  .await?;
      
                  // 记录token使用记录
                  usage += resp.usage;
      
                  // hook on_completion_response 执行，模型请求后 hook执行
                  if let Some(ref hook) = self.hook
                      && let HookAction::Terminate { reason } =
                          hook.on_completion_response(&prompt, &resp).await
                  {
                      return Err(PromptError::prompt_cancelled(chat_history.to_vec(), reason));
                  }
      
                  // 提取内容，将llm相应分为 工具列表和文本
                  // partition 根据条件把一个迭代器拆分成两个集合。
                  let (tool_calls, texts): (Vec<_>, Vec<_>) = resp
                      .choice
                      .iter()
                      .partition(|choice| matches!(choice, AssistantContent::ToolCall(_)));
      
                  // 添加到chat_history
                  chat_history.push(Message::Assistant {
                      id: resp.message_id.clone(),
                      content: resp.choice.clone(),
                  });
                  
                  // 如果没有工具调用，直接返回文本内容
                  if tool_calls.is_empty() {
                      // 文本合并
                      let merged_texts = texts
                          .into_iter()
                          .filter_map(|content| {
                              if let AssistantContent::Text(text) = content {
                                  Some(text.text.clone())
                              } else {
                                  None
                              }
                          })
                          .collect::<Vec<_>>()
                          .join("\n");
      
                      if self.max_turns > 1 {
                          tracing::info!("Depth reached: {}/{}", current_max_turns, self.max_turns);
                      }
      
                      agent_span.record("gen_ai.completion", &merged_texts);
                      agent_span.record("gen_ai.usage.input_tokens", usage.input_tokens);
                      agent_span.record("gen_ai.usage.output_tokens", usage.output_tokens);
      
                      // If there are no tool calls, depth is not relevant, we can just return the merged text response.
                      // 函数执行成功，返回文本内容
                      return Ok(
                          PromptResponse::new(merged_texts, usage).with_messages(chat_history.to_vec())
                      );
                  }
      
                  // 开始处理工具调用
                  let hook = self.hook.clone();
                  let tool_server_handle = self.tool_server_handle.clone();
      
                  let tool_calls: Vec<AssistantContent> = tool_calls.into_iter().cloned().collect();
                  // 迭代开始工具调用，结果为工具调用的结果
                  let tool_content = stream::iter(tool_calls)
                      .map(|choice| {
                          let hook1 = hook.clone();
                          let hook2 = hook.clone();
                          let tool_server_handle = tool_server_handle.clone();
      
                          let tool_span = info_span!(
                              "execute_tool",
                              gen_ai.operation.name = "execute_tool",
                              gen_ai.tool.type = "function",
                              gen_ai.tool.name = tracing::field::Empty,
                              gen_ai.tool.call.id = tracing::field::Empty,
                              gen_ai.tool.call.arguments = tracing::field::Empty,
                              gen_ai.tool.call.result = tracing::field::Empty
                          );
      
                          let tool_span = if current_span_id.load(Ordering::SeqCst) != 0 {
                              let id = Id::from_u64(current_span_id.load(Ordering::SeqCst));
                              tool_span.follows_from(id).to_owned()
                          } else {
                              tool_span
                          };
      
                          if let Some(id) = tool_span.id() {
                              current_span_id.store(id.into_u64(), Ordering::SeqCst);
                          };
      
                          let cloned_chat_history = chat_history.clone().to_vec();
      
                          async move {
                              if let AssistantContent::ToolCall(tool_call) = choice {
                                  // 工具名称
                                  let tool_name = &tool_call.function.name;
                                  // 工具参数
                                  let args =
                                      json_utils::value_to_json_string(&tool_call.function.arguments);
                                  let internal_call_id = nanoid::nanoid!();
                                  let tool_span = tracing::Span::current();
                                  tool_span.record("gen_ai.tool.name", tool_name);
                                  tool_span.record("gen_ai.tool.call.id", &tool_call.id);
                                  tool_span.record("gen_ai.tool.call.arguments", &args);
                                  // hook on_tool_call 执行，模型请求前 hook执行
                                  if let Some(hook) = hook1 {
                                      let action = hook
                                          .on_tool_call(
                                              tool_name,
                                              tool_call.call_id.clone(),
                                              &internal_call_id,
                                              &args,
                                          )
                                          .await;
      
                                      if let ToolCallHookAction::Terminate { reason } = action {
                                          return Err(PromptError::prompt_cancelled(
                                              cloned_chat_history,
                                              reason,
                                          ));
                                      }
      
                                      if let ToolCallHookAction::Skip { reason } = action {
                                          // Tool execution rejected, return rejection message as tool result
                                          tracing::info!(
                                              tool_name = tool_name,
                                              reason = reason,
                                              "Tool call rejected"
                                          );
                                          if let Some(call_id) = tool_call.call_id.clone() {
                                              return Ok(UserContent::tool_result_with_call_id(
                                                  tool_call.id.clone(),
                                                  call_id,
                                                  OneOrMany::one(reason.into()),
                                              ));
                                          } else {
                                              return Ok(UserContent::tool_result(
                                                  tool_call.id.clone(),
                                                  OneOrMany::one(reason.into()),
                                              ));
                                          }
                                      }
                                  }
                                  // 调用工具函数
                                  let output = match tool_server_handle.call_tool(tool_name, &args).await
                                  {
                                      Ok(res) => res,
                                      Err(e) => {
                                          tracing::warn!("Error while executing tool: {e}");
                                          e.to_string()
                                      }
                                  };
                                  // 调用工具函数后，hook on_tool_result 执行，模型请求后 hook执行
                                  if let Some(hook) = hook2
                                      && let HookAction::Terminate { reason } = hook
                                          .on_tool_result(
                                              tool_name,
                                              tool_call.call_id.clone(),
                                              &internal_call_id,
                                              &args,
                                              &output.to_string(),
                                          )
                                          .await
                                  {
                                      return Err(PromptError::prompt_cancelled(
                                          cloned_chat_history,
                                          reason,
                                      ));
                                  }
      
                                  tool_span.record("gen_ai.tool.call.result", &output);
                                  tracing::info!(
                                      "executed tool {tool_name} with args {args}. result: {output}"
                                  );
                                  // 返回工具调用的结果
                                  if let Some(call_id) = tool_call.call_id.clone() {
                                      Ok(UserContent::tool_result_with_call_id(
                                          tool_call.id.clone(),
                                          call_id,
                                          ToolResultContent::from_tool_output(output),
                                      ))
                                  } else {
                                      Ok(UserContent::tool_result(
                                          tool_call.id.clone(),
                                          ToolResultContent::from_tool_output(output),
                                      ))
                                  }
                              } else {
                                  unreachable!(
                                      "This should never happen as we already filtered for `ToolCall`"
                                  )
                              }
                          }
                          .instrument(tool_span)
                      })
                      .buffer_unordered(self.concurrency)
                      .collect::<Vec<Result<UserContent, PromptError>>>()
                      .await
                      .into_iter()
                      .collect::<Result<Vec<_>, _>>()?;
      
                  // 将工具调用结果加入到聊天记录中
                  chat_history.push(Message::User {
                      content: OneOrMany::many(tool_content).expect("There is atleast one tool call"),
                  });
              };
      
              // If we reach here, we never resolved the final tool call. We need to do ... something.
              
              // 当 循环超过最大循环次数，返回MaxTurnsError报错信息........
              Err(PromptError::MaxTurnsError {
                  max_turns: self.max_turns,
                  chat_history: Box::new(chat_history.clone()),
                  prompt: Box::new(last_prompt),
              })
          }
      ```



* send和provider 的结合，通过 build_completion_request，返回 `CompletionRequestBuilder<M>`，调用 CompletionRequestBuilder<M> 的 `send`方法

  ```rust
      pub async fn send(self) -> Result<CompletionResponse<M::Response>, CompletionError> {
          let model = self.model.clone();
          model.completion(self.build()).await
      }
  ```

* 开始调用各个provider的的 completion方法

    

