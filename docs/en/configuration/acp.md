# ACP Integration

## What ACP Means in Peekoo

Peekoo uses ACP to run and coordinate AI agents. It also exposes native productivity features through an embedded MCP server, so agents can move beyond plain chat and work with tasks, pomodoro, settings, and plugin tools.

## Embedded MCP Server

When Peekoo starts its runtime, it launches a shared MCP server on a local HTTP port.

Endpoints:

- `/mcp`: native tools for tasks, pomodoro, and settings
- `/mcp/plugins`: third-party plugin tools

## Tool Families

The native endpoint exposes tool groups for:

- task management
- pomodoro control and history
- app settings such as active sprite and theme

## Environment Variables

Agent processes receive MCP connection details through:

- `PEEKOO_MCP_HOST`
- `PEEKOO_MCP_PORT`

## Using mcporter

If your agent environment does not support MCP natively, you can use `mcporter`:

```bash
npx mcporter list peekoo-native --config <path>/mcporter.json
npx mcporter list peekoo-plugins --config <path>/mcporter.json
```

Then call tools with:

```bash
npx mcporter call <server>.<tool_name> --config <path>/mcporter.json
```

See [Skills](./skills.md) for how Peekoo provides this bridge automatically.
