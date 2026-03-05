# Hello Skill

这是一个示例 skill，用于展示 rig-extra skills 模块的使用方式。

## 功能

这个 skill 提供了一个简单的问候功能，可以根据用户输入生成友好的问候语。

## 使用方式

```rust
use rig_extra::skills::SkillLoader;

// 加载 skill
let skill = SkillLoader::from_directory("./skills/hello-skill").load()?;

// 获取 preamble
let preamble = skill.preamble();
println!("Skill preamble: {}", preamble);
```
