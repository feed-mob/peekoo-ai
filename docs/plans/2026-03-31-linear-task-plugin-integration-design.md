# Linear 任务管理插件集成产品设计文档

**日期**: 2026-03-31  
**状态**: Implemented (API Key)  
**作者**: Peekoo 产品/工程

---

## 1. 背景

当前 Peekoo 已有基础任务管理能力（本地任务 CRUD、任务状态与活动流），并已具备插件系统（WASM 插件、插件商店、插件权限、插件面板）。

我们希望新增 Linear 集成，让用户可以把 Linear 作为第三方任务系统接入 Peekoo，且该能力必须遵守以下原则：

1. Linear 作为独立插件存在。
2. 只有用户安装并启用插件后才生效。
3. 不破坏未安装插件用户的现有任务体验。

---

## 2. 目标与范围

### 2.1 本期目标（In Scope）

1. 实现 Linear 连接能力（API Key）。
2. 实现 Linear 与 Peekoo 任务双向同步。
3. 实现连接状态管理，并在设置界面可查看。

### 2.2 非目标（Out of Scope）

1. 不实现多第三方平台统一抽象层（本期仅 Linear）。
2. 不实现评论/附件/子任务的完整双向同步（仅任务主体字段）。
3. 不在本期引入云端中转服务（桌面端本地直连 Linear API）。

---

## 3. 用户故事

1. 作为 Peekoo 用户，我可以在插件商店安装 Linear 插件，并在插件面板中配置我的 Linear API Key。
2. 作为已连接用户，我希望 Linear 里的任务自动出现在 Peekoo 任务列表中。
3. 作为已连接用户，我在 Peekoo 创建或更新任务后，可以同步到 Linear。
4. 作为已连接用户，我可以在设置页看到当前连接状态（已连接/未连接/同步失败等）。

---

## 4. 验收标准映射

| 验收标准 | 设计落点 |
|---|---|
| 用户可以连接他们的 Linear 帐户 | Linear 插件提供 `linear_set_api_key/linear_disconnect` 工具与面板连接流程 |
| 第三方任务可以同步到 Peekoo | 周期性 Linear -> Peekoo 拉取增量并创建/更新本地任务 |
| Peekoo 任务可以同步到第三方系统 | 周期性 Peekoo -> Linear 推送增量并创建/更新远程 issue |
| 连接状态可在设置界面中查看 | 设置页新增 Integrations 区块，读取 Linear 插件状态数据提供器 |

---

## 5. 产品方案概览

### 5.1 方案选型

采用“**插件内完成 API Key 鉴权 + 同步编排**”方案，核心应用仅提供：

1. 插件运行时（已存在）。
2. 任务 Host 能力（`peekoo_task_*`，已存在）。
3. 设置页状态展示入口（新增轻量 UI）。

### 5.2 为什么这样做

1. 满足“独立插件、安装后生效”要求。
2. 复用现有插件化面板交互模式，降低实现风险。
3. 将第三方 API 耦合限制在插件内，减少核心应用复杂度。

---

## 6. 信息架构与交互

### 6.1 插件安装与启用

1. 用户在 `Plugins -> Store` 安装 `Linear` 插件。
2. 用户在 `Installed` 中启用插件。
3. 插件启用后才注册工具、面板、定时同步。

### 6.2 连接流程

1. 用户打开 Linear 插件面板，输入 `Linear API Key` 并点击保存。
2. 插件调用 Linear GraphQL `viewer` 接口校验 API Key，并缓存 workspace/user 信息。
3. 校验成功后将密钥保存到插件 secret storage，状态切换为 `connected`。
4. 首次连接成功后执行一次初始化同步（先拉后推，防止重复创建）。

### 6.3 设置页状态展示

设置页新增 `Integrations` 区块，展示 Linear 条目：

1. 未安装
2. 已安装但未启用
3. 未连接
4. 已连接
5. 同步中
6. 同步异常（含最后错误时间）

---

## 7. 功能需求（FR）

### FR-1 连接管理

1. 支持 API Key 连接、断开连接。
2. 连接成功后保存账号信息（workspace/user display）与密钥元数据。
3. API Key 无效或撤销时转为 `error`，提示重新配置。

### FR-2 双向任务同步

1. 支持 Linear -> Peekoo 增量同步（按 `updatedAt` 游标）。
2. 支持 Peekoo -> Linear 增量同步（按本地 `updated_at` 与同步影子状态）。
3. 支持首次全量导入（范围可配置：仅未完成 + 最近 N 天已完成）。
4. 支持手动“立即同步”。

### FR-3 连接状态管理

1. 插件维护明确状态机（见第 9 节）。
2. 状态可通过数据提供器对外查询。
3. 设置页统一显示状态与最后同步时间。

---

## 8. 技术设计（基于现有架构）

### 8.1 新增插件

新增目录：`plugins/linear/`

建议清单：

1. `plugins/linear/peekoo-plugin.toml`
2. `plugins/linear/src/lib.rs`
3. `plugins/linear/ui/panel.html`
4. `plugins/linear/ui/panel.css`
5. `plugins/linear/ui/panel.js`

manifest 权限：

1. `secrets:read`
2. `secrets:write`
3. `state:read`
4. `state:write`
5. `net:http`
6. `scheduler`
7. `tasks`

`allowed_hosts` 建议至少包含：

1. `api.linear.app`
2. `linear.app`（可选，仅用于未来文档跳转/扩展）

### 8.2 SDK 增强（必要）

当前 Host 已有 `peekoo_task_*`，但 SDK 尚无对外安全封装。为支持插件后台同步，新增：

