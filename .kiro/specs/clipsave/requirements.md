# Requirements Document

## Introduction

ClipSave 是一款基于 Tauri v2 的跨平台媒体保存工具，前端使用 React + TypeScript + Vite + Tailwind CSS + shadcn/ui，后端使用 Rust，数据存储使用 SQLite。目标平台包括 Windows、Android 和 iOS。

ClipSave 仅面向用户有权保存、公开可访问、非付费、非 DRM、非登录专属的媒体内容（如抖音、小红书的公开分享链接），帮助用户将其合法持有或已获授权的媒体资源进行本地个人备份。工具严格遵守平台条款、版权和当地法律，**不提供任何绕过访问控制、DRM、登录、付费墙、验证码、风控或水印的能力**，也不进行账号自动化、批量抓取、模拟登录或 cookie 窃取。

本文档使用 EARS（Easy Approach to Requirements Syntax）模式描述所有需求，并遵守 INCOSE 质量规则（可验证、无歧义、正向表述、一条需求一个主张）。合规边界作为一等需求被编码进文档，所有下游设计与任务必须继承这些约束。

## Glossary

- **ClipSave**: 本应用的整体系统名称
- **Frontend**: 基于 React + TypeScript + Vite 的前端界面层
- **Backend**: 基于 Rust 的 Tauri 后端，负责网络、下载、解析与持久化
- **Link_Extractor**: 从分享文本或粘贴内容中提取 URL 的后端模块
- **URL_Normalizer**: 负责 URL 解码、去追踪参数、短链解析、规范化的后端模块
- **Parser**: 实现 `Parser` trait 的平台解析器模块（每个平台一个实例）
- **Douyin_Parser**: 处理 `douyin.com`、`iesdouyin.com`、`v.douyin.com` 的解析器
- **Xiaohongshu_Parser**: 处理 `xiaohongshu.com`、`xhslink.com` 的解析器
- **Resolver**: 解析器返回 `ResolvedMedia` 的阶段
- **ResolvedMedia**: 解析结果结构，包含平台、原始/规范化 URL、标题、作者、媒体项列表
- **MediaItem**: 单个媒体条目，类型为 `video` / `image` / `gif` / `unknown`
- **Downloader**: 负责执行实际下载、断点续传、重试、并发控制的后端模块
- **DownloadTask**: 下载任务实体，具备状态机与持久化
- **Task_Queue**: 下载任务队列，受最大并发限制
- **History_Store**: 基于 SQLite 的历史记录存储
- **Settings_Store**: 用户偏好设置存储
- **Clipboard_Reader**: 读取系统剪贴板的模块（桌面手动触发，移动端由用户粘贴触发）
- **Theme_Controller**: 主题控制器，支持跟随系统 / 手动浅色 / 手动深色
- **I18n_Controller**: 国际化控制器，支持 `zh-CN`（默认）与 `en-US`
- **Logger**: Rust 结构化日志组件
- **Restricted_Content**: 需要登录、付费、DRM、加密签名、反爬限制或私有 API 的内容
- **Public_Content**: 无需登录即可通过公开网页访问的内容
- **Share_Text**: 用户从平台"复制链接"功能粘贴的原始文本，通常包含中文描述与 URL
- **Filename_Sanitizer**: 将标题/作者转换为安全文件名的纯函数模块
- **Tauri_Capability**: Tauri v2 权限声明，遵循最小权限原则


## Requirements

### Requirement 1: Compliance Red Lines (合规红线)

**User Story:** As a user, I want ClipSave to enforce strict compliance boundaries, so that I only save content I have rights to and the tool never bypasses access controls.

#### Acceptance Criteria

1. THE ClipSave SHALL restrict processing to Public_Content that is non-paid, non-DRM, non-login-gated, and publicly accessible without authentication.
2. THE ClipSave SHALL NOT implement logic that bypasses login, captcha, paywall, DRM, encrypted signature, private API, risk-control, or anti-crawling mechanisms.
3. THE ClipSave SHALL NOT provide watermark removal, copyright notice removal, bulk scraping automation, account automation, simulated login, or cookie theft features.
4. WHEN a link is identified as Restricted_Content, THE Parser SHALL return a structured error of type `RestrictedContent` and SHALL NOT attempt further resolution.
5. WHEN a platform changes its public page structure such that parsing fails, THE Parser SHALL fail gracefully by returning a `ParseFailed` error without attempting to circumvent the change.
6. THE ClipSave SHALL display a compliance notice in the main UI stating that users must only save content they own or are authorized to save.
7. THE ClipSave SHALL include in the About page a statement that ClipSave is not affiliated with Douyin or Xiaohongshu and that the tool does not bypass access controls or copyright protection.
8. THE README SHALL include a compliance section describing platform terms, copyright, and local-law obligations in both Chinese and English summaries.
9. THE ClipSave SHALL NOT persist cookies, account credentials, passwords, or authentication tokens in any store.
10. WHERE a URL requires authenticated access to retrieve public metadata, THE Parser SHALL return `RestrictedContent` and SHALL NOT prompt the user for credentials.

