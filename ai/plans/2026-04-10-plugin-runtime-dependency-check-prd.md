# Plugin Runtime Dependency Check PRD

## Overview

插件商店当前只区分“已安装 / 可安装 / 可更新”，但没有表达插件对本机系统软件环境的依赖。对于依赖外部运行时或宿主软件的插件，如果用户在缺少依赖的情况下点击安装，往往会在安装后或首次运行时才失败，错误发现过晚，且定位成本高。

本 PRD 目标是在插件安装前引入“依赖环境检查”能力。插件可声明自己依赖的系统软件环境，应用在展示插件商店时主动检查本机环境，并据此决定：

- 是否允许点击 `Install`
- 是否展示红色错误提示
- 是否在插件详情中展示环境依赖说明
- 是否在安装后或更新后重新校验依赖状态

首个明确场景是 `mijia-smart-home` 插件依赖本地 Python 环境，因此如果本机 Python 不可用，安装按钮应置灰，并在按钮附近以红字说明原因。

## Goals

- [ ] 为插件 manifest 新增一组可扩展的“运行时依赖”声明字段
- [ ] 在插件商店目录拉取后，对声明了依赖的插件执行本机环境检查
- [ ] 当依赖不满足时，禁用安装按钮，并显示清晰、可操作的红字提示
- [ ] 支持常见运行时/系统软件依赖，包括 Python、Node.js、.NET、Ruby、Rust、Java
- [ ] 设计结果可扩展到未来更多依赖类型，如 CLI 工具、系统应用、路径文件、环境变量

## Non-Goals

- [ ] 本期不负责自动安装缺失依赖
- [ ] 本期不负责修复系统环境，只负责检测与提示
- [ ] 本期不做复杂版本求解器，不做跨依赖冲突分析
- [ ] 本期不在插件运行中持续监控环境变化，只在关键时机触发检查

## Problem Statement

当前插件系统默认假设：

- 插件下载成功即可安装
- 插件被启用后即可工作

这个假设对纯 WASM 插件成立，但对需要系统软件配合的插件不成立。典型问题包括：

- 插件依赖 `python3`，但用户机器没有 Python
- 插件依赖 `node`，但 PATH 中不可用
- 插件依赖 Java 或 .NET 运行时，但版本不满足
- 插件依赖 Ruby 或 Rust 工具链，但宿主只装了部分组件

用户视角会出现两个明显问题：

- 安装入口没有提前告知风险
- 错误发生在点击后或运行时，体验像“安装成功但不能用”

## User Scenarios

### Scenario 1: Python 依赖缺失

用户打开插件商店，看到 `Mijia Smart Home Manager`。系统发现该插件声明依赖 `python3 >= 3.10`，但本机未检测到可用 Python。

期望行为：

- `Install` 按钮置灰
- 插件卡片展示红字：`需要 Python 3 环境，当前未检测到可用 python3`
- 鼠标悬浮或详情区域可显示更完整说明，例如期望命令、版本要求、建议安装方式

### Scenario 2: 运行时存在但版本过低

某插件声明依赖 `node >= 20`，本机检测到 `node 18.19.0`。

期望行为：

- `Install` 按钮置灰
- 红字提示：`需要 Node.js >= 20，当前检测到 18.19.0`

### Scenario 3: 依赖满足

某插件声明依赖 Java 运行时，本机检测通过。

期望行为：

- `Install` 正常可点击
- 插件可选展示“环境检查通过”或不额外展示错误信息

### Scenario 4: 已安装插件在系统升级后依赖失效

用户已安装某插件，后来本机环境变化导致依赖失效。

期望行为：

- 已安装插件列表可显示 warning 状态，但不强制自动卸载
- 插件启用或打开面板时，可再次提示环境缺失

## Product Requirements

### 1. Manifest 新增依赖声明

在 `peekoo-plugin.toml` 中新增 `runtime_dependencies`（命名可讨论，建议避免与 WASM/插件内依赖混淆）。建议作为顶层数组字段：

```toml
[[runtime_dependencies]]
kind = "python"
required = true
min_version = "3.10"
command = "python3"
display_name = "Python 3"
install_hint = "请安装 Python 3，并确保 python3 可在 PATH 中访问"

[[runtime_dependencies]]
kind = "node"
required = false
min_version = "20.0.0"
command = "node"
display_name = "Node.js"
```

建议字段说明：

