# 202603142230 - feat: Font System Optimization

## 概述
优化 Peekoo 的字体加载系统，清理未使用的字体定义，添加字体显示优化，并建立完整的字体大小和行高系统，确保字体与组件完美搭配。

## 变更内容

### 字体加载优化

#### 1. 清理未使用的字体
```html
<!-- 之前 -->
<link href="https://fonts.bunny.net/css?family=inter:400,500,600,700|jetbrains-mono:400,500" rel="stylesheet" />

<!-- 之后 -->
<link href="https://fonts.bunny.net/css?family=inter:400,500,600,700&display=swap" rel="stylesheet" />
<link href="https://fonts.bunny.net/css?family=jetbrains-mono:400,500,600&display=swap" rel="stylesheet" />
```

**改进点**：
- ✅ 分离字体加载链接，提升可维护性
- ✅ 添加 `&display=swap` 优化首屏渲染
- ✅ 为 JetBrains Mono 添加 600 字重（用于番茄钟）

#### 2. 优化字体定义
```css
/* 之前 */
--font-sans: "Inter", "Nunito", system-ui, -apple-system, sans-serif;
--font-mono: "JetBrains Mono", "Fira Code", monospace;

/* 之后 */
--font-sans: "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
--font-mono: "JetBrains Mono", "SF Mono", "Consolas", "Liberation Mono", monospace;
```

**改进点**：
- ✅ 移除未加载的 Nunito 和 Fira Code
- ✅ 添加更完整的系统字体回退链
- ✅ Windows: Segoe UI
- ✅ macOS: SF Mono
- ✅ Windows: Consolas
- ✅ Linux: Liberation Mono

### 字体大小系统

#### 建立完整的字体大小变量
```css
/* Font sizes */
--text-xs: 0.75rem;      /* 12px - 小标签、徽章 */
--text-sm: 0.875rem;     /* 14px - 正文、按钮 */
--text-base: 1rem;       /* 16px - 标题、强调 */
--text-lg: 1.125rem;     /* 18px - 大标题 */
--text-xl: 1.25rem;      /* 20px - 主标题 */
--text-2xl: 1.5rem;      /* 24px - 特大标题 */
--text-4xl: 2.25rem;     /* 36px - 番茄钟时间 */

/* Line heights */
--leading-tight: 1.25;   /* 紧凑行高 - 标题 */
--leading-normal: 1.5;   /* 正常行高 - 正文 */
--leading-relaxed: 1.75; /* 宽松行高 - 长文本 */
```

### 字体与组件搭配

#### 1. 全局默认
```css
body {
  font-family: var(--font-sans);
  font-size: var(--text-sm);        /* 14px - 舒适的阅读大小 */
  line-height: var(--leading-normal); /* 1.5 - 正常行高 */
}
```

#### 2. 组件字体搭配指南

| 组件 | 字体 | 大小 | 字重 | 行高 | 说明 |
|------|------|------|------|------|------|
| **聊天消息** | Inter | 14px (sm) | 400 | 1.5 | 正文阅读 |
| **任务标题** | Inter | 14px (sm) | 500 | 1.5 | 中等强调 |
| **任务优先级** | Inter | 10px (xs) | 700 | 1.25 | 紧凑徽章 |
| **番茄钟时间** | JetBrains Mono | 36px (4xl) | 600 | 1.25 | 大号数字 |
| **番茄钟状态** | Inter | 14px (sm) | 400 | 1.5 | 状态文字 |
| **按钮文字** | Inter | 14px (sm) | 500 | 1.5 | 中等强调 |
| **面板标题** | Inter | 14px (sm) | 600 | 1.25 | 强调标题 |
| **插件名称** | Inter | 14px (sm) | 600 | 1.25 | 强调标题 |
| **配置标签** | Inter | 14px (sm) | 500 | 1.5 | 中等强调 |
| **徽章文字** | Inter | 12px (xs) | 600 | 1.25 | 紧凑徽章 |
| **小标签** | Inter | 11px | 500 | 1.25 | 极小文字 |
| **气泡标题** | Inter | 9px | 600 | 1.25 | 超小标题 |

#### 3. 字体大小使用原则

**超小文字（9-11px）**：
- 气泡标题、Peek 徽章
- 使用场景：空间极度受限
- 字重：600（semibold）增强可读性

**小文字（12px - xs）**：
- 徽章、标签、辅助信息
- 使用场景：次要信息
- 字重：600-700（semibold-bold）

