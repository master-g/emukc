# Plan 007: 区分「资源不存在」与「瞬时网络失败」

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_cache/src crates/emukc_bootstrap/src/make_list/source/kcs2/resources/gauge.rs`

## 状态

- **优先级**: P1
- **工作量**: M
- **风险**: MED
- **依赖**: 001
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

缓存层把「服务器说这个资源不存在（404）」和「网络/代理出问题，没问出结果」压成了
同一个错误值。两个后果：

1. **生成缓存清单时会静默丢条目**。`gauge.rs:122` 用
   `exists_on_remote(...).await.unwrap_or(false)` 判断某个阶段变体是否存在，
   `false` 就 `break` 停止这一系列的探测。代理抖一下，`Err` 被 `unwrap_or(false)`
   读成「不存在」，于是 `00506_2..9` 整串从清单里消失——没有报错，没有警告，
   要等玩家碰到缺失资源才会发现。
2. **无法只重试真正该重试的东西**。populate 的失败清单里，404 和握手失败混在一起
   （`populate.rs:194-196` 只能按 `InvalidFileVersion` 分流），第二轮重试会把
   确定不存在的资源再问一遍。

`Error::FileNotFound` 这个变体**已经存在**（`crates/emukc_cache/src/error.rs:11-12`），
只是 CDN 循环没有用它。

## 当前状态

`crates/emukc_cache/src/kache.rs:564-588`，404 与全失败返回同一个变体：

```rust
            match self.fetch_from_url(&url, path, local_path, version).await {
                Ok(f) => {
                    info!("🛬 {}", url);
                    return Ok(f);
                }
                Err(Error::Download(ref de))
                    if matches!(de.as_ref(), download::DownloadError::FileNotFound { .. }) =>
                {
                    warn!("🚫 404 on {}, skipping remaining CDNs", url);
                    return Err(Error::FailedOnAllCdn);
                }
                Err(e) => {
                    error!("💥 url:{}, err:{:?}", url, e);
                }
            }
        }

        error!("🚫 all cdn failed for {}", path);

        Err(Error::FailedOnAllCdn)
```

注意 404 分支返回的也是 `FailedOnAllCdn`——调用方无从区分。

`crates/emukc_cache/src/kache.rs:296-316`，`exists_on_remote` 的三态被压成两态
加一个错误：

```rust
                Ok(resp) => match resp.status() {
                    reqwest::StatusCode::OK => {
                        trace!("✅ {}", &url);
                        return Ok(true);
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        trace!("🚫 not found: {}", &url);
                        return Ok(false);
                    }
                    _ => {
                        trace!("💥 url:{}, status:{:?}", url, resp.status());
                    }
                },
                Err(e) => {
                    trace!("💥 url:{}, error:{:?}", url, e);
                }
            }
        }

        Err(Error::FailedOnAllCdn)
```

`crates/emukc_bootstrap/src/make_list/source/kcs2/resources/gauge.rs:114-127`，
把 `Err` 当成「不存在」的那一处：

```rust
    for i in 2..=9 {
        let variant_id = format!("{base}_{i}");
        // Probe with a quiet HEAD first: most bases have no further phase, and routing the
        // expected 404 through the full `get` path would log it at ERROR. `exists_on_remote`
        // reports a missing file at trace level, so the crawl stays silent.
        let json_path = format!("kcs2/resources/gauge/{variant_id}.json");
        if !cache.exists_on_remote(&json_path, NoVersion).await.unwrap_or(false) {
            break;
        }
        make_gauge_by_id(cache, &variant_id, list).await?;
    }
```

`crates/emukc_bootstrap/src/populate.rs:193-196`，只能按版本回退分流：

```rust
    let (skipped, retry_items): (Vec<_>, Vec<_>) = pass1_failures
        .into_iter()
        .partition(|f| matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)));
```

**仓库约定**：Rust edition 2024，软 tab 4 空格，`missing_docs` 为 warning。
`Error` 用 `thiserror`，照 `crates/emukc_cache/src/error.rs` 现有变体的写法加。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 缓存测试 | `cargo test -p emukc_cache` | 全部通过 |
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_cache/src/error.rs`
- `crates/emukc_cache/src/kache.rs`（`fetch_from_remote` 的 CDN 循环、
  `exists_on_remote`）
- `crates/emukc_bootstrap/src/make_list/source/kcs2/resources/gauge.rs`
- `crates/emukc_cache/tests/remote_fetch.rs`（更新计划 001 留下的断言）

**范围外**：

- `crates/emukc_bootstrap/src/populate.rs` 的分流逻辑——本计划**只提供**区分能力，
  用它来改重试策略是计划 009 的事。
- `is_valid` 的空文件/html 判定——计划 008。
- 负缓存（把 404 持久化到 redb）——**不做**。它需要 TTL 设计，否则以后真的上线的
  资源永远取不到。本计划只在单次运行内区分错误类型。
- `crates/emukc_network/src/download.rs` 的 `DownloadError::FileNotFound`——
  它已经正确区分了，不要动。

## Git 工作流

- 分支：`fix/distinguish-missing-from-failure`
- 提交信息：`fix(cache): tell a missing resource apart from a failed probe`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：让 CDN 循环返回不同的错误

