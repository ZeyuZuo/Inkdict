# Web

TypeScript + 轻量框架（Solid 或 Svelte，开工时定）。负责联想、词条渲染、发音、链接跳转、历史、深色模式。

两种运行方式（见 `docs/architecture.md`）：

- 在线：调用 `server/` 的 API；
- 离线：下载 `.zdpack`，用 WASM 版核心在浏览器里查。

桌面端（`apps/desktop`）复用这里的界面。阶段：`docs/roadmap.md` M3。
