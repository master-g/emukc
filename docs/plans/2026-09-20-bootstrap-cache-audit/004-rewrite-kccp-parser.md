# Plan 004: 重写 kccp 任务解析器，消除状态机失步

> **执行者须知**：逐步执行，每步跑验证命令确认预期后再继续。触发「STOP 条件」立即
> 停止汇报。完成后更新 `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- crates/emukc_bootstrap/src/parser/kccp`
> 有变化就先比对下文代码摘录。

## 状态

- **优先级**: P0
- **工作量**: S
- **风险**: LOW
- **依赖**: 002（需要 fixture 与锁定当前行为的断言）
- **类别**: bug
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

`kccp_quests.json` 是任务**名称和描述的唯一来源**。当前解析器是一个按行驱动的
三状态机，假定每个任务固定占三行（id / name / desc）。实际数据里有 13 个任务不是
三行，每碰到一个，状态机就失步一次，**连带毁掉下一个任务**。

后果已经在发布产物里：`.data/codex/quest.json` 当前含

- **8 条任务的 `detail` 是字面量** `"_quest_id_257"`、`"_quest_id_616"` 等内部 key
  （api_no 256、615、622、627、630、632、648、652），玩家看到的是这串东西；
- **12 条任务的 `name` 是 `"n/a"`**（api_no 257、616、623、628、631、633、649、653，
  外加 20326、160313、221114、230326 四条上游确实没有的）。

bootstrap 日志里那 8 行 `quest info not found: NNN` ERROR 就是这个原因，且
bootstrap 仍然 exit 0。

数据从来没缺过：`kccp_quests.json` 里 771 个 id 一个不少。这是本仓库的解析器 bug。

## 当前状态

`crates/emukc_bootstrap/src/parser/kccp/quest.rs:39-95`，解析器全貌：

```rust
pub fn parse(raw: &str) -> Result<BTreeMap<i64, KccpQuestInfo>, ParseError> {
    let reg_id = Regex::new(r"_quest_id_(\d+)").unwrap();
    let reg_desc = Regex::new(r#""([^"]+)""#).unwrap();

    let mut status = ParserStatus::Id;
    // ...
    for line in raw.lines() {
        match status {
            ParserStatus::Id => {
                if let Some(caps) = reg_id.captures(line)
                    && let Some(matched) = caps.get(1)
                {
                    quest_id = Some(matched.as_str().parse().unwrap());
                    status = ParserStatus::Name;
                }
            }
            ParserStatus::Name => {
                if let Some(name) = line.split("\":").next() {
                    quest_name = Some(name.trim_start().replace('"', ""));
                    status = ParserStatus::Desc;
                }
            }
            ParserStatus::Desc => {
                let mut matches = reg_desc.captures_iter(line);
                if let Some(cap) = matches.next() {
                    // ... insert into result ...
                    status = ParserStatus::Id;
                }
            }
        }
    }
    Ok(result)
}
```

**根因**：`ParserStatus::Name` 分支的 `line.split("\":").next()` **恒为 `Some`**
——`str::split` 的第一个元素永远存在，即使分隔符不出现。所以任何一行都会无条件
把状态推进到 `Desc`，包括下一个任务的 id 行。

**失步的具体过程**（以 615/616 为例，源数据里 615 只有 id 行 + 一条描述行）：

1. `Id` 状态匹配 `_quest_id_615`，状态 → `Name`
2. `Name` 状态读到 615 的**描述**行，把描述当成了 name，状态 → `Desc`
3. `Desc` 状态读到 `"_quest_id_616": "_quest_code_F15",`，正则 `"([^"]+)"` 匹配到
   `_quest_id_616`，于是把 615 的 desc 写成字面量 `"_quest_id_616"`，状态 → `Id`
4. `Id` 状态读到 616 的描述行——不含 `_quest_id_`，跳过
5. 下一个匹配到的是 `_quest_id_617`。**616 就此彻底消失**

