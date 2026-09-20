# Plan 010: 删除 Greedy / holes-report 死代码并修正文档

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/make_list src/bin/cli/cache BOOTSTRAP.md`

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: LOW
- **依赖**: 无
- **类别**: tech-debt
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`cache make-list --greedy` 在文档里被描述成「扫描所有可能资源（极慢，但最完整）」。
实际上它**一次网络探测都不做**，产出与默认策略逐字节相同，耗时也相同。

维护者正是基于这份文档在做决策——「要不要花几小时跑 greedy 换一份更完整的清单」
是个伪命题，而伪命题会一直被重新提起，直到代码和文档对齐。

同样，`Greedy` 唯一被记录在案的产出物 holes report 也**不可能生成**：收集器
`HOLES_COLLECTOR` 只有读和清空两个操作，全工作区没有任何一处向它写入。另有一个
独立的 `HolesReport` 结构体整个挂着 `#[expect(dead_code)]` 且从未被构造——两套
互不相干的空洞报告机制，都不工作。

## 当前状态

**Greedy 不可达的四处证据**：

`crates/emukc_bootstrap/src/make_list/source/mod.rs:55-60`，三个策略共用一个分支：

```rust
    } else if matches!(
        strategy,
        CacheListMakeStrategy::Default
            | CacheListMakeStrategy::Greedy(_)
            | CacheListMakeStrategy::Rules
    ) {
```

`crates/emukc_bootstrap/src/make_list/source/mod.rs:92`，进入该分支后策略被
硬编码成 `Rules`：

```rust
        kcs::make(codex, kache, CacheListMakeStrategy::Rules, Some(rules_bundle), list).await?;
```

`crates/emukc_bootstrap/src/make_list/source/kcs2/mod.rs:22` 与
`crates/emukc_bootstrap/src/make_list/source/kcs2/resources/mod.rs:35`，
各有一行把调用方传入的策略整个覆写：

```rust
    let strategy = CacheListMakeStrategy::Manifest;
```

结果：`voice.rs:33`、`use_item.rs:24`、`furniture.rs:71`、`img.rs:596` 里的
greedy 分支全部不可达。

**死掉的探测函数**（没有任何可达调用点）：

- `make_list/source/kcs/voice.rs:187` `make_special_greedy`
- `make_list/source/kcs2/resources/use_item.rs:32`
- `make_list/source/kcs2/resources/furniture.rs:207`
- `make_list/source/kcs2/versioned/img.rs:605`
- `make_list/source/kcs2/resources/map.rs:230` `get_event_area_greedy`
  ——已经带 `#[expect(unused)]`，调用点在 `map.rs:189` 被注释掉

`make_list/source/kcs/voice.rs:1` 的 `#![allow(unused)]` 压住了本该暴露这些的
编译器警告。

**空洞报告的两套死机制**：

`make_list/source/kcs2/resources/ship.rs:11-19`：

```rust
static HOLES_COLLECTOR: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

pub fn get_holes_report() -> Vec<String> {
    HOLES_COLLECTOR.lock().unwrap().clone()
}

pub fn clear_holes_report() {
    HOLES_COLLECTOR.lock().unwrap().clear();
}
```

全工作区对它的引用只有这三处加上 `make_list/mod.rs:871` 的读和 `:887` 的清空——
**没有任何写入**。因此 `mod.rs:868-888` 的写文件分支恒不执行。

`make_list/holes_report.rs` 整个 `impl` 带 `#[expect(dead_code)]`，`HolesReport`
从未被构造。该文件 `:56-68` 还有一处会生成非法 Rust 的 bug：
`self.event_ship_full.iter().collect::<Vec<_>>().sort()` 格式化的是 `sort()` 的
返回值 `()`，生成出来的是 `full: vec!(),`。

（注：`z/cache/holes_report.txt` 这个文件在本地存在，是更早版本留下的产物，
不代表当前代码会生成它。）

**文档与代码不一致的三处**：

- `src/bin/cli/cache/make_list.rs:17` 的 help 文本："Greedy mode, which can be
  extremely slow"
- `BOOTSTRAP.md:166`："扫描所有可能资源（极慢，但最完整）"；`:167` 把
  `--concurrent` 描述成有效旋钮
- `docs/solutions/conventions/rules-default-strategy.md:43-47` 记载
  "Greedy SHALL delegate to the Rules code path, then produce a holes_report.txt"
  ——这条约定描述的行为现在做不到

**策略枚举现状**（`make_list/mod.rs:26-38`）：五个变体
`Default` / `Minimal` / `Greedy(GreedyConfig)` / `Manifest` / `Rules`，
其中 CLI 只能构造出 `Manifest` / `Greedy` / `Default` 三种
（`src/bin/cli/cache/make_list.rs:38-48`）。

