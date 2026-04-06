# Mijia Smart Home Plugin

This plugin runs on the shared Peekoo Python SDK runtime, so end users do not need system Python installed.

## Build shared Python SDK runtime

```bash
just plugin-install-mijia-python-sdk
```

This script will:
- build a standalone Python runtime package with Mijia dependencies
- install it to `~/.peekoo/python-sdk/python`
- install Python dependencies from `companions/requirements.txt`

You can override the runtime archive URL:

```bash
PEEKOO_PYTHON_STANDALONE_URL="<archive-url>" just plugin-install-mijia-python-sdk
```

## Build and install plugin

```bash
just plugin-mijia-smart-home
```

This command packages runtime + builds WASM + installs the plugin to `~/.peekoo/plugins/mijia-smart-home`.

## Runtime lookup order

At runtime, the plugin tries Python interpreters in this order:
1. `~/.peekoo/python-sdk/python/bin/python3`
2. `~/.peekoo/python-sdk/python/bin/python`
3. `~/.peekoo/python-sdk/python/python.exe`
4. `~/.peekoo/python-sdk/python/bin/python.exe`
5. `runtime/python/bin/python3` (plugin-local fallback)
6. `runtime/python/bin/python` (plugin-local fallback)
7. `runtime/python/python.exe` (plugin-local fallback)
8. `runtime/python/bin/python.exe` (plugin-local fallback)
9. system `python3`
10. system `python`
11. system `py`
