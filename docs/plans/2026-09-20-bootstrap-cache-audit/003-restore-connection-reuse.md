# Plan 003: 恢复 HTTP 连接池复用并去掉每文件多余的 HEAD 请求

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_network/src/client.rs crates/emukc_cache/src/kache.rs`
> 有变化就先比对下文代码摘录。

## 状态

- **优先级**: P0
- **工作量**: S
- **风险**: LOW
- **依赖**: 001（需要 mock CDN 测试基线来验证行为不变）
- **类别**: perf
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

填充完整缓存要下载 94,558 个文件。当前两处配置叠加，使这件事的成本翻倍：

1. `crates/emukc_network/src/client.rs:21` 的 `.pool_max_idle_per_host(0)` 把
   reqwest 的连接池**完全关闭**——不保留任何空闲连接，因此没有一个连接会被复用。
2. `crates/emukc_cache/src/kache.rs:514-518` 构造下载请求时没有设
   `.skip_header_check(true)`，而该字段默认 `false`，于是每个文件先发一次 HEAD
   再发一次 GET。

合起来：一轮完整填充约 18.9 万次 TCP + TLS 握手（经 SOCKS5 代理时还要加一次
CONNECT）。实测吞吐约 7 个文件/秒（≈143 ms/文件），与「受握手限制而非受带宽限制」
吻合。运行中观察到的 `tls handshake eof` 错误正发生在握手阶段——而握手之所以存在，
就是因为没有连接可复用。

`git log` 中查不到关闭连接池的任何理由记录，代码里也没有注释说明。

## 当前状态

`crates/emukc_network/src/client.rs:19-22`：

```rust
    let builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(0)
        .user_agent(ua.unwrap_or(DEFAULT_UA));