### Requirement 2: Link Input and Share-Text Extraction

**User Story:** As a user, I want to paste a raw share text from Douyin or Xiaohongshu and have ClipSave extract the URL, so that I do not need to clean the text manually.

#### Acceptance Criteria

1. WHEN the user submits Share_Text containing zero or more URLs, THE Link_Extractor SHALL extract every HTTP or HTTPS URL present in the text and return them in the order they appear.
2. WHEN the submitted text contains no HTTP or HTTPS URL, THE Link_Extractor SHALL return an empty list and THE Frontend SHALL display a user-friendly "未识别到链接" message.
3. THE Link_Extractor SHALL support extracting both short links (such as `v.douyin.com/xxx`, `xhslink.com/xxx`) and long links (such as `www.douyin.com/video/xxx`, `www.xiaohongshu.com/explore/xxx`).
4. THE URL_Normalizer SHALL perform percent-decoding on extracted URLs before further processing.
5. THE URL_Normalizer SHALL strip known tracking parameters including `utm_source`, `utm_medium`, `utm_campaign`, `utm_term`, `utm_content`, `share_token`, `share_from`, `share_app_id`, `app_platform`, `timestamp`, `xhsshare`, `appuid`, `apptime`, and `share_id`.
6. THE URL_Normalizer SHALL preserve path segments and any query parameters required for content resolution (such as Xiaohongshu `xsec_token` when it is part of the public share link).
7. WHEN the input contains a short link, THE URL_Normalizer SHALL follow HTTP redirects up to a maximum of 5 hops to obtain the canonical URL.
8. IF the redirect chain exceeds 5 hops, THEN THE URL_Normalizer SHALL abort resolution and return a `TooManyRedirects` error.
9. IF a redirect target is to a non-HTTP(S) scheme or to a non-whitelisted host, THEN THE URL_Normalizer SHALL abort resolution and return an `UnsafeRedirect` error.
10. WHEN the user submits multiple links at once, THE Link_Extractor SHALL limit batch parsing to at most 20 URLs per submission.
11. IF a submission exceeds 20 URLs, THEN THE Frontend SHALL display a message indicating the per-batch limit and SHALL process only the first 20.
12. THE URL_Normalizer SHALL be a pure function of its input URL string, producing the same canonical URL for equivalent inputs regardless of invocation order.

### Requirement 3: Douyin Platform Support

**User Story:** As a user, I want ClipSave to resolve public Douyin share links, so that I can save videos, images, or animated images I have rights to.

#### Acceptance Criteria

1. THE Douyin_Parser SHALL handle URLs whose host is `douyin.com`, `www.douyin.com`, `iesdouyin.com`, `www.iesdouyin.com`, or `v.douyin.com`.
2. WHEN a Douyin URL is resolved, THE Douyin_Parser SHALL return a `ResolvedMedia` populated with platform `douyin`, canonical URL, and zero or more MediaItem entries.
3. THE Douyin_Parser SHALL support MediaItem types `video`, `image`, and `gif` as exposed by publicly accessible Douyin share pages.
4. WHEN public metadata contains a title or author, THE Douyin_Parser SHALL populate `ResolvedMedia.title` and `ResolvedMedia.author` from publicly visible fields (such as page meta tags or publicly available JSON-LD).
5. IF a Douyin URL requires login, is marked private, or returns HTTP 401/403/451, THEN THE Douyin_Parser SHALL return `RestrictedContent`.
6. IF a Douyin URL returns HTTP 404 or the resource no longer exists, THEN THE Douyin_Parser SHALL return `ContentNotFound`.
7. THE Douyin_Parser SHALL NOT invoke any private or undocumented Douyin API, SHALL NOT solve captcha, and SHALL NOT forge device signatures.
8. WHEN Douyin modifies its public page structure such that extraction fails, THE Douyin_Parser SHALL return `ParseFailed` with a platform-version hint for debugging.

### Requirement 4: Xiaohongshu Platform Support

**User Story:** As a user, I want ClipSave to resolve public Xiaohongshu share links, so that I can save image-text posts, videos, or animated images I have rights to.

#### Acceptance Criteria

1. THE Xiaohongshu_Parser SHALL handle URLs whose host is `xiaohongshu.com`, `www.xiaohongshu.com`, or `xhslink.com`.
2. THE Xiaohongshu_Parser SHALL handle URL paths matching `/explore/{id}` and `/discovery/item/{id}`.
3. WHEN a Xiaohongshu URL is resolved, THE Xiaohongshu_Parser SHALL return a `ResolvedMedia` populated with platform `xiaohongshu`, canonical URL, and zero or more MediaItem entries.
4. THE Xiaohongshu_Parser SHALL support MediaItem types `image`, `video`, and `gif` as exposed by publicly accessible Xiaohongshu pages.
5. IF a Xiaohongshu URL requires login, is private, or returns HTTP 401/403/451, THEN THE Xiaohongshu_Parser SHALL return `RestrictedContent`.
6. IF a Xiaohongshu URL returns HTTP 404, THEN THE Xiaohongshu_Parser SHALL return `ContentNotFound`.
7. THE Xiaohongshu_Parser SHALL NOT invoke any private or undocumented Xiaohongshu API, SHALL NOT solve captcha, and SHALL NOT bypass `xsec_token` validation.
8. WHEN Xiaohongshu modifies its public page structure such that extraction fails, THE Xiaohongshu_Parser SHALL return `ParseFailed` with a platform-version hint for debugging.

