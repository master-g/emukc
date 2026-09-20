# Plan 002: 为第三方数据解析器建立 fixture 测试基线

> **执行者须知**：逐步执行，每步跑验证命令确认预期结果后再继续。触发「STOP 条件」
> 立即停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md`
> 中本计划的状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/parser`
> 有变化就先比对下文「当前状态」的代码摘录，不一致按 STOP 处理。

## 状态

- **优先级**: P1
- **工作量**: M
- **风险**: LOW
- **依赖**: 无
- **类别**: tests
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`crates/emukc_bootstrap/tests/fixtures/` 目前**只有 `map_overlay/` 一个目录**，
`res.rs` 里 12 个第三方数据源**一个 fixture 都没有**。结果是：解析器出错的唯一
发现途径，是跑一轮完整的联网 bootstrap 然后人工读日志。

这不是假设。当前 `.data/codex/quest.json` 里就有 8 条任务的 `detail` 是字面量
`"_quest_id_257"` 这种内部 key，12 条任务的 `name` 是 `"n/a"`——已经这样发布了
一段时间，没有任何测试会失败。

计划 004（重写 kccp 解析器）、006（空需求判定）、011（label_type 表）都要改这条
数据链，且都会改变 codex 产物。没有 fixture，它们只能靠联网验证，既慢又不可重复。

本计划只加 fixture 和测试，不改任何解析逻辑。

## 当前状态

**解析器测试普查**（`crates/emukc_bootstrap/src/parser/` 下）：

- 有测试：`kcanotify/expedition.rs`（14 个，全仓库最好）、`wikiwiki_map/mod.rs`（6 个）、
  `tsunkit_quest/requirement/mod.rs`（4 个）、`kcwiki/enemy.rs`（2 个）
- **零测试**：`kccp/quest.rs`、`tsunkit_quest/label_type.rs`、`tsunkit_quest/types.rs`、
  `tsunkit_quest/reward.rs`、`tsunkit_quest/mod.rs` 的 `parse`、`kcwikizh_kcdata.rs`、
  `kc3kai/quote.rs`、`music.rs`、`kcwiki/slot_item.rs`、`kcwiki/ship.rs`、`kcwiki/use_item.rs`

**两个已知有问题的现有测试**（本计划要处理）：

1. `crates/emukc_bootstrap/src/parser/kcwiki/mod.rs:159-203` 的三个测试从
   `../../.data/temp/*.json` 读文件，并**往 `.data/temp/` 写四个文件**。干净 clone
   上跑不过，而且有文件系统副作用。
2. `crates/emukc_bootstrap/src/res.rs:107-115` 唯一的测试只 `println!` 每个条目，
   不断言任何东西。

**本计划要覆盖的第一优先数据源**：`kccp_quests.json`。它是任务 name/detail 的
**唯一**来源——`tsunkit_quest/mod.rs:453-456` 取不到就回退到
`KccpQuestInfo::default()`，其 name 和 desc 都是 `"n/a"`。

该文件的真实结构（`.data/temp/kccp_quests.json`，2302 行，JSON 对象，key 是日文
原文，value 是英文翻译）：

```json
{
	"_quest_id_101": "_quest_code_A1",
	"はじめての「編成」！": "The First Attempt at Fleet Organization!",
	"２隻以上の艦で構成される「艦隊」を編成せよ！": "Have 2 ships in your main fleet.",
	"_quest_id_102": "_quest_code_A2",
```

即：一个 `_quest_id_N` 条目，后面跟 name 条目和 desc 条目。**但这个三段式并不总是
成立**，实测 771 个 id 的分布是：

| id 后的条目数 | 数量 | 含义 |
|---|---|---|
| 2 | 757 | 正常：name + desc |
| 1 | 13 | 缺 name 或缺 desc |
| 3 | 1 | id 1169，多一个 `"dummy": "forNoComma"` 哨兵 |

