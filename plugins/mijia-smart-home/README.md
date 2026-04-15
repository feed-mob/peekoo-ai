# Mijia Smart Home Plugin

This plugin requires system Python (Python 3 recommended).

During plugin installation/update from the store, Peekoo will automatically run:

- `python3 -m pip install --user -r companions/requirements.txt`

## Build and install plugin

```bash
just plugin-mijia-smart-home
```

This command builds WASM and installs the plugin to `~/.peekoo/plugins/mijia-smart-home`.

## Runtime lookup order

At runtime, the plugin tries Python interpreters in this order:
1. system `python3`
2. system `python`
3. system `py`
