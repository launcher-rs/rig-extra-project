## rig-extra

基于[rig-core](https://github.com/0xPlaygrounds/rig)的简单扩展

## 扩展功能
* 添加 Z智谱 bigmodel provider, 【注意: 虽然rig-core中智谱使用 openai provider有问题,但是可以使用anthropic provider,也可以使用本crate的 bigmodel provider】
  * anthropic provider 使用示例,注意必须设置 max_tokens
    ```
         let client = providers::anthropic::ClientBuilder::new("xxxxxxxxxxxxxxxxxxx")
            .base_url("https://open.bigmodel.cn/api/anthropic")
            .build()?;
        let agent = client.agent("glm-4.5-flash")
            // .preamble("你是一个ai助手")
            .max_tokens(10000)
            .build();
    
    ```
* 添加随机agent
* 添加失败重试功能
* ...