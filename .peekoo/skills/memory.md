---
name: memory_manager
description: Update your long-term memory when you learn new user preferences, project context, or durable facts.
---

# Automatic Memory Management

You have a special directive: you must proactively curation and update your **long-term memory** files.
Your long-term memory is fed directly into your system prompt on initialization, ensuring you do not forget important project details or user rules across sessions.

## Where Memories Live
Your memories live in your workspace's `.peekoo` persona directory:
1. `.peekoo/memory.md` - Core project facts and primary memory.
2. `.peekoo/memories/*.md` - Topic-specific memory files (e.g., `user_prefs.md`, `deploy_notes.md`).

## When to update memory
You should update your memory files ONLY when:
1. The user explicitly asks you to remember something.
2. You successfully complete a major task or figure out a complex bug, and the solution is a durable fact that you will need to know in the future.
3. The user states a preference for how you should behave, format code, or communicate.

You should NOT update memory for transient tasks, daily logs, or temporary scratchpad notes. Memories are for *durable* facts.

## How to update memory
Use your built-in filesystem tools to curate memory:

1. **Adding a small fact / preference**: 
   - Use `read` or `view_file` to read `.peekoo/memory.md`.
   - Use `replace_file_content` to append or edit the relevant section.
   
2. **Adding a large, entirely new topic**:
   - If a topic is sufficiently large or distinct (e.g., "Architecture of the Billing Service"), create a *new* file in the memories folder.
   - Use `write_to_file` on `.peekoo/memories/billing_architecture.md`.

*Important: Do not overwrite the entire file with `write_to_file` if you just mean to append or edit a line. Use `replace_file_content`.*
