# Plan 001: 为 emukc_cache 的写入、过期与失败路径建立 mock CDN 测试基线

> **执行者须知**：逐步执行本计划，每一步都要运行其验证命令并确认预期结果后再进入
> 下一步。出现「STOP 条件」中任何一条，立即停止并汇报，不要自行发挥。完成后更新
> `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 中本计划的状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_cache crates/emukc_network`
> 如果有任何范围内文件发生变化，先把下文「当前状态」中的代码摘录与实际代码比对，
> 不一致就按 STOP 条件处理。

## 状态

- **优先级**: P1
- **工作量**: M
- **风险**: LOW
- **依赖**: 无
- **类别**: tests
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`emukc_cache` 负责把 94,558 个游戏资源下载到本地并判断缓存是否有效，但它的写入
路径、有效性判定、过期判定和失败处理**全部没有测试覆盖**。现有 4 个集成测试文件
里没有一个发起过网络请求，其中 `version_management.rs` 名为 `test_version_expired`
的测试甚至根本没有触发过期逻辑——把版本比较函数整个删掉它照样通过。

计划 003、007、008 都要改这些路径。没有基线，那三个改动无法验证，也无法防止回归。
本计划只加测试，不改任何生产代码。

## 当前状态

**测试现状**（`crates/emukc_cache/tests/`）：

- `basic_operations.rs` — 只覆盖 builder 成功/失败与本地命中/未命中
- `get_options.rs` — 5 个测试只断言 flag 传递，全部带 `disable_remote`
- `cdn_operations.rs` — 只覆盖 CDN **配置**，没有任何一个测试发起请求
- `version_management.rs` — 6 个测试全是本地路径

`version_management.rs:31-46` 的 `test_version_expired` 写入一个文件，用空字符串
`""` 作为 version 调用 `get` 两次，两次都断言 `is_ok()`。它不验证任何过期行为。

**未覆盖的关键函数**（本计划要覆盖的目标）：

`crates/emukc_cache/src/kache.rs:596-635` 的 `is_valid`：

```rust
    async fn is_valid(path: &std::path::Path) -> bool {
        if !path.exists() || !path.is_file() {
            trace!("File does not exist or is not a file: {:?}", path);
            return false;
        }

        // HTML files are always valid
        if path.extension().is_some_and(|ext| ext == "html") {
            trace!("File is a HTML file: {:?}", path);
            return true;
        }
        // ...
        // Empty files are valid
        if metadata.len() == 0 {
            return true;
        }
        // ...
        // Check if content looks like HTML error page
        let content = String::from_utf8_lossy(&buffer);
        !content.contains("<!DOCTYPE html>") && !content.contains("<html")
    }
```

`crates/emukc_cache/src/kache.rs:280-317` 的 `exists_on_remote`，其 CDN 循环结尾为：

```rust
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

`crates/emukc_cache/src/kache.rs:507-530` 的 `fetch_from_url`：下载 → `is_valid`
→ `set_version` → 打开文件返回。`is_valid` 失败时返回 `Err(Error::InvalidFile)`
且**不删除**已落盘的坏文件。

**一个必须一并处理的问题**：`crates/emukc_network/src/client.rs:39-42` 存在一个
发起真实网络请求并 `.unwrap()` 的单元测试：

```rust
    #[tokio::test]
    async fn test_new_reqwest_client() {
        let client = new_reqwest_client(None, None).unwrap();
        client.get("http://w00g.kancolle-server.com/kcs2/world.html").send().await.unwrap();
    }
```

它使得离线或代理不可用时 `cargo test` 必然失败，违背「一条命令就能知道代码是否正常」
的前提。本计划把它改成不依赖外网。

**仓库约定**：

- Rust edition 2024，软 tab 4 空格（`.rustfmt.toml`、`.editorconfig`）。
- `unsafe_code` 工作区禁用，`missing_docs` 为 warning。
- 集成测试放 `crates/emukc_cache/tests/`，每个文件一个主题。参照
  `crates/emukc_cache/tests/basic_operations.rs` 的结构编写新文件。
- `crates/emukc_cache/Cargo.toml` 的 `[dev-dependencies]` 现有
  `criterion`、`tempfile = "3.27.0"`、`tokio-test = "0.4"`。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 编译 | `cargo build -p emukc_cache` | exit 0 |
| 本 crate 测试 | `cargo test -p emukc_cache` | 全部通过 |
| 网络层测试 | `cargo test -p emukc_network` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0，无任何 warning |

## 范围

**范围内**（只允许改这些文件）：

- `crates/emukc_cache/Cargo.toml`（只加 dev-dependency）
- `crates/emukc_cache/tests/remote_fetch.rs`（新建）
- `crates/emukc_cache/tests/validity.rs`（新建）
- `crates/emukc_cache/tests/version_expiry.rs`（新建）
- `crates/emukc_network/src/client.rs`（**仅**改 `#[cfg(test)] mod tests` 内部）

