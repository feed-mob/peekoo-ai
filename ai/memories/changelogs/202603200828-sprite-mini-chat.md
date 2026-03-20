## 2026-03-20 08:28: feat: Add sprite mini chat

**What changed:**
- Added inline mini chat UI for the sprite with a compact input tray and latest-reply bubble.
- Refactored shared chat session behavior into a reusable frontend hook so the sprite and full chat panel use the same agent/session pipeline.
- Extended sprite window sizing rules and added tests for chat session mapping and mini chat layout behavior.
- Added a temporary expanded reading mode for longer mini chat replies, a thinking state bubble, and an inline `Open full chat` affordance.
- Expanded and re-centered the sprite window horizontally for long-form mini chat replies so the reading card fits inside the actual window bounds.

**Why:**
- Let users quick-chat directly from the sprite without losing the full chat history in the main chat panel.

**Files affected:**
- `apps/desktop-ui/src/features/chat/chat-session.ts`
- `apps/desktop-ui/src/features/chat/ChatPanel.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteMiniChat.tsx`
- `apps/desktop-ui/src/components/sprite/SpriteMiniChatBubble.tsx`
- `apps/desktop-ui/src/views/SpriteView.tsx`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.ts`
- `apps/desktop-ui/src/features/chat/chat-session.test.ts`
- `apps/desktop-ui/src/lib/sprite-bubble-layout.test.ts`