13 个单条目组中，**11 个缺 name**（唯一那条是描述，日文以「！」或「。」结尾）：
256、615、616、622、627、628、630、632、633、648、652；
**2 个缺 desc**（唯一那条是【】开头的短标题）：1124、1125。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 本 crate 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| 单个测试 | `cargo test -p emukc_bootstrap kccp` | 通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_bootstrap/tests/fixtures/kccp/quests_sample.json`（新建）
- `crates/emukc_bootstrap/tests/fixtures/tsunkit/quests_sample.json`（新建）
- `crates/emukc_bootstrap/src/parser/kccp/quest.rs`（**只加** `#[cfg(test)] mod tests`）
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/label_type.rs`（**只加**测试模块）
- `crates/emukc_bootstrap/src/parser/kcwiki/mod.rs`（**只改** `#[cfg(test)]` 部分）

**范围外**：

- `kccp/quest.rs` 的 `parse` 函数体——它有已知缺陷，由计划 004 重写。**本计划要把
  它当前的错误行为如实断言下来**，并在断言旁注明「计划 004 会翻转」。
- `label_type.rs` 的 `extract_label_type` 函数体——由计划 011 修。同样如实断言现状。
- `crates/emukc_bootstrap/src/res.rs`
- 任何 `.data/` 下的文件（那是本地状态，不进仓库）
- 其余 9 个零测试的解析器——本计划只做 quest 链路这两个。贪多会让评审无法聚焦，
  其它源等有具体缺陷再补。

## Git 工作流

- 分支：`test/parser-fixture-baseline`
- 提交信息：`test(bootstrap): add kccp and tsunkit quest parser fixtures`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：造 kccp fixture

新建 `crates/emukc_bootstrap/tests/fixtures/kccp/quests_sample.json`，手写一个
**小而全**的样本（不要整份拷贝 2302 行）。必须包含以下五种结构，顺序也要保持，
因为被测解析器是顺序敏感的：

1. 两个正常的三段式任务（如 101、102），用真实内容。
2. 一个缺 name 的任务（用 615 的真实内容：id 行 + 一条以「！」结尾的长描述）。
3. 紧跟其后的下一个任务（616，它本身也缺 name）——这一对正是现网失步的触发点。
4. 一个缺 desc 的任务（用 1124 的真实内容：id 行 + 一条【】开头的短标题）。
5. 一个带 `"dummy": "forNoComma"` 哨兵的任务（1169 的形状）。

真实内容可从 `.data/temp/kccp_quests.json` 摘取；若本地没有该文件，用上文
「当前状态」里给出的片段加上结构等价的占位内容即可，关键是**结构**要对。

**验证**：`python3 -c "import json;d=json.load(open('crates/emukc_bootstrap/tests/fixtures/kccp/quests_sample.json'));print(len(d))"`
→ 输出条目数，不报错（即文件是合法 JSON）

### 步骤 2：如实记录 kccp 解析器的当前（错误）行为

在 `crates/emukc_bootstrap/src/parser/kccp/quest.rs` 末尾加 `#[cfg(test)] mod tests`，
用 `include_str!` 读入步骤 1 的 fixture，调用 `parse`，断言**当前**结果：

- 正常三段式任务：name 和 desc 都正确。
- 615/616 这一对：断言当前的错误产出——615 的 `desc` 是字面量 `"_quest_id_616"`，
  且 616 **完全不在**结果 map 里。每条断言上方注释：
  `// 当前行为，计划 004 会翻转：615 的 desc 应为日文描述，616 应存在`
- 1124（缺 desc）：断言当前产出。
- 哨兵组：断言当前产出。
- 总条目数：断言 `result.len()` 的当前值。

**这些断言故意锁定 bug**。它们的价值是：计划 004 改完之后，必须逐条翻转，
翻转的 diff 就是修复效果的证据。

**验证**：`cargo test -p emukc_bootstrap kccp` → 全部通过

### 步骤 3：给 label_type 加表驱动测试

在 `crates/emukc_bootstrap/src/parser/tsunkit_quest/label_type.rs` 末尾加测试模块，
覆盖 `extract_label_type`：

