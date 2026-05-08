# 快速开始

Peekoo 的目标是让你在几分钟内就能开始使用：安装、启动，然后马上和桌面伙伴互动。

## 安装 Peekoo

从 GitHub Releases 下载最新版本，然后按平台查看安装说明：

- [macOS](./installation/macos.md)
- [Windows](./installation/windows.md)
- [Linux](./installation/linux.md)

## 认识界面

启动后，Peekoo 会以一个小型桌面角色的形式出现在屏幕上。

![Peekoo 桌面](assets/screenshots/sprite-desktop.png)

- 单击 Sprite → 回到默认待机状态
- 双击 Sprite → 打开 Mini Chat，直接开始快速对话
- 右键 Sprite → 打开菜单，进入聊天、任务、番茄钟或插件面板
- 拖拽 Sprite → 移动到桌面任意位置
- 系统托盘图标右键 → 显示/隐藏精灵、设置、关于 Peekoo、退出 Peekoo

## 配置 Agent

使用 Peekoo 的 AI 聊天功能前，需要先配置一个 Agent 运行时。

入口：系统托盘图标右键 → 设置 → ACP 运行时

**新手推荐：OpenCode（免费上手）**

1. 在「可用运行时」列表中找到 OpenCode，点击安装
2. 点击「测试连接」确认就绪
3. 返回运行时列表，将 OpenCode 设为默认

**其他运行时：**

在「可用运行时」列表中选择并安装，点击「配置」后按页面提示完成登录或输入 API Key，再选择模型即可。

## 开始第一次对话

Agent 配置完成后：

1. 双击 Sprite 打开 Mini Chat，发送一条简短消息
2. 或右键 Sprite → 聊天，打开完整聊天面板

如果这是你第一次使用 Peekoo，建议先从最简单的路径开始：

1. 打开聊天并发送一条简短消息
2. 创建一个你今天真正要处理的任务
3. 为这个任务启动一次番茄钟专注
4. 在熟悉核心流程后，再继续探索插件

## 任务和番茄钟

**创建任务：**

右键 Sprite → 任务，在输入框用自然语言描述任务，例如：

```
明天下午3点开会1小时，高优先级
```

Peekoo 会自动解析时间、优先级等信息。

**启动番茄钟：**

右键 Sprite → 番茄钟，选择要专注的任务，点击开始。专注结束后可以保存一条备注记录完成内容或对应的任务。

## 安装插件

右键 Sprite → 插件 → 商店，浏览来自 GitHub 的可用插件，点击安装后启用即可使用。

目前可用的插件包括健康提醒、Google Calendar、Linear、Mijia Smart Home 等，详见 [插件列表](./usage/plugins/index.md)。

## 外观和语言

系统托盘图标右键 → 设置 → 外观：

- 切换主题：浅色 / 深色 / 跟随系统
- 切换语言：简体中文、繁體中文、English、日本語、Español、Français
- 切换 Sprite：选择不同的桌面角色形象

## 检查更新

系统托盘图标右键 → 关于 Peekoo → 检查更新。有新版本时点击「安装并重启」即可完成升级。

## 关键概念

- `Sprite`：桌面上可见的角色
- `ACP`：用于运行 Agent 的通信层
- `Skill`：Agent 可按需加载的说明或能力包
- `Plugin`：可扩展工具、面板和事件的插件

## 面向开发者

如果你是从源码运行 Peekoo：

```bash
just setup
just dev
```

更多内容见 [开发总览](./develop/index.md)。