1. `crates/peekoo-plugin-sdk/src/tasks.rs`
2. 在 `lib.rs` 与 `prelude` 中导出 `peekoo::tasks`

封装方法：

1. `tasks::create`
2. `tasks::list`
3. `tasks::update`
4. `tasks::delete`
5. `tasks::toggle`
6. `tasks::assign`

### 8.3 设置页改造（最小可行）

新增设置页 Integration 状态卡片：

1. 读取 `plugins_list` 判断是否安装/启用 Linear。
2. 若插件启用，调用 `plugin_query_data(plugin_key="linear", provider_name="connection_status")`。
3. 展示连接状态、最后同步时间、错误摘要。

建议文件：

1. `apps/desktop-ui/src/features/settings/SettingsPanel.tsx`
2. `apps/desktop-ui/src/features/settings/useLinearIntegrationStatus.ts`（新增）

---

## 9. 数据模型与状态机

### 9.1 插件状态存储（plugin state）

使用插件私有 state 保存同步状态，避免核心库表结构膨胀：

```json
{
  "connection": {
    "status": "connected",
    "workspaceName": "Acme",
    "userName": "Alice",
    "credentialType": "api_key",
    "lastError": null
  },
  "sync": {
    "lastSyncAt": "2026-03-31T07:20:00Z",
    "lastPullCursor": "2026-03-31T07:19:00Z",
    "lastPushCursor": "2026-03-31T07:19:20Z",
    "errorCount": 0,
    "nextRunAt": "2026-03-31T07:25:00Z"
  },
  "mapping": {
    "taskToIssue": {
      "task_uuid": "linear_issue_id"
    },
    "issueToTask": {
      "linear_issue_id": "task_uuid"
    },
    "shadow": {
      "task_uuid": {
        "lastLocalUpdatedAt": "...",
        "lastRemoteUpdatedAt": "..."
      }
    }
  }
}
```

### 9.2 连接状态机

`uninstalled -> disabled -> disconnected -> connected -> syncing -> connected`

异常分支：

1. 任意状态遇到网络/鉴权错误 -> `error`。
2. API Key 失效/权限不足 -> `error`，等待用户重新配置密钥。

---

## 10. 同步策略

### 10.1 同步触发

1. 插件启动后注册周期任务（默认每 5 分钟）。
2. 用户点击“立即同步”触发一次手动同步。
3. 首次连接成功后立即触发一次全量初始化同步。

### 10.2 字段映射（V1）

| Peekoo | Linear |
|---|---|
| `title` | `issue.title` |
| `description` | `issue.description` |
| `status` (`todo/in_progress/done/cancelled`) | 映射到 Linear workflow state |
| `priority` (`low/medium/high`) | 映射到 Linear priority |
| `labels` | 仅同步 `linear:*` 前缀标签（避免污染） |

### 10.3 冲突处理

1. 若两端都发生更新，按 `updatedAt` 最近者优先（LWW）。
2. 被覆盖一侧写入任务活动日志（`event_type=comment`，payload 标记 `sync_conflict_resolved`）。
3. 若远端删除：默认在 Peekoo 标记 `cancelled`，不硬删除。

### 10.4 首次同步规则

1. 默认先拉取 Linear（避免本地空白体验）。
2. 本地 -> Linear 推送仅针对：
   - 已存在映射的任务更新。
   - 用户开启“自动推送新建 Peekoo 任务到 Linear”后新建任务。

---

## 11. Linear API 约束（实施注意）

1. GraphQL endpoint：`https://api.linear.app/graphql`。
2. 需处理 `RATELIMITED` 错误码与相关限流头，采用指数退避。
3. 桌面端本期不依赖 webhook（需要公网 HTTPS 回调），采用增量轮询。
4. API Key 建议最小权限原则，并支持用户随时轮换与重置。

---

## 12. 实施阶段

### Phase 1：插件骨架与连接能力

1. 新建 `plugins/linear`。
2. 完成 API Key set/disconnect/status。
3. 面板展示连接状态。

### Phase 2：双向同步 MVP

1. 增加 SDK tasks 封装。
2. 完成拉取/推送增量同步与 mapping。
3. 增加手动同步、周期同步。

### Phase 3：设置页状态管理

1. 设置页新增 Integrations 区块。
2. 展示 Linear 连接与同步状态。
3. 异常状态提示与重连入口。

### Phase 4：稳定性与体验

1. 冲突日志与错误恢复。
2. 限流退避与重试策略。
3. 同步性能与边界测试。

---

## 13. 测试与验收计划

### 13.1 核心测试

1. API Key 校验成功/失败/撤销场景。
2. Linear -> Peekoo 创建、更新、关闭同步。
3. Peekoo -> Linear 创建、更新、完成同步。
4. 双向并发编辑冲突处理。
5. API Key 失效后错误提示与重连恢复。
6. 插件禁用/卸载后同步停止。

### 13.2 UI 验收

1. 插件面板可完成连接与手动同步。
2. 设置页可看到连接状态与最后同步时间。
3. 异常时有可理解错误信息。

---

## 14. 风险与缓解

1. **风险：无 webhook 导致同步延迟**  
   缓解：默认 5 分钟周期 + 手动立即同步。
2. **风险：限流导致同步失败**  
   缓解：增量查询、最小字段、指数退避。
3. **风险：字段语义不一致（状态流）**  
   缓解：团队级状态映射配置，提供默认映射并可调整。
4. **风险：重复创建任务**  
   缓解：首次同步“先拉后推” + 映射索引去重。

---

## 15. 成功指标（上线后）

1. 连接成功率 >= 95%。
2. 同步成功率 >= 98%（24h 维度）。
3. 冲突自动解决率 >= 90%。
4. 设置页状态查询失败率 < 1%。
