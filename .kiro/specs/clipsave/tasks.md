# Implementation Plan: ClipSave

## Overview

ClipSave is a cross-platform media save tool built with Tauri v2 (Rust backend) and React + TypeScript + Tailwind CSS (frontend). This plan implements the full feature set incrementally: foundation (error types, traits, config), core modules (parsers, downloader, storage), frontend UI, CI/CD, and documentation. Each task builds on previous ones and references specific requirements for traceability.

**Backend Language:** Rust  
**Frontend Language:** TypeScript / React  
**Targets:** Windows, Android, iOS

## Tasks

- [x] 1. Set up Rust backend foundation
  - [x] 1.1 Create Cargo.toml with dependencies and tauri.conf.json configuration
    - Initialize `src-tauri/Cargo.toml` with dependencies: tauri, serde, serde_json, tokio, reqwest, rusqlite, tracing, tracing-subscriber, regex, url, uuid, thiserror
    - Create `src-tauri/tauri.conf.json` with app identifier, window config, and Tauri v2 capability declarations
    - Create `src-tauri/capabilities/default.json` with minimum permissions: fs (scoped to download_dir), dialog, clipboard, opener, http
    - Create `src-tauri/src/main.rs` with Tauri app builder
    - _Requirements: 15.1, 15.2, 15.3_

  - [x] 1.2 Define the AppError enum and error taxonomy
    - Create `src-tauri/src/error.rs` with `AppError` enum variants: `ParseFailed`, `UnsupportedPlatform`, `RestrictedContent`, `ContentNotFound`, `NetworkError`, `TimeoutError`, `PermissionDenied`, `DiskFullOrIoError`, `InvalidInput`, `TooManyRedirects`, `UnsafeRedirect`, `InvalidTransition`
    - Implement `serde::Serialize` for AppError to pass to frontend
    - Implement `std::fmt::Display` and `std::error::Error` for AppError
    - Add machine-readable error codes and user-friendly message fields
    - _Requirements: 12.1, 12.2, 25.3_

  - [x] 1.3 Define core data types and models
    - Create `src-tauri/src/models.rs` with structs: `ResolvedMedia`, `MediaItem`, `DownloadTask`, `AppSettings`, `HistoryEntry`
    - Define enums: `Platform`, `MediaType`, `TaskStatus`, `ParserError`
    - Implement `serde::Serialize` and `serde::Deserialize` for all types
    - Define the `TaskStatus` state machine transitions as a method
    - _Requirements: 6.3, 6.4, 6.5, 5.1_

  - [ ]* 1.4 Write unit tests for error types and state machine transitions
    - Test that all AppError variants serialize correctly
    - Test valid state transitions succeed
    - Test invalid state transitions return `InvalidTransition`
    - Test terminal states (`completed`, `cancelled`) reject all transitions except retry on `failed`
    - _Requirements: 6.4, 6.5, 27.5, 28.6, 28.7_

- [x] 2. Implement Link Extractor and URL Normalizer
  - [x] 2.1 Implement the Link_Extractor module
    - Create `src-tauri/src/parser/link_extractor.rs`
    - Extract all HTTP/HTTPS URLs from arbitrary text using regex
    - Return URLs in left-to-right order of appearance
    - Enforce batch limit of 20 URLs per submission, return first 20 if exceeded
    - Return empty vec when no URLs found
    - _Requirements: 2.1, 2.2, 2.3, 2.10, 2.11_

  - [x] 2.2 Implement the URL_Normalizer module
    - Create `src-tauri/src/parser/url_normalizer.rs`
    - Implement percent-decoding of extracted URLs
    - Strip known tracking parameters: `utm_source`, `utm_medium`, `utm_campaign`, `utm_term`, `utm_content`, `share_token`, `share_from`, `share_app_id`, `app_platform`, `timestamp`, `xhsshare`, `appuid`, `apptime`, `share_id`
    - Preserve path segments and required query parameters (e.g., `xsec_token`)
    - Follow HTTP redirects up to 5 hops for short links
    - Return `TooManyRedirects` if redirect chain exceeds 5
    - Return `UnsafeRedirect` if redirect target is non-HTTP(S) or non-whitelisted host
    - Ensure idempotence: `normalize(normalize(u)) == normalize(u)`
    - Ensure output is always HTTP or HTTPS scheme
    - _Requirements: 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 2.12, 27.1, 27.6, 14.5_

  - [ ]* 2.3 Write property tests for Link_Extractor
    - **Property 1: URL extraction completeness** — for all texts with embedded HTTP/HTTPS URLs, extractor returns every URL in order
    - **Property 11: Batch limit enforcement** — for texts with >20 URLs, exactly 20 are returned
    - **Property 4 (partial): Monotonicity** — URLs from `a+b` is superset of URLs from `a` and `b`
    - **Validates: Requirements 2.1, 2.10, 27.4, 28.1, 28.11**

  - [ ]* 2.4 Write property tests for URL_Normalizer
    - **Property 2: URL normalization idempotence** — `normalize(normalize(u)) == normalize(u)`
    - **Property 3: Tracking-parameter stripping** — output never contains known tracking params, non-tracking params preserved
    - **Property 8: Redirect bound** — chains >5 return error, chains ≤5 return canonical URL
    - **Property 12: Safe-redirect property** — non-HTTP(S) redirect targets rejected with `UnsafeRedirect`
    - **Validates: Requirements 2.5, 2.7, 2.8, 2.9, 27.1, 27.6, 28.2, 28.3, 28.8, 28.12**