### Requirement 5: Parser Architecture and Graceful Failure

**User Story:** As a maintainer, I want parsers to be modular and to fail gracefully, so that adding or updating a platform does not destabilize the rest of the app.

#### Acceptance Criteria

1. THE Backend SHALL expose a `Parser` trait with methods `can_handle(url) -> bool`, `normalize(url) -> NormalizedUrl`, and `resolve(normalized_url) -> Result<ResolvedMedia, ParserError>`.
2. THE Backend SHALL select a Parser by iterating registered parsers and choosing the first whose `can_handle` returns true.
3. WHEN no registered Parser matches a URL, THE Backend SHALL return an `UnsupportedPlatform` error.
4. THE Parser SHALL prefer public meta tags, Open Graph tags, JSON-LD, and publicly accessible JSON embedded in the HTML over any non-public endpoint.
5. THE Parser SHALL NOT sign requests with platform-specific tokens derived by reverse engineering.
6. WHEN a parser encounters an unexpected HTML structure, THE Parser SHALL return `ParseFailed` and SHALL NOT panic.
7. THE Parser SHALL be unit-testable with HTML fixtures without requiring live network calls.

### Requirement 6: Download Management and Task Queue

**User Story:** As a user, I want to manage downloads with a queue, pause, resume, cancel, and retry controls, so that I can control bandwidth and recover from failures.

#### Acceptance Criteria

1. THE Downloader SHALL maintain a Task_Queue where each DownloadTask has a unique `id`.
2. THE Downloader SHALL limit concurrently running DownloadTasks to the value configured in `AppSettings.max_concurrency`.
3. THE Downloader SHALL support task states `waiting`, `parsing`, `downloading`, `paused`, `completed`, `failed`, and `cancelled`.
4. THE Downloader SHALL only permit state transitions defined by the following state machine:
   - `waiting` → `parsing` | `cancelled`
   - `parsing` → `downloading` | `failed` | `cancelled`
   - `downloading` → `paused` | `completed` | `failed` | `cancelled`
   - `paused` → `downloading` | `cancelled`
   - `failed` → `waiting` (via retry) | `cancelled`
   - `completed` and `cancelled` are terminal states.
5. IF a state transition is requested that is not in the state machine, THEN THE Downloader SHALL reject the transition and return an `InvalidTransition` error.
6. WHEN a DownloadTask is in state `downloading`, THE Downloader SHALL support HTTP Range requests to resume partial downloads after pause or transient failure.
7. THE Downloader SHALL write downloads to a temporary file with suffix `.part` and SHALL atomically rename it to the final filename upon successful completion.
8. WHEN a DownloadTask fails due to a transient error (network timeout, 5xx), THE Downloader SHALL retry up to 3 times with exponential backoff starting at 1 second and capped at 30 seconds.
9. IF a DownloadTask fails after exhausting retries, THEN THE Downloader SHALL set status to `failed` and SHALL record a machine-readable error code plus a user-friendly message.
10. THE Downloader SHALL emit a Tauri event `task_progress` at most every 250 ms per task with fields `id`, `progress`, `speed`, `downloaded_size`, `total_size`.
11. WHEN a DownloadTask completes, THE Downloader SHALL emit a Tauri event `task_completed` with the final save path.
12. WHEN a DownloadTask fails terminally, THE Downloader SHALL emit a Tauri event `task_failed` with `id` and error details.
13. THE Downloader SHALL estimate remaining time using a rolling average of speed over the last 5 seconds.
14. WHEN the user cancels a DownloadTask, THE Downloader SHALL abort the in-flight HTTP request via a cancellation token and SHALL delete the `.part` file.
15. THE Frontend SHALL provide per-task buttons for pause, resume, cancel, retry, open file, and open folder, with each button enabled only in states permitted by the state machine.
16. THE Downloader SHALL classify downloaded files on disk by platform, author (when available), and date, using the directory structure `{download_dir}/{platform}/{author_or_unknown}/{YYYY-MM-DD}/`.
17. THE Filename_Sanitizer SHALL replace characters invalid on Windows, Android, or iOS filesystems (`< > : " / \ | ? *`, control characters) with `_`, SHALL trim trailing dots and spaces, and SHALL truncate filenames to 180 bytes UTF-8 while preserving the file extension.
18. IF a sanitized filename collides with an existing file in the destination directory, THEN THE Filename_Sanitizer SHALL append a numeric suffix `-1`, `-2`, ... before the extension until the name is unique.