| 字段 | 必填 | 说明 |
|------|------|------|
| `kind` | 是 | 依赖类型，首批支持 `python` `node` `dotnet` `ruby` `rust` `java` |
| `required` | 否 | 是否为硬性依赖，默认 `true` |
| `min_version` | 否 | 最低版本要求 |
| `max_version` | 否 | 最高版本要求，首期可只存储不强校验 |
| `command` | 否 | 用于检测的命令，如 `python3`、`node`、`java` |
| `display_name` | 否 | 前端显示名称，缺省时按 `kind` 生成 |
| `install_hint` | 否 | 给用户的安装提示文案 |
| `platforms` | 否 | 生效平台，如 `["macos", "windows", "linux"]` |
| `notes` | 否 | 作者补充说明 |

说明：

- `kind` 用于决定默认探测逻辑
- `command` 用于覆盖默认命令名
- `install_hint` 让插件作者给出最贴近插件场景的提示
- `platforms` 用于处理平台差异，例如 Windows 优先检查 `py` 或 `python`

### 2. 默认依赖探测规则

应用内置一套按 `kind` 匹配的默认检测器。首批建议支持：

| `kind` | 默认命令候选 | 版本命令策略 | 备注 |
|------|------|------|------|
| `python` | `python3`, `python`, Windows 可补 `py -3` | `--version` | 输出可能在 stdout 或 stderr |
| `node` | `node` | `--version` | 版本格式通常为 `v20.11.0` |
| `dotnet` | `dotnet` | `--version` | 用于 .NET SDK/Runtime 基础探测 |
| `ruby` | `ruby` | `--version` | 版本格式较稳定 |
| `rust` | `rustc`, `cargo` | `--version` | 优先 `rustc`，必要时可扩展检查 `cargo` |
| `java` | `java` | `-version` | 版本信息通常在 stderr |

设计原则：

- 优先使用轻量、只读、无副作用的版本命令
- 同一 `kind` 允许多个候选命令逐个尝试
- 版本解析失败时，若命令存在，则状态为“存在但无法确认版本”
- 如果 `required = false`，即便失败也不禁用安装，但可以展示 warning

### 3. 检查结果模型

系统应为每个插件返回结构化检查结果，而不是只返回字符串。

建议模型：

```ts
type DependencyCheckStatus = "satisfied" | "missing" | "version_mismatch" | "unknown";

interface RuntimeDependencyStatus {
  kind: string;
  required: boolean;
  displayName: string;
  commandTried: string | null;
  status: DependencyCheckStatus;
  detectedVersion: string | null;
  minVersion: string | null;
  message: string;
  installHint: string | null;
}
```

插件整体可聚合为：

```ts
interface PluginDependencySummary {
  hasRequiredDependencies: boolean;
  blockingIssues: number;
  warnings: number;
  dependencies: RuntimeDependencyStatus[];
}
```

### 4. 安装按钮交互规则

#### Store 页面

- 当 `hasRequiredDependencies = false` 时：
  - `Install` 按钮置灰
  - 按钮文案保持 `Install`，避免引入新的主动作
  - 按钮附近展示红字错误
- 当只有非必需依赖失败时：
  - `Install` 可点击
  - 展示黄色或弱提示 warning

#### 已安装插件页面

- 不因依赖失效自动卸载插件
- 可在插件行上展示 `Environment issue` / `环境异常`
- 若插件被用户重新启用，可在启用前再次检查并阻止启用，或允许启用但提示风险

建议首期策略：

- `安装` 前强校验并阻止
- `启用` 时先不阻止，只展示 warning

这样可以降低对现有已安装用户的破坏性

### 5. 错误文案要求

前端文案需要满足：

- 短
- 明确指出缺什么
- 最好包含版本要求
- 尽量给出命令名

示例：

- `需要 Python 3.10+，当前未检测到 python3`
- `需要 Node.js 20+，当前检测到 v18.19.0`
- `需要 Java 运行时，版本信息无法识别，请确认 java -version 可正常执行`

## Proposed UX

### Store 卡片最小改动

在现有插件卡片中新增“依赖状态区域”：

- 位于描述和权限标签之间，或位于安装按钮下方
- 有阻塞问题时显示红字
- 有 warning 时显示黄字
- 无问题时不强制展示

建议样式：

- 红字：`text-danger`
- Warning：使用现有 muted/warn 样式，避免和权限 badge 混淆

示例文案：

- `环境要求: Python 3.10+ 未满足`
- `环境要求: Node.js 20+ 已满足`

如果需要更强可解释性，可增加可折叠详情：