```

同一个 client 在 `crates/emukc_cache/src/kache.rs:145` 构造一次，之后克隆给每次
下载使用（`kache.rs:519`），所以这个设置作用于全部资源抓取。

`crates/emukc_cache/src/kache.rs:507-521`：

```rust
    async fn fetch_from_url(
        &self,
        url: &str,
        rel_path: &str,
        local_path: &PathBuf,
        version: &str,
    ) -> Result<tokio::fs::File, Error> {
        download::Request::builder()
            .url(url)
            .save_as(local_path)
            .overwrite(true)
            .build()?
            .execute(Some(self.client.clone()))
            .await?;
```

注意这里没有 `.skip_header_check(true)`。

`crates/emukc_network/src/download.rs:28-30` 的字段定义与 `:38` 的 builder 默认值：

```rust
    /// Skip header check
    pub skip_header_check: bool,
```

`crates/emukc_network/src/download.rs:247` 起的 HEAD 分支：

```rust
        if !self.skip_header_check {
            trace!("checking if the file exists via a HEAD request");
            // check if the file exists via a HEAD request
            let head =
                client.head(&self.url).send().await.map_err(|source| DownloadError::Reqwest {
                    phase: DownloadPhase::SendHead,
```

HEAD 分支在 `:261-274` 对非 2xx 和 404 的处理，与 GET 分支在 `:297-315` 的处理
**是重复的**——GET 自己就能识别 404 并在读 body 前返回。所以 HEAD 并没有提供
GET 不具备的信息。

**仓库约定**：Rust edition 2024，软 tab 4 空格，`unsafe_code` 禁用，
`missing_docs` 为 warning（改动涉及 pub 项时要带文档注释）。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 编译 | `cargo build --release` | exit 0 |
| 缓存测试 | `cargo test -p emukc_cache` | 全部通过（含 001 新增的） |
| 网络测试 | `cargo test -p emukc_network` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_network/src/client.rs`（改 builder 配置）
- `crates/emukc_cache/src/kache.rs`（只改 `fetch_from_url` 里的 builder 链）

**范围外**：

- `crates/emukc_network/src/download.rs` 的 HEAD 分支本身——**不要删除它**。
  它是公开 API 的一部分（`skip_header_check` 是 `pub` 字段），别的调用方可能需要。
  本计划只改 kache 的调用方式。
- `danger_accept_invalid_certs(true)`（`client.rs:20`）——它没有记录在案的理由，
  值得单独讨论，但改它会影响全进程所有 HTTPS 调用，风险与本计划不是一个量级。
  **不要顺手改**，在提交信息或 PR 里提一句即可。
- `crates/emukc_bootstrap/src/populate.rs` 的并发结构

## Git 工作流

- 分支：`perf/restore-connection-reuse`
- 提交信息：`perf(cache): reuse connections and skip the redundant HEAD probe`
- 正文里写明：改前每文件 2 次连接、连接池禁用；改后 1 次请求、连接复用。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：先量出基线

改代码**之前**，先用当前二进制测一个可对比的数字，否则无法证明改动有效。

准备一个 200 条的小清单（从现有缓存清单里截取，写到临时目录，不要污染工作树）：

```
head -200 z/cache/cache_resources.nedb > /tmp/populate-bench-200.nedb
```

跑一次并计时（用一个**干净的**缓存根，避免命中已有文件；通过临时改 `emukc.config.toml`
的 `cache_root` 指向临时目录，或用 `--src` 配合一个空的缓存目录）：

```
time cargo run --release -- cache populate --src /tmp/populate-bench-200.nedb --concurrent 16
```

记下耗时与失败数。**这个数字要写进提交信息。**

**验证**：命令跑完，得到一个基线耗时

### 步骤 2：开启连接池复用

改 `crates/emukc_network/src/client.rs`：把 `.pool_max_idle_per_host(0)` 换成一个
与并发度相称的值，并补上空闲超时。目标形态：

```rust
    let builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .pool_max_idle_per_host(16)
        .pool_idle_timeout(std::time::Duration::from_secs(90))
        .user_agent(ua.unwrap_or(DEFAULT_UA));
```

同时补一条注释，说明这个值与 populate 的默认并发度对应——下一个读到这段代码的人
不该再对着一个裸数字猜。

**验证**：`cargo build --release` → exit 0；`cargo test -p emukc_network` → 全部通过

### 步骤 3：去掉每文件多余的 HEAD

改 `crates/emukc_cache/src/kache.rs` 的 `fetch_from_url`，在 builder 链上加一行：

```rust
        download::Request::builder()
            .url(url)
            .save_as(local_path)
            .overwrite(true)
            .skip_header_check(true)
            .build()?
```

**在这一行上方加注释**，说明 GET 分支自己处理 404（指向
`crates/emukc_network/src/download.rs:299-307`），HEAD 是纯粹的重复往返。

**验证**：`cargo test -p emukc_cache` → 全部通过，**特别是计划 001 新增的
`remote_fetch.rs` 中的 404 用例必须仍然通过**——它证明去掉 HEAD 之后 404 依然被
正确识别、不留文件、不写 version 行。

### 步骤 4：用同一份清单复测

用与步骤 1 **完全相同**的清单和完全相同的空缓存目录，重跑：

```
time cargo run --release -- cache populate --src /tmp/populate-bench-200.nedb --concurrent 16
```

记下耗时与失败数。

**预期**：耗时显著下降（握手数从 400 降到 200 且可复用）。如果**没有**下降甚至
变慢，不要强行上线——按 STOP 条件汇报实测数字。

**验证**：新耗时 < 基线耗时；失败数 ≤ 基线失败数

### 步骤 5：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只有 `client.rs` 和 `kache.rs` 两个文件被修改

## 测试计划

本计划**不新增测试**——计划 001 已经覆盖了受影响的行为（200 正常 body、200 空
body、404、全 CDN 失败），本计划的正确性标准就是那些测试**继续通过**。

这是有意的：改动是两处配置，新行为没有新分支。为配置改动单独造测试属于测试设施
膨胀。

如果 001 尚未完成，本计划的步骤 3 无法验证——那种情况下按 STOP 条件处理。

## 完成标准

- [ ] `crates/emukc_network/src/client.rs` 不再含 `pool_max_idle_per_host(0)`
- [ ] `crates/emukc_cache/src/kache.rs` 的 `fetch_from_url` 含 `.skip_header_check(true)`
- [ ] `cargo test -p emukc_cache` exit 0，其中 404 用例通过
- [ ] `cargo test` exit 0
- [ ] 步骤 1 与步骤 4 的两个实测耗时都已记录，且改后 < 改前
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] `git status` 只显示两个文件被改
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 001 尚未完成（`crates/emukc_cache/tests/remote_fetch.rs` 不存在）——
  没有基线就无法证明这次改动没破坏 404 处理。先做 001。
- 步骤 4 实测**没有**变快，或失败数**上升**：汇报两次的实测数字。有可能该代理
  对 keep-alive 处理有问题（这正是当初关掉连接池的可能原因），那属于需要操作者
  决策的事，不要自行把值调来调去试。
- 去掉 HEAD 之后 001 的 404 用例失败——说明 GET 分支对 404 的处理与本计划描述
  不符，停止并汇报。
- 发现有其它调用方依赖「kache 会先发 HEAD」这个行为。

## 维护须知

- `pool_max_idle_per_host(16)` 与 populate 的默认并发度（`Makefile:13` 的
  `CONCURRENT ?= 16`）是配套的。如果以后默认并发度改了，这里要一起看。
- 评审重点：确认 `skip_header_check(true)` 只加在 kache 的抓取路径上，没有顺手
  改 `download.rs` 里 HEAD 分支本身的逻辑。
- 明确推迟、且**不属于**本计划：`danger_accept_invalid_certs(true)` 关闭了全进程
  的证书校验且无任何记录在案的理由。它值得一个独立决策，但和本计划的性能问题
  没有关系。
- 后续可叠加的优化（本计划不做）：`populate.rs` 的 16 路并发全部跑在**同一个
  task** 上（`FuturesUnordered` 而非 `tokio::spawn`），其中还夹着阻塞的
  `std::fs` 调用，因此任何一个 future 写文件时其余 15 个不前进。这会限制提高
  `--concurrent` 的收益，但在握手成本消除之前它不是瓶颈。
