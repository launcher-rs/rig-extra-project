## rig extract

基于rig的扩展


#### bigmodel

openai_provider 不使用于 bigmodel

* 请求: 要求 system prompt不能为空,可以通过agent的 preamble 设置 system prompt 来解决
* 响应: 
```
called `Result::unwrap()` on an `Err` value: CompletionError(JsonError(Error("data did not match any variant of untagged enum ApiResponse", line: 0, column: 0)))

```
    * 没有object字段 