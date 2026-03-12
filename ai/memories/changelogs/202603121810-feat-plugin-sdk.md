## 2026-03-12 18:10 feat: add plugin SDKs for Rust and AssemblyScript

**What changed:**
- Added `crates/peekoo-plugin-sdk/` with safe wrappers for all 10 Peekoo host functions plus shared plugin types and a `prelude`
- Rewrote `plugins/example-minimal/` to use the Rust SDK instead of hand-written host bindings
- Added `plugins/template-rust/` as a `cargo-generate` starter template for Rust plugins
- Added `packages/plugin-sdk/` as a local `@peekoo/plugin-sdk` AssemblyScript package
- Added `plugins/as-example-minimal/` as a working AssemblyScript plugin example using the new package
- Rewrote `docs/plugin-authoring.md` and updated `justfile` with `check-sdk`, `plugin-build-as`, and expanded plugin build workflows

**Why:**
- Plugin authors were duplicating large amounts of boilerplate for host function bindings and JSON request/response types
- The repo had no documented, supported path for AssemblyScript plugins and the existing docs were outdated about the WASM target and available host APIs
- A shared SDK and templates make plugin authoring faster, safer, and easier to document

**Files affected:**
- `crates/peekoo-plugin-sdk/`
- `plugins/example-minimal/`
- `plugins/template-rust/`
- `packages/plugin-sdk/`
- `plugins/as-example-minimal/`
- `docs/plugin-authoring.md`
- `docs/plans/2026-03-12-plugin-sdk.md`
- `justfile`