### Requirement 7: Clipboard UX

**User Story:** As a user, I want to paste share links easily without the app silently monitoring my clipboard, so that my privacy is respected.

#### Acceptance Criteria

1. THE Clipboard_Reader SHALL NOT poll the clipboard in the background on any platform.
2. WHERE the platform is Windows, THE Frontend SHALL expose an explicit "从剪贴板读取" button that invokes Clipboard_Reader only upon user click.
3. WHERE the platform is Android or iOS, THE Frontend SHALL rely on the standard OS paste gesture into the input field rather than programmatic clipboard reads.
4. WHERE `AppSettings.auto_clipboard` is enabled AND the app window gains focus on desktop, THE Frontend SHALL offer a non-intrusive prompt to use the current clipboard content, without reading the clipboard until the user confirms.
5. THE Clipboard_Reader SHALL NOT persist clipboard content beyond the current parsing request.

### Requirement 8: Theme (Dark / Light / System)

**User Story:** As a user, I want to switch between light, dark, and system themes, so that the UI matches my environment.

#### Acceptance Criteria

1. THE Theme_Controller SHALL support three modes: `system`, `light`, and `dark`.
2. THE Theme_Controller SHALL default to `system` on first launch.
3. WHILE the theme mode is `system`, THE Theme_Controller SHALL apply light or dark based on the OS preference and SHALL update within 500 ms when the OS preference changes.
4. WHEN the user selects a specific theme in Settings, THE Theme_Controller SHALL persist the choice in Settings_Store and SHALL apply it immediately to all open views.

### Requirement 9: Internationalization (i18n)

**User Story:** As a user, I want ClipSave in Simplified Chinese by default with English available, so that I can use the app in my preferred language.

#### Acceptance Criteria

1. THE I18n_Controller SHALL support locales `zh-CN` and `en-US`.
2. THE I18n_Controller SHALL default to `zh-CN` on first launch regardless of OS locale.
3. THE I18n_Controller SHALL allow the user to switch locales from Settings, and SHALL apply the new locale immediately without restart.
4. THE Frontend SHALL source all user-facing strings from the i18n resource files and SHALL NOT hard-code Chinese or English text in components.
5. IF a translation key is missing in the active locale, THEN THE I18n_Controller SHALL fall back to `zh-CN` and SHALL log a warning in debug builds.

### Requirement 10: Settings

**User Story:** As a user, I want a Settings page to control download behavior, clipboard, logs, and UI preferences, so that I can tailor the app to my needs.

#### Acceptance Criteria

1. THE Settings_Store SHALL persist `download_dir`, `max_concurrency`, `filename_template`, `auto_clipboard`, `keep_history`, `debug_log`, `theme`, and `language`.
2. WHEN the user selects a download directory, THE Frontend SHALL invoke the Tauri dialog plugin and SHALL store only the directory chosen by the user.
3. THE Settings_Store SHALL constrain `max_concurrency` to integers between 1 and 8 inclusive, and SHALL default to 3.
4. THE Settings_Store SHALL validate `filename_template` against the supported token set `{platform}`, `{author}`, `{title}`, `{date}`, `{index}`, `{ext}` and SHALL reject templates containing unknown tokens.
5. WHEN `debug_log` is enabled, THE Logger SHALL write structured logs at `info` level or higher to a rotating file; WHEN disabled, THE Logger SHALL write at `warn` level or higher.
6. THE Settings_Store SHALL apply setting changes atomically such that a partial failure does not leave the app in an inconsistent state.

### Requirement 11: History

**User Story:** As a user, I want a searchable history of completed, failed, and cancelled tasks, so that I can review past downloads and re-open files.

#### Acceptance Criteria

1. THE History_Store SHALL persist every DownloadTask that reaches a terminal state (`completed`, `failed`, `cancelled`) in SQLite.
2. THE Frontend SHALL display history with the original URL, platform, author, title, status, created_at, and save_path.
3. THE Frontend SHALL provide a search field that filters history by substring match on title, author, or original URL.
4. THE Frontend SHALL provide filters to show only `completed`, only `failed`, or only `cancelled` entries.
5. THE Frontend SHALL provide a "复制原始链接" button per history row that copies the original URL to the clipboard via the Tauri clipboard plugin.
6. WHEN the user invokes "清空历史", THE Frontend SHALL require a confirmation dialog before calling `clear_history`.
7. WHERE `AppSettings.keep_history` is disabled, THE History_Store SHALL NOT persist new terminal-state entries.

### Requirement 12: Error Handling Taxonomy

**User Story:** As a user, I want friendly, consistent error messages, so that I can understand and act on failures without seeing stack traces.

#### Acceptance Criteria

