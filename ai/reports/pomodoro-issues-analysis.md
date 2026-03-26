# 番茄钟问题分析报告

## 问题 1: 编译错误

**位置**: `crates/peekoo-pomodoro-app/src/lib.rs:778`

**错误信息**:
```
error: expected a literal
   --> crates\peekoo-pomodoro-app\src\lib.rs:778:17
    |
778 |                 MIGRATION_0010_POMODORO_RUNTIME,
    |                 ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**原因**: 
测试代码中使用 `concat!()` 宏拼接 SQL 迁移脚本，但 `concat!()` 只接受字面量，不接受常量。

**修复方案**:
```rust
// 错误的写法
conn.execute_batch(concat!(
    MIGRATION_0010_POMODORO_RUNTIME,  // ❌ 常量不能用于 concat!
    "\nALTER TABLE..."
))

// 正确的写法
conn.execute_batch(MIGRATION_0010_POMODORO_RUNTIME)?;
conn.execute_batch(
    "ALTER TABLE pomodoro_state ADD COLUMN long_break_minutes INTEGER NOT NULL DEFAULT 15;
     ALTER TABLE pomodoro_state ADD COLUMN long_break_interval INTEGER NOT NULL DEFAULT 4;
     ALTER TABLE pomodoro_state ADD COLUMN auto_advance INTEGER NOT NULL DEFAULT 0;"
)?;
```

---

## 问题 2: Focus 完成后没有庆祝动画和 Memo 弹窗

### 2.1 后端行为分析

**完成流程** (`crates/peekoo-pomodoro-app/src/lib.rs`):

1. **定时器到期时** (`refresh_runtime_if_due()`):
   ```rust
   if status.time_remaining_secs == 0 {
       let record = status.complete(now)?;  // 状态变为 Completed
       insert_cycle_record(&conn, &record)?;
       
       if status.settings.auto_advance {
           // 自动开始下一阶段
           status.start(next_mode, next_minutes, now)?;
       }
       
       save_status(&conn, &status)?;
       
       if status.state == PomodoroState::Running {
           // 自动推进：发送开始情绪
           self.publish_start_mood(&status);
       } else {
           // 非自动推进：发送完成副作用
           self.publish_completion_side_effects(&status, &record.mode);
       }
   }
   ```

2. **完成副作用** (`publish_completion_side_effects()`):
   ```rust
   self.mood_reactions.set("pomodoro-completed", false);  // ✅ 发送情绪事件
   self.notifications.notify(...);  // ✅ 发送系统通知
   self.publish_badges(status);  // ✅ 清除徽章
   ```

### 2.2 前端行为分析

**问题所在** (`apps/desktop-ui/src/features/pomodoro/PomodoroPanel.tsx`):

1. **状态轮询**: 每 3 秒调用 `getPomodoroStatus()`
2. **状态处理**: 只更新 UI 显示，没有检测状态变化
3. **缺失逻辑**:
   - ❌ 没有检测 `Completed` 状态
   - ❌ 没有触发庆祝动画
   - ❌ 没有显示 memo 弹窗

**当前 UI 行为**:
- 状态变为 `Completed` 时，倒计时显示 `00:00`
- 进度条显示 100%
- 按钮显示 "Start"（因为 `isActive = status.state === "Running"` 为 false）
- 没有任何视觉反馈表明完成

### 2.3 情绪反应系统

**后端发送** (`peekoo-pomodoro-app`):
```rust
self.mood_reactions.set("pomodoro-completed", false);
```

**前端映射** (`apps/desktop-ui/src/hooks/use-sprite-reactions.ts`):
```typescript
"pomodoro-completed": "happy",  // ✅ 映射到 happy 动画
```

**问题**: 前端可能没有正确监听 mood_reactions 事件，或者事件在状态轮询之前就被清除了。

### 2.4 Memo 弹窗

**后端支持**:
- `PomodoroCycleRecord.memo_requested` 字段标记是否需要 memo
- 条件: `mode == Work && outcome == Completed && settings.enable_memo`

**前端缺失**:
- ❌ 没有实现完成后的 memo 弹窗
- ✅ 只有历史记录中的 memo 编辑功能

---

## 问题 3: 徽章没有每日刷新

### 3.1 当前行为

**计数器累积**:
```rust
pub struct PomodoroStatus {
    pub completed_focus: u32,    // 永久累积
    pub completed_breaks: u32,   // 永久累积
    // ...
}
```

**完成时递增**:
```rust
match self.mode {
    PomodoroMode::Work => self.completed_focus += 1,
    PomodoroMode::Break => self.completed_breaks += 1,
}
```

**问题**: 
- 计数器从不重置
- 跨天后仍然显示历史累积数
- 没有"今日完成数"的概念

### 3.2 缺失的功能

1. **每日重置逻辑**:
   - 需要存储"最后重置日期"
   - 启动时检查日期，如果跨天则重置计数器

2. **数据库字段**:
   ```sql
   ALTER TABLE pomodoro_state 
   ADD COLUMN last_reset_date TEXT;  -- 存储最后重置的日期 (YYYY-MM-DD)
   ```

3. **重置逻辑**:
   ```rust
   fn check_and_reset_daily_counters(&mut self, now: DateTime<Utc>) {
       let today = now.date_naive().to_string();
       if self.last_reset_date != Some(today) {
           self.completed_focus = 0;
           self.completed_breaks = 0;
           self.last_reset_date = Some(today);
       }
   }
   ```

---

## 修复优先级

### 🔴 P0 - 立即修复
1. **编译错误**: 阻止测试运行
2. **完成状态处理**: 核心用户体验问题

### 🟡 P1 - 重要但不紧急
3. **每日重置**: 功能增强，需要数据库迁移

---

## 建议的修复方案

### 方案 1: 最小修复（快速）

**只修复编译错误和完成状态**:
1. 修复测试代码中的 `concat!()` 使用
2. 在 `PomodoroPanel` 中添加状态变化检测
3. 检测到 `Completed` 时触发庆祝动画
4. 如果 `enable_memo` 且是工作完成，显示 memo 输入弹窗

**优点**: 快速修复核心问题
**缺点**: 徽章仍然不会每日重置

### 方案 2: 完整修复（推荐）

**修复所有问题**:
1. 修复编译错误
2. 添加完成状态处理和 memo 弹窗
3. 添加数据库迁移（`last_reset_date` 字段）
4. 实现每日重置逻辑
5. 添加"今日统计"显示

**优点**: 完整解决所有问题
**缺点**: 需要更多时间和测试

---

## 技术细节

### Completed 状态检测

**方法 1: 使用 useEffect 监听状态变化**
```typescript
const [prevState, setPrevState] = useState<PomodoroState | null>(null);

