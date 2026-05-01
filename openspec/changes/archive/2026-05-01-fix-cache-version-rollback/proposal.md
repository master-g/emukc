## Why

`cache make-list` 生成的 cache list 中存在 version rollback：某些资源的 version 比已缓存的 version 更旧（如 `bg_map/bg_y.png` 生成 6.2.0.0 但缓存已有 6.2.9.0）。这导致 `populate` 阶段请求被拒绝 (`InvalidFileVersion`)，资源无法更新。

**精确根因**: `versioned/mod.rs` 使用 `GetOption::new_non_mod().get(cache, "kcs2/version.json", NoVersion)` 获取 version.json。`NoVersion` 导致 `find_in_local` 跳过版本检查，直接返回本地缓存的旧 version.json。make-list 用过时的 category→version 映射生成 list，造成 rollback。

## What Changes

- **修复 version.json 获取逻辑**: 将 `NoVersion` 改为 force remote 获取，确保 make-list 始终使用最新 version.json
- **在 make-list 中添加 version monotonicity 检查**: 需要先暴露 kache 的 version 查询 API（如 `get_cached_version(path)`），然后对每个 entry 检测 rollback
- **改进 populate 对 `InvalidFileVersion` 的处理**: 当前 populate 会重试 `InvalidFileVersion`（无意义），应改为跳过
- **处理 category 缺失 edge case**: `versions.get(category)` 返回 None 时应有明确行为

## Capabilities

### New Capabilities

- `kache.get_cached_version(path)` — 公共 API，返回已缓存资源的 version（供 make-list 和其他模块使用）

### Modified Capabilities

- `cache-make-list-versioning`: make-list 的 version 分配逻辑，确保 version 单调递增或至少不回退
- `cache-populate-error-handling`: populate 阶段区分可重试错误与不可重试错误（InvalidFileVersion）

## Impact

- `crates/emukc_bootstrap/src/make_list/source/kcs2/versioned/mod.rs` — version.json 获取方式（NoVersion → force remote）
- `crates/emukc_bootstrap/src/make_list/source/kcs2/versioned/img.rs` — category 缺失处理、rollback 检测
- `crates/emukc_cache/src/kache.rs` — 暴露 `get_cached_version` 公共方法
- `crates/emukc_cache/src/ver.rs` — IntoVersion trait（无变更，仅引用）
- `crates/emukc_bootstrap/src/populate.rs` — `InvalidFileVersion` 跳过而非重试

## Non-goals

- 不修改 redb 存储引擎
- 不修改 HTTP cache proxy 行为
- 不改变 version 的四段式格式（a.b.c.d）

## Relationship to Recent Changes

- `32aa4e1` (harden cache list generation with decoder-driven IDs and filters) 加强了 make-list 的 ID 和过滤逻辑，但未触及 version.json 获取方式。本 change 与之互补，不冲突。