1. THE Backend SHALL categorize errors into the taxonomy: `ParseFailed`, `UnsupportedPlatform`, `RestrictedContent`, `ContentNotFound`, `NetworkError`, `TimeoutError`, `PermissionDenied`, `DiskFullOrIoError`, `InvalidInput`, `TooManyRedirects`, `UnsafeRedirect`, `InvalidTransition`.
2. THE Frontend SHALL map each error category to a localized, user-friendly message defined in the i18n resources.
3. THE Frontend SHALL NOT display raw Rust panic messages, backtraces, or internal identifiers to the user.
4. WHEN an error occurs, THE Frontend SHALL provide a "复制错误详情" action that copies a redacted error report containing category, code, timestamp, and URL host (but SHALL NOT include full URL, cookies, tokens, credentials, or absolute file paths containing user home).
5. IF the error category is `RestrictedContent`, THEN THE Frontend SHALL display a message explaining that the link is not publicly accessible or requires authorization and SHALL NOT offer a retry.

### Requirement 13: Logging and Redaction

**User Story:** As a developer, I want structured logs on the Rust side for diagnostics, while ensuring user privacy, so that I can debug without leaking sensitive data.

#### Acceptance Criteria

1. THE Logger SHALL produce structured logs (key-value or JSON lines) including fields `timestamp`, `level`, `module`, `event`, and a correlation `task_id` when applicable.
2. THE Logger SHALL NOT log cookies, authentication headers, tokens, full request bodies, or user-entered personally identifying information.
3. THE Logger SHALL log URL hosts and path patterns but SHALL redact query string values for URLs that may contain tokens.
4. THE Logger SHALL rotate log files, retaining at most 5 files of up to 5 MB each.
5. WHEN `debug_log` is disabled, THE Logger SHALL emit only `warn` and `error` entries.

### Requirement 14: Performance and Concurrency

**User Story:** As a user, I want the app to remain responsive during downloads, so that parsing and UI interactions stay smooth.

#### Acceptance Criteria

1. THE Downloader SHALL cap total concurrent HTTP connections at `max_concurrency`, regardless of the number of queued tasks.
2. WHEN the user submits up to 20 URLs in one batch, THE Backend SHALL return a parse response within 10 seconds on a reference machine under normal network conditions.
3. THE Frontend SHALL keep the main thread responsive by running parsing and downloads exclusively in the Rust backend.
4. THE Backend SHALL set HTTP request timeouts to 15 seconds for metadata requests and 120 seconds idle timeout for media downloads.
5. THE Backend SHALL limit the maximum number of HTTP redirects per request to 5.
6. THE Backend SHALL impose a per-host request rate limit of at most 2 requests per second during batch operations to avoid triggering platform rate limiting.

### Requirement 15: Security and Minimum Capabilities

**User Story:** As a user, I want ClipSave to run with the smallest possible set of system permissions, so that the app cannot misuse my device.

#### Acceptance Criteria

1. THE Backend SHALL declare Tauri_Capability entries limited to: filesystem access scoped to the user-selected `download_dir`, dialog plugin for directory selection, clipboard plugin for explicit read/write, opener plugin for opening selected files or folders, and HTTP client for outgoing requests to public hosts.
2. THE Backend SHALL NOT expose a generic shell execution capability.
3. THE Backend SHALL NOT request filesystem access outside `download_dir` and the app's own data directory.
4. THE Backend SHALL NOT store cookies, account tokens, or credentials anywhere on disk or in memory beyond the lifetime of a single request.
5. THE Backend SHALL validate that any file path passed from the Frontend resolves within `download_dir` before any filesystem write.
6. IF a path traversal attempt is detected (`..`, absolute path outside `download_dir`, or symlink escape), THEN THE Backend SHALL return `PermissionDenied` and SHALL NOT perform the operation.
7. THE Backend SHALL send a generic, non-deceptive `User-Agent` header for outgoing requests and SHALL NOT impersonate mobile clients for the purpose of bypassing anti-crawling.

### Requirement 16: Accessibility

**User Story:** As a user relying on assistive tech or keyboard navigation, I want ClipSave to be accessible, so that I can use every feature.

#### Acceptance Criteria

1. THE Frontend SHALL provide `aria-label` attributes for every interactive element that lacks visible text (icon buttons, progress bars, status badges).
2. THE Frontend SHALL associate every form input with a `label` element or `aria-labelledby`.
3. THE Frontend SHALL support full keyboard navigation with a visible focus ring that meets a minimum contrast ratio of 3:1 against adjacent colors.
4. THE Frontend SHALL meet a minimum text contrast ratio of 4.5:1 for body text and 3:1 for large text in both light and dark themes.
5. WHEN a toast or status change occurs, THE Frontend SHALL announce it through an ARIA live region.

### Requirement 17: Responsive Design Across Form Factors

**User Story:** As a user on Windows desktop, Android phone, iPhone, or tablet, I want ClipSave to adapt its layout, so that it is usable on any device.

#### Acceptance Criteria

