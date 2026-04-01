---
name: memory-manager
description: Update durable workspace memory when you learn user preferences, stable project context, or other long-lived facts.
---

# Automatic Memory Management

You must keep durable memory concise, factual, and useful across future sessions.

The files you can update are in your current workspace directory.

## Memory Files

- `USER.md`: structured user profile and preferred form of address
- `MEMORY.md`: durable preferences and long-term facts
- `memories/*.md`: optional topic-specific durable notes

## When To Update Memory

Update memory only when at least one of these is true:

1. The user explicitly asks you to remember something.
2. The user states a durable preference about reminders, tone, work habits, or how you should help.
3. The user corrects or updates profile information in `USER.md`.
4. You finish a substantial task and the result is a durable fact that should matter in future sessions.

Do not save:

- temporary tasks
- one-off requests
- scratch notes
- raw conversation transcripts
- secrets, unless the user explicitly wants them stored for future use

## How To Update Memory

Use only the built-in filesystem tools that actually exist.

1. Read the relevant file first with `read`.
2. Use `edit` for targeted changes when a file already exists.
3. Use `write` only when creating a new memory file or replacing placeholder template content intentionally.

## File Choice Rules

- If the fact is about who the user is or how to address them, update `USER.md`.
- If the fact is a durable general preference or long-lived project fact, update `MEMORY.md`.
- If the fact belongs to a clearly separate topic and is large enough to justify separation, create `memories/<topic>.md`.

## Style Rules

- Keep entries short and specific.
- Prefer updating an existing section over adding duplicates.
- Keep memory organized so it can be read back into future prompts without clutter.
