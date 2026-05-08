<p align="right">
  中文 | <a href="./README.md">English</a>
</p>

<div align="center">

# Peekoo AI

**无限扩展、量身定制的桌面小精灵**

![Peekoo Demo](assets/Peekoo_Peek.gif)

[![观看演示视频](https://img.youtube.com/vi/TCFbKJELtig/maxresdefault.jpg)](https://youtu.be/TCFbKJELtig)

[![Release](https://img.shields.io/github/v/release/feed-mob/peekoo-ai)](https://github.com/feed-mob/peekoo-ai/releases)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)](#安装)

*Smart extensions, warm intentions.*

</div>

---

## 为什么是 Peekoo？

这个项目起源于一个问题：**AI 到底应该如何以更温和的方式融入工作和学习？**

AI 工具众星云集之下，我们想探索一种新的可能：把桌宠最基础的**陪伴感**和 AI 的**拓展性**相结合，各取所长。

Peekoo 的核心是陪伴。大多数情况下，它只是趴在桌面的角落里悄悄地偷看你，时不时打打瞌睡——就像每一个桌宠能做的一样。与众不同的是，在必要的时刻，它也做好了成为你帮手的准备。

我们把 Peekoo 的设计理念归结为两点：

- **分离"基础陪伴"与"定制需求"** — 保持核心轻量，通过插件按需扩展，避免功能堆砌
- **将 Agent 的工作过程可视化、人格化** — 为冷硬的工具赋予温度和反馈

---

## 核心功能

### ★ AI 对话

随时开启聊天，连接 OpenCode 等 CLI 工具，一键集成 AI Agent，轻松配置。

流式响应，支持 OpenCode、Claude 等多种 Provider，无需重启即可切换模型。Chat 窗口和 Mini Chat 边栏随意切换。所有内置功能和插件都可以通过 Chat 快速调用。

支持加载工作区上下文文件（`AGENTS.md`、`SOUL.md`、`MEMORY.md`），让 Agent 具备项目专属知识。

![聊天面板](assets/screenshots/chat.png)

### ★ 任务管理

用自然语言描述任务，Peekoo 自动解析时间、优先级等信息。内置智能拆解任务功能，支持与外部任务管理工具互通（目前支持 Linear）。

任务还可以直接委派给 Peekoo AI 执行——把任务交给它，它会通过 ACP 调度 Agent 尝试自动完成。

![任务面板](assets/screenshots/tasks.png)

### ★ 番茄钟

专注计时，支持开始、暂停、继续和历史记录。计时状态与桌宠的视觉表现深度融合。

每次专注结束后，可在极简的历史归档面板中记录灵感或感悟（Memo），并将记录与任务相关联——让每一次专注都留下痕迹。每日徽章墙记录你的专注轨迹。

![番茄钟面板](assets/screenshots/pomodoro.png)

### ★ 自定义桌宠形象

可上传自定义图片，打造专属桌宠形象。设置页面内置了图片 Prompt，也支持自己绘制后按格式上传。上传图片后，Peekoo AI 可以帮你自动生成动画配置（manifest），预览、校验、一键保存。内置 Mimi、Snoopy 等多款桌宠开箱即用。

![桌宠在桌面上](assets/screenshots/sprite-desktop.png)

### ★ Skills 与 MCP

支持上传 Skill 文件，按需扩展 Agent 能力。内置 Skill 模板随应用更新自动同步到工作区。MCP 配置支持即将推出。

### ★ 多语言界面

支持 English、简体中文、繁體中文、日本語、Español、Français，可在设置中随时切换。

### ★ 自动更新

系统托盘 → 关于 Peekoo → 检查更新。有新版本时一键安装并重启，无需手动下载。

---

## 插件生态

Peekoo 的设计理念是：**万物皆可 Plugin**。

基于内建的 MCP（Model Context Protocol）架构，Peekoo 向外伸出了无数条触角。用户可以根据自身使用习惯，按需安装插件，量身定制功能边界。

| 插件 | 功能 |
|------|------|
| 健康提醒 | 在工作和学习之余，提醒你喝水、放松眼睛、起身活动 |
| OpenClaw Sessions | 直接通过 Peekoo 打开和管理 OpenClaw 浏览器会话 |
| Claude Code 伴侣 | 你的 Claude Code Agent 每一次思考和输出，都能和 Peekoo 的表情同步 |
| OpenCode 伴侣 | 你的 OpenCode Agent 每一次思考和输出，都能和 Peekoo 的表情同步 |
| 米家智能家居 | 用 Peekoo 帮你管理智能家居设备 |
| Google 日历 | 导入日程，通过聊天直接进入约好的 Google Meet |
| Linear | 导入已有的 Todolist，与 Peekoo 任务同步 |

我们期待凭借无限的插件扩建 Peekoo 的神经网络——它将拥有无限的可能性。

![插件商店](assets/screenshots/plugin-store.png)

### ACP 运行时架构

Peekoo 使用 ACP（Agent Client Protocol）作为 Agent 任务执行的通信层。调度器（AgentScheduler）通过 stdio 启动 `peekoo-agent-acp` 子进程，经由 ACP 协议传递任务上下文和 MCP 工具配置，驱动 Agent 完成任务后回收进程。这套架构让 Agent 运行时与应用主进程完全解耦。

Agent 运行时通过 ACP Registry 发现和安装——注册表中收录了 OpenCode、Kimi、Qwen、Hermes 等主流 Agent，一键安装，无需手动配置环境。

**Roadmap：** 我们计划将 ACP + MCP 工具链演进为首要 Agent 运行时，并支持将 Peekoo 的内置工具和插件工具统一暴露为 MCP 服务，供外部 Agent 框架直接调用。

---

## 安装

从 [GitHub Releases](https://github.com/feed-mob/peekoo-ai/releases) 下载最新版本。支持 Windows、macOS 和 Linux。

### Windows
运行 `x64-setup.exe` 安装程序。

### macOS
下载 `.dmg`，将 `Peekoo.app` 移动到 `/Applications`，然后移除隔离标记：
```bash
xattr -cr /Applications/Peekoo.app
```
→ [macOS 详细安装指南](docs/zh/installation/macos.md)

### Linux（Arch）
```bash
yay -S peekoo-bin
```
→ [AUR 包](https://aur.archlinux.org/packages/peekoo-bin)

---

## 快速开始（开发者）

```bash
just setup   # 安装所有依赖
just dev     # 开发模式运行
```

→ [完整快速开始指南](docs/zh/quick-start.md)

---

## 技术栈

基于 Tauri v2 + Rust 后端，体积轻盈，对内存极度友好。所有任务内容、插件配置均储存在内置的 SQLite 数据库中。所有凭证与 API Key 均享受系统级 Keychain 的防护。

| 层级 | 技术 |
|------|------|
| 桌面外壳 | Tauri v2 |
| 前端 | React 18 + TypeScript 5 + Vite 5 |
| 样式 | Tailwind CSS v4 |
| 后端 | Rust（edition 2024，MSRV 1.85） |
| Agent 运行时 | pi_agent_rust |
| 持久化 | SQLite（内嵌迁移） |
| 密钥存储 | 系统钥匙串，文件系统兜底 |

---

## 参与贡献

欢迎贡献代码。请参阅 [docs/zh/contributing.md](docs/zh/contributing.md) 了解如何开始。

插件开发请参阅 [docs/zh/develop/plugins.md](docs/zh/develop/plugins.md)，以及 [`plugins/openclaw-sessions/`](plugins/openclaw-sessions) 完整示例。

---

## 许可证

MIT
