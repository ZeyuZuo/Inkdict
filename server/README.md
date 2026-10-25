# Server

Web 在线模式的后端：Cloudflare Worker，数据放 D1。查询逻辑来自编译为 WASM 的 `zdict-core`，与各端一致。

计划接口：`/suggest?q=`、`/search?q=`、`/entry/:key`。Android 和桌面不需要这个后端。阶段：`docs/roadmap.md` M3。
