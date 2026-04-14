# Sprite 配置

## 什么是 Sprite

Sprite 是 Peekoo 在透明主窗口中渲染的桌面角色。每个 Sprite 都是自包含的，位于 `apps/desktop-ui/public/sprites/` 下的独立目录中。

## 目录结构

```text
apps/desktop-ui/public/sprites/
└── [sprite-id]/
    ├── manifest.json
    └── sprite.png
```

## Manifest 字段

每个 Sprite 都通过 `manifest.json` 描述，常见字段包括：

- `id`
- `name`
- `description`
- `image`
- `layout.columns`
- `layout.rows`
- `scale`
- `frameRate`
- `chromaKey`

这些字段共同决定 Peekoo 如何在桌面上加载、摆放、播放动画，以及如何把背景抠成透明。

## 如何选择或准备 Sprite

准备 Sprite 资源时，最重要的是三点：

- 清晰规范的精灵图布局
- 动画帧之间稳定一致的位置
- 能够干净去除背景的抠图配置

Peekoo 目前使用的是精灵图，而不是把每一帧拆成独立图片。这能让动画加载和渲染保持简单一致。

## 窗口行为

当 UI 展开时，例如打开迷你聊天窗口，Peekoo 会自动调整主 Sprite 窗口大小。窗口默认保持不可调整大小，只在程序触发 resize 时暂时切换，这样可以避免 Linux 和 Wayland 下的点击行为异常。

## 添加新 Sprite

1. 在 `apps/desktop-ui/public/sprites/[new-id]` 下创建目录。
2. 添加精灵图图片。
3. 添加 `manifest.json`。
4. 校验缩放、抠图参数和帧布局。

## 实用建议

- 尽量让角色在相邻帧之间保持居中，避免动画时明显跳动。
- 不要只看原图尺寸，也要按 Peekoo 实际桌面显示尺寸检查效果。
- `frameRate` 要足够有生命感，但不要过快以免分散注意力。
- 在你常用的桌面背景颜色上实际测试一次抠图效果。

## 当前限制

Sprite 切换能力仍在演进中。仓库已经支持 Sprite 元数据和当前激活角色设置，但自定义 Sprite 工作流还在持续开发。
