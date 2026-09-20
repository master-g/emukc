# Plan 012: 为 cache-list 增加客户端版本校验

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/make_list main-decoder/src`

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: LOW
- **依赖**: 无
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`cache make-list` 生成的是接下来几小时下载工作的依据。它**从客户端拉取了实时版本号，
却从不拿它和自己正在使用的资产比对**。

具体地：make-list 在运行时远程拉取 `gadget_html5/js/kcs_const.js`，解析出
`scriptVesion`（注意上游把 version 拼错了），但这个值只被用来给清单里
`kcs2/js/main.js` 那一行打版本标签，然后丢弃。与此同时它正在用
`crates/emukc_bootstrap/assets/` 下的 decoder 资产生成路径——那些资产可能是几个
客户端版本之前解码出来的。

后果：资产过期时，新增的舰娘/装备立绘不会进入清单，populate 不会下载它们，
第一个碰到那艘船的玩家会拿到 404。整个过程没有任何警告。

而且这件事**本来就差一点点就能做**：9 个 decoder 资产里有 8 个都带
`scriptVersion` 字段，唯独 `resource_manifest.json` 没有——因为它的生成函数是
唯一一个不接收版本参数的。

## 当前状态

**实时版本是拿得到的**。`crates/emukc_bootstrap/src/make_list/source/kcs2/plain.rs:34-50`
强制远程拉取并解析：

```rust
// 该文件用 GetOption::new_remote_only() 拉 gadget_html5/js/kcs_const.js，
// 解析出 VersionInfo 的 scriptVesion，赋给 mainjs_ver，
// 随后只用于给 kcs2/js/main.js 这一行打版本标签（plain.rs:29-30）。
```

`crates/emukc_bootstrap/src/make_list/source/kcs2/versioned/mod.rs:53-62` 同样
远程拉取 `kcs2/version.json`，只用于给各资源类别打版本标签。

**唯一的资产版本检查**，`crates/emukc_bootstrap/src/make_list/manifest/loader.rs:198-209`：

```rust
    let manifest: ResourceManifest = serde_json::from_str(&raw).map_err(|e| {
        CacheListMakingError::Other(format!("Failed to parse resource manifest: {e}"))
    })?;

    if manifest.version != MANIFEST_VERSION {
        warn!(
            "Resource manifest version mismatch: expected {}, got {}. Proceeding anyway.",
            MANIFEST_VERSION, manifest.version
        );
    }