**正文（14px - sm）**：
- 聊天消息、任务标题、按钮
- 使用场景：主要内容
- 字重：400-500（regular-medium）

**标题（16-20px - base-xl）**：
- 面板标题、插件标题
- 使用场景：区域标题
- 字重：600（semibold）

**大号数字（36px - 4xl）**：
- 番茄钟时间
- 使用场景：重要数据展示
- 字重：600（semibold）
- 字体：JetBrains Mono（等宽）

### 字体渲染优化

#### 1. 抗锯齿优化
```css
body {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}
```

**效果**：
- ✅ macOS/iOS: 更清晰的字体渲染
- ✅ 减少字体边缘锯齿
- ✅ 提升小字号可读性

#### 2. 字体显示优化
```html
<link href="...&display=swap" rel="stylesheet" />
```

**效果**：
- ✅ `font-display: swap` - 立即显示回退字体
- ✅ 字体加载完成后平滑切换
- ✅ 避免 FOIT（Flash of Invisible Text）
- ✅ 提升首屏渲染速度

### 字体加载性能

#### 加载的字体和字重
```
Inter:
- 400 (Regular)   - 正文、描述
- 500 (Medium)    - 任务标题、按钮、配置标签
- 600 (Semibold)  - 面板标题、插件标题、徽章
- 700 (Bold)      - 优先级徽章

JetBrains Mono:
- 400 (Regular)   - 备用
- 500 (Medium)    - 备用
- 600 (Semibold)  - 番茄钟时间（新增）✨
```

#### 性能指标
- **字体数量**：2 个字体家族
- **字重数量**：7 个字重（Inter 4 + JetBrains Mono 3）
- **加载方式**：异步加载 + swap 显示
- **回退链**：完整的系统字体回退

### 字体回退链优化

#### Sans-serif 回退链
```
"Inter" → system-ui → -apple-system → "Segoe UI" → sans-serif
```

**覆盖平台**：
1. **Inter**：主字体（所有平台）
2. **system-ui**：现代浏览器系统字体
3. **-apple-system**：macOS/iOS 系统字体
4. **Segoe UI**：Windows 系统字体
5. **sans-serif**：通用无衬线字体

#### Monospace 回退链
```
"JetBrains Mono" → "SF Mono" → "Consolas" → "Liberation Mono" → monospace
```

**覆盖平台**：
1. **JetBrains Mono**：主字体（所有平台）
2. **SF Mono**：macOS 系统等宽字体
3. **Consolas**：Windows 系统等宽字体
4. **Liberation Mono**：Linux 系统等宽字体
5. **monospace**：通用等宽字体

### 字体特性

#### Inter 特性
- **设计**：现代几何无衬线字体
- **优势**：
  - ✅ 数字清晰，易于区分（0 vs O）
  - ✅ 字母间距优秀，阅读舒适
  - ✅ 多语言支持（包括中文回退）
  - ✅ 小字号下依然清晰
- **适用场景**：
  - UI 界面文字
  - 正文内容
  - 标题和标签
  - 按钮和徽章

#### JetBrains Mono 特性
- **设计**：专业编程等宽字体
- **优势**：
  - ✅ 数字对齐，适合时间显示
  - ✅ 字符区分度高（1 vs l vs I）
  - ✅ 连字支持（可选）
  - ✅ 清晰的数字和符号
- **适用场景**：
  - 番茄钟时间显示
  - 代码片段
  - 数据展示
  - 等宽布局

### 字体大小响应式

#### 基础字号
```css
body {
  font-size: var(--text-sm); /* 14px */
}
```

#### 组件字号
- **极小**：9-11px - 气泡、Peek 徽章
- **小**：12px - 徽章、标签
- **正常**：14px - 正文、按钮
- **中**：16px - 标题
- **大**：18-20px - 主标题
- **特大**：36px - 番茄钟

### 可访问性

#### 最小字号
- ✅ 正文最小 14px（符合 WCAG AA）
- ✅ 小文字最小 12px（徽章、标签）
- ✅ 极小文字 9-11px（仅用于空间受限场景）

#### 行高
- ✅ 正文行高 1.5（符合 WCAG AA）
- ✅ 标题行高 1.25（紧凑但清晰）
- ✅ 长文本行高 1.75（宽松舒适）

