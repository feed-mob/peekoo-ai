/**
 * Minimal Peekoo plugin — AssemblyScript version.
 *
 * Mirrors the Rust `example-minimal` plugin: a single echo tool that
 * counts invocations using plugin state.
 */

import { Host } from "@extism/as-pdk";
import * as state from "@peekoo/plugin-sdk/assembly/state";
import * as log from "@peekoo/plugin-sdk/assembly/log";
import { extractStringField, quote } from "@peekoo/plugin-sdk/assembly/json";

// ── Abort handler (required by AssemblyScript WASM) ────────────

export function myAbort(
  message: string | null,
  fileName: string | null,
  lineNumber: u32,
  columnNumber: u32,
): void {
  const msg = message !== null ? message : "unknown error";
  log.error(`abort: ${msg}`);
}

// ── Plugin lifecycle ───────────────────────────────────────────

export function plugin_init(): i32 {
  log.info("as-example-minimal plugin initialized");
  Host.outputString('{"status":"ok"}');
  return 0;
}

// ── Tool export ────────────────────────────────────────────────

export function tool_as_example_echo(): i32 {
  const inputStr = Host.inputString();
  const echoValue = extractStringField(inputStr, "input");

  // Read and increment call count
  const countStr = state.get("call_count");
  let callCount: u64 = 0;
  if (countStr.length > 0) {
    callCount = u64.parse(countStr);
  }
  callCount += 1;
  state.set("call_count", callCount.toString());

  // Return response
  const output = "{\"echoed\":" + quote(echoValue) + ",\"call_count\":" + callCount.toString() + "}";
  Host.outputString(output);
  return 0;
}