**源数据的真实结构**（`.data/temp/kccp_quests.json`，JSON 对象，key 日文 value 英文）：

| id 后的条目数 | 数量 | 含义 |
|---|---|---|
| 2 | 757 | 正常：name + desc |
| 1 | 13 | 缺 name 或缺 desc |
| 3 | 1 | id 1169，多一个 `"dummy": "forNoComma"` 哨兵 |

13 个单条目组的归属**已逐条人工判定**，可直接作为实现依据和测试期望：

- **缺 name（唯一条目是描述）**，共 11 个：
  256、615、616、622、627、628、630、632、633、648、652
  这些的日文描述全部以「！」或「。」结尾，例如 256 是
  `潜水艦戦力を中核とした艦隊で中部海域哨戒線へ反復出撃、敵戦力を漸減せよ！`
- **缺 desc（唯一条目是短标题）**，共 2 个：1124、1125
  这两条都是【】开头的短标题，例如 1124 是 `【早春限定任務】夜間航空作戦能力の増強`

**消费方**（`crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs:453-456`）：

```rust
        let default_quest_info = KccpQuestInfo::default();
        let info = quest_info.get(&game_id).unwrap_or_else(|| {
            error!("quest info not found: {}", self.game_id);
            &default_quest_info
        });
```

`KccpQuestInfo::default()` 的 name 和 desc 都是 `"n/a"`（`kccp/quest.rs:16-24`）。

**仓库约定**：Rust edition 2024，软 tab 4 空格，`unsafe_code` 禁用。该 crate 已有
`serde_json` 依赖。

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 本 crate 测试 | `cargo test -p emukc_bootstrap` | 全部通过 |
| kccp 测试 | `cargo test -p emukc_bootstrap kccp` | 全部通过 |
| 全量测试 | `cargo test` | 全部通过 |
| 格式 | `cargo fmt --all --check` | exit 0 |
| Clippy | `cargo clippy --workspace -- -W warnings` | exit 0 无 warning |

## 范围

**范围内**：

- `crates/emukc_bootstrap/src/parser/kccp/quest.rs`
- `crates/emukc_bootstrap/Cargo.toml`（**仅当**需要为 serde_json 开启
  `preserve_order` feature 时）

**范围外**：

- `crates/emukc_bootstrap/src/parser/tsunkit_quest/mod.rs` 的消费逻辑——
  `quest info not found` 那条 ERROR 分支**保留**。修好解析器后它自然不再触发；
  它对上游真的缺数据的那 4 条（20326、160313、221114、230326）仍然是正确行为。
- `KccpQuestInfo::default()` 的 `"n/a"` 取值——不要改。
- `.data/codex/quest.json`——那是产物，重跑 bootstrap 自然更新，不要手改。
- 其它任何解析器。

## Git 工作流

- 分支：`fix/kccp-quest-parser`
- 提交信息：`fix(bootstrap): parse kccp quests by entry order instead of line position`
- 正文里写明：修复前 771 个 id 解析出 762 个、9 个 desc 被写成内部 key；修复后 771 个全部解析。
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：确认基线断言存在

确认计划 002 已完成：`crates/emukc_bootstrap/tests/fixtures/kccp/quests_sample.json`
存在，且 `quest.rs` 里有锁定当前错误行为的测试。

```
cargo test -p emukc_bootstrap kccp
```

**验证**：当前测试全部通过（它们断言的是**错误**行为，这是预期的）

### 步骤 2：改用保序 JSON 解析

把 `parse` 的实现从「按行 + 正则 + 状态机」换成「保序 JSON 遍历」。

该文件是**合法 JSON 对象**，key 的出现顺序即语义顺序，所以必须保序解析。两种做法
任选，优先第一种：

1. 给 `serde_json` 开启 `preserve_order` feature，反序列化成
   `serde_json::Map<String, Value>`（该 feature 下它由 `IndexMap` 支持，保序）。
   **注意**：feature 是 crate 级全局生效的，开启前先确认不会改变工作区里其它
   `serde_json::Map` 使用处的行为预期——如果有任何地方依赖 key 排序，改用方案 2。
