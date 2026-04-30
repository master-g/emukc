## 1. 新增 Web 资源下载函数

- [ ] 1.1 在 `crates/emukc_bootstrap/src/` 新增 `web_assets.rs` 模块，导出 `download_web_assets(cache_root, cdn_config, overwrite, proxy)` 异步函数
- [ ] 1.2 函数内构造临时 `Kache` 实例（使用传入的 cache_root 和 CDN 配置），依次调用 `get_with_opt()` 下载 `gadget_html5/js/kcs_const.js`、`kcs2/js/main.js`、`kcs2/version.json` 到 `z/cache/` 对应路径
- [ ] 1.3 CDN 配置为空时 warn 并跳过对应文件下载，不报错
- [ ] 1.4 已有文件且 overwrite=false 时跳过

## 2. Bootstrap CLI 集成

- [ ] 2.1 在 `BootstrapArgs` 新增 `--skip-web-assets` 布尔标志（默认 false）
- [ ] 2.2 在 `exec()` 中 Codex 构建完成后调用 `download_web_assets()`
- [ ] 2.3 将 `gadgets_cdn` 和 `game_cdn` 从 `AppConfig` 传入下载函数
- [ ] 2.4 `--force-update` 分支：删除版本文件后继续执行 `download_web_assets()` 恢复

## 3. 模块注册与导出

- [ ] 3.1 在 `crates/emukc_bootstrap/src/lib.rs` 或 `mod.rs` 注册 `web_assets` 模块
- [ ] 3.2 确保 `download_web_assets` 通过 `emukc_internal::prelude` 可达

## 4. 测试验证

- [ ] 4.1 单元测试：CDN 配置为空时函数不报错并输出 warn
- [ ] 4.2 手动验证：`cargo run -- bootstrap` 后 `z/cache/gadget_html5/js/kcs_const.js` 存在
- [ ] 4.3 手动验证：`bun run decode -- --sync-battle-assets` 能成功执行
- [ ] 4.4 手动验证：`cargo run -- bootstrap --force-update` 删除后重新下载
