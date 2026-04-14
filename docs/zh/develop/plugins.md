# 插件开发

## 概览

Peekoo 插件是通过 Extism 由插件宿主加载的 WASM 模块。插件可以使用 Rust 或 AssemblyScript 编写。

## 插件可以扩展什么

- 可被 Agent 运行时调用的工具
- 在 Peekoo 中渲染的 UI 面板
- 事件订阅
- 持久化状态
- 配置字段

## 快速开始

### Rust

1. 安装目标平台：

```bash
rustup target add wasm32-wasip1
```

2. 在 `plugins/<name>/` 下创建插件。
3. 添加 `peekoo-plugin.toml` 和源代码。
4. 构建并安装：

```bash
just plugin-build <name>
just plugin-install <name>
```

### AssemblyScript

1. 在 `plugins/<name>/` 下创建插件。
2. 安装依赖。
3. 使用 `plugin-build-as` 和 `plugin-install-as` 进行构建与安装。

## 必要文件

典型插件目录通常包括：

- `peekoo-plugin.toml`
- 插件源代码
- 构建后的 `.wasm`
- 可选的 `ui/` 资源

## Manifest 可定义的内容

插件 manifest 可以定义：

- 插件元数据
- 权限
- 工具定义
- 事件订阅
- 配置字段
- 数据提供者
- UI 面板

## 运行时说明

插件运行在沙箱中。宿主函数负责日志、状态和 HTTP 等能力，并受权限控制。
