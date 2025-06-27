## rig-extra

* 提供 bigmodel_provider, bigmodel 虽然支持openai标准，但是使用rig-core 的openai_provider时会报错。
  * 请求: 要求 system prompt不能为空,可以通过agent的 preamble 设置 system prompt 来解决
  * 响应:
    ```
    called `Result::unwrap()` on an `Err` value: CompletionError(JsonError(Error("data did not match any variant of untagged enum ApiResponse", line: 0, column: 0)))
    
    ```
    
    * 没有object字段 
  * 所有编写 bigmodel_provider,但是只测试了 glm-4-flash,毕竟只有这个是免费的