#### 字重对比
- ✅ Regular (400) vs Medium (500) - 清晰区分
- ✅ Medium (500) vs Semibold (600) - 层次分明
- ✅ Semibold (600) vs Bold (700) - 强调明显

### 修改的文件

1. **`apps/desktop-ui/index.html`**
   - 分离字体加载链接
   - 添加 `&display=swap` 优化
   - 为 JetBrains Mono 添加 600 字重

2. **`apps/desktop-ui/src/index.css`**
   - 清理未使用的字体定义
   - 优化字体回退链
   - 添加字体大小变量系统
   - 添加行高变量系统
   - 优化全局字体样式

### 使用示例

#### 在组件中使用字体大小
```typescript
// 使用 Tailwind 类名
<p className="text-sm">正文内容</p>
<h2 className="text-base font-semibold">标题</h2>
<span className="text-xs font-bold">徽章</span>

// 使用 CSS 变量
<div style={{ fontSize: 'var(--text-sm)' }}>正文</div>
```

#### 在组件中使用字体家族
```typescript
// Sans-serif（默认）
<p className="font-sans">正文内容</p>

// Monospace
<span className="font-mono">12:34</span>
```

#### 在组件中使用行高
```typescript
// 紧凑行高（标题）
<h2 className="leading-tight">标题</h2>

// 正常行高（正文）
<p className="leading-normal">正文</p>

// 宽松行高（长文本）
<article className="leading-relaxed">长文本</article>
```

### 设计优势

#### 1. 性能优化
- ✅ 字体加载优化（display: swap）
- ✅ 完整的回退链（快速渲染）
- ✅ 只加载需要的字重
- ✅ 分离加载链接（并行加载）

#### 2. 可维护性
- ✅ 统一的字体大小系统
- ✅ 统一的行高系统
- ✅ 清晰的字体定义
- ✅ 完整的使用文档

#### 3. 可读性
- ✅ 合适的字号（14px 正文）
- ✅ 合适的行高（1.5 正文）
- ✅ 清晰的字重对比
- ✅ 优秀的字体渲染

#### 4. 一致性
- ✅ 统一的字体家族
- ✅ 统一的字号系统
- ✅ 统一的字重使用
- ✅ 统一的行高规范

### 后续优化建议

1. **字体子集化**（可选）
   - 只加载需要的字符集
   - 减少字体文件大小
   - 提升加载速度

2. **本地字体缓存**（可选）
   - 使用 Service Worker 缓存字体
   - 离线可用
   - 提升二次加载速度

3. **可变字体**（未来）
   - 使用 Variable Fonts
   - 一个文件包含所有字重
   - 更小的文件大小

4. **字体预加载**（可选）
   - 使用 `<link rel="preload">` 预加载关键字体
   - 提升首屏渲染速度
   - 减少字体闪烁

## 测试验证

### 本地测试
```bash
# 启动开发服务器
just dev

# 测试清单
1. ✅ 字体加载正常
2. ✅ 字体回退正常
3. ✅ 字体大小合适
4. ✅ 字体渲染清晰
5. ✅ 番茄钟数字清晰
6. ✅ 小字号可读
7. ✅ 行高舒适
8. ✅ 字重对比明显
```

### 性能测试
- [ ] 字体加载时间 < 500ms
- [ ] 首屏渲染无字体闪烁
- [ ] 字体回退平滑
- [ ] 字体文件大小合理

## 相关文件

### 修改的文件
- `apps/desktop-ui/index.html` - 字体加载优化
- `apps/desktop-ui/src/index.css` - 字体系统定义

### 相关文档
- `ai/memories/changelogs/202603142200-feat-warm-forest-color-system.md` - 配色系统

## 技术债务

无

## 备注

这次字体系统优化建立了完整的字体大小和行高系统，清理了未使用的字体定义，优化了字体加载性能，并确保字体与组件完美搭配。所有字体大小和字重都经过精心设计，确保在不同场景下都有良好的可读性和视觉效果。

**关键改进**：
- 添加 `font-display: swap` 优化首屏渲染 ✨
- 建立完整的字体大小变量系统 ✨
- 建立完整的行高变量系统 ✨
- 优化字体回退链，覆盖所有主流平台 ✨
- 为 JetBrains Mono 添加 600 字重，优化番茄钟显示 ✨

---

**变更时间**: 2026-03-14 22:30
**变更类型**: feat (功能)
**影响范围**: 字体系统
**向后兼容**: ✅ 是
