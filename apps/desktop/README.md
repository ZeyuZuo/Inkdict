# 桌面

Tauri 2。查词由 Tauri 的 Rust 部分直接调用 `zdict-core`，完全离线。界面用 TypeScript + 轻量框架（Solid 或 Svelte，开工时定）写在本目录，只运行在 Tauri 的 WebView 里，不单独部署。

计划功能：即时联想、词条渲染、发音、链接跳转、历史、深色模式；全局快捷键、剪贴板取词、迷你窗口。阶段：`docs/roadmap.md` M3。