- [x] 3. Implement Filename Sanitizer
  - [x] 3.1 Implement the Filename_Sanitizer module
    - Create `src-tauri/src/download/filename_sanitizer.rs`
    - Replace invalid filesystem characters (`< > : " / \ | ? *`, control chars) with `_`
    - Trim trailing dots and spaces
    - Truncate to 180 bytes UTF-8 while preserving file extension
    - Handle collision by appending `-1`, `-2`, etc. before extension
    - Fall back to `"untitled"` when input sanitizes to empty
    - Ensure idempotence: `sanitize(sanitize(s)) == sanitize(s)`
    - _Requirements: 6.17, 6.18, 27.2, 27.3_

  - [ ]* 3.2 Write property tests for Filename_Sanitizer
    - **Property 4: Filename sanitization closure** — output contains only portable filename characters, non-empty
    - **Property 5: Filename length bound** — output ≤180 bytes UTF-8, extension preserved
    - **Property 2 (filename): Idempotence** — `sanitize(sanitize(s)) == sanitize(s)`
    - **Validates: Requirements 6.17, 6.18, 27.2, 27.3, 28.4, 28.5**

- [x] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. Implement Parser trait and platform parsers
  - [x] 5.1 Define the Parser trait and registry
    - Create `src-tauri/src/parser/mod.rs` with `Parser` trait: `can_handle(url) -> bool`, `normalize(url) -> Result<NormalizedUrl, ParserError>`, `resolve(normalized_url) -> Result<ResolvedMedia, ParserError>`
    - Implement parser registry that iterates registered parsers and selects first match
    - Return `UnsupportedPlatform` when no parser matches
    - _Requirements: 5.1, 5.2, 5.3_

  - [x] 5.2 Implement the Douyin_Parser
    - Create `src-tauri/src/parser/douyin.rs`
    - Handle hosts: `douyin.com`, `www.douyin.com`, `iesdouyin.com`, `www.iesdouyin.com`, `v.douyin.com`
    - Return `ResolvedMedia` with platform `douyin`, canonical URL, MediaItem entries (video, image, gif)
    - Extract title and author from public meta tags, Open Graph, or JSON-LD
    - Return `RestrictedContent` for HTTP 401/403/451 or login-required pages
    - Return `ContentNotFound` for HTTP 404
    - Return `ParseFailed` with platform-version hint on unexpected HTML structure
    - Do NOT use private APIs, solve captcha, or forge device signatures
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 5.4, 5.5, 5.6_

  - [x] 5.3 Implement the Xiaohongshu_Parser
    - Create `src-tauri/src/parser/xiaohongshu.rs`
    - Handle hosts: `xiaohongshu.com`, `www.xiaohongshu.com`, `xhslink.com`
    - Handle URL paths: `/explore/{id}`, `/discovery/item/{id}`
    - Return `ResolvedMedia` with platform `xiaohongshu`, canonical URL, MediaItem entries (image, video, gif)
    - Return `RestrictedContent` for HTTP 401/403/451 or login-required pages
    - Return `ContentNotFound` for HTTP 404
    - Return `ParseFailed` with platform-version hint on unexpected HTML structure
    - Do NOT use private APIs, solve captcha, or bypass `xsec_token` validation
    - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7, 4.8, 5.4, 5.5, 5.6_

  - [ ]* 5.4 Write unit tests for parsers with HTML fixtures
    - Create HTML fixture files for Douyin and Xiaohongshu public pages
    - Test successful resolution returns correct ResolvedMedia
    - Test restricted content detection (401/403/451 responses)
    - Test content not found (404 responses)
    - Test malformed HTML returns `ParseFailed` without panic
    - **Property 13: Parser error graceful failure** — malformed HTML never panics
    - **Validates: Requirements 3.1–3.8, 4.1–4.8, 5.6, 5.7, 28.13**

