## 2026-03-13 10:00: fix: resolve Windows LNK1169 linker error caused by duplicate wasmtime versions

**What changed:**
- Disabled `default-features` on `pi_agent_rust` in `crates/peekoo-agent/Cargo.toml`, explicitly opting in to `image-resize`, `clipboard`, and `sqlite-sessions` features only -- excluding `wasm-host` and `jemalloc`.
- Applied the same change to the dev-dependency in `crates/peekoo-agent-app/Cargo.toml`.

**Why:**
- Windows MSVC (`LINK.exe`) was failing with `LNK1169: one or more multiply defined symbols` due to two different versions of `wasmtime` being linked into the same binary:
  - `wasmtime 37.0.3` via `peekoo-plugin-host` -> `extism 1.13`
  - `wasmtime 41.0.4` via `peekoo-agent` -> `pi_agent_rust 0.1.7`
- Both versions compile a C file (`gdbjit.c`) that defines the global C symbols `__jit_debug_descriptor` and `__jit_debug_register_code` without weak linkage on Windows, causing a fatal duplicate-symbol error.
- On Linux/macOS these symbols use `__attribute__((weak))` so the linker silently deduplicates them -- the error only manifests on Windows.
- `pi_agent_rust`'s `wasm-host` feature is the sole source of `wasmtime 41`. It enables WASM-based tool execution inside pi's agent runtime, which peekoo does not use -- the project has its own WASM plugin system via `peekoo-plugin-host` and `extism`.
- Disabling `wasm-host` and `jemalloc` (which also has known Windows MSVC issues) removes `wasmtime 41` entirely from the dependency tree.

**Architecture note:**
- `peekoo-agent` uses `pi_agent_rust` purely for LLM session management (`AgentSessionHandle`, `create_agent_session`, prompt/subscribe/set_model). None of these code paths require `wasm-host`.
- The WASM plugin system remains fully functional -- it runs through `peekoo-plugin-host` -> `extism` -> `wasmtime 37`, unchanged.

**Files affected:**
- `crates/peekoo-agent/Cargo.toml` (disabled default features on `pi_agent_rust`)
- `crates/peekoo-agent-app/Cargo.toml` (same, dev-dependency)

**Verification:**
- `cargo check` -- clean, 0 errors
- `cargo tree -i wasmtime` -- only `wasmtime v37.0.3` present; `wasmtime 41.0.4` fully removed
- `cargo tree -d` -- no duplicate wasmtime crates
