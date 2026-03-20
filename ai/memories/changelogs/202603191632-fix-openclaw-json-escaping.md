## 2026-03-19 16:32 fix: Harden plugin JSON escaping for OpenClaw websocket payloads

**What changed:**
- Updated `packages/plugin-sdk/assembly/json.ts` `quote()` to escape all JSON control characters (`\b`, `\f`, and `\u00XX` for remaining `0x00-0x1F` bytes), not just slash/quote/newline/tab variants
- Updated `plugins/openclaw-sessions/assembly/index.ts` gateway handshake logic to read `connect.challenge` nonce from `payload.nonce` first and send `device.nonce` as a JSON string (never `null`)
- Updated `plugins/openclaw-sessions/assembly/index.ts` response handling to return the full gateway `res` envelope on success instead of slicing `payload` with string-based raw extraction
- Updated `plugins/openclaw-sessions/ui/panel.html` tool-invoke JSON parsing to surface raw plugin output preview when parsing fails, making malformed payloads directly diagnosable in the UI
- Rebuilt `plugins/openclaw-sessions/build/openclaw_sessions.wasm` so the OpenClaw plugin runtime picks up the new escaping behavior

**Why:**
- OpenClaw gateway requests are JSON-over-websocket; if credential text contains hidden control characters, the previous escaping could generate malformed JSON and surface gateway parse errors such as `JSON Parse error: Expected '}'`
- The current gateway connect schema rejects `device.nonce = null` with `invalid connect params: at /device/nonce: must be string`; this blocked `refresh_sessions` before `sessions.list` could run
- The previous payload extraction depended on manual JSON substring parsing, which is brittle for large nested payloads; returning the full envelope avoids truncation-induced parse failures in the panel

**Files affected:**
- `packages/plugin-sdk/assembly/json.ts`
- `plugins/openclaw-sessions/build/openclaw_sessions.wasm`
