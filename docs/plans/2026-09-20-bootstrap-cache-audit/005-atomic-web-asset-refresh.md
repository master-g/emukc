# Plan 005: bootstrap web 资产改为先下后替，全 CDN 失败时硬报错

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- src/bin/cli/bootstrap.rs crates/emukc_bootstrap/src/download.rs`
> 有变化就先比对下文代码摘录。

## 状态

- **优先级**: P0
- **工作量**: S
- **风险**: LOW
- **依赖**: 无
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`bootstrap --force-update` 会**先无条件删除** `main.js`、`version.json`、
`kcs_const.js`，然后才去下载。而下载函数在所有 CDN 都失败时只打一行 `warn!` 就
返回 `Ok(())`，bootstrap 随后照常打印 "Bootstrap completed successfully." 并
exit 0。

于是 CDN 不可达时的净效果是：**删掉了用户原本能用的 `main.js`，什么也没换回来，
并报告成功**。`make update` 的下一步 `bun run decode` 直接死在「输入文件不存在」，
而那份 `main.js` 已经没了。

这不是假设。`PROJECT_MEMORY.md:46-47` 记着：「Verified on 6.3.2.1 and 6.3.5.0
(the latter with main.js fetched by hand instead of bootstrap)」——维护者已经
手工补过一次。

`main.js` 是整条解码链的唯一输入，丢了要重新联网取；在 CDN 或代理不稳的环境下
这可能要试很多次。

## 当前状态

`src/bin/cli/bootstrap.rs:53-73`，删除发生在下载**之前**：

```rust
    if args.force_update {
        let p = cfg.cache_root.join("gadget_html5").join("js").join("kcs_const.js");
        if p.exists() {
            std::fs::remove_file(&p)?;
        } else {
            warn!("{:?} not found.", p);
        }
        let p = cfg.cache_root.join("kcs2").join("version.json");
        if p.exists() {
            std::fs::remove_file(&p)?;
        } else {
            warn!("{:?} not found.", p);
        }
        let p = cfg.cache_root.join("kcs2").join("js").join("main.js");
        if p.exists() {
            std::fs::remove_file(&p)?;
        } else {
            warn!("{:?} not found.", p);
        }
        info!("version files in kcs cache removed.");
    }

    // Phase 4: Download web assets
    if !args.skip_web_assets {
        info!("Phase 4/4: Downloading web assets...");
        download_web_assets(
```

`crates/emukc_bootstrap/src/download.rs:394-401`，全 CDN 失败只 warn：

```rust
        if !downloaded {
            log_with_mp(&mp, || {
                warn!("All CDN sources failed for {}", asset.path);
            });
        }
    }

    Ok(())
```

`crates/emukc_bootstrap/src/download.rs:330-351`：当 `gadgets_cdn` 或 `game_cdn`
配置为空时，该资产被 `continue` 跳过（只 warn）。配合 `--force-update`，净效果同样是
「删了不补」。

`src/bin/cli/bootstrap.rs:88`：

```rust
    info!("Bootstrap completed successfully.");
```

无论前面丢了多少东西，这一行都会打印，函数返回 `Ok(())`。

三个资产及其 CDN 归属（`crates/emukc_bootstrap/src/download.rs:292-306`）：
`kcs_const.js` 走 `gadgets_cdn`，`main.js` 和 `version.json` 走 `game_cdn`——
**两组 CDN 相互独立，一组成功另一组失败是可能的**。

**仓库约定**：Rust edition 2024，软 tab 4 空格。错误类型见
`crates/emukc_bootstrap/src/download.rs` 的 `BootstrapDownloadError`（用 `thiserror`），
新增变体照它的写法。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 编译 | `cargo build --release` | exit 0 |
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| 二进制测试 | `cargo test --bin emukcd` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `src/bin/cli/bootstrap.rs`
- `crates/emukc_bootstrap/src/download.rs`（只改 `download_web_assets` 与
  `BootstrapDownloadError`）

**范围外**：

- `download_all`（第三方数据下载，Phase 1）——那条路径的失败语义是另一回事，
  不在本计划内。
- `crates/emukc_cache/` 的任何文件——游戏资源缓存的失败处理由计划 007/008 负责。
- `Makefile` 的 `update` 目标——由计划 010 一并整理文档时处理。
- Phase 1/2/3 的顺序——审计确认 Phase 2 的 codex 解析**不读** `main.js`
  （`parser/mod.rs:67-136` 只读第三方数据和仓库内嵌资产），所以当前的
  「先解析后换 main.js」顺序是无害的，不要动它。

## Git 工作流

- 分支：`fix/atomic-web-asset-refresh`
- 提交信息：`fix(bootstrap): replace web assets atomically and fail on total CDN failure`
- 正文说明：改前 `--force-update` 先删后下，全失败时仍 exit 0；改后先下到临时文件，
  成功才替换，全失败则返回错误。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：让全 CDN 失败成为错误

在 `crates/emukc_bootstrap/src/download.rs` 的 `BootstrapDownloadError` 上新增一个
变体，形如：

```rust
    /// All configured CDN sources failed for a required web asset.
    #[error("all CDN sources failed for web asset {path}")]
    WebAssetUnavailable {
        /// The asset path that could not be fetched.
        path: String,
    },
```

`download_web_assets` 的签名增加一个参数，用来表达「这次调用是否要求必须成功」，
例如 `require_all: bool`；或者更直接地，让它收集失败列表并在结尾返回错误。
选一种，**保持简单**，不要引入新的配置结构体。

行为：

- 某资产全部 CDN 失败 → 记录下来。
- CDN 列表为空（`cdns.is_empty()`）→ 同样记作失败，而不是静默 `continue`
  ——从调用方角度看，「没配 CDN」和「CDN 全挂」的后果完全一样。
- 函数结尾：若 `require_all` 为真且失败列表非空 → 返回
  `Err(WebAssetUnavailable { .. })`（多个失败时报第一个，并把完整列表 `error!` 出来）。

`missing_docs` 是 warning，新增的 pub 变体和字段要带文档注释。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 2：改成先下载到临时文件，成功后再替换

这是本计划的核心。`download_web_assets` 中每个资产的目标路径 `dest` 改为：

1. 下载到同目录下的临时文件，例如 `dest.with_extension("part")`（**必须同目录**，
   跨文件系统 rename 不是原子的）。
2. 下载成功后 `std::fs::rename(&tmp, &dest)` 替换。
3. 下载失败（该 CDN 这一轮）→ 删掉临时文件再试下一个 CDN。
4. 全部 CDN 失败 → 确保临时文件已清理，**原有的 `dest` 保持不动**。

注意 `download_web_assets` 现在已经在用 `.skip_header_check(true)`
（`download.rs:368`），保持不变。

**验证**：`cargo build -p emukc_bootstrap` → exit 0；
`cargo test -p emukc_bootstrap` → 全部通过

### 步骤 3：把 `--force-update` 的删除去掉

`src/bin/cli/bootstrap.rs` 中那段先删三个文件的代码（`:53-73`）**整段删除**。

它存在的理由是「让下载覆盖旧文件」，但 `download_web_assets` 的
`.overwrite(overwrite)` 已经能做到这件事，而步骤 2 的临时文件 + rename 让覆盖
变成原子的。删除只剩下破坏性。

`--force-update` 这个 flag **保留**（`bootstrap.rs:20-22` 的定义不要动），它现在
的语义变成「即使本地已有也重新下载」，也就是把 `true` 传给
`download_web_assets` 的 `overwrite` 参数。确认该参数确实是这么接的
（`bootstrap.rs:75-86` 当前传的是 `args.overwrite`——注意这里传的是
`overwrite` 而不是 `force_update`，**改动时要想清楚该传哪个**，并在提交信息
里说明；如果行为有变化，那是一处需要显式承认的语义调整）。

同时更新该 flag 的 `help` 文本（`bootstrap.rs:20`，当前是
"Remove main.js and version files from cache folder"），让它与新语义一致。

**验证**：`cargo build --release` → exit 0

### 步骤 4：让 bootstrap 在 Phase 4 失败时真的失败

`src/bin/cli/bootstrap.rs:75-86` 调用处，把 `download_web_assets` 的错误用 `?`
传播（现在已经是 `?`，但函数以前从不返回 Err，所以形同虚设）。确认
`info!("Bootstrap completed successfully.")` 只在全部阶段成功后才会执行。

**验证**：`cargo build --release` → exit 0

### 步骤 5：实测两种失败场景

**场景 A（CDN 不可达时不丢文件）**：

1. 确认 `z/cache/kcs2/js/main.js` 存在，记下它的大小和 md5：
   `ls -l z/cache/kcs2/js/main.js && md5 z/cache/kcs2/js/main.js`
2. 临时把 `emukc.config.toml` 的 `game_cdn` 改成一个不可达的主机名
   （例如 `["invalid.example.invalid"]`）。**改完记得还原**。
3. 跑 `cargo run --release -- bootstrap --overwrite --force-update`
4. **预期**：命令以**非零**退出码结束，且 `main.js` 的大小和 md5 **与步骤 1 相同**
   （没被删）。
5. 还原 `emukc.config.toml`。

**场景 B（正常路径不回归）**：

1. 跑 `cargo run --release -- bootstrap --overwrite --force-update`
2. **预期**：exit 0，三个 web 资产都刷新（mtime 变新），目录下**没有**残留的
   `.part` 文件：`find z/cache -name '*.part'` → 无输出。

如果本地没有可用的 CDN/代理环境，跳过场景 B 并在汇报中说明；场景 A **不能跳过**,
它才是本计划的核心。

**验证**：场景 A 非零退出且 `main.js` 未变；场景 B exit 0 且无 `.part` 残留

### 步骤 6：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改了范围内文件，且 `emukc.config.toml` 已还原（它不该进提交）

## 测试计划

新增单元测试，放在 `crates/emukc_bootstrap/src/download.rs` 的 `#[cfg(test)]` 模块：

- `download_web_assets` 在 CDN 列表为空时返回 `Err(WebAssetUnavailable)`
  （这个用例不需要网络，直接传空 slice 即可，是最容易验证的一条）。
- 临时文件命名与清理：构造一个必然失败的下载（不可达主机名），断言目标路径的
  原有文件**没有被修改**，且临时文件**不残留**。用 `tempfile` 建缓存根，
  `crates/emukc_bootstrap/Cargo.toml` 的 dev-dependencies 已经有 `tempfile = "3.27.0"`。

参照 `src/bin/cli/bootstrap.rs:92-` 现有的 `#[cfg(test)] mod tests` 的写法
（它用 `sample_config` / `sample_args` 构造输入）。

不要为这个计划引入 HTTP mock——两个用例都不需要成功的下载。

## 完成标准

- [ ] `src/bin/cli/bootstrap.rs` 中不再有删除 `main.js` / `version.json` /
      `kcs_const.js` 的代码
- [ ] `download_web_assets` 在有资产全部 CDN 失败（或 CDN 未配置）时返回 `Err`
- [ ] 场景 A 实测：CDN 不可达时 bootstrap 非零退出，且 `main.js` 的 md5 不变
- [ ] `find z/cache -name '*.part'` 无输出
- [ ] 新增的两个单元测试存在并通过
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] `emukc.config.toml` 未被提交（它在 `.gitignore` 里，确认 `git status` 干净）
- [ ] README.md 状态行已更新

