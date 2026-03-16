# Fix: Plugin Panel UI Consistency

**Date:** 2026-03-13 18:00  
**Type:** fix  
**Scope:** ui, plugins

## Problem

Health Reminder 插件面板存在三个问题：
1. 窗口不可拖动 - 缺少可拖拽的标题栏
2. 没有关闭按钮 - 用户无法关闭窗口
3. UI 风格不统一 - 插件使用自定义深色背景、大标题和不同的字体/配色，与主应用的设计系统完全不一致

## Solution

### 1. 统一面板包装器

修改 `PluginPanelView.tsx` 使用 `PanelShell` 组件包装插件内容：

```typescript
// Before: 直接渲染 iframe
return <iframe srcDoc={html} className="h-screen w-full" />

// After: 使用 PanelShell 包装
return (
  <PanelShell title={title}>
    <iframe srcDoc={html} className="h-full w-full" />
  </PanelShell>
);
```

**效果：**
- ✅ 添加可拖拽的标题栏（带 `data-tauri-drag-region`）
- ✅ 添加关闭按钮（右上角 X 按钮）
- ✅ 统一的毛玻璃背景和边框样式

### 2. 动态获取面板标题

从 `plugin_panels_list` API 获取插件面板的标题：

```typescript
useEffect(() => {
  invoke<PluginPanelDto[]>("plugin_panels_list")
    .then((panels) => {
      const panel = panels.find((p) => p.label === label);
      if (panel) setTitle(panel.title);
    });
}, [label]);
```

### 3. 完全重写插件 UI

移除所有自定义样式，采用主应用的设计语言：

**HTML 结构简化：**
- 移除 `PLUGIN` 标签和大标题
- 移除 `REMINDER` / `STATUS` 列表头
- 使用语义化的 HTML 结构

**CSS 完全重写：**

```css
/* Before: 硬编码深色背景 */
body {
  background: #262136;
  color: #f2f0f9;
}

/* After: 透明背景，使用主题变量 */
body {
  background: transparent;
  color: var(--text-primary, oklch(0.95 0.01 280));
}
```

**设计系统对齐：**
- 字体：Inter, "Nunito", system-ui（与主应用一致）
- 圆角：1rem（与主应用按钮和卡片一致）
- 间距：使用 12px/16px 的统一间距系统
- 按钮：采用主应用的按钮样式（高度 32px，圆角 1rem）
- 卡片：使用 `space-surface` 背景和 `glass-border` 边框
- 颜色：完全使用 OKLCH 色彩空间的主题变量

**主题变量使用：**
```css
/* 文本颜色 */
color: var(--text-primary, oklch(0.95 0.01 280));
color: var(--text-secondary, oklch(0.75 0.03 280));

/* 背景和边框 */
background: var(--space-surface, oklch(0.28 0.05 280));
border: 1px solid var(--glass-border, rgba(255, 255, 255, 0.1));

/* 强调色 */
background: var(--glow-blue, oklch(0.72 0.14 260));
background: var(--glow-purple, oklch(0.68 0.18 300));
```

### 4. 更新 JavaScript 渲染逻辑

更新类名以匹配新的 CSS：

```javascript
// Before
pill.className = "pill";
card.className = "card";

// After
badge.className = "status-badge";
card.className = "reminder-card";
```

## Files Changed

- `apps/desktop-ui/src/views/PluginPanelView.tsx` - 添加 PanelShell 包装器
- `plugins/health-reminders/ui/panel.html` - 简化 HTML 结构
- `plugins/health-reminders/ui/panel.css` - 完全重写，对齐设计系统
- `plugins/health-reminders/ui/panel.js` - 更新类名和渲染逻辑

## Visual Changes

**Before:**
- 深色背景 (#262136)
- 大标题 "Health Reminders" (28px)
- "PLUGIN" 标签
- 列表头 "REMINDER" / "STATUS"
- 自定义紫色按钮
- 不同的字体和间距

**After:**
- 透明背景，继承主题
- 小标题在 PanelShell 中 (14px)
- 简洁的副标题
- 无列表头，直接显示内容
- 统一的蓝色/紫色按钮
- 与主应用完全一致的字体和间距

## Testing

手动测试：
1. 启动应用 `just dev`
2. 打开 Health Reminder 插件面板
3. 验证：
   - ✅ 可以通过标题栏拖动窗口
   - ✅ 右上角有关闭按钮且可用
   - ✅ UI 风格与其他面板（Chat、Tasks、Pomodoro）完全一致
   - ✅ 字体、配色、间距、圆角都统一
   - ✅ 支持亮色/暗色主题切换
   - ✅ 按钮样式与主应用一致
   - ✅ 卡片样式与主应用一致

## Impact

- 所有插件面板现在都有统一的 UI 外观
- 插件开发者可以专注于内容，无需实现标题栏和关闭按钮
- 更好的用户体验和一致性
- 插件自动继承主题变化（亮色/暗色模式）

## Design System Benefits

通过使用主题 CSS 变量，插件现在可以：
- 自动适配亮色/暗色主题
- 保持与主应用的视觉一致性
- 减少维护成本（主题更新自动应用）
- 提供更专业的用户体验

## Notes

- 插件 HTML 仍然在 iframe 中渲染（沙箱隔离）
- PanelShell 提供的标题栏在 iframe 外部，不受插件控制
- 插件通过 CSS 变量访问主题颜色，无需硬编码
- 所有 OKLCH 颜色值都有回退值，确保兼容性
