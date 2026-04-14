# Google Calendar 插件

## 概览

Peekoo 的 Google Calendar 集成通过插件提供。它不会自带 Google OAuth 凭据。

如果你希望 Peekoo 使用你的日历数据，同时又不把个人凭据预置进应用发行版里，这个插件就是合适的方式。

## 你需要准备

- 一个 Google 账号
- Google Cloud Console 访问权限
- Peekoo 桌面应用

## 配置流程

1. 创建或选择一个 Google Cloud 项目。
2. 启用 Google Calendar API。
3. 配置 OAuth consent screen。
4. 如果应用仍处于测试模式，把自己加入 test users。
5. 创建一个 `Desktop app` 类型的 OAuth client。
6. 下载得到的 `client.json`。
7. 在 Peekoo 中打开 Google Calendar 面板。
8. 上传 `client.json`。
9. 点击 `Connect` 并完成 Google 登录授权。

## 配置完成后

连接成功后，Peekoo 就可以通过这个插件刷新并读取日历事件。具体 UI 形态以后可能继续演进，但凭据路径是稳定的：你的应用实例使用的是你自己创建的 OAuth client。

## 重要提示

请妥善保管 `client.json`。不要把它提交到 git，也不要公开分享。

## 常见错误

- 没有启用 Calendar API
- 当前账号没有加入 test users
- 选择了错误的凭据类型

请使用 `Desktop app`，不要使用 `Web application`。

## 隐私说明

由于这个 OAuth client 属于你自己，你可以完全控制对应的 Google Cloud 项目、test users 和使用方式。虽然配置步骤会更手动一些，但账号归属也更清晰。
