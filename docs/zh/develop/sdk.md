# SDK

## 范围

Peekoo 当前同时存在 Rust 和 JavaScript 方向的 SDK 表面，但仓库里最成熟、最适合实际扩展的仍然是插件 SDK。

## 当前 SDK 形态

### Rust 插件 SDK

`crates/peekoo-plugin-sdk` 为插件作者提供了类型化的宿主绑定。它把宿主函数封装起来，避免插件代码手写原始 `extern` 绑定和 `unsafe` 包装。

主要能力包括：

- 状态访问
- 日志
- 通知
- 调度
- 配置读取
- badge 更新
- 事件发射

### AssemblyScript 插件 SDK

`packages/plugin-sdk` 提供了 AssemblyScript 侧的插件 SDK，目前主要通过本地路径依赖使用。

## 相关包

- `crates/peekoo-plugin-sdk`
- `packages/plugin-sdk`

## 典型用法

当你要在 `plugins/<name>/` 下开发 Peekoo 插件时，就会使用这些 SDK。

典型流程包括：

1. 添加 SDK 依赖
2. 定义 `peekoo-plugin.toml`
3. 实现插件导出函数或工具函数
4. 构建为 WASM
5. 在 Peekoo 中安装并测试

## SDK 解决了什么问题

这些 SDK 的核心价值，是把底层插件样板代码隐藏起来。插件作者不需要手动拼接宿主调用，而是可以直接使用类型化帮助函数和更直接的开发方式。

这在以下场景里尤其有价值：

- 读取或保存插件状态
- 暴露一个或多个工具
- 发射事件
- 渲染插件面板
- 响应调度或通知

## 构建说明

- Rust 插件目标平台为 `wasm32-wasip1`
- AssemblyScript 插件需要提供本地 `abort(...)` 处理器，避免运行时缺少 `env::abort`

## 如何选择 Rust 或 AssemblyScript

如果你更看重强类型、crate 复用，或者已经主要工作在 Rust 侧，优先选 Rust。若你更希望使用接近 TypeScript 的编写体验，或者只是做一个较小的插件，AssemblyScript 会更轻量。

## 下一步阅读

如果你要做实际扩展，建议先看 [插件开发](./plugins.md)。