**范围外**（不要碰，即使看起来相关）：

- `crates/emukc_cache/src/kache.rs` 的任何生产代码——本计划的全部价值在于先如实
  记录当前行为。其中几条行为是已知缺陷（空文件被判定为有效、`.html` 跳过校验），
  由计划 008 修正。**本计划要把当前行为如实写成断言，并在断言旁标注
  「计划 008 会翻转此断言」**，不要顺手修。
- `crates/emukc_cache/src/ver.rs`、`version_cache.rs`、`download_lock.rs`
- `crates/emukc_network/src/download.rs` 的生产代码
- 现有 4 个测试文件的内容（不删不改，新测试写在新文件里）

## Git 工作流

- 分支：`test/cache-verification-baseline`
- 提交信息用 Conventional Commits，英文，不带任何 AI 署名。
  参考 `git log` 中的既有风格，例如 `test(port): pin the event-object capability flags`。
  本计划建议：`test(cache): cover remote fetch, validity and expiry paths`
- 不要 push，不要开 PR，除非操作者明确要求。

## 步骤

### 步骤 1：引入一个本地 mock HTTP 服务端

在 `crates/emukc_cache/Cargo.toml` 的 `[dev-dependencies]` 增加 `wiremock`：

```toml
wiremock = "0.6"
```

如果 `cargo add` 解析出的最新 0.6.x 与 workspace 的 tokio/http 版本冲突，改用第二
方案：用 `tokio::net::TcpListener` 手写一个最小的一次性 HTTP 响应器，不引入新依赖。
两种方案都可以，选能跑通的那个，并在提交信息里说明选了哪个。

**验证**：`cargo build -p emukc_cache --tests` → exit 0

### 步骤 2：覆盖远端抓取的三种结局

新建 `crates/emukc_cache/tests/remote_fetch.rs`，用 mock 服务端作为 CDN
（`Kache` 的 CDN 列表接受任意主机名，把 mock 的 `127.0.0.1:<port>` 配进去），
覆盖以下用例，每个用例独立一个 `#[tokio::test]`：

1. **200 带正常 body** → 文件落盘，内容与 body 一致，redb 中有对应 version 行。
2. **200 但 body 为空** → 当前行为是「文件落盘且被判定为有效、写入 version 行」。
   如实断言，并在上方加注释：
   `// 计划 008 会把空文件改判为无效，届时此断言翻转`。
3. **404** → 返回 `Err`，磁盘上不留文件，redb 中不写 version 行。
4. **所有 CDN 都返回 500** → 返回 `Err`。断言当前 `Err` 的具体变体，并加注释：
   `// 计划 007 会把「全部失败」与「404」分成不同变体，届时此断言需更新`。

**验证**：`cargo test -p emukc_cache --test remote_fetch` → 4 个测试全部通过

### 步骤 3：覆盖 `is_valid` 的每个分支

新建 `crates/emukc_cache/tests/validity.rs`。`is_valid` 是私有函数，通过 `get`
的可观测行为间接覆盖：预先在缓存目录放置文件，再用 `disable_remote` 的
`GetOption` 调用 `get`，断言成功或失败。

覆盖：普通非空文件（有效）、零长度文件（**当前**有效）、`.html` 扩展名且内容是
HTML 错误页（**当前**有效，跳过校验）、非 `.html` 扩展名但内容以 `<!DOCTYPE html>`
开头（无效）、路径不存在（无效）。

后两类「当前有效」的用例同样加注释标明计划 008 会翻转。

**验证**：`cargo test -p emukc_cache --test validity` → 全部通过

### 步骤 4：写一个真正会失败的过期测试

新建 `crates/emukc_cache/tests/version_expiry.rs`。与现有那个空壳测试不同，这个
必须能在过期逻辑被破坏时失败：