- 各周期前缀：`d` → 2、`w` → 3、`m` → 6、`q` → 7，各取一个真实 wiki_id。
- `By`/`Cy` 表中**命中**的用例，各取两个。
- `By`/`Cy` 表中**未命中**的 7 个真实 wiki_id：`By14`、`By15`、`By16`、`Cy13`、
  `Cy14`、`Cy15`、`Cy16`——断言当前返回 `1`，并注释：
  `// 当前行为，计划 011 会翻转：这些是年任务，不应落到 label_type 1`

**验证**：`cargo test -p emukc_bootstrap label_type` → 全部通过

### 步骤 4：让 kcwiki 的测试不再依赖 .data 且不写文件

`crates/emukc_bootstrap/src/parser/kcwiki/mod.rs:159-203` 的三个测试读
`../../.data/temp/*.json` 并写出四个文件。二选一处理，优先第一种：

1. 从 `.data/temp/` 摘取**小样本**存进 `tests/fixtures/kcwiki/`，测试改读 fixture，
   并**删掉所有写文件的代码**（那是调试残留，不是断言）。
2. 如果样本裁剪代价过大（单个源结构复杂到摘不出自洽样本），退而求其次：给这三个
   测试加 `#[ignore]` 并在属性上方写明原因和恢复方式。**采用此方案必须在提交信息
   里显式说明**，因为仓库规定被跳过的测试必须在 PR 中暴露。

**验证**：在**干净 clone 或临时移走 `.data/` 之后**跑
`cargo test -p emukc_bootstrap` → 全部通过（或按方案 2 显示为 ignored）

### 步骤 5：全量门禁

**验证**：
- `cargo test -p emukc_bootstrap` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改了范围内文件

## 测试计划

产出即测试。新增：`kccp/quest.rs` 测试模块（≥ 6 个断言点）、`label_type.rs` 测试
模块（≥ 11 个用例）、两个 fixture 文件。结构参照
`crates/emukc_bootstrap/src/parser/kcanotify/expedition.rs` 的测试模块——它是本仓库
覆盖最好的解析器测试，照它的写法。

不要引入新的测试框架或 mock 设施。不要给范围外的解析器补测试。

## 完成标准

- [ ] `crates/emukc_bootstrap/tests/fixtures/kccp/quests_sample.json` 存在且是合法 JSON，
      包含上文列出的全部五种结构
- [ ] `cargo test -p emukc_bootstrap` exit 0
- [ ] 在 `.data/` 被临时移走的情况下 `cargo test -p emukc_bootstrap` 仍然 exit 0
- [ ] kccp 测试模块中，锁定当前错误行为的断言都带有「计划 004 会翻转」注释
- [ ] label_type 测试覆盖了 7 个未命中 wiki_id，且带「计划 011 会翻转」注释
- [ ] `cargo fmt --all --check` 与 `cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 「当前状态」里的结构描述与 `.data/temp/kccp_quests.json` 实际不符（上游改版了）——
  汇报实际分布，不要自行调整断言去迁就。
- 步骤 2 中某条「当前行为」断言写出来发现**通不过**——说明解析器行为与本计划描述
  不一致，停止并汇报实际产出，不要改解析器让断言通过。
- 步骤 4 两种方案都走不通。
- 任一验证命令连续两次修复后仍失败。

## 维护须知

- 本计划**刻意锁定 bug**。计划 004 和 011 的第一步就是来翻转这些断言的，翻转时
  连带删掉「计划 XXX 会翻转」注释。看到这些注释不要当成待清理的垃圾。
- 评审重点：fixture 是不是**结构完整**（五种形态都在），而不是内容多。结构缺一种，
  004 就可能改出一个只对样本成立的解析器。
- 明确推迟：其余 9 个零测试解析器（`kcwiki/slot_item.rs` 486 行 10 个 error 点、
  `kcwiki/ship.rs` 325 行等）不在本计划内。它们值得补，但要在有具体缺陷驱动时补，
  否则是在给不知道对错的行为写断言。
