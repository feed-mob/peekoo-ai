# Skill

## 什么是 Skill

Skill 是 Agent 可以按需加载的指令包。它让 Agent 的能力可以持续扩展，而不需要每次都改动产品界面。在 Peekoo 中，Skill 位于工作区内的 `.agents/skills/` 目录下。

## 内置的 Peekoo Agent Skill

Peekoo 内置了 `peekoo-agent-skill`。它帮助那些不原生支持 MCP 的 ACP Agent 通过 `mcporter` 访问 Peekoo 工具，让更高级的工作流在能力较弱的环境里也能成立。

这个 Skill 目录中包含：

- `SKILL.md`
- `mcporter.json`

当 Peekoo 启动时，`mcporter.json` 会被更新为实际的本地 MCP Server 端口。

## 发现工具

```bash
npx mcporter list peekoo-native --config <path-to-skill>/mcporter.json
npx mcporter list peekoo-plugins --config <path-to-skill>/mcporter.json
```

## 调用工具

```bash
npx mcporter call <server>.<tool_name> [args...] --config <path-to-skill>/mcporter.json
```

## 服务名

- `peekoo-native`：任务、番茄钟、设置
- `peekoo-plugins`：插件提供的工具，例如 Google Calendar

## 当前行为

Skill 采用目录扫描发现。只要一个目录中包含 `SKILL.md`，它就会被识别为一个 Skill。
