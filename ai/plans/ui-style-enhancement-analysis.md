# Peekoo UI 设计与结构分析

## 项目概述

Peekoo 是一个基于 Tauri 的 AI 桌面宠物应用，采用 React + TypeScript + Tailwind CSS 构建前端界面。项目具有独特的"深空"主题设计系统，支持明暗双主题。

## 当前设计系统分析

### 1. 色彩系统 (Color Palette)

#### 深空主题 (Dark Mode - 默认)
```css
--space-void: oklch(0.20 0.03 280)      /* 最深背景 */
--space-deep: oklch(0.24 0.04 280)      /* 卡片/面板表面 */
--space-surface: oklch(0.28 0.05 280)   /* 按钮表面 */
--space-overlay: oklch(0.32 0.06 280)   /* 悬停效果 */

/* 发光色 - 主要交互色 */
--glow-blue: oklch(0.72 0.14 260)       /* 主色调 */
--glow-purple: oklch(0.68 0.18 300)     /* 强调色 */
--glow-cyan: oklch(0.75 0.12 200)       /* 信息色 */
--glow-pink: oklch(0.70 0.16 340)       /* 装饰色 */

/* 柔和色 */
--muted-blue: oklch(0.45 0.08 260)
--muted-purple: oklch(0.40 0.10 300)
```

#### 浅色主题 (Light Mode)
```css
--space-void: oklch(0.98 0.01 280)      /* 干净白色背景 */
--space-deep: oklch(0.95 0.02 280)      /* 卡片表面 */
--glow-blue: oklch(0.65 0.18 260)       /* 更深的蓝色 */
--glow-purple: oklch(0.62 0.22 300)     /* 更深的紫色 */
```

**特点**：
- 使用现代 OKLCH 色彩空间，色彩过渡更自然
- 明暗主题色彩一致性好
- 发光色系列提供科技感

### 2. 视觉效果系统

#### 玻璃态效果 (Glassmorphism)
```css
--glass: oklch(0.24 0.04 280 / 0.65)           /* 半透明玻璃 */
--glass-border: oklch(0.85 0.02 280 / 0.15)    /* 玻璃边框 */
backdrop-blur-2xl                               /* 背景模糊 */
```

#### 阴影系统
```css
--shadow-glow-blue: 0 4px 20px var(--glow-blue) / 0.2
--shadow-glow-purple: 0 4px 20px var(--glow-purple) / 0.2
--shadow-panel: 0 12px 40px oklch(0 0 0 / 0.4)
```

#### 圆角系统
```css
--radius-panel: 1.5rem      /* 面板圆角 */
--radius-button: 1rem       /* 按钮圆角 */
```

### 3. 组件架构

#### 核心视图 (Views)
```
src/views/
├── SpriteView.tsx       # 主精灵窗口（可拖拽桌面宠物）
├── ChatView.tsx         # AI 聊天面板
├── PomodoroView.tsx     # 番茄钟面板
├── TasksView.tsx        # 任务管理面板
├── PluginsView.tsx      # 插件管理面板
└── PluginPanelView.tsx  # 插件面板容器
```

#### 功能模块 (Features)
```
src/features/
├── chat/                # 聊天功能
│   ├── ChatPanel.tsx
│   ├── ChatMessage.tsx
│   └── settings/
├── pomodoro/            # 番茄钟功能
│   ├── PomodoroPanel.tsx
│   ├── TimerDisplay.tsx
│   └── TimerControls.tsx
├── tasks/               # 任务管理
│   ├── TasksPanel.tsx
│   ├── TaskItem.tsx
│   └── TaskInput.tsx
└── plugins/             # 插件系统
```

#### UI 组件库 (Shadcn/UI)
```
src/components/ui/
├── button.tsx           # 带 Framer Motion 动画的按钮
├── input.tsx
├── checkbox.tsx
├── badge.tsx
└── scroll-area.tsx
```

#### 精灵组件 (Sprite Components)
```
src/components/sprite/
├── Sprite.tsx                    # 精灵主组件
├── SpriteAnimation.tsx           # 精灵动画引擎
├── SpriteActionMenu.tsx          # 右键菜单（圆形布局）
├── SpriteBubble.tsx              # 提示气泡
├── SpritePeekBadge.tsx           # 状态徽章（健康提醒等）
├── spriteAtlas.ts                # 精灵图集管理
└── chromaKey.ts                  # 色度键透明处理
```

### 4. 动画系统

#### Framer Motion 集成
- 所有按钮都有 `whileHover` 和 `whileTap` 动画
- 面板使用 `AnimatePresence` 实现进出场动画
- 精灵菜单使用弹簧动画 (spring physics)

```typescript
// 按钮动画示例
<motion.button
  whileHover={{ scale: 1.02 }}
  whileTap={{ scale: 0.96 }}
  transition={{ type: "spring", stiffness: 400, damping: 25 }}
/>
```

#### 精灵动画系统
- 基于 Canvas 的精灵图动画
- 支持多种状态：idle, happy, working, thinking, sleepy, reminder, dragging
- 帧率可配置（默认 8fps）
- 支持色度键透明和像素艺术模式

### 5. 交互模式

#### 精灵窗口 (SpriteView)
- **左键拖拽**：移动窗口位置
- **右键点击**：打开圆形动作菜单
- **菜单选项**：
  - Chat (聊天)
  - Tasks (任务)
  - Pomodoro (番茄钟)
  - Plugins (插件)

#### 面板窗口 (Panel Windows)
- 统一的 `PanelShell` 容器
- 玻璃态背景 + 模糊效果
- 可拖拽标题栏
- 右上角关闭按钮

