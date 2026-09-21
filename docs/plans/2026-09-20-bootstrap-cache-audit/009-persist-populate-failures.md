# Plan 009: populate 失败清单落盘，并把 404 从重试路径里分流出去

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 9bd9f59..HEAD -- crates/emukc_bootstrap/src/populate.rs crates/emukc_bootstrap/src/progress.rs src/bin/cli/cache`
>
> 基线已从 `82d2203` 推进到 `9bd9f59` + 一次未提交的 `populate.rs` 改动（删除逐项
> spinner）。下文「当前状态」的摘录已按该改动刷新。`progress.rs` 与
> `src/bin/cli/cache/` 未被触及。

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: LOW
- **依赖**: 步骤 1-3、5-7 无依赖；步骤 4 依赖计划 007 提供的错误区分能力
- **类别**: dx
- **计划基线**: commit `9bd9f59` + 未提交的 spinner 删除, 2026-09-20

## 为什么这件事重要

填满缓存要跑 73,031 个条目（默认 Rules 策略的实测条数，不是 manifest 策略的
94,558——见 README「rules 与 manifest 两种策略的清单差异」），受网络限制要数十分钟
到数小时；2026-09-20 的一次实测是 89 分 14 秒。跑完如果有失败项，失败清单**只存在
于内存和终端输出里**——`populate` 把它们格式化成 `eprintln!` 之后就返回错误退出，
什么也不落盘。

于是用户想重试那几百个失败项，唯一的办法是重跑整个清单。虽然已下载的会在本地命中
而快速跳过，但每条仍要付一次文件 stat 加一次 redb 读，而且真正 404 的条目会被再问
一遍（见计划 007）。

**404 清单本身是有价值的产物，不只是噪声。** 2026-09-20 那轮 populate 的 19 条失败
全是 404，事后靠它补上了 `EVENT_SHIP_HOLES`（6299/6301/6303）和新增的
`ALBUM_STATUS_HOLES`（743/744/745/748/749），清单从 73,050 条收敛到 73,031 条。
当时那份列表只能从 29 MB 的 `emukc.log.*` 里 grep 出来。落盘之后，下一次游戏更新
冒出新的缺口时，这份 404 清单就是 holes 表的直接数据源。

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

`crates/emukc_bootstrap/src/populate.rs:182-189`，pass 1 的失败项只按版本回退分流，
其余**全部**进入 pass 2 重试队列——包括确定不存在的 404：

```rust
    let (skipped, retry_items): (Vec<_>, Vec<_>) = pass1_failures
        .into_iter()
        .partition(|f| matches!(f.error.as_ref(), KacheError::InvalidFileVersion(_)));
    if !skipped.is_empty() {
        warn!("skipping {} items with version rollback", skipped.len());
    }
    let retry_items: Vec<(String, Option<String>)> =
        retry_items.into_iter().map(|f| (f.path, f.version)).collect();
