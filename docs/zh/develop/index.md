# 开发总览

Peekoo 是一个基于 Tauri 的桌面应用，前端使用 React，后端由多个 Rust crate 组成。

## 本地开发

```bash
just setup
just dev
```

常用命令：

```bash
just check
just test
just fmt
just lint
just build
```

## 主要目录

- `apps/desktop-ui/`：React + Vite 前端
- `apps/desktop-tauri/`：桌面运行时
- `crates/`：Agent 运行时、认证、生产力、持久化和安全相关的后端 crate

## 开发文档

- [SDK](./sdk.md)
- [插件开发](./plugins.md)