**仓库约定**：`docs/solutions/**` 是制度化知识，**只在对应行为变化时有意更新**，
不随无关任务删改。本计划改变的正是它记录的那个行为，所以更新它是必须的，
而且要在提交信息里说明。

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

- `crates/emukc_bootstrap/src/make_list/mod.rs`（策略枚举、holes 写文件分支、
  `batch_check_exists` 及其常量）
- `crates/emukc_bootstrap/src/make_list/config.rs`（`GreedyConfig`）
- `crates/emukc_bootstrap/src/make_list/holes_report.rs`（整个文件）
- `crates/emukc_bootstrap/src/make_list/progress.rs`（仅当确认只服务于 greedy 探测）
- `crates/emukc_bootstrap/src/make_list/source/` 下的五个死探测函数及其 greedy 分支
- `crates/emukc_bootstrap/src/lib.rs`（prelude 中相应的 re-export）
- `src/bin/cli/cache/make_list.rs`（`--greedy` / `--concurrent` flag）
- `BOOTSTRAP.md`
- `docs/solutions/conventions/rules-default-strategy.md`

**范围外**：

- `CacheListMakeStrategy::Minimal` 与 `::Rules`——它们虽然 CLI 构造不出来，但在
  六个 source 模块里**被实际分支判断**，且 `Rules` 是 `Default` 的实际语义。
  **不要删**。
- `kcs2/mod.rs:22` 与 `kcs2/resources/mod.rs:35` 那两行策略覆写——**不要改**。
  它们是让 Greedy 失效的原因之一，但改它们等于「复活 Greedy」，那是相反方向的
  决策。本计划走的是删除路线；两条路只能选一条，见下方「如果要复活而不是删除」。
- `make_list/mod.rs` 的比较诊断子系统（`:254-721`）——它服务于
  `examples/decoder_cachelist_compare.rs`，是活的。
- 拆分大文件——见 README 的「已考虑并否决」。

## Git 工作流

- 分支：`refactor/remove-greedy-deadcode`
- 提交建议拆两个：
  1. `refactor(bootstrap): remove the unreachable greedy cache-list path`
  2. `docs: correct greedy and holes-report descriptions`
  第二个提交的正文要说明为什么改 `docs/solutions/`：记录的行为已不存在。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 0：确认这是要走的方向

本计划**删除** Greedy。另一条路是复活它（去掉两处策略覆写、把 `strategy` 真正
透传）。审计给出的判断是删除：历史上 greedy 也只是对四个家族做约 2,700 次 HEAD
的窄刷新，从来不是「最完整」，而那四个家族现在都有 decoder 规则覆盖。

**如果操作者想复活而不是删除**，本计划不适用，停止并汇报。

### 步骤 1：删除死探测函数与 greedy 分支

删除上文列出的五个函数，以及 `voice.rs:33`、`use_item.rs:24`、`furniture.rs:71`、
`img.rs:596` 的 greedy 分支（保留这些 match 的其它分支）。

删完把 `make_list/source/kcs/voice.rs:1` 的 `#![allow(unused)]` **也删掉**——
它当初就是用来压住这些死代码警告的。如果删掉后出现新的 unused 警告，说明还有
残留死代码，一并清理。

**验证**：`cargo build -p emukc_bootstrap` → exit 0 且无 warning

### 步骤 2：删除两套 holes-report 机制

- 删除 `make_list/holes_report.rs` 整个文件及其 `mod` 声明。
- 删除 `ship.rs:11-19` 的 `HOLES_COLLECTOR`、`get_holes_report`、`clear_holes_report`。
- 删除 `make_list/mod.rs:868-888` 的写文件分支。

**验证**：`cargo build -p emukc_bootstrap` → exit 0；
`grep -rn 'holes' crates/ src/ --include='*.rs' | grep -v target` → 无输出

### 步骤 3：删除 Greedy 策略与其配置

- `make_list/mod.rs` 的 `CacheListMakeStrategy` 去掉 `Greedy(config::GreedyConfig)` 变体。
- 删除 `make_list/config.rs`（如果 `GreedyConfig` 是其唯一内容）及其 `mod` 声明。
- `crates/emukc_bootstrap/src/lib.rs` 的 prelude 里去掉 `config::GreedyConfig`
  的 re-export（`lib.rs:64` 附近）。
- `source/mod.rs:55-60` 的 `matches!` 去掉 `Greedy(_)` 分支项。

**验证**：`cargo build --workspace` → exit 0

### 步骤 4：删除只服务于 greedy 的探测设施

`make_list/mod.rs:893-970` 的 `batch_check_exists` 和 `MAX_CHECK_SIZE`：
先确认没有别的调用者：

```
grep -rn 'batch_check_exists\|MAX_CHECK_SIZE' crates/ src/ examples/ --include='*.rs' | grep -v target
```

