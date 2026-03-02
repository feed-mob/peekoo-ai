# Agent Architecture Changelog

## 2026-03-02
- **Removed Native Rust Skills**: Fully transitioned to Markdown-based Agent Skills following the [agentskills.io](https://agentskills.io) specification. Removed `skill.rs` and native skill injection logic from `peekoo-agent`.
- **OpenClaw-style Personas**: Added support for `.peekoo/IDENTITY.md`, `.peekoo/SOUL.md`, and `.peekoo/memory.md`. These files are automatically loaded and compose the underlying LLM system prompt in a defined hierarchy (`IDENTITY` → `SOUL` → `MEMORY` → `system_prompt` → `agent_skills`).
- **Convention-based Auto-discovery**: By default (`auto_discover: true`), `AgentService::new` automatically scans the working directory for a `.peekoo/` folder (falling back to `~/.peekoo/`) to configure `persona_dir` and `agent_skills` paths.
- **AgentSkills Directory Support**: Skills in `.peekoo/skills/` are auto-discovered, natively supporting both flat `.md` files and nested `skills/xxx/SKILL.md` subdirectories according to the spec.
- **Advanced Memory Management**: `peekoo-agent` now natively concatenates `memory.md` along with any markdown files found inside `.peekoo/memories/*.md`.
- **Self-Editing Memory Skill**: Created the `memory_manager` AgentSkill (`.peekoo/skills/memory.md`) detailing how the agent should proactively curate its long-term memory via file system tools (`read`, `write_to_file`, `replace_file_content`).
