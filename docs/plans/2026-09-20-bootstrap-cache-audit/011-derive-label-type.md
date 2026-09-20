# Plan 011: 用 release_date 推导年任务 label_type，未命中改为硬错误

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/parser/tsunkit_quest`

## 状态

- **优先级**: P1
- **工作量**: S
- **风险**: LOW
- **依赖**: 002
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

年任务（`By*` / `Cy*`）的 `label_type` 由一张硬编码的「任务编号 → 月份」表决定。
这张表 `B` 侧只枚举到编号 13、`C` 侧只到 12，而上游现在各有 16 个。**7 个年任务
落在表外**：By14、By15、By16、Cy13、Cy14、Cy15、Cy16。

未命中时函数 `return 1`。而 `crates/emukc_gameplay/src/game/view/quest_list.rs:160-167`
里 `label_type == 1` 归到客户端的「一次性」标签页，`101..=112` 才归到「季/年」
标签页。所以这 7 个年任务在游戏里显示在**错误的标签页**下。
（重置周期本身不受影响，那个来自 `frequency` 字段。）

表会继续落后——上游每加一个年任务就多一条。所以本计划不是「把表补到 16」，
而是换一个不会过期的推导来源。

## 当前状态

`crates/emukc_bootstrap/src/parser/tsunkit_quest/label_type.rs:29-71`：

```rust
    match period.as_str() {
        "d" => return 2,
        "w" => return 3,
        "m" => return 6,
        "q" => return 7,
        "y" => match category.as_str() {
            "B" => {
                // (label_type, [quest_number])
                // label_type, 100 + month, eg. 101 for January, 102 for February etc.
                let table = [
                    (101, vec![13]),
                    (102, vec![1, 2]),
                    (103, vec![3, 4]),
                    (105, vec![11, 12]),
                    (106, vec![6, 7, 8, 9, 10]),
                    (107, vec![5]),
                ];
                if let Some(t) = table.iter().find(|(_, l)| l.contains(&num)).map(|(t, _)| *t) {
                    return t;
                }
                error!("Failed to find label type for wiki_id: {}", wiki_id);
                return 1;
            }
            "C" => {
                let table = [
                    (102, vec![3]),
                    (103, vec![4]),
                    (104, vec![10, 12]),
                    (105, vec![8]),
                    (106, vec![5, 9]),
                    (107, vec![6, 11]),
                    (110, vec![1, 2, 7]),
                ];
                // ... 同样的 find / error! / return 1
```

调用点 `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs:468`：

```rust
            label_type: extract_label_type(&self.wiki_id),
```

**关键事实（已实测验证）**：上游 tsunkit 记录里有 `release_date` 字段，格式
`YYYY-MM-DD`。用它的月份与现有硬编码表交叉比对，**27 条已映射记录里 26 条吻合**：

```
By1  release=2020-02-07 month=2  table=102  MATCH
By3  release=2020-03-27 month=3  table=103  MATCH
By5  release=2020-09-17 month=9  table=107  MISMATCH   ← 唯一例外
By6  release=2021-06-22 month=6  table=106  MATCH
By13 release=2024-01-25 month=1  table=101  MATCH
Cy1  release=2020-10-16 month=10 table=110  MATCH
Cy12 release=2024-04-10 month=4  table=104  MATCH
（其余 20 条同样 MATCH）
```

也就是说 `label_type = 100 + release_date 的月份` 是正确规则，**By5 是唯一需要
保留的特例**（表说 7 月，发布日期是 9 月）。

7 个未映射任务按该规则应得：

| wiki_id | game_id | release_date | 应得 label_type |
|---------|---------|--------------|------------------|
| By14 | 1012 | 2024-05-29 | 105 |
| By15 | 1018 | 2024-09-24 | 109 |
| By16 | 1045 | **None** | 见步骤 3 |
| Cy13 | 372 | 2024-06-27 | 106 |
| Cy14 | 373 | 2024-07-27 | 107 |
| Cy15 | 375 | 2024-09-24 | 109 |
| Cy16 | 377 | 2024-10-18 | 110 |

By16 的 `release_date` 是 `None`，它的 `updated` 是 `2026-05-26T...`。

tsunkit 记录的字段（顶层 key 是 game_id 字符串）：
`game_id`、`wiki_id`、`category`、`frequency`、`release_date`、`updated`、
`edited`、`prereqs`、`requirements`、`rewards`。

**仓库约定**：Rust edition 2024，软 tab 4 空格。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| bootstrap 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| label_type 测试 | `cargo test -p emukc_bootstrap label_type` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_bootstrap/src/parser/tsunkit_quest/label_type.rs`
- `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs`（只改 `extract_label_type`
  的调用点与 `TsunkitQuestValue` 的字段声明，如果 `release_date` 尚未被反序列化）

**范围外**：

- `crates/emukc_gameplay/src/game/view/quest_list.rs` 的 tab 路由逻辑——它是对的，
  错的是喂给它的 `label_type`。
- 非年任务的分支（`d`/`w`/`m`/`q`）——它们不依赖表，不要动。
- `frequency` → `period` 的映射——与本计划无关。

## Git 工作流

- 分支：`fix/derive-label-type-from-release-date`
- 提交信息：`fix(bootstrap): derive yearly quest label_type from release_date`
- 正文列出 7 个受影响的 wiki_id 及其新旧 label_type。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：确认 release_date 已被反序列化

检查 `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs` 里
`TsunkitQuestValue` 结构体是否已经有 `release_date` 字段。没有就加上：

```rust
    release_date: Option<String>,
```

serde 默认忽略未知字段，所以之前没有它也不会报错。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 2：把 release_date 传给 extract_label_type

`extract_label_type` 的签名从 `(wiki_id: &str)` 改成同时接受 release_date，
例如 `(wiki_id: &str, release_date: Option<&str>)`。更新
`mod.rs:468` 的调用点。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 3：改写年任务分支

`"y"` 分支的新逻辑，按优先级：

1. **特例表优先**：`By5` → 107。用一个显式的小表承载**已知与发布月份不符的例外**，
   目前只有这一条。表上方注释说明：该条目的发布日期是 2020-09-17（9 月）但
   label_type 是 107（7 月），已与现有硬编码表交叉验证过 27 条记录，只有它不符。
2. **从 `release_date` 推导**：解析 `YYYY-MM-DD` 的月份 `m`，返回 `100 + m`。
   解析失败则进入第 3 步。
3. **`release_date` 缺失**（当前只有 By16）：这是本计划最需要判断的一处。
   `return 1` 会让它落到「一次性」标签页，是**明确错误**的。
   改为：记一条 `warn!`（带 wiki_id），返回一个**落在 101..=112 区间内**的值，
   这样至少标签页是对的。取值方式二选一，在代码注释里写明选了哪个及理由：
   - (a) 退回到旧硬编码表（By16 不在表里，所以这条对它无效）
   - (b) 用 `updated` 字段的月份（By16 是 2026-05 → 105）
   - (c) 固定一个中性值，例如 101

   推荐 (c) 配合 `warn!`：它不假装知道月份，而标签页归类是正确的。
   (b) 看似更聪明，但 `updated` 是编辑时间，与重置月份没有语义关系。

**不要**保留原来那两张按编号映射的表作为主路径——那正是会继续过期的东西。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 4：翻转并扩充测试

计划 002 在 `label_type.rs` 的测试模块里留了 7 个带「计划 011 会翻转」注释的用例。
按上文表格翻转，删掉注释：

- By14 → 105、By15 → 109、Cy13 → 106、Cy14 → 107、Cy15 → 109、Cy16 → 110
- By16 → 按步骤 3 选定的方案断言，并断言它**落在 101..=112 内**

新增回归用例，防止推导规则改错：

- By5 → 107（特例仍然生效，**这条最重要**）
- By13 → 101、Cy12 → 104、Cy1 → 110（从 release_date 推导，与旧表一致）
- 一个 `d`/`w`/`m`/`q` 前缀的用例各一条（非年任务路径不回归）

**验证**：`cargo test -p emukc_bootstrap label_type` → 全部通过

### 步骤 5：全量交叉验证

写一个临时脚本（放 `/tmp`，**不要进工作树**），对 `.data/temp/tsunkit_quests.json`
里**所有** `By*` / `Cy*` 记录跑新逻辑，断言：

- 每一条的结果都落在 `101..=112`
- 除 By5 外，每条结果都等于 `100 + release_date 的月份`
- 没有任何一条返回 `1`

**验证**：脚本输出无违例

### 步骤 6：重新生成 codex 并核对

```
cargo run --release -- bootstrap --overwrite
```

**不要带 `--force-update`**（见计划 005）。

```
python3 -c "
import json
d=json.load(open('.data/codex/quest.json'))
qs=d if isinstance(d,list) else (list(d.values()) if isinstance(d,dict) else d)
bad=[(q['api_no'],q.get('wiki_id'),q.get('label_type')) for q in qs
     if str(q.get('wiki_id','')).startswith(('By','Cy')) and not (101<=q.get('label_type',0)<=112)]
print('yearly quests with wrong label_type:',bad)
"
```

**预期**：输出 `[]`（修复前会列出 7 条 label_type 为 1 的）。

**验证**：列表为空

### 步骤 7：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只改范围内文件

## 测试计划

见步骤 4。测试写在 `label_type.rs` 的测试模块里（计划 002 已建好）。
覆盖：7 个原本未映射的、By5 特例、3 个从 release_date 推导且与旧表一致的、
4 个非年任务前缀。

## 完成标准

- [ ] `label_type.rs` 不再含按任务编号映射的两张表（By5 特例表除外）
- [ ] 7 个原本落到 `label_type = 1` 的年任务，现在都在 `101..=112` 内
- [ ] By5 仍然返回 107
- [ ] 计划 002 留下的「计划 011 会翻转」注释已全部消除
- [ ] 步骤 5 的全量交叉验证无违例
- [ ] 步骤 6 的 codex 核对输出 `[]`
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 002 未完成。
- 步骤 5 的交叉验证发现**除 By5 之外**还有记录不符合「月份 = label_type - 100」——
  汇报清单。那说明推导规则有本计划未发现的例外，需要先弄清楚再继续。
- `TsunkitQuestValue` 加 `release_date` 字段后反序列化失败——汇报错误。
- 步骤 6 的 codex 核对不为空——汇报剩下的是哪些。

## 维护须知

- 这次改动的核心价值是**去掉一张会过期的表**。以后上游加年任务不需要改代码。
  评审时确认新实现没有把编号映射表换个写法留下来。
- By5 是唯一特例，它的存在说明「发布月份」和「重置月份」偶尔会不一致。如果以后
  再发现类似的不一致，加进特例表并在注释里记录验证方式，而不是推翻推导规则。
- `release_date` 缺失的处理（步骤 3 第 3 条）是有意保守的：它不猜月份，只保证
  标签页归类正确，并留下 `warn!`。如果以后 By16 的 release_date 被上游补上，
  这条 warn 会自动消失。
- 明确不做：让 `label_type` 影响重置周期。周期来自 `frequency`，两者无关，
  不要把它们耦合起来。
