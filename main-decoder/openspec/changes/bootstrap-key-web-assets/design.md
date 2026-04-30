## Context

Bootstrap 命令下载游戏数据 JSON（start2、quests 等）到临时目录，解析后生成 Codex。但 KanColle 客户端核心 JS 文件（kcs_const.js、main.js、version.json）不在 RES_LIST 中，仅在使用缓存服务器时通过 `make_list` → `Kache::get_with_opt()` 按需从 CDN 获取。

main-decoder 工具需要这些文件作为直接输入。当前工作流要求用户先启动缓存服务器或手动下载，造成开发摩擦。

CDN URL 模式：
- `kcs_const.js` → `http://w00g.kancolle-server.com/gadget_html5/js/kcs_const.js`（gadgets CDN）
- `main.js` → `http://w0{x}{y}.kancolle-server.com/kcs2/js/main.js`（game CDN）
- `version.json` → 同上 game CDN

下载需要 CDN 配置（`emukc.config.toml` 中的 `gadgets_cdn` 和 `game_cdn`）。

## Goals / Non-Goals

**Goals:**
- Bootstrap 能下载 kcs_const.js、main.js、version.json 到 `z/cache/` 对应路径
- `--force-update` 删除版本文件后能自动恢复
- 复用现有 `Kache` 基础设施的 CDN 配置和下载能力
- 不影响现有 bootstrap 流程（Codex 构建不变）

**Non-Goals:**
- 不改造 main-decoder 的输入处理（那是另一个 change）
- 不添加增量更新或 diff 能力
- 不下载全部 Web 资源（CSS、字体等），仅限解码器依赖的三个文件

## Decisions

### D1: 使用 Kache 下载而非扩展 RES_LIST

**选择**: 在 bootstrap CLI 层新增 `download_web_assets()` 函数，使用 `Kache::get_with_opt()` 下载。

**理由**: RES_LIST 使用固定 URL（GitHub raw 等），而 kcs_const.js/main.js 在 KanColle CDN 上需要 CDN 配置。Kache 已经封装了 CDN 选择、重试、代理等逻辑。在 bootstrap 阶段短暂初始化一个 Kache 实例用于下载。

**替代方案**: 在 RES_LIST 新增条目，硬编码 kcwiki GitHub cache 镜像 URL。被否决——URL 不稳定且可能过时，应直接从官方 CDN 获取。

### D2: 新增 CLI 标志 `--skip-web-assets`（默认下载）

**选择**: 默认行为包含下载 Web 资源。`--skip-web-assets` 跳过。

**理由**: 大多数用户需要这些文件。提供一个跳过选项给不需要解码器的场景（纯 Codex 构建）。

### D3: --force-update 流程调整

**选择**: `--force-update` 删除版本文件后，继续执行 Web 资源下载步骤，自动恢复。

**理由**: 当前 `--force-update` 只删除不恢复，违反最小惊讶原则。既然已加入下载流程，删除后重下是自然的。

### D4: 下载目标路径

**选择**: 直接写入 `z/cache/` 对应子目录，与 Kache 缓存结构一致。

- `z/cache/gadget_html5/js/kcs_const.js`
- `z/cache/kcs2/js/main.js`
- `z/cache/kcs2/version.json`

**理由**: main-decoder 默认路径（`../z/cache/...`）直接匹配，无需修改 TypeScript 侧。

## Risks / Trade-offs

- **CDN 配置依赖**: 用户必须配置 `gadgets_cdn` 和 `game_cdn` 才能下载。未配置时 skip 并 warn，不阻塞 bootstrap。→ 缓解：打印清晰的配置指引。
- **main.js 文件体积**: ~10MB，首次下载较慢。→ 缓解：已有文件则跳过（与 overwrite 语义一致）。
- **Kache 初始化开销**: bootstrap 阶段需要构建 Kache 实例。→ 可接受，开销极小。
