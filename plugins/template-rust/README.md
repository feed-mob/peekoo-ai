# Rust Plugin Template

Scaffold a new Peekoo Rust plugin with `cargo-generate`:

```bash
cargo generate --path plugins/template-rust --destination plugins --name my-plugin
```

The generated plugin includes:
- `peekoo-plugin.toml`
- `Cargo.toml` with `peekoo-plugin-sdk`
- `.cargo/config.toml` targeting `wasm32-wasip1`
- `src/lib.rs` with `plugin_init` and a sample `tool_greet`

Build it with:

```bash
just plugin-build my-plugin
```
