# AssemblyScript Example Plugin

Minimal Peekoo plugin written in AssemblyScript.

Build it with:

```bash
just plugin-build-as as-example-minimal
```

Install it locally with:

```bash
just plugin-install-as as-example-minimal
```

This plugin demonstrates:
- `plugin_init`
- one tool export: `tool_as_example_echo`
- plugin state via `@peekoo/plugin-sdk`