改 `crates/emukc_cache/src/kache.rs` 的 `fetch_from_remote`：404 短路那一支改为
返回 `Err(Error::FileNotFound(path.to_string()))`（这个变体已存在），
全部 CDN 失败仍返回 `Err(Error::FailedOnAllCdn)`。

**验证**：`cargo build -p emukc_cache` → exit 0

### 步骤 2：给存在性探测一个三态返回

`exists_on_remote` 目前是 `Result<bool, Error>`，语义上是三态
（在 / 不在 / 没问出来）被塞进两层。改成显式三态，例如在
`crates/emukc_cache/src/lib.rs` 或 `kache.rs` 导出：

```rust
/// Outcome of probing whether a resource exists on the CDN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteExistence {
    /// The CDN answered 200.
    Present,
    /// The CDN answered 404.
    Absent,
    /// No CDN gave a conclusive answer (network error, 5xx, timeout).
    Indeterminate,
}
```

`exists_on_remote` 返回 `RemoteExistence`（不再需要 `Result` 包一层，
因为 `Indeterminate` 就表达了失败）。若有其它调用方，一并更新。
用 `grep -rn 'exists_on_remote' crates/ src/` 找全。

**验证**：`cargo build --workspace` → exit 0

### 步骤 3：修 gauge 的变体探测

`gauge.rs` 中把 `unwrap_or(false)` 换成对三态的显式处理：

- `Present` → 继续探测下一个变体
- `Absent` → `break`（这是唯一该停的情况）
- `Indeterminate` → **不能当作不存在**。两种可做法，选一种并在注释里说明理由：
  (a) 重试若干次后仍 `Indeterminate` 则让整个 make-list 返回错误——宁可失败也不要
  产出一份静默残缺的清单；
  (b) 记入 `CacheList` 的诊断侧带（`list.record_*` 系列方法，见
  `make_list/mod.rs`），让运行结束时能报告「有 N 个探测没有结论」。

推荐 (a)：清单是后续几小时下载的依据，悄悄少一截比失败更糟。

同时更新 `:116-120` 那段注释——它现在描述的是旧行为。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 4：更新计划 001 留下的断言

计划 001 在 `crates/emukc_cache/tests/remote_fetch.rs` 中对「所有 CDN 都返回 500」
的用例断言了当时的错误变体，并注明「计划 007 会更新」。现在：

- 404 用例 → 断言 `Error::FileNotFound`
- 全 500 用例 → 断言 `Error::FailedOnAllCdn`
- 删掉那些「计划 007 会更新」的注释

再新增一个用例：**一个 CDN 返回 500、下一个返回 200** → 断言最终成功
（证明瞬时失败不会提前终止 CDN 轮询）。

**验证**：`cargo test -p emukc_cache --test remote_fetch` → 全部通过

### 步骤 5：给 gauge 加一个针对性测试

在 `gauge.rs` 的测试模块（没有就新建 `#[cfg(test)] mod tests`）加一个用例：
mock 一个对变体探测返回 `Indeterminate` 的 `Kache`，断言 make-list **不会**
静默截断，而是按步骤 3 选定的方案报错或记录诊断。

如果构造 mock `Kache` 的成本过高（它需要 redb 和缓存目录），退而求其次：
把变体探测的决策逻辑抽成一个接受 `RemoteExistence` 的纯函数并测它。抽取时
**不要**顺手重构 `gauge.rs` 的其它部分。

**验证**：`cargo test -p emukc_bootstrap gauge` → 通过

### 步骤 6：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改范围内文件

## 测试计划

见步骤 4、5。覆盖点：404 → `FileNotFound`；全失败 → `FailedOnAllCdn`；
一个 CDN 失败但下一个成功 → 成功；gauge 探测遇 `Indeterminate` → 不静默截断。

## 完成标准

- [ ] `Error::FileNotFound` 在 404 路径上被实际返回（`grep` 确认
      `fetch_from_remote` 的 404 分支不再返回 `FailedOnAllCdn`）
- [ ] `exists_on_remote` 返回三态，且全工作区无 `unwrap_or(false)` 形式的调用
      （`grep -rn 'exists_on_remote' crates/ src/` 逐个确认）
- [ ] `gauge.rs` 的变体探测对 `Indeterminate` 有显式处理，且注释已更新
- [ ] 计划 001 中「计划 007 会更新」的注释已全部消除
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 001 未完成。
- 步骤 3 选了方案 (a) 之后，正常网络环境下 make-list **经常**失败——说明
  `Indeterminate` 比预期频繁，汇报频率，这会变成「要不要加重试」的设计决策。
- 发现 `exists_on_remote` 有本计划范围外的调用方，且改签名会波及大片代码——汇报
  调用点清单。
- 任一验证命令连续两次修复后仍失败。

## 维护须知

- 本计划**刻意不做负缓存**。把 404 写进 redb 能省掉每轮对不存在资源的重复探测，
  但没有 TTL 的负缓存会让以后真的上线的资源永远取不到。要做的话必须先定 TTL 策略。
- 评审重点：`gauge.rs` 那一处是本计划的实际价值所在。确认 `Indeterminate` 没有
  被任何形式的「当作 false」处理掉——包括 `unwrap_or`、`ok().unwrap_or_default()`
  和 `if let Ok(true)` 这类写法。
- 计划 009 会用到本计划提供的区分能力，把 404 从重试队列里排除。