1. WHERE the viewport width is at least 1024 px, THE Frontend SHALL render a left sidebar navigation with the main content on the right.
2. WHERE the viewport width is less than 1024 px and at least 640 px, THE Frontend SHALL render a condensed top navigation suitable for tablets.
3. WHERE the viewport width is less than 640 px, THE Frontend SHALL render a bottom tab bar for primary navigation.
4. THE Frontend SHALL support portrait and landscape orientations on Android and iOS without layout overflow.
5. THE Frontend SHALL render the home page with a brand area, a large input field, a primary action button, and a task list visible without horizontal scrolling on a 360-px-wide phone.

### Requirement 18: Home Page and Task Card UI

**User Story:** As a user, I want a clear home page with an input and ongoing task list, so that I can paste a link and see progress immediately.

#### Acceptance Criteria

1. THE Frontend SHALL display on the home page a brand area, an input field with placeholder "粘贴抖音/小红书分享链接或分享文本" (in `zh-CN`), a primary button labelled "解析并下载" (in `zh-CN`), and a task list.
2. THE Frontend SHALL display per-task card: platform icon, media-type label, status label, progress bar, estimated remaining time, and action buttons.
3. THE Frontend SHALL display a visible compliance notice reading "请仅保存你拥有权利或已获授权的内容" (in `zh-CN`) on or above the input area.
4. THE Frontend SHALL use `lucide-react` icons consistently and SHALL apply rounded corners, soft shadows, and generous spacing as defined in the Tailwind theme.

### Requirement 19: Tauri Invoke API Surface

**User Story:** As a frontend developer, I want a stable invoke API, so that the UI talks to Rust through a documented surface.

#### Acceptance Criteria

1. THE Backend SHALL expose Tauri invoke commands: `parse_links`, `add_download_task`, `pause_task`, `resume_task`, `cancel_task`, `retry_task`, `open_file`, `open_folder`, `get_settings`, `update_settings`, `get_history`, `clear_history`, `select_directory`, `read_clipboard`.
2. THE Backend SHALL return every command result as a `Result<T, AppError>` where `AppError` maps to the error taxonomy of Requirement 12.
3. THE Backend SHALL validate all command inputs before performing any side effect.
4. IF a command receives an input that fails validation, THEN THE Backend SHALL return `InvalidInput` without mutating any state.

### Requirement 20: Windows Platform Build and Distribution

**User Story:** As a Windows user, I want a native installer, so that I can install ClipSave like any other desktop app.

#### Acceptance Criteria

1. THE Backend SHALL produce NSIS and MSI installers via `tauri build` on Windows x64.
2. THE Frontend SHALL provide an "打开下载目录" action that invokes `open_folder` on the configured `download_dir`.
3. THE Backend SHALL sign Windows installers when `TAURI_SIGNING_PRIVATE_KEY` is present in the build environment, and SHALL produce unsigned installers otherwise without failing the build.

### Requirement 21: Android Platform Build and Distribution

**User Story:** As an Android user, I want an installable APK, so that I can use ClipSave on my phone.

#### Acceptance Criteria

1. THE Backend SHALL produce an APK via `tauri android build`, and SHALL optionally produce an AAB when configured.
2. THE Android build SHALL request only the storage permissions required to write to the user-selected download directory scoped to app-specific storage or the Downloads collection via MediaStore.
3. THE Android build SHALL sign the APK when `ANDROID_KEYSTORE` and related secrets are present in the environment, and SHALL produce a debug-signed APK otherwise.

### Requirement 22: iOS Platform Build

**User Story:** As a developer, I want to build ClipSave for iOS locally, so that it can be installed on a development device without requiring App Store distribution.

#### Acceptance Criteria

1. THE Backend SHALL support `tauri ios build` producing an iOS product when signing materials are available.
2. IF Apple signing secrets (`APPLE_CERTIFICATE`, `APPLE_PROVISIONING_PROFILE`, `APPLE_TEAM_ID`) are absent, THEN the iOS build step SHALL print a clear message explaining that iOS artifacts cannot be signed and SHALL exit with a non-failing status that does not break the overall release.
3. THE Frontend SHALL operate within iOS sandbox constraints, using app-specific storage for downloads and SHALL NOT attempt to write outside the sandbox.
4. THE README SHALL document the Xcode and local iOS build steps.

### Requirement 23: CI Workflow

**User Story:** As a maintainer, I want CI to validate every push and PR, so that regressions are caught early.

#### Acceptance Criteria

1. THE CI workflow SHALL run on `push` and `pull_request` events against the default branch.
2. THE CI workflow SHALL use Node LTS and Rust stable toolchains.
3. THE CI workflow SHALL run `npm run lint`, `npm run typecheck`, `npm run test`, and `cargo test --manifest-path src-tauri/Cargo.toml`.
4. IF any CI step fails, THEN the workflow SHALL exit with a non-zero status.
5. THE CI workflow SHALL NOT require platform signing secrets to pass.

### Requirement 24: Release Workflow and Versioning

**User Story:** As a maintainer, I want one-click releases with consistent versioning across manifests, so that each release is reproducible.