2. 不加 feature，自己做一次轻量的保序扫描：仍然按行读，但只用 JSON 转义规则提取
   每行的 key 和 value，不做任何状态机跳转。

目标算法（与具体方案无关）：

- 顺序遍历全部条目。
- 遇到 key 形如 `_quest_id_(\d+)` → 开启一条新记录，记下 id。
- 其余条目归属当前记录，**但要先跳过哨兵条目**：key 为 `dummy` 且 value 为
  `forNoComma` 的条目直接忽略。
- 遇到下一个 `_quest_id_` 或输入结束 → 结算当前记录。

结算规则：

- 归属条目有 **2 个**：第一个的 key 是 name，第二个的 key 是 desc。
- 归属条目有 **1 个**：需要判定它是 name 还是 desc。判据：**日文 key 以「！」或
  「。」结尾，或长度大于 30 字符 → 视为 desc（name 缺失）；否则视为 name
  （desc 缺失）**。这条规则在当前全部 13 个样本上 13/13 正确。
  判定走这条分支时**必须 `warn!` 一行**，带上 id 和判定结果——上游结构一旦变化，
  日志里要看得见。
- 归属条目有 **0 个**：`warn!` 并跳过该 id。
- 归属条目 **超过 2 个**（哨兵已剔除后）：取前两个作 name/desc，`warn!` 其余被忽略。

缺失的一侧沿用现有默认值 `"n/a"`（与 `KccpQuestInfo::default()` 保持一致），
**不要**拿描述去填充 name。

删掉不再需要的 `ParserStatus` enum 和两个 `Regex`。

**验证**：`cargo build -p emukc_bootstrap` → exit 0

### 步骤 3：翻转基线断言

计划 002 在 `quest.rs` 测试模块里留下的、带「计划 004 会翻转」注释的断言，现在
逐条翻转成正确期望，并**删掉那些注释**：

- 615：`desc` 是那条日文描述（不再是字面量 `"_quest_id_616"`），`name` 是 `"n/a"`
- 616：**存在于结果中**，`desc` 是它的日文描述，`name` 是 `"n/a"`
- 1124：`name` 是 `【早春限定任務】夜間航空作戦能力の増強`，`desc` 是 `"n/a"`
- 哨兵组（1169 形状）：name 和 desc 都正确，`dummy` 条目不出现在任何字段里
- 总条目数断言：改成 fixture 里的 id 总数（**每个 id 都应产出一条记录**）

**验证**：`cargo test -p emukc_bootstrap kccp` → 全部通过

### 步骤 4：用完整真实数据核对

如果本地有 `.data/temp/kccp_quests.json`，写一个临时脚本（放
`/tmp`，**不要进工作树**）验证全量：

```
cargo test -p emukc_bootstrap kccp -- --nocapture
```

并另外确认：解析结果的条目数 == 源文件中 `_quest_id_` 的个数（771）。可以临时加一个
`#[ignore]` 的测试从 `.data/temp/` 读全量文件来断言这一点，**跑完后删掉它**
（不要把依赖 `.data/` 的测试留在仓库里，那正是计划 002 在清理的东西）。

**验证**：全量解析出 771 条记录，`warn!` 行数恰好 13（11 条判定为缺 name，
2 条判定为缺 desc）

### 步骤 5：重新生成 codex 并核对产物

```
cargo run --release -- bootstrap --overwrite
```

**注意不要带 `--force-update`**——那个 flag 会删除 `z/cache/kcs2/js/main.js`，
且在 CDN 不可达时无法恢复（见计划 005）。本步只需要重新解析第三方数据。

然后核对产物：

```
grep -c '"_quest_id_' .data/codex/quest.json
```

**预期输出 0**（修复前是 8）。

再核对 `"n/a"` 的数量：