- [x] 6. Implement MCP-assisted resolution layer
  - [x] 6.1 Implement compliant page fetching for media resolution
    - Create `src-tauri/src/parser/fetcher.rs` for HTTP page fetching
    - Only request publicly accessible resources without authentication
    - Derive media URLs from public sources: HTML meta tags, OG tags, JSON-LD, embedded JSON
    - Set generic, non-deceptive User-Agent header
    - Set HTTP timeouts: 15s for metadata, 120s idle for media downloads
    - Implement per-host rate limit of 2 requests/second during batch operations
    - Retry up to 2 times with exponential backoff on transient failures (5xx, timeouts)
    - Return `RestrictedContent` or `ParseFailed` when anti-crawling blocks access
    - _Requirements: 29.1, 29.2, 29.3, 29.4, 29.5, 14.4, 14.6, 15.7_

- [x] 7. Implement Download Manager and Task Queue
  - [x] 7.1 Implement the Task_Queue with concurrency control
    - Create `src-tauri/src/download/task_queue.rs`
    - Maintain queue with unique task IDs
    - Limit concurrent downloads to `AppSettings.max_concurrency`
    - Promote tasks from `waiting` to `parsing` in FIFO order by `created_at`
    - Enforce state machine transitions, reject invalid transitions with `InvalidTransition`
    - _Requirements: 6.1, 6.2, 6.3, 6.4, 6.5, 14.1_

  - [x] 7.2 Implement the Downloader with resume, retry, and progress
    - Create `src-tauri/src/download/downloader.rs`
    - Support HTTP Range requests for resume after pause or transient failure
    - Write to temporary `.part` file, atomically rename on completion
    - Retry up to 3 times with exponential backoff (1s start, 30s cap) on transient errors
    - Set status to `failed` with error code and user-friendly message after exhausting retries
    - Emit Tauri events: `task_progress` (max every 250ms), `task_completed`, `task_failed`
    - Estimate remaining time using rolling average of speed over last 5 seconds
    - On cancel: abort HTTP request via cancellation token, delete `.part` file
    - _Requirements: 6.6, 6.7, 6.8, 6.9, 6.10, 6.11, 6.12, 6.13, 6.14_

  - [x] 7.3 Implement file organization and directory structure
    - Create `src-tauri/src/download/file_organizer.rs`
    - Classify files by: `{download_dir}/{platform}/{author_or_unknown}/{YYYY-MM-DD}/`
    - Integrate Filename_Sanitizer for safe filenames
    - Handle filename collisions with numeric suffix
    - Validate file paths resolve within `download_dir` before write
    - Detect path traversal (`..`, absolute paths, symlink escape) and return `PermissionDenied`
    - _Requirements: 6.16, 6.17, 6.18, 15.5, 15.6_

  - [ ]* 7.4 Write property tests for Task_Queue and Downloader
    - **Property 6: State-machine soundness** — all reachable states are valid per Req 6.4
    - **Property 7: Terminal-state stability** — `completed`/`cancelled` reject transitions
    - **Property 9: Concurrency invariant** — downloading count never exceeds max_concurrency
    - **Property 10: Queue ordering** — FIFO promotion by created_at
    - **Property 14: Atomic write property** — final file exists only on full success
    - **Validates: Requirements 6.1–6.18, 14.1, 27.5, 28.6, 28.7, 28.9, 28.10, 28.14**