#### 状态反馈
- **Peek Badge**：显示健康提醒等状态信息
  - 折叠模式：单行显示当前项
  - 展开模式：显示所有项目
- **Sprite Bubble**：临时提示气泡
  - 自动显示 3 秒后消失
  - 带有指向精灵的尾巴

### 6. 响应式布局

#### 窗口尺寸管理
```typescript
// 精灵窗口动态调整大小
getSpriteWindowSize({
  menuOpen,                    // 菜单是否打开
  bubbleOpen,                  // 气泡是否显示
  peekBadgeItemCount,          // 徽章项目数量
  peekBadgeExpanded,           // 徽章是否展开
})
```

#### 面板尺寸
- 固定宽度设计（约 400-500px）
- 高度自适应内容
- 使用 ScrollArea 处理溢出

## 设计优势

### ✅ 已实现的优秀设计

1. **统一的设计语言**
   - 一致的色彩系统
   - 统一的圆角和间距
   - 标准化的动画效果

2. **现代视觉效果**
   - 玻璃态设计
   - 发光效果
   - 流畅的动画过渡

3. **良好的组件复用**
   - PanelShell 统一面板容器
   - 基于 Shadcn/UI 的组件库
   - 可扩展的插件系统

4. **精致的交互细节**
   - 精灵拖拽动画
   - 圆形菜单布局
   - 状态徽章系统

5. **主题支持**
   - 完整的明暗主题
   - CSS 变量驱动
   - 平滑的主题切换

## 改进空间

### 🎨 视觉增强建议

#### 1. 色彩系统扩展
```css
/* 建议添加更多语义化颜色 */
--color-success-glow: oklch(0.72 0.14 150)
--color-warning-glow: oklch(0.75 0.15 80)
--color-danger-glow: oklch(0.68 0.18 20)

/* 渐变色系统 */
--gradient-primary: linear-gradient(135deg, var(--glow-blue), var(--glow-purple))
--gradient-success: linear-gradient(135deg, var(--glow-cyan), var(--color-success))
```

#### 2. 微交互增强
- 添加更多悬停状态反馈
- 按钮点击涟漪效果
- 加载状态动画
- 进度条动画

#### 3. 视觉层次优化
- 增强卡片阴影层次
- 优化文字对比度
- 添加更多视觉分隔

#### 4. 动画细节
- 页面切换过渡动画
- 列表项交错动画
- 数字滚动动画
- 粒子效果（可选）

### 🏗️ 结构优化建议

#### 1. 设计令牌系统
```typescript
// design-tokens.ts
export const spacing = {
  xs: '0.25rem',
  sm: '0.5rem',
  md: '1rem',
  lg: '1.5rem',
  xl: '2rem',
}

export const typography = {
  sizes: {
    xs: '0.75rem',
    sm: '0.875rem',
    base: '1rem',
    lg: '1.125rem',
    xl: '1.25rem',
  },
  weights: {
    normal: 400,
    medium: 500,
    semibold: 600,
    bold: 700,
  },
}
```

#### 2. 组件变体系统
```typescript
// 扩展按钮变体
const buttonVariants = cva({
  variants: {
    variant: {
      glow: "bg-glow-blue shadow-glow-blue",
      glass: "bg-glass border-glass-border backdrop-blur",
      gradient: "bg-gradient-primary",
    },
    animation: {
      bounce: "animate-bounce",
      pulse: "animate-pulse",
      glow: "animate-glow",
    }
  }
})
```

#### 3. 布局组件
```typescript
// 添加更多布局组件
<Stack spacing="md" direction="vertical">
<Grid columns={2} gap="lg">
<Flex justify="between" align="center">
```

### 📱 响应式增强

虽然是桌面应用，但可以优化不同窗口尺寸：
- 小窗口模式（紧凑布局）
- 标准窗口模式
- 大窗口模式（展示更多信息）

### ♿ 可访问性

- 添加键盘导航支持
- ARIA 标签完善
- 焦点管理优化
- 高对比度模式

## 技术栈总结

### 前端框架
- **React 18** - UI 框架
- **TypeScript** - 类型安全
- **Vite** - 构建工具
- **Bun** - 包管理器

### 样式方案
- **Tailwind CSS 4** - 原子化 CSS
- **CSS Variables** - 主题系统
- **OKLCH** - 现代色彩空间

### 动画库
- **Framer Motion** - 声明式动画
- **Canvas API** - 精灵动画

### UI 组件
- **Shadcn/UI** - 组件库基础
- **Radix UI** - 无障碍组件原语
- **Lucide React** - 图标库

### 状态管理
- **React Hooks** - 本地状态
- **Tauri Events** - 跨窗口通信

## 下一步行动建议

### 短期优化（1-2 周）
1. 完善设计令牌系统
2. 添加更多按钮和卡片变体
3. 优化动画性能
4. 增强色彩对比度

### 中期增强（1 个月）
1. 实现完整的组件库文档
2. 添加主题编辑器
3. 优化响应式布局
4. 增加更多微交互

### 长期规划（2-3 个月）
1. 自定义精灵皮肤系统
2. 高级动画效果
3. 插件 UI 模板
4. 性能监控和优化

## 参考资源

- [Tailwind CSS 文档](https://tailwindcss.com)
- [Framer Motion 文档](https://www.framer.com/motion/)
- [Shadcn/UI 组件](https://ui.shadcn.com)
- [OKLCH 色彩空间](https://oklch.com)
- [Tauri 文档](https://tauri.app)

---

**文档创建时间**: 2026-03-14
**分析范围**: apps/desktop-ui 完整前端代码库
**目的**: 为 UI 风格化改进提供全面的技术和设计分析