#### Acceptance Criteria

1. THE Release workflow SHALL trigger on `workflow_dispatch`, on push to `main` or `release` branches, and on tags matching `app-v*`.
2. WHEN the Release workflow starts without an explicit version override, THE Release workflow SHALL auto-increment the patch version and SHALL synchronize the new version into `package.json`, `src-tauri/tauri.conf.json`, and `src-tauri/Cargo.toml`.
3. THE Release workflow SHALL create a Git tag of the form `app-v{MAJOR}.{MINOR}.{PATCH}` for each release.
4. THE Release workflow SHALL invoke `tauri-action` to build Windows installers, `tauri android build` for Android artifacts, and `tauri ios build` for iOS artifacts.
5. WHEN iOS signing secrets are absent, THE Release workflow SHALL skip iOS signing gracefully and SHALL still publish Windows and Android artifacts.
6. THE Release workflow SHALL generate a changelog from commit messages since the previous tag.
7. THE Release workflow SHALL publish a GitHub Release named "ClipSave v{MAJOR}.{MINOR}.{PATCH}" and SHALL attach Windows installers, Android APK/AAB, and iOS artifacts or an explanatory note.
8. THE Release workflow SHALL reference these secrets as optional: `GITHUB_TOKEN`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`, `ANDROID_KEYSTORE`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS`, `ANDROID_KEY_PASSWORD`, `APPLE_CERTIFICATE`, `APPLE_CERTIFICATE_PASSWORD`, `APPLE_PROVISIONING_PROFILE`, `APPLE_TEAM_ID`.
9. THE repository SHALL NOT contain committed secrets or credentials.

### Requirement 25: Code Quality Baseline

**User Story:** As a maintainer, I want strict code quality rules, so that the codebase remains maintainable.

#### Acceptance Criteria

1. THE Frontend SHALL compile under TypeScript strict mode with `noImplicitAny`, `strictNullChecks`, and `strictFunctionTypes` enabled.
2. THE Frontend SHALL NOT use the `any` type in application code; the linter SHALL flag new occurrences as errors.
3. THE Backend SHALL use `Result<T, AppError>` for fallible operations and SHALL define a custom `AppError` enum mapped to the error taxonomy of Requirement 12.
4. THE Frontend SHALL keep parsing logic in the Rust backend and SHALL NOT implement URL parsing or platform resolution inside React components.
5. THE Frontend SHALL NOT hard-code filesystem paths and SHALL read `download_dir` from `AppSettings`.
6. THE Backend SHALL include rustdoc comments on every public function in the parser, downloader, and commands modules.

### Requirement 26: README and Documentation

**User Story:** As a user or contributor, I want a complete README, so that I can install, build, and contribute responsibly.

#### Acceptance Criteria

1. THE README SHALL include a project introduction, screenshot placeholders, feature list, compliance statement, local development and build commands, GitHub Actions documentation, automatic versioning description, optional-secrets configuration, FAQ, contribution guide, and license.
2. THE README SHALL state explicitly that ClipSave is not affiliated with Douyin or Xiaohongshu and that users are responsible for compliance with platform terms, copyright, and local law.
3. THE README SHALL describe the scope restriction to Public_Content that the user owns or is authorized to save.

### Requirement 27: Parsing and Pretty-Printing Round-Trip Properties

**User Story:** As a developer, I want round-trip guarantees for URL normalization and filename sanitization, so that these building blocks are reliable under property-based testing.

#### Acceptance Criteria

1. THE URL_Normalizer SHALL be idempotent: for every input URL `u`, `normalize(u)` equals `normalize(normalize(u))`.
2. THE Filename_Sanitizer SHALL be idempotent: for every input string `s`, `sanitize(s)` equals `sanitize(sanitize(s))`.
3. THE Filename_Sanitizer SHALL produce only characters drawn from the portable filename set defined by Requirement 6.17 for all inputs.
4. THE Link_Extractor SHALL satisfy a monotonicity property: for any two texts `a` and `b`, the set of URLs extracted from `a + b` SHALL be a superset of those extracted from `a` and those extracted from `b` (for the purpose of property tests with non-overlapping URL boundaries).
5. THE DownloadTask state machine SHALL satisfy: for every sequence of transition events, if any event is rejected, the task state SHALL remain unchanged after the rejected event.
6. THE URL_Normalizer SHALL produce only HTTP or HTTPS URLs as output, and SHALL reject any input that resolves to a different scheme.

### Requirement 28: Correctness Properties for Property-Based Testing

**User Story:** As a developer, I want explicit correctness properties documented, so that property-based tests exercise the invariants of URL handling, filename sanitization, state machines, and queue invariants.

#### Acceptance Criteria