## STOP 条件

- 步骤 3 中发现 `--force-update` 与 `overwrite` 的传参关系与描述不符，且改动会
  让「不带 `--force-update` 时是否重新下载」的行为发生变化——停下来汇报，
  这是需要操作者确认的语义问题。
- 场景 A 实测中 `main.js` **仍然被删**——说明还有别的删除点，汇报你找到的位置。
- `std::fs::rename` 在目标已存在时报错（某些平台/文件系统行为差异）——汇报，
  不要退回成「先删后 rename」，那等于把 bug 搬了个家。
- 有其它调用方依赖 `download_web_assets` 「永不返回 Err」的行为。

## 维护须知

- 这里的原子替换模式（同目录临时文件 + rename）在 `emukc_cache` 的下载路径上
  **还没有**应用。计划 008 处理那边的有效性判定，如果将来要给资源缓存也加原子
  替换，本计划的实现是现成的参照。
- 评审重点：确认临时文件与目标文件**同目录**（跨设备 rename 不原子），且失败路径
  上临时文件一定被清理。
- `download.rs:330-351` 原本把「CDN 未配置」当作可跳过的情况。本计划把它改成
  失败。如果将来真有「只想跑第三方数据、不碰 web 资产」的场景，正确做法是用已有的
  `--skip-web-assets` flag（`bootstrap.rs:26-28`），而不是把 CDN 配成空。
