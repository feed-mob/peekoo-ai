# AGENTS.md - peekoo-agent-acp

## Overview

ACP (Agent Client Protocol) server implementation for peekoo-agent. This crate wraps the core `peekoo-agent` functionality in the ACP protocol, allowing external ACP clients (like our task scheduler) to invoke the agent via JSON-RPC over stdio.

## Architecture

- Implements ACP `Agent` trait from `agent-client-protocol` crate
- Wraps `peekoo-agent` for core LLM/agent functionality
- Communicates via JSON-RPC over stdio (stdin/stdout)
- Handles task execution prompts with context (title, description, comments, etc.)

## Key Types

- `PeekooAgent` - Main ACP agent implementation
- `TaskContext` - Context passed to agent for task execution

## Binary Target

This crate produces a binary that can be spawned as a subprocess:
```
peekoo-agent-acp
```

The binary reads from stdin and writes to stdout, following ACP protocol.

## Usage

```bash
# Spawn as subprocess
peekoo-agent-acp
```

The scheduler uses ACP Client to communicate with the agent over stdio.
