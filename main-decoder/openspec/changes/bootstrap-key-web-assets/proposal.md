## Why

Bootstrap 命令 (`cargo run -- bootstrap`) 不下载 `kcs_const.js` 和 `kcs2/js/main.js` 等关键 Web 资源。这些文件仅在使用缓存服务器时通过 `make_list` 按需获取，但 `main-decoder`（TypeScript 解码工具）直接依赖它们。当这些文件缺失时，`bun run decode` 完全阻塞。此外 `--force-update` 会主动删除 `kcs_const.js` 却不负责恢复。

## What Changes

- 将 `gadget_html5/js/kcs_const.js` 纳入 bootstrap 下载范围，保存到 `z/cache/gadget_html5/js/kcs_const.js`
- 将 `kcs2/js/main.js` 纳入 bootstrap 下载范围，保存到 `z/cache/kcs2/js/main.js`
- 将 `kcs2/version.json` 纳入 bootstrap 下载范围，保存到 `z/cache/kcs2/version.json`
- bootstrap CLI 新增 `--include-web-assets` 标志控制是否下载这些文件（默认开启）
- `--force-update` 清理版本文件后重新下载恢复

## Capabilities

### New Capabilities
- `web-asset-bootstrap`: 将 KanColle 客户端关键 Web 资源（kcs_const.js、main.js、version.json）纳入 bootstrap 下载流程

### Modified Capabilities

## Impact

- `crates/emukc_bootstrap/src/res.rs` — RES_LIST 新增资源条目或新增专用下载函数
- `crates/emukc_bootstrap/src/download.rs` — 可能需要支持 CDN 路径下载（现有下载走固定 URL）
- `src/bin/cli/bootstrap.rs` — CLI 参数扩展，force-update 后重新下载
- `crates/emukc_bootstrap/src/make_list/source/kcs2/plain.rs` — `parse_main_js_version()` 可改为从已缓存文件读取
- `main-decoder/src/io.ts` — 无需改动，文件就位后自然可用
