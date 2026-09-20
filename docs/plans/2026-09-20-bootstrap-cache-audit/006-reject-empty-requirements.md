# Plan 006: 禁止空需求被判定为「任务已完成」

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_model/src/thirdparty/quest crates/emukc_bootstrap/src/parser/tsunkit_quest`
> 有变化就先比对下文代码摘录。

## 状态

- **优先级**: P0
- **工作量**: M
- **风险**: MED
- **依赖**: 002（需要解析器 fixture 基线）
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

任务需求的解析失败会**降级成空条件列表**，而空条件列表在进度计算里被判定为
「已完成」。两者相接，等于「解析器看不懂的任务 = 白送奖励」。

这不是理论风险。当前 `.data/codex/quest.json` 里 **api_no 1033（wiki_id B210,
「二等輸送艦の積極運用」）的 requirements 就是 `{"And": []}`**——零条件。上游给它
声明了 `category: "sortie"` 但没有 `sortie` 块，命中
`requirement/sortie.rs:21-22` 的降级分支，于是条件被清空。

更糟的是这个模式会自我繁殖：解析链里有 **14 处**「记一条 error 日志然后返回空
vec」的降级点。上游每加一种本解析器不认识的结构，就多一个白送的任务，而唯一的
信号是一行没人盯的 ERROR 日志（bootstrap 本身仍然 exit 0）。

## 当前状态

**进度计算侧** `crates/emukc_model/src/thirdparty/quest/progress.rs:66-80`：

```rust
fn progress_from_ratio(completed: usize, total: usize) -> QuestProgressStatus {
    if total == 0 {
        return QuestProgressStatus::Completed;
    }
    let ratio = completed as f64 / total as f64;
    if ratio >= 1.0 {
        QuestProgressStatus::Completed
    } else if ratio >= 0.8 {
```

`And(vec![])` 走 `progress.rs:14-18`：`total = 0` → `progress_from_ratio(0, 0)`
→ `Completed`。`Sequential(vec![])` 走 `:26-28`：`position` 返回 `None` →
直接 `Completed`。

**解析降级侧**，14 处返回空 vec，分布如下（用
`grep -rn 'return vec!\[\]\|Ok(vec!\[\])' crates/emukc_bootstrap/src/parser/tsunkit_quest/`
可复现）：

- `requirement/mod.rs`：`:57`、`:82`、`:122`、`:258`、`:263`、`:274`
- `requirement/modernization.rs`：`:19`、`:26`、`:31`、`:39`、`:45`、`:50`
- `requirement/sortie.rs`：`:22`
- `requirement/simple.rs`：`:9`

`requirement/sortie.rs:15-24`，1033 命中的那一处：

```rust
        if let Some(sorties) = &self.sortie {
            sorties.iter().for_each(|sortie| {
                let kc3_sortie = sortie.to_kc3rd_sortie();
                result.push(Kc3rdQuestCondition::Sortie(kc3_sortie));
            });
        } else {
            error!("sortie requirement must have a 'sortie' field");
            return vec![];
        }
```

注意它**丢弃了已经解析出来的 `Composition`**（`:11-14` 先 push 的那个），
返回彻底的空 vec。

**已有的正确模式可以照抄**：
`crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs:47` 的
`RequirementsCategory::Unknown => Err(ParseError::UnknownCategory)`，配合
`tsunkit_quest/mod.rs:436-443` 的处理：

```rust
            Err(ParseError::UnknownCategory) => {
                warn!(
                    wiki_id = %self.wiki_id,
                    game_id = self.game_id,
                    "skipping quest with unknown requirement category"
                );
                return Ok(None);
            }
```

即「解析不出来就跳过这条任务」，返回 `Ok(None)`，上层不收录。这正是本计划要把
降级点接上的路径。

**一个已经核验过、不要被误导的点**：`Kc3rdQuestCondition::Composition(_)` 在
`progress.rs:56` 恒为 `false`（注释：`// Composition validated separately via
fleet check`）。因此 `OneOf` 里只含 `Composition` 的分支**不会**让任务被判完成。
审计初稿里说 api_no 1019 可以被白嫖，经核验**不成立**，不要把它当作本计划的目标。
本计划唯一确凿的目标是空条件列表。

**仓库约定**：Rust edition 2024，软 tab 4 空格。`emukc_model` 位于
`emukc_bootstrap` 的下游（依赖方向向下），改 `emukc_model` 会影响 gameplay。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| model 测试 | `cargo test -p emukc_model` | 全部通过 |
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| gameplay 测试 | `cargo test -p emukc_gameplay` | 全部通过 |
| 集成测试 | `cargo test --test gameplay_tests` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/sortie.rs`
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/simple.rs`
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/modernization.rs`
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs`
- `crates/emukc_bootstrap/src/parser/error.rs`（如需新增错误变体）
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs`（只改错误处理分支）

**范围外**：

- `crates/emukc_model/src/thirdparty/quest/progress.rs` 的 `total == 0 →
  Completed` 分支。**不要改它**。理由：那是运行时进度计算的通用规则，可能有
  合法的无条件任务依赖它；而且 `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs:377-387`
  有一个现存测试 `and_category_with_empty_list_succeeds` **明确断言**空 `And`
  是合法结果。正确的修法是在**解析期**不产出空需求，而不是在运行期改判。
  如果评审坚持要在运行期加防线，那是一个独立决策，不要夹带在本计划里。
- `crates/emukc_gameplay/src/game/quest/` 的任何文件
- `From<List>` 丢字段的问题（`types.rs:241-285`）——那是另一条线，不在本计划内

## Git 工作流

- 分支：`fix/reject-empty-quest-requirements`
- 提交信息：`fix(bootstrap): skip quests whose requirements degrade to empty`
- 正文列出：修复前 api_no 1033 的 requirements 为 `{"And": []}` 且激活即可领奖。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：给降级点一个可传播的错误

在 `crates/emukc_bootstrap/src/parser/error.rs` 增加一个变体，语义是
「这条任务的需求无法解析成任何条件」，例如：

```rust
    /// A quest requirement block could not be resolved into any condition.
    #[error("requirement could not be resolved: {reason}")]
    EmptyRequirement {
        /// Why the requirement produced no conditions.
        reason: String,
    },
```

`missing_docs` 是 warning，文档注释不能省。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 2：把降级点改成返回错误

把上文列出的 14 处 `return vec![]` / `Ok(vec![])` 逐个检查，分两类处理：

**A 类——真正的「数据不合法」**，改成返回 `Err(ParseError::EmptyRequirement { .. })`，
`reason` 里带上足以定位的信息（类别名 + 缺失的字段名）。`sortie.rs:22`、
`simple.rs:9`、`modernization.rs` 的六处、`mod.rs` 的 `:82`/`:122`/`:258`/`:263`/`:274`
都属于此类——它们的上方都已经有一条 `error!` 说明「必须有某字段」。

**B 类——合法的「本来就没有」**，保持返回空 vec。`mod.rs:57` 的
`extract_list` 在 `self.list` 为 `None` 时返回 `Ok(vec![])`，这是「这条需求没有
嵌套列表」的正常情况，不是失败。**逐个确认，别一刀切**。

改签名时注意：这些函数目前返回 `Vec<Kc3rdQuestCondition>`，改成
`Result<Vec<Kc3rdQuestCondition>, ParseError>` 会波及调用方
（`requirement/mod.rs:30-48` 的 match）。照着现有的
`RequirementsCategory::Unknown => Err(...)` 那一行的形状改，保持一致。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 3：让新错误走「跳过这条任务」的既有路径

在 `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs:433-443`，
把新变体接到已有的 `UnknownCategory` 处理旁边：

```rust
            Err(ParseError::UnknownCategory) => { /* 现有逻辑 */ }
            Err(ParseError::EmptyRequirement { reason }) => {
                warn!(
                    wiki_id = %self.wiki_id,
                    game_id = self.game_id,
                    reason = %reason,
                    "skipping quest with unresolvable requirements"
                );
                return Ok(None);
            }
```

**不要**让它变成 `Err` 向上传播——那会让一条坏任务炸掉整轮 bootstrap。跳过单条
是正确的粒度，和 `UnknownCategory` 保持一致。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 4：加解析期回归测试

在 `crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs` 的测试
模块中新增：

- 一个 `category: "sortie"` 但没有 `sortie` 字段的输入 → 断言返回
  `Err(ParseError::EmptyRequirement { .. })`，**而不是** `Ok(vec![])`。
  这正是 1033 的形状。
- 一个正常的 sortie 需求 → 断言仍然正确解析（不回归）。
- `extract_list` 在 `list` 为 `None` 时 → 断言仍返回 `Ok(vec![])`（B 类不受影响）。

现存的 `and_category_with_empty_list_succeeds`（`:377-387`）：确认它测的是 B 类
路径。**如果它测的是 A 类**（即它断言的正是本计划要改成错误的那条路径），
不要直接删——把它改成断言新的 `Err`，并在提交信息里说明这条测试的语义变更。

**验证**：`cargo test -p emukc_bootstrap tsunkit` → 全部通过

### 步骤 5：重新生成 codex 并核对

```
cargo run --release -- bootstrap --overwrite
```

**不要带 `--force-update`**（见计划 005，它在 CDN 不可达时会删掉 `main.js`）。

核对不再有空需求：

```
python3 -c "
import json
d=json.load(open('.data/codex/quest.json'))
qs=d if isinstance(d,list) else (list(d.values()) if isinstance(d,dict) else d)
bad=[q['api_no'] for q in qs if q.get('requirements') in ({'And':[]},{'Sequential':[]},{'OneOf':[]})]
print('empty requirement quests:',bad)
print('total quests:',len(qs))
"
```

**预期**：`empty requirement quests: []`，且 api_no 1033 **不再出现在 codex 中**
（它被跳过了）。同时记录任务总数的变化——被跳过的任务数就是总数的减少量，
这个数字要写进提交信息。

**验证**：空需求列表为空

### 步骤 6：确认没有把好任务误杀

被跳过的任务数应当**很小**。如果步骤 5 显示任务总数减少超过 5 条，说明某个降级点
被误判成了 A 类（实际是合法的「本来就没有」）。逐条看 `warn!` 日志里的 `reason`，
把误判的那类改回 B 类。

**验证**：跳过的任务数 ≤ 5，且每一条都能从 `warn!` 的 reason 说清为什么该跳过

### 步骤 7：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo test --test gameplay_tests` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改了范围内文件

## 测试计划

见步骤 4。新增测试写在
`crates/emukc_bootstrap/src/parser/tsunkit_quest/requirement/mod.rs` 现有的
`#[cfg(test)] mod tests` 里，照它现有 4 个测试的写法。

**不要**给 `emukc_gameplay` 或 `emukc_model` 补测试——本计划不改那两个 crate。
如果 `cargo test --test gameplay_tests` 因为 codex 里少了几条任务而失败，
那是真实的影响，按 STOP 条件汇报，不要改测试去迁就。

## 完成标准

- [ ] `.data/codex/quest.json` 中不存在 requirements 为空的任务
      （步骤 5 的脚本输出 `[]`）
- [ ] api_no 1033 不再出现在 codex 中
- [ ] 被跳过的任务数 ≤ 5，且每条都有说明原因的 `warn!`
- [ ] 新增的三个解析期测试存在并通过
- [ ] `crates/emukc_model/src/thirdparty/quest/progress.rs` **未被修改**
- [ ] `cargo test` 与 `cargo test --test gameplay_tests` 都 exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 002 未完成。
- 步骤 6 中被跳过的任务数 > 5，且逐条检查后发现它们**都该跳过**——那说明上游
  数据里不认识的结构比预期多得多，跳过这么多任务是产品决策，停下来汇报清单。
- `cargo test --test gameplay_tests` 因为任务数变化而失败——汇报具体哪个测试、
  依赖哪条任务，不要修改测试期望。
- 发现 `progress.rs` 的 `total == 0 → Completed` 有其它合法依赖方，而本计划的
  解析期修复不足以覆盖——汇报，这会把问题升级成需要运行期防线的设计讨论。
- 步骤 2 中某个降级点的 A/B 分类判断不了——汇报那一处，不要猜。

## 维护须知

- 本计划修的是**解析期**：坏数据不进 codex。运行期的
  `total == 0 → Completed` 规则原样保留，因为可能有合法的无条件任务。
  如果将来发现确实没有合法无条件任务，可以再考虑在运行期加断言——那是独立议题。
- 14 个降级点的 A/B 分类是本计划最容易出错的地方。评审时逐个看：改成 A 类的每一处，
  上方原本是不是都有一条「must have XXX」的 `error!`？有就是 A 类。
- 上游 tsunkit 每次新增需求结构，都可能让跳过的任务数上升。`warn!` 的
  `reason` 字段就是为此设计的——它应该足以直接指出缺了哪个字段。
- 明确推迟：`From<List> for Requirements`（`types.rs:241-285`）硬编码了 10 个
  `None`，导致上游的 `fleet_id`、`secretary` 等嵌套字段被静默丢弃，
  api_no 1160 的 `fleet_id` 因此是 0 而非上游声明的 1。那是独立的一条，
  本计划不碰。