- `检查命令: python3 --version`
- `检测结果: command not found`
- `建议: 安装 Python 3 并重新打开插件商店`

### 交互时机

建议在以下时机触发检查：

1. 打开 Store tab 并拉取 catalog 后
2. 点击刷新按钮时
3. 点击 Install 前再次做一次后端兜底校验
4. 插件更新后重新拉取状态

这样可以避免“页面展示是可安装，但点击时机器环境刚变化”的竞态问题。

## Technical Design

## Manifest Changes

### Rust Manifest Struct

`crates/peekoo-plugin-host/src/manifest.rs`

为 `PluginManifest` 增加：

```rust
#[serde(default)]
pub runtime_dependencies: Vec<RuntimeDependencyDef>,
```

新增结构：

```rust
pub struct RuntimeDependencyDef {
    pub kind: RuntimeDependencyKind,
    pub required: Option<bool>,
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub command: Option<String>,
    pub display_name: Option<String>,
    pub install_hint: Option<String>,
    #[serde(default)]
    pub platforms: Vec<String>,
    pub notes: Option<String>,
}
```

`RuntimeDependencyKind` 首批枚举：

- `python`
- `node`
- `dotnet`
- `ruby`
- `rust`
- `java`
- 预留 `custom`

备注：

- 为了后续扩展，建议保留 `custom`，但首期不开放给商店插件使用
- 这样不会把模型写死在固定几种语言运行时上

## Backend Dependency Check Service

建议新增专门服务，避免把探测逻辑直接塞进 store crate 或 tauri command 中。

候选位置：

- `crates/peekoo-agent-app/src/plugin_dependency.rs`
- 或新增 `crates/peekoo-plugin-runtime-check/`

推荐首期放在 `peekoo-agent-app`：

- 它已经是应用编排层
- `desktop-tauri` 只负责 transport，不应承载探测规则
- `peekoo-plugin-host` 也不应承担系统环境检查职责，否则边界会混入宿主能力探测

### 服务职责

- 接收 manifest 中的 `runtime_dependencies`
- 按平台和 `kind` 执行本机检查
- 标准化输出结果
- 聚合成插件级别状态

## Store DTO Changes

`crates/peekoo-plugin-store/src/lib.rs`

为 `StorePluginDto` 增加：

- `dependency_summary`
- 或扁平字段：
  - `install_allowed`
  - `dependency_errors`
  - `dependency_warnings`

推荐保留结构化字段：

```rust
pub dependency_summary: PluginDependencySummaryDto,
```

理由：

- 前端无需自行推导禁用逻辑
- 为后续插件详情页保留更多信息
- 便于国际化前后端职责划分

## Frontend Type Changes

`apps/desktop-ui/src/types/plugin.ts`

需要新增：

- `runtimeDependencyStatusSchema`
- `pluginDependencySummarySchema`
- `storePluginSchema` 中加入依赖状态

前端安装按钮禁用条件从：

- `disabled={installing}`

变为：

- `disabled={installing || !plugin.dependencySummary.hasRequiredDependencies}`

## Frontend Rendering Rules

`apps/desktop-ui/src/features/plugins/PluginStoreCatalog.tsx`

需要新增显示逻辑：

- 若存在 blocking issue，渲染红字信息
- 若存在 warning，渲染浅黄提示
- 安装按钮置灰并可配 tooltip

建议不要只显示“依赖检查失败”，而是展示首条阻塞原因，附带查看更多的入口。

示例：

- 首条：`需要 Python 3.10+，当前未检测到 python3`
- 其余：`还有 1 项环境要求未满足`

## Tauri Command / API Considerations

两种方案：

### 方案 A: catalog 拉取时后端直接附带依赖检查结果

优点：

- 前端简单
- 检查和安装判定一致
- 减少前端多次 invoke

缺点：

- 每次拉 catalog 都会执行本机命令探测

### 方案 B: catalog 与 dependency check 分离

优点：

- 责任更清晰
- 可按需懒加载

缺点：

- 前端组合逻辑更复杂
- 安装前仍需要后端兜底

推荐方案：`A`

因为插件商店数据量通常不大，且系统依赖检查是轻量级命令调用，优先保证交互一致性。

## Runtime Support Matrix

首批支持的软件依赖类别如下：