- [x] 8. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Implement SQLite storage layer
  - [x] 9.1 Implement Settings_Store with SQLite
    - Create `src-tauri/src/storage/settings.rs`
    - Persist: `download_dir`, `max_concurrency`, `filename_template`, `auto_clipboard`, `keep_history`, `debug_log`, `theme`, `language`
    - Constrain `max_concurrency` to 1–8, default 3
    - Validate `filename_template` against supported tokens: `{platform}`, `{author}`, `{title}`, `{date}`, `{index}`, `{ext}`
    - Apply changes atomically (transaction-based)
    - _Requirements: 10.1, 10.3, 10.4, 10.6_

  - [x] 9.2 Implement History_Store with SQLite
    - Create `src-tauri/src/storage/history.rs`
    - Persist tasks reaching terminal states (`completed`, `failed`, `cancelled`)
    - Store: original URL, platform, author, title, status, created_at, save_path
    - Support search by substring match on title, author, or URL
    - Support filter by status
    - Support `clear_history` operation
    - Respect `keep_history` setting — skip persistence when disabled
    - _Requirements: 11.1, 11.2, 11.3, 11.4, 11.6, 11.7_

  - [ ]* 9.3 Write unit tests for storage layer
    - Test settings CRUD and validation (invalid max_concurrency, invalid template tokens)
    - Test history persistence and search/filter
    - Test atomic settings update (partial failure doesn't corrupt state)
    - Test clear_history removes all entries
    - _Requirements: 10.1–10.6, 11.1–11.7_

- [x] 10. Implement Logger with structured output and redaction
  - [x] 10.1 Implement the Logger module
    - Create `src-tauri/src/logger.rs`
    - Produce structured logs (JSON lines) with fields: `timestamp`, `level`, `module`, `event`, `task_id`
    - Rotate log files: max 5 files, 5 MB each
    - When `debug_log` enabled: log at `info` level and above
    - When `debug_log` disabled: log at `warn` level and above
    - Redact: cookies, auth headers, tokens, request bodies, PII
    - Log URL hosts and path patterns, redact query string values for token-bearing URLs
    - _Requirements: 13.1, 13.2, 13.3, 13.4, 13.5, 10.5_

- [x] 11. Implement Tauri commands (invoke API surface)
  - [x] 11.1 Implement parse_links and add_download_task commands
    - Create `src-tauri/src/commands/mod.rs` and individual command files
    - `parse_links(input: String) -> Result<Vec<ResolvedMedia>, AppError>`: extract URLs, normalize, resolve via parsers
    - `add_download_task(task)`: add resolved media to download queue
    - Validate all inputs before side effects
    - Return `InvalidInput` for failed validation without mutating state
    - _Requirements: 19.1, 19.2, 19.3, 19.4_

  - [x] 11.2 Implement task control commands
    - `pause_task(id)`, `resume_task(id)`, `cancel_task(id)`, `retry_task(id)`
    - Enforce state machine transitions
    - Return `InvalidTransition` for disallowed operations
    - _Requirements: 19.1, 6.4, 6.5_

  - [x] 11.3 Implement file and folder commands
    - `open_file(path)`: validate path within download_dir, open via opener plugin
    - `open_folder(path)`: validate path, open containing folder
    - Return `PermissionDenied` for path traversal attempts
    - _Requirements: 19.1, 15.5, 15.6, 20.2_

  - [x] 11.4 Implement settings and history commands
    - `get_settings()`, `update_settings(settings)`: read/write via Settings_Store
    - `get_history()`, `clear_history()`: read/clear via History_Store
    - `select_directory()`: invoke Tauri dialog plugin for directory selection
    - `read_clipboard()`: read clipboard via Tauri clipboard plugin (explicit invocation only)
    - _Requirements: 19.1, 10.2, 7.2, 7.5_

  - [x] 11.5 Register all commands in Tauri app builder
    - Update `src-tauri/src/main.rs` to register all invoke handlers
    - Configure Tauri plugins: dialog, clipboard, opener, fs
    - Initialize SQLite database on app start
    - Initialize logger on app start
    - _Requirements: 19.1, 15.1_

- [x] 12. Checkpoint - Ensure all Rust tests pass
  - Ensure all tests pass (`cargo test --manifest-path src-tauri/Cargo.toml`), ask the user if questions arise.

- [x] 13. Implement Frontend i18n system
  - [x] 13.1 Expand i18n resource files with all user-facing strings
    - Update `src/i18n/zh-CN/common.json` with all UI strings, error messages, labels, placeholders
    - Update `src/i18n/en-US/common.json` with English translations
    - Include error category messages mapped from AppError taxonomy
    - Include compliance notice text, about page disclaimers
    - _Requirements: 9.1, 9.4, 12.2, 1.6, 1.7_

  - [x] 13.2 Enhance i18n controller with locale switching and fallback
    - Update `src/lib/i18n.ts` to support runtime locale switching without restart
    - Implement fallback to `zh-CN` when translation key missing
    - Log warning in debug builds for missing keys
    - Default to `zh-CN` on first launch
    - _Requirements: 9.1, 9.2, 9.3, 9.5_

- [x] 14. Implement Frontend Home Page and Task Cards
  - [x] 14.1 Implement the Home Page with input area and compliance notice
    - Update `src/routes/HomePage.tsx`
    - Add brand area with app name/logo
    - Add input field with placeholder "粘贴抖音/小红书分享链接或分享文本" (from i18n)
    - Add primary button "解析并下载" (from i18n)
    - Display compliance notice "请仅保存你拥有权利或已获授权的内容" (from i18n)
    - Wire input submission to `parseLinks` Tauri command
    - Display "未识别到链接" message when no URLs found
    - Display batch limit message when >20 URLs submitted
    - All strings sourced from i18n, no hard-coded text
    - _Requirements: 18.1, 18.3, 18.4, 1.6, 2.2, 2.11, 9.4_

  - [x] 14.2 Implement TaskCard component with full controls
    - Update `src/features/downloads/TaskCard.tsx`
    - Display: platform icon, media-type label, status badge, progress bar, estimated remaining time, speed
    - Add action buttons: pause, resume, cancel, retry, open file, open folder
    - Enable/disable buttons based on current task state per state machine
    - Wire buttons to Tauri commands (pauseTask, resumeTask, cancelTask, retryTask, openFile, openFolder)
    - Use `lucide-react` icons consistently
    - _Requirements: 18.2, 6.15, 18.4_

  - [x] 14.3 Implement Clipboard UX
    - Add "从剪贴板读取" button on desktop (Windows) that invokes `readClipboard` on click
    - On mobile (Android/iOS), rely on standard OS paste gesture into input field
    - When `auto_clipboard` enabled and window gains focus on desktop, show non-intrusive prompt
    - Do NOT poll clipboard in background on any platform
    - Do NOT persist clipboard content beyond current parsing request
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 15. Implement Frontend Settings Page
  - [x] 15.1 Build the Settings Page UI
    - Create/update `src/routes/SettingsPage.tsx`
    - Download directory selector (invokes `selectDirectory` command)
    - Max concurrency slider/input (1–8)
    - Filename template input with token validation feedback
    - Auto-clipboard toggle
    - Keep history toggle
    - Debug log toggle
    - Theme selector (system/light/dark)
    - Language selector (zh-CN/en-US)
    - All labels from i18n resources
    - Associate every input with label element or aria-labelledby
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 8.1, 9.3, 16.2_

- [x] 16. Implement Frontend History Page
  - [x] 16.1 Build the History Page UI
    - Create/update `src/routes/HistoryPage.tsx`
    - Display history entries: original URL, platform, author, title, status, created_at, save_path
    - Add search field filtering by substring on title, author, or URL
    - Add status filter buttons: all, completed, failed, cancelled
    - Add "复制原始链接" button per row (copies URL via clipboard plugin)
    - Add "清空历史" button with confirmation dialog before clearing
    - Wire to `getHistory` and `clearHistory` Tauri commands
    - _Requirements: 11.2, 11.3, 11.4, 11.5, 11.6_

- [x] 17. Implement Frontend About Page
  - [x] 17.1 Build the About Page with compliance statements
    - Create/update `src/routes/AboutPage.tsx`
    - Display app version, description
    - Include statement: ClipSave is not affiliated with Douyin or Xiaohongshu
    - Include statement: tool does not bypass access controls or copyright protection
    - All text from i18n resources
    - _Requirements: 1.7_

- [x] 18. Implement Error Display and User Feedback
  - [x] 18.1 Implement error message mapping and display
    - Create `src/lib/errors.ts` mapping AppError categories to i18n keys
    - Display localized user-friendly messages for each error category
    - Never display raw Rust panic messages, backtraces, or internal identifiers
    - For `RestrictedContent`: explain link is not publicly accessible, do NOT offer retry
    - Add "复制错误详情" action: copies redacted report (category, code, timestamp, URL host only)
    - Do NOT include full URL, cookies, tokens, credentials, or absolute paths with user home
    - _Requirements: 12.2, 12.3, 12.4, 12.5_

- [x] 19. Implement Responsive Layout and Theme
  - [x] 19.1 Implement responsive navigation layout
    - Update `src/components/Layout.tsx`
    - ≥1024px: left sidebar navigation with main content on right
    - 640–1023px: condensed top navigation for tablets
    - <640px: bottom tab bar for primary navigation
    - Support portrait and landscape orientations without overflow
    - Home page usable without horizontal scroll on 360px-wide phone
    - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5_

  - [x] 19.2 Implement theme controller integration
    - Ensure `useTheme` hook applies theme within 500ms of OS preference change
    - Persist theme choice in Settings_Store via `updateSettings`
    - Apply theme immediately to all open views on change
    - _Requirements: 8.1, 8.2, 8.3, 8.4_

- [x] 20. Implement Accessibility
  - [x] 20.1 Add ARIA attributes and keyboard navigation
    - Add `aria-label` to all icon buttons, progress bars, status badges
    - Associate all form inputs with labels or `aria-labelledby`
    - Ensure full keyboard navigation with visible focus ring (3:1 contrast minimum)
    - Ensure text contrast: 4.5:1 body text, 3:1 large text in both themes
    - Add ARIA live regions for toasts and status changes
    - _Requirements: 16.1, 16.2, 16.3, 16.4, 16.5_

- [x] 21. Checkpoint - Ensure frontend builds and all tests pass
  - Run `npm run typecheck`, `npm run lint`, `npm run test`
  - Ensure all tests pass, ask the user if questions arise.

- [x] 22. Implement CI workflow
  - [x] 22.1 Create CI workflow for push and PR validation
    - Create `.github/workflows/ci.yml`
    - Trigger on `push` and `pull_request` to default branch
    - Use Node LTS and Rust stable toolchains
    - Run: `npm run lint`, `npm run typecheck`, `npm run test`, `cargo test --manifest-path src-tauri/Cargo.toml`
    - Exit non-zero on any step failure
    - Do NOT require platform signing secrets to pass
    - _Requirements: 23.1, 23.2, 23.3, 23.4, 23.5_

- [x] 23. Implement Release workflow
  - [x] 23.1 Create Release workflow with versioning and multi-platform builds
    - Create `.github/workflows/release.yml`
    - Trigger on `workflow_dispatch`, push to `main`/`release`, tags matching `app-v*`
    - Auto-increment patch version, sync to `package.json`, `tauri.conf.json`, `Cargo.toml`
    - Create Git tag `app-v{MAJOR}.{MINOR}.{PATCH}`
    - Build Windows installers (NSIS + MSI) via `tauri-action`
    - Build Android APK/AAB via `tauri android build`
    - Build iOS via `tauri ios build` (skip gracefully when signing secrets absent)
    - Generate changelog from commits since previous tag
    - Publish GitHub Release with all artifacts
    - Reference optional secrets: `GITHUB_TOKEN`, `TAURI_SIGNING_PRIVATE_KEY`, `ANDROID_KEYSTORE`, `APPLE_CERTIFICATE`, etc.
    - Do NOT commit secrets to repository
    - _Requirements: 24.1, 24.2, 24.3, 24.4, 24.5, 24.6, 24.7, 24.8, 24.9, 20.1, 20.3, 21.1, 21.2, 21.3, 22.1, 22.2_

- [x] 24. Create Documentation
  - [x] 24.1 Create README.md
    - Project introduction and screenshot placeholders
    - Feature list
    - Compliance statement (Chinese and English): platform terms, copyright, local-law obligations
    - Scope restriction to Public_Content
    - Affiliation disclaimer (not affiliated with Douyin or Xiaohongshu)
    - Local development and build commands
    - GitHub Actions documentation
    - Automatic versioning description
    - Optional-secrets configuration
    - Xcode and local iOS build steps
    - FAQ section
    - Contribution guide
    - License reference
    - _Requirements: 26.1, 26.2, 26.3, 1.8, 22.4_

  - [x] 24.2 Create SECURITY.md
    - Describe security policy and responsible disclosure process
    - Document minimum capabilities and permission model
    - _Requirements: 15.1_

  - [x] 24.3 Create LICENSE file
    - Add appropriate open-source license
    - _Requirements: 26.1_

- [x] 25. Final checkpoint - Ensure all tests pass and project builds
  - Run full test suite: `npm run lint`, `npm run typecheck`, `npm run test`, `cargo test`
  - Verify `npm run build` succeeds
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests validate universal correctness properties from Requirements 27 and 28
- Unit tests validate specific examples and edge cases
- The Rust backend handles all parsing, downloading, and storage — the frontend never implements URL parsing or platform resolution (Requirement 25.4)
- All user-facing strings come from i18n resources (Requirement 9.4)
- Compliance constraints (Requirement 1) are inherited by all implementation tasks
