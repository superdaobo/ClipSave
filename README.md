# ClipSave

> 保存你有权保存的公开媒体内容 | Save public media you have rights to

ClipSave 是一款基于 Tauri v2 的跨平台媒体保存工具，帮助用户保存其有权保存的公开内容链接中的媒体资源。

## 截图

<!-- TODO: Add screenshots -->
![Home Page](docs/screenshots/home.png)
![History](docs/screenshots/history.png)

## 功能特性

- 🔗 **链接解析** - 支持抖音、小红书分享文本自动提取 URL
- 📥 **下载管理** - 任务队列、暂停/继续/取消/重试、进度显示
- 📋 **剪贴板** - 桌面端手动读取，不做后台轮询
- 🌙 **深色模式** - 跟随系统或手动切换
- 🌐 **国际化** - 中文（默认）和英文
- 📱 **跨平台** - Windows、Android、iOS
- 🔒 **安全** - 最小权限、不存储凭证、路径验证

## ⚠️ 合规声明

**ClipSave 不隶属于抖音、小红书或任何第三方平台。**

- 本工具仅支持保存用户有权保存、公开可访问、非付费、非 DRM、非登录专属的内容
- 不提供绕过登录、验证码、付费墙、DRM、风控或反爬限制的功能
- 不提供去水印、批量抓取、账号自动化、模拟登录或 cookie 窃取功能
- 用户需自行确保使用行为符合法律和平台条款
- 对无法公开访问的内容，工具会返回明确错误提示

**Compliance Notice (English):**
ClipSave only processes publicly accessible content that users have rights to save. It does not bypass login, DRM, paywalls, captcha, or anti-crawling mechanisms. Users are responsible for compliance with applicable laws and platform terms of service.

## 本地开发

### 前置要求

- Node.js LTS (≥18)
- Rust stable (≥1.77)
- pnpm 或 npm

### 安装依赖

```bash
npm install
```

### 开发模式

```bash
# Windows 桌面
npm run tauri:dev

# Android
npm run tauri:android:dev

# iOS (需要 macOS + Xcode)
npm run tauri:ios:dev
```

## 构建

```bash
# Windows 桌面 (NSIS + MSI)
npm run tauri:build

# Android (APK/AAB)
npm run tauri:android:build

# iOS (需要 Apple 签名证书)
npm run tauri:ios:build
```

## GitHub Actions

### CI 工作流

每次 push 和 PR 自动运行：
- 前端 lint、typecheck、test
- Rust cargo test

### Release 工作流

触发方式：
- 手动触发 (workflow_dispatch)
- Push 到 main/release 分支
- 创建 `app-v*` 标签

自动执行：
1. 读取当前版本号，patch +1
2. 同步更新 `package.json`、`tauri.conf.json`、`Cargo.toml`
3. 创建 Git tag (如 `app-v1.0.1`)
4. 构建 Windows 安装包 (NSIS + MSI)
5. 构建 Android APK
6. 构建 iOS (需要签名证书，否则优雅跳过)
7. 发布 GitHub Release

### Secrets 配置

在 GitHub 仓库 Settings → Secrets and variables → Actions 中配置：

| Secret | 用途 | 必需 |
|--------|------|------|
| `GITHUB_TOKEN` | GitHub Release 发布 | 自动提供 |
| `TAURI_SIGNING_PRIVATE_KEY` | Windows 安装包签名 | 可选 |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | 签名密钥密码 | 可选 |
| `ANDROID_KEYSTORE` | Android APK 签名 | 可选 |
| `ANDROID_KEYSTORE_PASSWORD` | Keystore 密码 | 可选 |
| `ANDROID_KEY_ALIAS` | Key 别名 | 可选 |
| `ANDROID_KEY_PASSWORD` | Key 密码 | 可选 |
| `APPLE_CERTIFICATE` | iOS 签名证书 | 可选 |
| `APPLE_CERTIFICATE_PASSWORD` | 证书密码 | 可选 |
| `APPLE_PROVISIONING_PROFILE` | iOS 配置文件 | 可选 |
| `APPLE_TEAM_ID` | Apple Team ID | 可选 |

> 所有签名相关 secrets 均为可选。未配置时，构建仍会成功但产物未签名。

## 版本号自动迭代

Release 工作流会自动：
1. 从 `package.json` 读取当前版本
2. Patch 版本号 +1
3. 同步到 `package.json`、`src-tauri/tauri.conf.json`、`src-tauri/Cargo.toml`
4. 创建 commit 和 tag

## iOS 本地构建

1. 安装 Xcode (最新稳定版)
2. 安装 Rust iOS targets: `rustup target add aarch64-apple-ios x86_64-apple-ios`
3. 运行: `npm run tauri:ios:build`
4. 需要 Apple Developer 证书进行签名

> App Store 发布不是必须的，但需要能本地构建安装到开发设备。

## 常见问题

**Q: 为什么某些链接无法解析？**
A: ClipSave 仅支持公开可访问的内容。如果链接需要登录、是私密内容或平台已更改页面结构，会返回相应错误提示。

**Q: 支持哪些平台的链接？**
A: 目前支持抖音 (douyin.com, v.douyin.com, iesdouyin.com) 和小红书 (xiaohongshu.com, xhslink.com)。

**Q: 下载的文件保存在哪里？**
A: 默认保存在用户选择的下载目录中，按 `平台/作者/日期/` 分类。

**Q: 为什么没有去水印功能？**
A: ClipSave 严格遵守合规边界，不提供去水印、绕过版权保护或任何规避访问控制的功能。

## 贡献指南

1. Fork 本仓库
2. 创建功能分支: `git checkout -b feature/my-feature`
3. 提交更改: `git commit -m 'feat: add my feature'`
4. 推送分支: `git push origin feature/my-feature`
5. 创建 Pull Request

请确保：
- 代码通过 lint 和 typecheck
- 新功能有对应测试
- 遵守合规边界（不添加绕过访问控制的功能）

## 许可证

[MIT](LICENSE)
