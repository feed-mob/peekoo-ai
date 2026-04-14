# 在 macOS 上安装

macOS 版本可以让你最快把 Peekoo 作为一个轻量、随时可用的桌面伙伴跑起来。

## 下载

1. 打开 [最新 GitHub Release](https://github.com/feed-mob/peekoo-ai/releases/latest)。
2. 下载 macOS 的 `.dmg` 文件。
3. 打开 DMG，把 `Peekoo.app` 拖到 `/Applications`。

## Gatekeeper 警告

Peekoo 目前还没有完成 notarization。macOS 可能会提示：

> “Peekoo” 已损坏，无法打开。你应该将它移到废纸篓。

这通常是未 notarize 的应用被 macOS 隔离检查拦截导致的。

这并不表示 Peekoo 本身有问题，更准确地说，是 macOS 把它当作一个尚未 notarize 的下载应用来处理。

## 解决方法

运行：

```bash
xattr -cr /Applications/Peekoo.app
```

执行后再次启动应用即可。升级后可能需要重新执行一次。

## 备选方法

在部分 macOS 版本中，也可以通过 `系统设置 -> 隐私与安全性 -> 仍要打开` 放行应用。
