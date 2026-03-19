## 2026-03-19 01:35 fix: Harden OpenClaw JSON field extraction

**What changed:**
- Reworked the AssemblyScript JSON field lookup helper to find top-level object fields without matching quoted text inside string values
- Kept raw/string extraction behavior the same once the correct field value start is found
- Rebuilt the OpenClaw Sessions plugin WASM artifact after updating the shared SDK helper

**Why:**
- The previous `indexOf`-based field lookup could match `"payload":` and similar keys embedded inside message text before the real top-level field
- That produced truncated or malformed JSON during `sessions.list` refreshes, which surfaced in the panel as `JSON Parse error: Unterminated string`

**Files affected:**
- `packages/plugin-sdk/assembly/json.ts`
- `plugins/openclaw-sessions/build/openclaw_sessions.wasm`