useEffect(() => {
  if (status && prevState && prevState !== "Completed" && status.state === "Completed") {
    // 刚刚完成
    void emitPetReaction("pomodoro-completed");
    
    if (status.mode === "work" && enableMemo) {
      // 显示 memo 弹窗
      setShowMemoDialog(true);
    }
  }
  setPrevState(status?.state || null);
}, [status?.state]);
```

**方法 2: 在 fetchStatus 中检测**
```typescript
const fetchStatus = useCallback(async (forceSync = false) => {
  const prevStatus = status;
  const nextStatus = await getPomodoroStatus();
  
  // 检测状态变化
  if (prevStatus && nextStatus) {
    if (prevStatus.state !== "Completed" && nextStatus.state === "Completed") {
      void emitPetReaction("pomodoro-completed");
      // ...
    }
  }
  
  setStatus(nextStatus);
}, [status]);
```

### Memo 弹窗实现

**使用 Dialog 组件**:
```typescript
const [showMemoDialog, setShowMemoDialog] = useState(false);
const [memoText, setMemoText] = useState("");

// 在完成时显示
<Dialog open={showMemoDialog} onOpenChange={setShowMemoDialog}>
  <DialogContent>
    <DialogHeader>
      <DialogTitle>Focus Session Complete! 🎉</DialogTitle>
      <DialogDescription>
        What did you accomplish during this session?
      </DialogDescription>
    </DialogHeader>
    <Textarea
      value={memoText}
      onChange={(e) => setMemoText(e.target.value)}
      placeholder="Describe what you worked on..."
    />
    <DialogFooter>
      <Button variant="outline" onClick={() => setShowMemoDialog(false)}>
        Skip
      </Button>
      <Button onClick={handleSaveMemo}>
        Save
      </Button>
    </DialogFooter>
  </DialogContent>
</Dialog>
```

### 每日重置实现

**数据库迁移** (`0013_pomodoro_daily_reset.sql`):
```sql
ALTER TABLE pomodoro_state 
ADD COLUMN last_reset_date TEXT;

UPDATE pomodoro_state 
SET last_reset_date = date('now') 
WHERE id = 1;
```

**启动时检查**:
```rust
fn reconcile_runtime_state(&self) -> Result<(), String> {
    let mut status = self.refresh_runtime_if_due()?;
    
    // 检查是否需要每日重置
    let today = Utc::now().date_naive().to_string();
    if self.should_reset_daily_counters(&today)? {
        status.completed_focus = 0;
        status.completed_breaks = 0;
        self.update_last_reset_date(&today)?;
        save_status(&self.conn()?, &status)?;
    }
    
    self.sync_scheduler(&status)?;
    self.publish_badges(&status);
    Ok(())
}
```

---

## 总结

三个问题的根本原因:
1. **编译错误**: 测试代码使用了错误的宏语法
2. **完成状态**: 前端缺少状态变化检测和 UI 反馈
3. **徽章刷新**: 缺少每日重置机制和日期追踪

建议先修复问题 1 和 2（核心功能），然后再考虑问题 3（功能增强）。
