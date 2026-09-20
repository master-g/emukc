# Plan 009: populate 失败清单落盘，支持只重试失败项

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/populate.rs crates/emukc_bootstrap/src/progress.rs src/bin/cli/cache`

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: LOW
- **依赖**: 无（若计划 007 已完成，步骤 4 可以顺带做）
- **类别**: dx
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

填满缓存要跑 94,558 个条目、数小时。跑完如果有失败项，失败清单**只存在于内存和
终端输出里**——`populate` 把它们格式化成 `eprintln!` 之后就返回错误退出，什么也
不落盘。

于是用户想重试那几百个失败项，唯一的办法是重跑整个 94k 清单。虽然已下载的会在
本地命中而快速跳过，但每条仍要付一次文件 stat 加一次 redb 读，而且真正 404 的
条目会被再问一遍（见计划 007）。

失败项已经是结构化数据（`FailedItem` 有 `path` 和 `version`），而
`cache populate --src` 本来就接受任意清单路径。把失败项按同样的 JSONL 格式写出来，
重试命令就是现成的，不需要任何新 flag。

## 当前状态

`crates/emukc_bootstrap/src/progress.rs:136-140`，失败项的结构：

```rust
pub struct FailedItem {
    pub path: String,
    pub version: Option<String>,
    pub error: Arc<KacheError>,
}
```

`crates/emukc_bootstrap/src/make_list/mod.rs:42-50`，清单条目的结构——
注意 `path` 和 `version` 与上面**完全对应**：

```rust
/// A single cache list entry
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct CacheListItem {
    /// resource path
    pub path: String,

    /// Resource version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}
```

`crates/emukc_bootstrap/src/progress.rs:191-197`，失败项目前只进终端：

```rust
    if !failures.is_empty() {
        lines.push(String::new());
        lines.push("Failed files:".to_string());
        for f in failures {
            lines.push(format!("  ✗ {} ({})", f.path, f.error));
        }
    }
```

`crates/emukc_bootstrap/src/populate.rs:236-245`，结尾直接返回错误：

```rust
    if failed_count > 0 {
        // ... progress bar 收尾 ...
        return Err(KacheError::InvalidFile(format!("{failed_count} items failed after retry")));
    }
```

`src/bin/cli/cache/populate.rs:8-15`，`--src` 已经可以指向任意清单，
但 `--concurrent` 是**必填**（没有 `Option`，没有 `default_value`）：

```rust
    #[arg(help = "Path to cache list file.")]
    #[arg(long)]
    pub src: Option<String>,

    #[arg(help = "Number of concurrent tasks.")]
    #[arg(long)]
    pub concurrent: u8,
```

而 `BOOTSTRAP.md:116` 教人跑不带 `--concurrent` 的 `cargo run -- cache populate`，
`BOOTSTRAP.md:179` 还写「默认 16」——两处都是错的，那条命令会以 clap 用法错误退出。
本计划顺手修掉，因为它正好挡在「重试失败清单」这条路上。

**仓库约定**：Rust edition 2024，软 tab 4 空格。该 crate 已有 `serde_json`。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 编译 | `cargo build --release` | exit 0 |
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_bootstrap/src/populate.rs`
- `src/bin/cli/cache/populate.rs`（只加 `default_value_t`）
- `BOOTSTRAP.md`（只改 populate 相关的两处错误描述）

**范围外**：

- `crates/emukc_bootstrap/src/progress.rs` 的终端输出格式——不要动，
  落盘是新增的旁路，不是替换。
- 重试策略本身（退避、降并发）——本计划只负责让失败项**可被重新喂给命令**，
  不改两轮重试的结构。
- 并发模型（`FuturesUnordered` 改 `tokio::spawn`）。

## Git 工作流

- 分支：`feat/persist-populate-failures`
- 提交信息：`feat(cache): write populate failures to a retryable list file`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：把失败项写成 JSONL

在 `populate` 返回错误之前，把 `pass2_failures` 序列化成与输入清单**完全相同**的
JSONL 格式（每行一个 `CacheListItem`），写到输入清单的旁边，文件名在原名基础上
加后缀，例如输入 `cache_resources.nedb` → 输出 `cache_resources.failed.nedb`。

用 `CacheListItem` 本身序列化，不要另造结构——这样输出天然就能被 `--src` 读回去。

写文件失败只记 `error!`，**不要**覆盖原本要返回的那个错误（用户更需要知道的是
有多少条失败）。

在返回错误前打印一行提示，告诉用户重试命令长什么样，例如：

```
retry with: cache populate --src <失败清单路径>
```

**验证**：`cargo build --release` → exit 0

### 步骤 2：成功时清理旧的失败清单

如果本轮全部成功，而上一轮留下的失败清单文件还在，把它删掉——否则用户会对着
一份过期的失败清单重试。删除失败只记 `warn!`。

**验证**：`cargo build --release` → exit 0

### 步骤 3：给 --concurrent 一个默认值

`src/bin/cli/cache/populate.rs` 中：

```rust
    #[arg(help = "Number of concurrent tasks.")]
    #[arg(long, default_value_t = 16)]
    pub concurrent: u8,
```

`16` 与 `Makefile:13` 的 `CONCURRENT ?= 16` 一致。这样 `BOOTSTRAP.md` 里那条
命令就真的能跑了。

**验证**：`cargo run --release -- cache populate --help` → 显示
`[default: 16]`；不带 `--concurrent` 跑一个小清单 → 不报用法错误

### 步骤 4（若计划 007 已完成）：404 不进重试清单

如果 `Error::FileNotFound` 已经可以和 `FailedOnAllCdn` 区分（计划 007），
把 404 从写出的失败清单里排除——重试一个确定不存在的资源没有意义。
但**要在终端摘要里单独报一行** `Missing (404): N`，否则用户会以为它们成功了。

如果计划 007 尚未完成，跳过本步，并在提交信息里注明「404 与瞬时失败尚未区分，
失败清单包含两者」。

**验证**：`cargo build --release` → exit 0

### 步骤 5：修文档

`BOOTSTRAP.md` 两处：

- `:114-123` 第 3 步的命令说明——现在不带 `--concurrent` 真的能跑了，
  「默认 16 并发」这句话变成正确的，确认措辞与实现一致。
- `:176-180` 的参数表——同上核对。

补一段说明失败清单和重试命令。位置放在 populate 那一节末尾。

**验证**：人工核对文档里每条命令都能跑通

### 步骤 6：实测一轮

构造一个必然部分失败的清单（掺入几条不存在的路径），跑一次：

```
printf '{"path":"kcs2/img/nonexistent_abc.png"}\n{"path":"gadget_html5/js/kcs_const.js"}\n' > /tmp/populate-failtest.nedb
cargo run --release -- cache populate --src /tmp/populate-failtest.nedb
```

**预期**：非零退出；`/tmp/populate-failtest.failed.nedb` 存在，内容是合法 JSONL，
只含失败的那条；终端提示了重试命令。

再用失败清单重试一次，确认它能被直接喂回去（这次仍会失败，因为路径确实不存在，
但**不能**报格式错误）。

**验证**：失败清单生成、格式合法、能被 `--src` 读回

### 步骤 7：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改范围内文件

## 测试计划

在 `crates/emukc_bootstrap/src/populate.rs` 的 `#[cfg(test)] mod tests` 里新增
（照现有 `partition_by_error_variant` 的写法）：

- 把一组 `FailedItem` 序列化成 JSONL 之后，能用 `serde_json` 反序列化回
  `CacheListItem`，且 `path` / `version` 一致。
- `version` 为 `None` 的条目序列化后**不含** `version` 字段
  （`skip_serializing_if` 生效），与输入清单格式一致。

这两个都是纯函数测试，不需要网络。把序列化那段抽成一个可测的小函数。

**不要**为「整轮 populate」写集成测试——那需要 mock 整个下载栈，成本远超收益，
步骤 6 的手工实测已经覆盖。

## 完成标准

- [ ] populate 失败时在清单旁生成 `*.failed.nedb`，格式与输入清单一致
- [ ] 全部成功时旧的失败清单被清理
- [ ] `cargo run --release -- cache populate --help` 显示 `--concurrent` 的
      `[default: 16]`
- [ ] 不带 `--concurrent` 的 populate 命令能跑（不报 clap 用法错误）
- [ ] `BOOTSTRAP.md` 中 populate 的命令和参数说明与实现一致
- [ ] 步骤 6 实测：失败清单能被 `--src` 直接读回
- [ ] 新增的两个序列化测试通过
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 失败清单的写出位置不可写（例如清单在只读目录）——需要决定回退位置，汇报。
- 改 `--concurrent` 默认值后有测试或脚本依赖「不传就报错」这个行为——汇报。
- 步骤 6 中生成的失败清单**不能**被 `--src` 读回——说明格式不一致，
  这是本计划的核心价值，必须修好。

## 维护须知

- 失败清单与输入清单**必须保持同一格式**。以后 `CacheListItem` 加字段时，
  这条路径自动跟着走，前提是继续用 `CacheListItem` 序列化而不是另造结构。
  评审时确认这一点。
- 计划 007 完成后回来做步骤 4（把 404 排除出重试清单）。
- 明确不做：自动重试失败清单。现在的两轮重试已经在一次运行内覆盖瞬时失败；
  跨运行的自动重试会让「为什么它又在下载」变得难以解释。用户拿着清单手动重试
  是更可控的行为。