1. **URL extraction completeness**: FOR ALL texts `t` composed of one or more valid HTTP/HTTPS URLs separated by arbitrary whitespace or non-URL characters, THE Link_Extractor SHALL return every URL present in `t`, in left-to-right order.
2. **URL normalization idempotence**: FOR ALL valid input URLs `u`, THE URL_Normalizer SHALL satisfy `normalize(normalize(u)) == normalize(u)`.
3. **Tracking-parameter stripping invariance**: FOR ALL URLs `u` and any known tracking parameter key `k` appearing in `u`'s query string, THE URL_Normalizer SHALL produce an output whose query string SHALL NOT contain `k`, and any non-tracking parameter in `u` SHALL be preserved.
4. **Filename sanitization closure**: FOR ALL input strings `s`, every character in `sanitize(s)` SHALL be drawn from the portable filename character set defined by Requirement 6.17, and `sanitize(s)` SHALL be non-empty (falling back to `"untitled"` when input sanitizes to empty).
5. **Filename length bound**: FOR ALL input strings `s`, the byte length of `sanitize(s)` in UTF-8 SHALL be at most 180 bytes when an extension is not reserved, and SHALL preserve a provided extension when one is supplied.
6. **State-machine soundness**: FOR ALL sequences of transition requests applied to a DownloadTask, the final state SHALL be reachable from the initial state via the transitions defined in Requirement 6.4, and no sequence SHALL cause the task to enter an undefined state.
7. **Terminal-state stability**: FOR ALL DownloadTasks in state `completed` or `cancelled`, any subsequent transition request other than the explicit `retry` event (which only applies to `failed`) SHALL leave the state unchanged.
8. **Redirect bound**: FOR ALL redirect chains of length `n`, the URL_Normalizer SHALL terminate with either a canonical URL when `n <= 5` or an error when `n > 5`, and SHALL NOT follow more than 5 hops.
9. **Concurrency invariant**: FOR ALL Task_Queue states, the number of DownloadTasks with status `downloading` SHALL at no time exceed `AppSettings.max_concurrency`.
10. **Queue ordering**: WHEN capacity is available, THE Task_Queue SHALL promote DownloadTasks from `waiting` to `parsing` in FIFO order by `created_at` among equal-priority tasks.
11. **Batch limit enforcement**: FOR ALL input texts containing more than 20 URLs, THE Link_Extractor SHALL return exactly 20 URLs corresponding to the first 20 occurrences in the text.
12. **Safe-redirect property**: FOR ALL redirect targets with non-HTTP(S) schemes, THE URL_Normalizer SHALL reject the redirect with `UnsafeRedirect`.
13. **Parser error graceful failure**: FOR ALL malformed HTML fixtures supplied to a Parser, the Parser SHALL return a `ParseFailed` error and SHALL NOT panic.
14. **Atomic write property**: FOR ALL DownloadTask completions, the final destination file SHALL exist only when the full download succeeded; a partial or cancelled download SHALL NOT leave a non-`.part` file at the destination path.

### Requirement 29: MCP-Assisted Public Media Resolution (Scoped, Compliant)

**User Story:** As a developer, I want ClipSave to leverage MCP-based fetching of publicly available page metadata to obtain direct media URLs, so that resolution is robust while remaining strictly within the compliance boundary.

#### Acceptance Criteria

1. WHERE MCP-assisted fetching is used to obtain direct media links, THE Parser SHALL only request resources that are publicly accessible without authentication.
2. THE Parser SHALL derive direct media URLs exclusively from public sources such as HTML meta tags, Open Graph tags, JSON-LD blocks, or publicly embedded JSON data in the rendered page.
3. THE Parser SHALL NOT call undocumented or private APIs, SHALL NOT forge platform-specific signatures, SHALL NOT inject cookies harvested from user sessions, and SHALL NOT attempt watermark removal.
4. IF a platform introduces anti-crawling or risk-control protections that block public access, THEN THE Parser SHALL return `RestrictedContent` or `ParseFailed` as appropriate and SHALL NOT retry with evasion techniques.
5. WHEN MCP-based fetching fails for non-compliance-related transient reasons (timeouts, 5xx), THE Parser SHALL retry up to 2 times with exponential backoff before returning `NetworkError`.

## Non-Functional Requirements Summary

- Performance and concurrency: Requirement 14
- Security and minimum capabilities: Requirement 15
- Accessibility: Requirement 16
- Responsive design: Requirement 17
- Code quality baseline: Requirement 25
- Logging and redaction: Requirement 13
- Error taxonomy: Requirement 12

## Compliance Summary

Requirements 1, 3, 4, 5, 7, 15, 22, 26, and 29 jointly codify the hard compliance boundary:

- No bypass of login, captcha, paywall, DRM, risk control, or anti-crawling.
- No watermark removal, no copyright-evasion, no bulk scraping, no account automation, no credentials or cookie theft.
- No private APIs, no reverse-engineered signatures.
- Graceful failure when a platform changes or restricts access.
- Explicit user-rights prompts in UI and README.
- Affiliation disclaimer with respect to Douyin and Xiaohongshu.

These constraints are inherited by all downstream design and task artifacts and SHALL NOT be weakened without explicit user approval.
