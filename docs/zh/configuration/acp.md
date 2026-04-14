# ACP 集成

## Peekoo 中的 ACP 是什么

Peekoo 使用 ACP 来运行和协调 AI Agent。同时，它通过内嵌的 MCP Server 暴露原生生产力能力，让 Agent 不只是“聊天”，还可以访问任务、番茄钟、设置和插件工具。

## 内嵌 MCP Server

当 Peekoo 启动运行时时，会在本地 HTTP 端口上启动一个共享的 MCP Server。

端点包括：

- `/mcp`：任务、番茄钟和设置等原生工具
- `/mcp/plugins`：第三方插件工具

## 工具分类

原生端点当前提供以下几类工具：

- 任务管理
- 番茄钟控制与历史记录
- 应用设置，例如当前 Sprite 和主题

## 环境变量

Agent 进程通过以下环境变量获得 MCP 连接信息：

- `PEEKOO_MCP_HOST`
- `PEEKOO_MCP_PORT`

## 使用 mcporter

如果你的 Agent 环境不原生支持 MCP，可以使用 `mcporter`：

```bash
npx mcporter list peekoo-native --config <path>/mcporter.json
npx mcporter list peekoo-plugins --config <path>/mcporter.json
```

然后用下面的方式调用工具：

```bash
npx mcporter call <server>.<tool_name> --config <path>/mcporter.json
```

更多内容见 [Skill](./skills.md)，其中说明了 Peekoo 如何自动提供这个桥接能力。