```

`manifest.version` 是**schema 版本**（当前是 2），不是客户端版本。而且不匹配也只是
"Proceeding anyway."。

**资产的版本字段现状**（实测）：

```
cache_rules.json:            "version": 1, "generatedAt": ..., "scriptVersion": "6.3.5.0"
resource_manifest.json:      "version": 2, "generatedAt": ...        ← 没有 scriptVersion
```

`crates/emukc_bootstrap/src/make_list/manifest/types.rs:783-796` 的结构体确实没有
这个字段：

```rust
pub(crate) struct ResourceManifest {
    /// Schema version.
    pub version: i64,
    /// ISO 8601 generation timestamp.
    pub generated_at: String,
    /// Summary statistics.
    #[serde(default)]
    pub summary: ResourceManifestSummary,
    /// Optional path rules for default/greedy cache list generation.
    #[serde(default)]
    pub path_rules: Option<PathRules>,
    /// Resource entries.
    pub entries: Vec<ResourceManifestEntry>,
}
```

**生成侧**，`main-decoder/src/resource-manifest.ts:453-455` 返回的对象里没有
`scriptVersion`：

```typescript
	return {
		version: 2,
		generatedAt: new Date().toISOString(),
		pathRules: buildPathRules(),
```

对比 `main-decoder/src/pipeline.ts:144-156`，其它资产都是
`toXxxAsset(loaded.scriptVersion, ...)` 这个形状，唯独
`extractResourceManifest(moduleGraph)` 不接收版本。

注意 `pipeline.ts:126` 的 sync 分支**又调了一次** `extractResourceManifest`，
与 `:143` 那次重复——改动时两处要一致。

**重要约束**：`crates/emukc_bootstrap/assets/*.json` 是禁止手改的生成产物，
只能通过 `cd main-decoder && bun run decode -- --sync-resource-manifest` 重新生成。
本计划改的是**生成器**，然后重新同步。

**仓库约定**：Rust edition 2024，软 tab 4 空格；TypeScript 侧 2 空格
（见 `.editorconfig`）。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 解码器检查 | `cd main-decoder && bun run check` | exit 0 |
| 解码器测试 | `cd main-decoder && bun test` | 全部通过 |
| 同步资产 | `cd main-decoder && bun run decode -- --sync-resource-manifest` | 生成成功 |
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `main-decoder/src/resource-manifest.ts`
- `main-decoder/src/pipeline.ts`（两处 `extractResourceManifest` 调用）
- `crates/emukc_bootstrap/src/make_list/manifest/types.rs`（加字段）
- `crates/emukc_bootstrap/src/make_list/manifest/loader.rs`（加校验）
- `crates/emukc_bootstrap/src/make_list/source/mod.rs`（把实时版本传给校验）
- `src/bin/cli/cache/make_list.rs`（加 `--allow-stale-assets` flag）
- `crates/emukc_bootstrap/assets/resource_manifest.json`（**通过 decoder 重新生成，
  不手改**）

**范围外**：

- `.sync-fingerprint.json` 与 `battle drift-check`——计划 013 处理。
- 把四份版本记录统一成一份——计划 013。
- `kcs2/version.json` 的逐子系统版本——它与 `scriptVesion` 是**两个独立的版本轴**
  （`PROJECT_MEMORY.md:57-59` 有记录：main.js-only 的发布只动前者）。本计划只校验
  `scriptVersion`，不要把两者混为一谈。
- 其它 8 个已经带 `scriptVersion` 的资产——它们已经有这个字段了，本计划只补上缺的
  那个，并把校验接上。

## Git 工作流

- 分支：`feat/cache-list-version-check`
- 提交拆两个：
  1. `feat(decoder): stamp scriptVersion onto the resource manifest`
     （含重新同步的 `resource_manifest.json`，正文说明这是 decoder 产物的重新生成）
  2. `feat(bootstrap): refuse to build a cache list from stale decoder assets`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：让 decoder 给 resource_manifest 打上版本

`main-decoder/src/resource-manifest.ts` 的 `extractResourceManifest` 增加一个
`scriptVersion` 参数，并把它写进返回对象，字段名与其它资产保持一致
（`scriptVersion`，放在 `generatedAt` 之后）。

更新 `main-decoder/src/pipeline.ts` 的**两处**调用（`:126` 和 `:143`），
都传 `loaded.scriptVersion`。顺便确认这两处能否合并成一次调用——`:126` 是重复
计算，但**如果合并要动到控制流就先不动**，本计划的目标是加字段。

**验证**：`cd main-decoder && bun run check` → exit 0；
`cd main-decoder && bun test` → 全部通过

### 步骤 2：重新生成资产

```
cd main-decoder && bun run decode -- --sync-resource-manifest
```

**验证**：`head -5 crates/emukc_bootstrap/assets/resource_manifest.json` 显示
`"scriptVersion"` 字段，其值与 `crates/emukc_bootstrap/assets/cache_rules.json`
中的一致

### 步骤 3：Rust 侧接收该字段

`crates/emukc_bootstrap/src/make_list/manifest/types.rs` 的 `ResourceManifest`
加字段：

```rust
    /// Client script version this manifest was decoded from.
    #[serde(default)]
    pub script_version: Option<String>,
```

用 `Option` + `#[serde(default)]`，这样旧的 manifest 文件仍然能加载
（结构体已有 `#[serde(rename_all = "camelCase")]`，字段名会自动映射到
`scriptVersion`）。

**验证**：`cargo test -p emukc_bootstrap` → 全部通过

### 步骤 4：把实时版本接到校验上

在 `crates/emukc_bootstrap/src/make_list/source/mod.rs` 生成流程的开头，
把已经拿到的实时 `scriptVesion`（来自 `plain.rs` 解析 `kcs_const.js` 的那个值）
与资产里的 `scriptVersion` 比对。

要比对的资产：`cache_rules.json` 的 `scriptVersion`（`Default`/`Rules` 路径用）
和 `resource_manifest.json` 的 `scriptVersion`（`Manifest` 路径用）。

不一致时的行为：**返回错误，拒绝生成清单**，错误信息要同时给出两个版本号和
修复命令（`make update` 或 `cd main-decoder && bun run decode -- --sync-assets`）。

注意实时版本的获取时机：`plain.rs` 当前是在生成过程中才拉取的。如果把校验放到
开头需要提前拉取，**不要为此重构整个流程**——放在能拿到该值的最早位置即可，
哪怕是在部分路径已经生成之后。清单只有在写文件时才落盘，中途报错不会留下
半份清单（确认 `make_list/mod.rs` 的写文件时机确实如此）。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 5：留一个逃生口

给 `src/bin/cli/cache/make_list.rs` 加 flag：

```rust
    #[arg(help = "Build the list even when decoder assets are stale")]
    #[arg(long)]
    pub allow_stale_assets: bool,
```

传下去，为真时把步骤 4 的错误降级成 `warn!`。

这个逃生口是必要的：离线环境、或者上游刚发版而解码还没跟上时，用户仍然需要能
生成一份清单。但默认必须是拒绝。

**验证**：`cargo run --release -- cache make-list --help` → 显示
`--allow-stale-assets`

### 步骤 6：两种情况都实测

**情况 A（版本一致，正常生成）**：

```
cargo run --release -- cache make-list --output /tmp/cachelist-check.nedb --overwrite
```

**预期**：exit 0，正常生成。

**情况 B（人为制造过期）**：把本地 `crates/emukc_bootstrap/assets/cache_rules.json`
的 `scriptVersion` **临时**改成一个假版本（例如 `"0.0.0.0"`），重跑：

```
cargo run --release -- cache make-list --output /tmp/cachelist-check2.nedb --overwrite
```

**预期**：非零退出，错误信息里同时出现假版本和实时版本，且**没有**生成
`/tmp/cachelist-check2.nedb`。

再加 `--allow-stale-assets` 重跑，**预期**：exit 0 且打印警告。

**改完必须还原**：`git checkout crates/emukc_bootstrap/assets/cache_rules.json`
——该文件是生成产物，不能带着假版本提交。

**验证**：情况 B 按预期拒绝；`git status` 中 `cache_rules.json` 干净

### 步骤 7：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cd main-decoder && bun run check && bun test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 改动文件在范围内；`resource_manifest.json` 的改动是 decoder
  重新生成的结果（提交信息里要说明）

## 测试计划

新增：

- `crates/emukc_bootstrap/src/make_list/manifest/loader.rs` 的测试模块中，
  加载一个**没有** `scriptVersion` 字段的 manifest JSON → 断言能成功加载且
  `script_version` 为 `None`（向后兼容）。
- 版本比对函数（抽成一个接受两个 `Option<&str>` 的纯函数）的表驱动测试：
  相同 → 通过；不同 → 拒绝；资产侧为 `None` → 按选定策略（建议：`warn!` 但放行，
  因为老资产没有这个字段）。
- `main-decoder` 侧：在 `main-decoder/test/` 加一个断言
  `extractResourceManifest` 输出含 `scriptVersion` 的测试，照该目录现有测试的写法。

## 完成标准

- [ ] `crates/emukc_bootstrap/assets/resource_manifest.json` 含 `scriptVersion`，
      值与 `cache_rules.json` 一致
- [ ] `resource_manifest.json` 的改动是 `bun run decode` 生成的，**不是手改的**
- [ ] 版本不一致时 `cache make-list` 非零退出且不产出清单文件
- [ ] `--allow-stale-assets` 能绕过，并打印警告
- [ ] 旧格式（无 `scriptVersion`）的 manifest 仍能加载
- [ ] `cargo test` 与 `cd main-decoder && bun test` 都通过
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] `git status` 中没有手改的 assets 文件残留
- [ ] README.md 状态行已更新

## STOP 条件

- `bun run decode` 跑不起来（缺 Bun 依赖，或 `z/cache/kcs2/js/main.js` 不存在）——
  汇报。**特别注意**：不要用 `bootstrap --force-update` 去取 `main.js`，
  在 CDN 不可达时它会删掉现有的那份（这正是计划 005 要修的）。
- 步骤 4 中发现实时版本在生成流程里拿不到，除非大改流程——汇报，
  不要为了加校验而重构生成流程。
- 步骤 6 情况 A 就失败（版本本来就不一致）——那说明当前仓库里的资产确实过期了，
  这是真实发现，汇报两个版本号。
- 比对时发现 `scriptVesion`（上游拼写）在两处被两个不同的正则解析
  （`main-decoder/src/io.ts` 和 `make_list/source/kcs2/plain.rs`）导致结果不同——
  汇报，这属于计划 013 的范畴。

## 维护须知

- 本计划只校验 `scriptVersion` 这一个轴。`kcs2/version.json` 里的逐子系统版本是
  **另一个独立的轴**，两者会各自移动（`PROJECT_MEMORY.md:57-59`）。不要把它们
  合并校验。
- 同一个上游字段 `scriptVesion` 目前被 TypeScript 和 Rust 各用一个正则解析。
  两边哪天不一致，本计划的校验会开始误报。计划 013 要处理这件事。
- 评审重点：`--allow-stale-assets` 的默认值必须是 `false`。一个默认放行的校验
  等于没有校验。
- `resource_manifest.json` 与 `cache_rules.json` 里各有一份 resource manifest
  （后者嵌套了前者，`types.rs:807`），两份都是 decoder 产出。本计划给独立那份补上
  版本字段；嵌套那份通过 `cache_rules.scriptVersion` 已经有了。要不要合并成一份
  是独立议题。
