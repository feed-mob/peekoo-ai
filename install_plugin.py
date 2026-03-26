import os
import shutil
import json
from pathlib import Path

plugin_dir = Path("plugins") / "pomodoro"
wasm_src = plugin_dir / "target" / "wasm32-wasip1" / "release" / "pomodoro.wasm"
home = Path.home()
dest_dir = home / ".peekoo" / "plugins" / "pomodoro"
wasm_dest_dir = dest_dir / "target" / "wasm32-wasip1" / "release"

print(f"Creating {wasm_dest_dir}")
wasm_dest_dir.mkdir(parents=True, exist_ok=True)

print(f"Copying {wasm_src} to {wasm_dest_dir}")
shutil.copy2(wasm_src, wasm_dest_dir / "pomodoro.wasm")

print(f"Copying peekoo-plugin.toml to {dest_dir}")
shutil.copy2(plugin_dir / "peekoo-plugin.toml", dest_dir / "peekoo-plugin.toml")
print("Done")
