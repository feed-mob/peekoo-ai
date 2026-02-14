# Desktop Tauri App

This is the Tauri-based desktop application for Peekoo.

## Comparison with GPUI

| Feature | Tauri | GPUI |
|---------|-------|------|
| Plugin UI | ✅ Custom React components | ❌ Data-driven only |
| Cross-platform | ✅ Windows/macOS/Linux | ❌ macOS/Linux only |
| Maturity | ✅ Stable v2 | ❌ pre-1.0 |
| Performance | WebView overhead | ✅ Native GPU |
| Bundle size | ~600KB base | Smaller |

## Testing

Compare both implementations:
```bash
# Tauri version
cargo run --bin peekoo-desktop-tauri

# GPUI version  
cargo run --bin peekoo-gpui-app
```
