# 快速开始

Peekoo 的目标是让你在几分钟内就能开始使用：安装、启动，然后马上和桌面伙伴互动。

## 安装 Peekoo

从 GitHub Releases 下载最新版本，然后按平台查看安装说明：

- [macOS](./installation/macos.md)
- [Windows](./installation/windows.md)
- [Linux](./installation/linux.md)

## 首次运行

启动后，Peekoo 会以一个小型桌面伙伴的形式出现。你可以立刻从这里：

- 打开聊天
- 创建和跟踪任务
- 启动番茄钟
- 打开插件面板

如果这是你第一次使用 Peekoo，建议先从最简单的路径开始：

1. 打开聊天并发送一条简短消息。
2. 创建一个你今天真正要处理的任务。
3. 为这个任务启动一次番茄钟专注。
4. 在熟悉核心流程后，再继续探索插件。

## 关键概念

- `Sprite`：桌面上可见的角色
- `ACP`：用于运行 Agent 的通信层
- `Skill`：Agent 可按需加载的说明或能力包
- `Plugin`：可扩展工具、面板和事件的插件

## 推荐的第一次体验

你可以用下面的顺序快速理解 Peekoo：

1. 在聊天中向 Peekoo 提一个实际问题。
2. 记录一个今天要完成的真实任务。
3. 为这个任务启动一次专注会话。
4. 回看任务和番茄钟历史。

这样你可以一次体验 Peekoo 的三个核心面：辅助、规划和专注。

## 面向开发者

如果你是从源码运行 Peekoo：

```bash
just setup
just dev
```

更多内容见 [开发总览](./develop/index.md)。