无其它调用者则删除。`make_list/progress.rs` 同样先 grep 确认只被这条路径使用，
是则删除，否则保留。

**验证**：grep 无残留；`cargo build --workspace` → exit 0

### 步骤 5：删除 CLI flag

`src/bin/cli/cache/make_list.rs` 去掉 `--greedy` 和 `--concurrent` 两个参数
及其在 `exec` 中的分支（`:38-48` 的策略选择简化为 `Manifest` 或 `Default`）。

**验证**：`cargo run --release -- cache make-list --help` → 不再显示
`--greedy` / `--concurrent`

### 步骤 6：修正文档

- `BOOTSTRAP.md:166-167`：删除 `--greedy` 与 `--concurrent` 两行。同时通读这一节，
  确认其余描述与现状一致。
- `docs/solutions/conventions/rules-default-strategy.md`：这份约定的标题和内容都
  建立在 Greedy 存在的前提上。**不要删除整个文件**（它记录了 `Default == Rules`
  这条仍然有效且重要的约定），而是：
  - 保留 `Default` 策略那一节
  - 把 Greedy 那一节改写成一条历史记录：说明 Greedy 已于本次变更删除、原因是
    其探测路径长期不可达、四个相关家族现由 decoder 规则覆盖
  - 更新文件末尾的策略对照表
  - 在 frontmatter 的 `date` 之外保留原始日期，正文里标注本次更新的日期

**验证**：人工通读两份文档，其中提到的每个 flag 都在 `--help` 里真实存在

### 步骤 7：确认清单产出未变

删除死代码**不应该**改变任何输出。用同一份 codex 生成清单并与改动前比对：

改动前（在删除提交之前）先存一份：

```
cargo run --release -- cache make-list --output /tmp/cachelist-before.nedb --overwrite
```

改动后：

```
cargo run --release -- cache make-list --output /tmp/cachelist-after.nedb --overwrite
sort /tmp/cachelist-before.nedb > /tmp/a.sorted
sort /tmp/cachelist-after.nedb > /tmp/b.sorted
diff /tmp/a.sorted /tmp/b.sorted && echo "IDENTICAL"
```

**预期**：输出 `IDENTICAL`。如果有差异，说明删掉了实际在用的东西，按 STOP 处理。

注意 make-list 会发起网络请求（拉 `kcs_const.js` 和 `version.json`），两次运行
之间上游若有更新会造成假差异——若出现差异，先确认不是版本行的差别。

**验证**：`diff` 无输出

### 步骤 8：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改范围内文件

## 测试计划

本计划**不新增测试**。它删除的是不可达代码，正确性标准是：现有测试全部继续通过，
且步骤 7 的清单产出逐行相同。

如果删除过程中有测试失败，说明那段代码并非不可达——按 STOP 条件处理。

## 完成标准

- [ ] `grep -rn 'Greedy\|greedy' crates/ src/ --include='*.rs' | grep -v target`
      无输出
- [ ] `grep -rn 'holes' crates/ src/ --include='*.rs' | grep -v target` 无输出
- [ ] `cargo run --release -- cache make-list --help` 不含 `--greedy` / `--concurrent`
- [ ] `make_list/source/kcs/voice.rs` 不再有 `#![allow(unused)]`
- [ ] 步骤 7 的清单 diff 为空（`IDENTICAL`）
- [ ] `BOOTSTRAP.md` 与 `docs/solutions/conventions/rules-default-strategy.md`
      中不再有与代码矛盾的描述
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 操作者想复活 Greedy 而不是删除它——本计划不适用。
- 步骤 7 的清单 diff **不为空**（且差异不是版本行）——说明删掉了活代码，
  汇报差异内容。
- 删除某个函数后有测试失败——说明它可达，汇报测试名和调用链。
- `docs/solutions/conventions/rules-default-strategy.md` 的改写被评审认为丢失了
  仍然有效的约定——按评审意见保留，不要为了「清理干净」而删除制度化知识。

## 维护须知

- 这次删除**关闭了一条路**：以后若确实需要「探测式补全清单」，不要从 git 历史里
  捡回这套代码。它的四个探测函数已经与 decoder 规则体系脱节，而且它们共享的那个
  五行复制粘贴块在四份副本里已经各自漂移过
  （`img.rs:631` 用 `add_unversioned` 而另外三处用 `add(p, v)`，会导致缓存永不刷新）。
  真要重做应当基于 decoder 规则的覆盖缺口来生成候选，而不是暴力枚举。
- 评审重点：`Minimal` 和 `Rules` 两个变体**没有**被顺手删掉。
- `docs/solutions/` 的改动必须在提交信息里说明理由——仓库规定它只在对应行为变化时
  有意更新。本次正是那种情况。