```
python3 -c "
import json
d=json.load(open('.data/codex/quest.json'))
qs=d if isinstance(d,list) else (list(d.values()) if isinstance(d,dict) else d)
print(sorted(q['api_no'] for q in qs if q.get('name')=='n/a'))
"
```

**预期**：不再包含 257、616、623、628、631、633、649、653 这 8 个（它们现在有
正确的 desc，name 仍是 `"n/a"` 属于源数据确实缺 name，是正确结果）。
上游确实没有的 4 条（20326、160313、221114、230326）应当**仍在**列表里。

如果 `.data/codex` 不可用（没 bootstrap 过），跳过本步并在汇报中说明。

**验证**：`grep -c '"_quest_id_' .data/codex/quest.json` → 0

### 步骤 6：全量门禁

**验证**：
- `cargo test` → 全部通过
- `cargo fmt --all --check` → exit 0
- `cargo clippy --workspace -- -W warnings` → exit 0
- `git status` → 只有 `quest.rs`（可能加上 `Cargo.toml`）被改

## 测试计划

不新建测试文件——计划 002 已经建好了 fixture 和测试模块，本计划在其中翻转断言
并补充新增覆盖：

- 615/616 这一对（失步触发点）：两条都正确产出
- 1124（缺 desc）：name 正确、desc 为 `"n/a"`
- 哨兵组：`dummy` 被剔除
- 正常三段式：不回归
- 新增：一个「文件以 id 行结尾、后面没有任何条目」的用例，断言不 panic、该 id 被
  `warn!` 跳过

## 完成标准

- [ ] `crates/emukc_bootstrap/src/parser/kccp/quest.rs` 不再含 `ParserStatus`
      和按行状态机
- [ ] `cargo test -p emukc_bootstrap kccp` exit 0，且 002 留下的「会翻转」注释
      已全部删除
- [ ] fixture 中每个 `_quest_id_` 都产出了一条记录（无丢失）
- [ ] 没有任何 `desc` 或 `name` 的值形如 `_quest_id_NNN`
- [ ] 重跑 bootstrap 后 `grep -c '"_quest_id_' .data/codex/quest.json` 为 0
      （若跳过此步，需在汇报中说明原因）
- [ ] `cargo test` exit 0
- [ ] `cargo fmt --all --check`、`cargo clippy --workspace -- -W warnings` 都 exit 0
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 002 未完成（fixture 不存在）——先做 002，否则这次重写无从验证。
- 开启 `serde_json/preserve_order` 后工作区有其它测试失败——改用方案 2，
  不要为了让测试通过而去改其它 crate。
- 步骤 4 的全量核对中，`warn!` 行数**不是 13**，或者出现了上文 13 个 id 之外的
  单条目组——说明上游数据已变，汇报实际分布，不要调整判据去迁就。
- 步骤 5 中 `grep -c '"_quest_id_'` 不为 0——说明还有未覆盖的失步路径，
  汇报剩余的 id。
- 判定规则（「！」「。」结尾或长度 > 30）在 fixture 上就判错——停止并汇报，
  不要靠加特例 id 白名单来凑。

## 维护须知

- 单条目组的判定是**启发式**，依赖上游的写作习惯。它带 `warn!` 是刻意的：
  上游哪天改了格式，日志里会立刻看到判定数量变化。评审时确认这个 warn 没被降级
  成 debug。
- 如果以后 `warn!` 的行数明显偏离 13，说明上游结构变了，应当回来重看判据，而不是
  调阈值。
- 本计划**不**解决「name 缺失时显示 `"n/a"`」。那 11 条任务在游戏里会显示
  `n/a` 作标题、正确的描述作正文。要真正补上标题需要另一个数据源，属于独立议题。
- 评审重点：确认新实现是按**条目顺序**分组，而不是换了个写法的按行状态机。
  判断方法：在 fixture 里把某个任务的 name 和 desc 写到同一行（JSON 允许），
  解析结果应当不变。