| 类别 | `kind` | 典型用途 | 默认阻塞安装 | 版本校验 |
|------|------|------|------|------|
| Python | `python` | companion 脚本、自动化、AI 工具桥接 | 是 | 是 |
| Node.js | `node` | JS companion、开发工具、脚本型插件 | 是 | 是 |
| .NET | `dotnet` | Windows 工具链或宿主程序集桥接 | 是 | 是 |
| Ruby | `ruby` | Ruby 脚本桥接 | 是 | 是 |
| Rust | `rust` | 需要本地工具链的开发型插件 | 视插件而定 | 是 |
| Java | `java` | Java/JAR companion、设备桥接 | 是 | 是 |

后续可扩展：

- `go`
- `bun`
- `deno`
- `powershell`
- `git`
- 自定义 CLI 二进制

## Platform Differences

平台差异需要进入设计，而不是实现时临时补丁。

### macOS / Linux

- 主要依赖 PATH 中的可执行文件
- 用户可能通过 Homebrew、asdf、mise、pyenv、nvm 安装运行时

### Windows

- `python` 可能需要优先尝试 `py -3`
- `java -version`、`dotnet --version`、`ruby --version` 的输出处理可能与 Unix 略有差异
- 某些运行时安装在 PATH 外，首期不做注册表深挖

首期策略：

- 只做 PATH 可达性检测
- 不扫注册表、不扫固定安装目录
- 保持逻辑简单稳定

## Security & Performance

### Security

- 只执行固定版本查询命令，如 `--version`
- 不拼接来自远端插件的任意 shell 脚本
- `command` 字段若允许覆盖，也必须按“命令 + 固定参数”方式执行，不能交给 shell 拼接
- 不记录敏感环境变量

### Performance

- catalog 中每个插件可能声明多个依赖，但商店插件数预计有限
- 可以按 `kind + command` 做本次请求内缓存，避免重复检查
- 示例：多个插件都依赖 `python3` 时，本次 catalog 拉取只执行一次 Python 探测

## Metrics / Success Criteria

- 用户在缺少依赖时，不会进入“安装成功但不可用”的状态
- 缺少系统依赖导致的插件失败反馈明显下降
- 支持环境依赖的插件安装失败率下降
- 用户能从界面直接理解缺失的软件环境

## Rollout Plan

### Phase 1

- Manifest 增加 `runtime_dependencies`
- 支持 `python` `node` `dotnet` `ruby` `rust` `java`
- Store catalog 返回依赖状态
- Store 页面禁用安装按钮并显示红字
- `mijia-smart-home` 补充 Python 依赖声明

### Phase 2

- 已安装插件页展示环境异常状态
- 启用插件前进行依赖提示
- 支持 warning 与 optional dependency

### Phase 3

- 增加“查看安装指引”交互
- 增加更多 runtime 类型和自定义探测器
- 视需要加入平台特定安装建议链接

## Files Likely To Change

- `crates/peekoo-plugin-host/src/manifest.rs`
- `crates/peekoo-plugin-store/src/lib.rs`
- `crates/peekoo-agent-app/src/plugin.rs`
- `apps/desktop-ui/src/types/plugin.ts`
- `apps/desktop-ui/src/features/plugins/PluginStoreCatalog.tsx`
- `apps/desktop-ui/src/hooks/use-plugin-store.ts`
- `plugins/mijia-smart-home/peekoo-plugin.toml`
- `docs/plugin-authoring.md`

## Testing Strategy

- Rust 单测：
  - manifest 新字段解析
  - 各类版本字符串解析
  - 缺失命令 / 版本过低 / 版本未知的结果聚合
- 前端单测：
  - 有阻塞依赖时按钮禁用
  - 红字文案渲染
  - optional dependency 不阻塞安装
- 集成验证：
  - `mijia-smart-home` 在无 Python 环境下显示禁用安装
  - 有 Python 环境时恢复可安装

## Open Questions

- `runtime_dependencies` 是否放顶层，还是放在 `[plugin]` 下更符合 manifest 习惯
- 首期是否需要支持 `custom` 命令型依赖，还是只允许白名单 `kind`
- 已安装插件依赖失效时，是否阻止“启用”动作
- 是否需要在 UI 中暴露“重新检查环境”按钮
- 版本比较是否统一采用 semver 宽松解析，还是对 Java/Python 做定制处理

## Recommendation

建议先以“白名单运行时 + PATH 检测 + Store 页面阻塞安装”为最小闭环上线。

原因：

- 能直接解决米家插件这类最明确的痛点
- manifest 字段设计足够向后扩展
- 不会把首版复杂度推高到“自动安装器”或“系统环境诊断器”
- 现有架构中容易落位，且不破坏 `desktop-tauri` 作为 transport layer 的边界
