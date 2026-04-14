# AI Agent Chat

## 它能做什么

Peekoo 内置了一个对话式 AI 聊天界面。Agent 运行时支持流式回复、可配置提供商、运行时模型切换，以及工作区上下文加载。

当你希望在不离开桌面的情况下获得快速帮助时，聊天就是最主要的入口。你可以在这里提问、整理思路、查看项目上下文，或者把 Agent 与 Peekoo 的生产力工具一起使用。

## 它在应用中的位置

聊天层采用 agent-first 设计，并与以下能力协同工作：

- 提供商配置与认证
- `AGENTS.md` 等工作区指令文件
- 自动发现的 Skill
- 通过 MCP 暴露的任务、番茄钟、设置和插件工具

## 运行模型

从高层看，聊天流程如下：

1. UI 通过 Tauri command 发送提示词。
2. 后端解析 Agent 配置。
3. Agent 运行时向所选提供商发起请求。
4. 如果模型需要工具，它可以使用内置工具、已加载 Skill 以及 Peekoo 的 MCP 工具。
5. 响应以流式方式返回 UI。

## 常见使用方式

- 围绕当前正在处理的任务寻求帮助
- 借助 Agent 的工具能力查看文件和项目上下文
- 把模糊想法整理成清晰的下一步
- 和 Tasks、Pomodoro 配合使用，形成更紧凑的工作流

## 当前配置面

当前仓库已经支持：

- 运行时切换 provider 和 model
- 加载工作区指令文件
- 从配置根目录发现 Skill
- 支持提供商认证流程

## 如何选择 Provider 或 Model

你可以按目标来选择：

- 日常快速辅助：使用默认的通用模型
- 更重视准确性和推理能力：切换到更强的模型
- 更关注成本：把轻量模型作为日常默认选择

如果你的环境支持多个 provider，Peekoo 可以在运行时切换 model，而不需要完整重启。

## 工作区上下文

Peekoo 的 Agent 运行时可以从工作区加载指令和记忆文件，包括：

- `AGENTS.md`
- `SOUL.md`
- `IDENTITY.md`
- `USER.md`
- `MEMORY.md`

这让 Agent 可以在不把上下文硬编码进应用的前提下，获得项目级和用户级信息。

## 适合新用户的首批提示词

如果你刚开始使用 Peekoo chat，可以试试这些提示词：

- `Summarize what this project does.`
- `Help me break this feature into tasks.`
- `Review this part of the codebase for risks.`
- `Draft a pomodoro plan for my next hour.`

## 相关概念

- 提供商与模型配置
- `AGENTS.md` 等工作区文件
- 按需加载的 Skill
- 通过 MCP 提供的生产力工具

## 状态说明

仓库已经具备较完整的运行时支持，但围绕每个聊天设置项和提供商流程的 UI 级用户指南仍需要后续补充。