```

`crates/emukc_bootstrap/src/populate.rs:221-230`，结尾直接返回错误：

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
- `crates/emukc_bootstrap/src/progress.rs`——**仅限**步骤 4(3) 需要的两处：
  `PopulateStats` 加一个 `missing: usize` 字段，`build_summary_lines` 在 `Failed`
  之后追加 `Missing (404): N`（只在 `missing > 0` 时出现）。摘要由该文件的
  `build_summary_lines`（当前 :176）拼装、字段来自同文件的 `PopulateStats`
  （当前 :124-131），不改它就交付不了那一行。
- `src/bin/cli/cache/populate.rs`（只加 `default_value_t`）
- `BOOTSTRAP.md`（只改 populate 相关的两处错误描述）

**范围外**：

- `progress.rs` 的其余部分——`Failed files:` 区块的格式、`Total/OK/Retried/
  Recovered/Failed/Time` 那一行的既有字段、`FailedItem` 结构，一律不动。
  落盘是新增的旁路，不是替换。
- 重试策略本身（退避、降并发）——本计划只负责让失败项**可被重新喂给命令**，
  不改两轮重试的结构。
- 并发模型（`FuturesUnordered` 改 `tokio::spawn`）。
- **退出码语义**——有失败就非零退出这条约定**不变**，理由见「维护须知」。
- 负缓存（把 404 持久化进 redb 让 `get` 直接短路）——计划 007 已否决，理由是
  没有 TTL 的负缓存会让以后真的上线的资源永远取不到。本计划写出的 404 清单是
  给人和 `make-list` 看的旁路产物，不参与 `get` 的判定。

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

**后缀不叠加**：当输入清单本身已经是 `*.failed.nedb`（或步骤 4 的 `*.missing.nedb`）
时，输出沿用同一路径，不再追加后缀。否则步骤 6 要求的「拿失败清单再跑一轮」会产出
`cache_resources.failed.failed.nedb`，而步骤 2 的清理对象是由本轮输入名派生的，
永远清理不到刚被消费的那一份——用户下次仍会对着一份过期清单重试。本轮该类仍有
失败就原地覆盖，该类为空就按步骤 2 删除它。

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

### 步骤 4（依赖计划 007）：把 404 从重试路径里分流出去

前提：计划 007 已让 `Error::FileNotFound`（CDN 明确答 404）与 `FailedOnAllCdn`
（没问出结果）成为两个不同的错误值。若 007 尚未完成，跳过本步，并在提交信息里
注明「404 与瞬时失败尚未区分，失败清单包含两者」。

本步有三件事，**缺一不可**：

**(1) 404 不进 pass 2 的内存重试。** 这是本计划与计划 007 之间真正的缺口：007 的
「范围外」把 populate 的分流推给了 009，而 009 原本的步骤 4 只管落盘清单，两边都
没有覆盖内存重试这一层。把 `populate.rs:182-184` 的二分改成三分：

- `InvalidFileVersion` → 跳过（现有行为，版本回退不是下载失败）
- `FileNotFound` → 跳过重试，计入 `missing`
- 其余 → 进入 pass 2 的 `retry_items`

pass 1 里每条 404 已经付过一次请求，pass 2 再问一次纯属浪费，且会让 `Retried` 这个
数字失去意义（2026-09-20 那轮的 `Retried: 19 / Recovered: 0` 就全是 404）。

**(2) 404 与瞬时失败分开落盘。** 步骤 1 写出的 `*.failed.nedb` 只含**可重试**的项；
404 另外写一份 `*.missing.nedb`，格式同样是 `CacheListItem` 的 JSONL。两份都写，
不要把 404 丢掉——它是 holes 表的数据源（见「为什么这件事重要」）。若某一类为空，
不要创建空文件，并按步骤 2 的规则清理上一轮的同名残留。

**注意 `FileNotFound` 的证明力（007 的 code review 发现）。** CDN 镜像列表是
shuffle 过的，`fetch_from_remote` 在**第一个**回 404 的镜像上就 return，所以这个变体
的含义是「本轮随机抽中的那个镜像说没有」，不是「所有镜像都没有」。作为「跳过 pass 2
重试」的依据这没问题（pass 1 已经问过一次）；但作为 **holes 表的数据源**就不够——
某个镜像在自己的同步窗口里回 404，会让一个真实存在的资源被永久写进 holes 表。
所以 `*.missing.nedb` 在被人拿去补 `EVENT_SHIP_HOLES` / `ALBUM_STATUS_HOLES` 之前，
必须先对全部镜像做一次确认性扫描。这件事放在本计划之外（那份清单只有几十条，且本来
就要人工过目），但**必须写进步骤 5 的文档说明里**，否则下一个用它的人不会知道。

**(3) 摘要单列一行。** `Missing (404): N`，与 `Failed: N` 分开，否则用户会以为
它们成功了。并提示 `*.missing.nedb` 的路径，因为那份文件要拿去比对 holes 表。

注意：本步**不改退出码语义**，但正因为语义不变，**必须相应放宽退出判定**。
`populate.rs:221` 现在的分支是 `if failed_count > 0`，而 `failed_count` 就是
`pass2_failures.len()`；404 被 (1) 移出 `retry_items` 之后 pass 2 收到空队列，
`failed_count` 归零，函数会直接走到末尾 `Ok(())`——2026-09-20 那种「19 条失败全是
404」的运行就会退化成 exit 0，正是「维护须知」里明确否决的方案。把判定改成同时看
两类计数（`failed_count + missing_count > 0`），并让错误文案同时反映两者，例如
`{failed_count} items failed after retry, {missing_count} missing (404)`。

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

**预期**：非零退出；终端提示了重试命令。若步骤 4 已做，那条不存在的路径会落在
`*.missing.nedb` 而不是 `*.failed.nedb`，摘要里报 `Missing (404): 1`，且
`Retried` 为 0（404 不再进 pass 2）；若步骤 4 跳过，它落在 `*.failed.nedb`。
两种情况下文件内容都是合法 JSONL，只含失败的那条。

再用生成的清单重试一次，确认它能被直接喂回去（这次仍会失败，因为路径确实不存在，
但**不能**报格式错误）。

**验证**：清单生成、格式合法、能被 `--src` 读回；步骤 4 已做时 404 与可重试项
分别落在两份文件里

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

若步骤 4 已做，再加一个（扩展现有 `partition_by_error_variant`，或新写一个）：

- 一组混合 `FailedItem`（`InvalidFileVersion` / `FileNotFound` / `FailedOnAllCdn`）
  经三分之后，只有 `FailedOnAllCdn` 那条进入重试队列，`FileNotFound` 那条进入
  `missing`，`InvalidFileVersion` 那条两边都不进。

这三个都是纯函数测试，不需要网络。把序列化那段和三分那段各抽成一个可测的小函数。

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
- [ ] （步骤 4）`FileNotFound` 不再出现在 pass 2 的 `retry_items` 里——用步骤 6 的
      实测确认 `Retried` 为 0，或用三分的纯函数测试确认
- [ ] （步骤 4）404 落在 `*.missing.nedb`，可重试项落在 `*.failed.nedb`，两者不混
- [ ] （步骤 4）摘要里 `Missing (404): N` 与 `Failed: N` 分行显示
- [ ] 退出码行为未变：有失败或有 404 时仍非零退出——特别确认「失败项全为 404」
      的一轮**不是** exit 0（步骤 6 的实测就是这个场景）
- [ ] 用 `*.failed.nedb` 重试一轮后，文件名没有叠加成 `*.failed.failed.nedb`
- [ ] 新增的序列化 / 三分测试通过
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 失败清单的写出位置不可写（例如清单在只读目录）——需要决定回退位置，汇报。
- 改 `--concurrent` 默认值后有测试或脚本依赖「不传就报错」这个行为——汇报。
- 步骤 6 中生成的失败清单**不能**被 `--src` 读回——说明格式不一致，
  这是本计划的核心价值，必须修好。
- 步骤 4 中发现 `FileNotFound` 在 pass 1 之外还有别的来源（例如本地路径判定也返回
  它），导致三分把本该重试的条目误判成 404——汇报来源清单，不要自行放宽判定。

## 维护须知

- 失败清单与输入清单**必须保持同一格式**。以后 `CacheListItem` 加字段时，
  这条路径自动跟着走，前提是继续用 `CacheListItem` 序列化而不是另造结构。
  评审时确认这一点。
- 计划 007 完成后回来做步骤 4（三件事：不进 pass 2、分开落盘、摘要单列）。
- **已评估并否决：让「全为 404」时 exit 0。** 曾考虑把 404 视为「清单与上游的差异」
  而非 populate 的失败，从而不再让 `make cache-populate` 挂掉。否决理由有三条：
  (a) 退出码在本仓库没有自动化消费者——没有 CI（`.github/` 只有 agent prompts），
  `populate` 也不被任何 make target 依赖（`update` 链走到 `make-list` 就停），
  所以非零退出的全部成本只是终端多一行 `make: *** Error 1`，不阻断任何东西；
  (b) 它的收益已被实证——2026-09-20 那 19 条 404 能被发现并修掉，因果链就是
  make 挂了、用户去看输出；(c) exit 0 方案依赖「人会主动去看落盘的 404 清单」，
  而本仓库恰好有反例：`z/cache/holes_report.txt` 从 2026-04-20 躺到 09-20 无人查看，
  且它一直是空的（`HOLES_COLLECTOR` 的写入方在某次重构中被删，五个月无人发现，
  见计划 010）。在没有 CI、全靠人手跑命令的项目里，用退出码承载「别忘了」比用
  一个文件可靠。若将来引入 CI 并需要区分，再单独立计划。
- 明确不做：自动重试失败清单。现在的两轮重试已经在一次运行内覆盖瞬时失败；
  跨运行的自动重试会让「为什么它又在下载」变得难以解释。用户拿着清单手动重试
  是更可控的行为。
