## 2026-03-31 10:19: fix: harden node runtime install paths and archive extraction

**What changed:**
- Updated `peekoo-node-runtime` to resolve its data directory through `peekoo-paths` so runtime assets follow the same `~/.peekoo` convention as the rest of the workspace.
- Switched Node runtime directory setup to `create_dir_all` so first-run installs succeed when parent `resources/` directories do not exist yet.
- Hardened zip extraction to reject absolute paths and `..` traversal entries before writing files.
- Added regression tests covering the shared data-dir contract and unsafe zip entry rejection.

**Why:**
- The initial crate introduced a divergent storage root, could fail managed installs on a clean machine, and trusted zip archive paths too broadly.

**Files affected:**
- `crates/peekoo-node-runtime/src/paths.rs`
- `crates/peekoo-node-runtime/src/node_runtime.rs`
- `crates/peekoo-node-runtime/src/archive.rs`