1. 用 mock CDN 以 version `"1"` 获取某路径，断言落盘且 redb 记录版本 `1`。
2. 让 mock 返回不同的 body，用 version `"2"` 再次 `get` 同一路径，断言**发生了
   重新下载**（通过 mock 的请求计数断言，而不是只断言 `is_ok()`），且本地内容更新为
   新 body。
3. 再用 version `"2"` 获取一次，断言**没有**发生新的请求（命中本地缓存）。
4. 用一个更低的版本 `"1"` 获取，断言当前行为（参照 `kache.rs:656` 那个版本回退
   回归测试的预期），不要臆测。

**验证**：`cargo test -p emukc_cache --test version_expiry` → 全部通过。
另外做一次**变异检验**：临时把 `crates/emukc_cache/src/ver.rs` 的
`ver_str_cmp` 改成永远返回 `std::cmp::Ordering::Equal`，重跑该测试文件，**必须有
测试失败**；确认后把改动还原（`git checkout crates/emukc_cache/src/ver.rs`）。
如果没有任何测试失败，说明测试没有真正覆盖过期逻辑，回到本步重写。

### 步骤 5：去掉打真实网络的单元测试

把 `crates/emukc_network/src/client.rs:39-42` 的 `test_new_reqwest_client` 改成
不依赖外网：断言 `new_reqwest_client(None, None)` 返回 `Ok` 即可，删掉那次
`client.get(...).send().await.unwrap()`。同文件内另外两个测试
（`test_new_reqwest_client_accepts_http_proxy` / `..._socks5_proxy`）只构造
client、不发请求，保持原样。

**验证**：先断网或临时停掉代理，再跑 `cargo test -p emukc_network` → 全部通过

### 步骤 6：全量门禁

**验证**：
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0，无 warning
- `cargo test -p emukc_cache` → 全部通过，且新增测试数 ≥ 12
- `cargo test -p emukc_network` → 全部通过

## 测试计划

本计划的产出**本身就是测试**。新增文件与覆盖点见步骤 2-4。结构上参照
`crates/emukc_cache/tests/basic_operations.rs`（同样的 `tempfile` 建临时缓存目录、
同样的 `Kache` 构造方式）。

不要新增任何 mock 框架之外的测试设施，不要给本计划范围外的函数补测试。

## 完成标准

全部满足才算完成：

- [ ] `cargo test -p emukc_cache` exit 0，且 `remote_fetch.rs`、`validity.rs`、
      `version_expiry.rs` 三个新文件存在并全部通过
- [ ] 步骤 4 的变异检验做过：把 `ver_str_cmp` 改成恒等后**确实有测试失败**，
      且改动已还原（`git status` 中 `ver.rs` 干净）
- [ ] `cargo test -p emukc_network` 在**断网或代理不可用**时仍然通过
- [ ] `cargo fmt --all --check` exit 0
- [ ] `cargo clippy --workspace -- -W warnings` exit 0
- [ ] `git status` 显示改动文件全部在「范围内」清单里，没有 `kache.rs` 等生产代码
- [ ] README.md 中本计划状态行已更新

## STOP 条件

出现以下任一情况，停止并汇报：

- 「当前状态」中的代码摘录与实际代码不符（说明基线已漂移）。
- `wiremock` 与 workspace 现有 tokio/http 版本冲突，且手写 TCP mock 也无法在
  2 小时内跑通——汇报冲突详情，不要为了绕开而去改生产代码。
- 发现某个测试用例要通过就必须改 `kache.rs` 的生产代码——那说明该用例属于计划
  007 或 008，把它记下来汇报，不要在本计划里改。
- 步骤 4 的变异检验做不出失败，且重写两次仍然如此。
- 任何一步的验证命令连续两次修复后仍失败。

## 维护须知

- 计划 007 与 008 会**故意翻转**本计划中几条带注释的断言（空文件有效性、
  `.html` 跳过校验、`FailedOnAllCdn` 的错误分型）。那些注释就是给它们的路标，
  不要在无关改动中清理掉。
- 评审时重点看：过期测试是不是用 mock 的请求计数断言「是否重新下载」，而不是
  只断言 `is_ok()`——后者正是现有那个空壳测试的毛病。
- 本计划**刻意不修**任何已知缺陷。如果评审觉得「顺手改了算了」，请拒绝：先有
  记录当前行为的基线，翻转才是可验证的。
