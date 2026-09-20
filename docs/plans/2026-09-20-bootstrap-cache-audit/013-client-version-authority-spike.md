# Plan 013: 设计单一权威的客户端版本记录（spike）

> **执行者须知**：这是一份**调研与设计**计划，不是实现计划。产出是一份写进
> `docs/plans/` 的设计文档加一个可运行的最小验证，**不要**动生产代码去实现完整方案。
> 触发「STOP 条件」立即停止汇报。完成后更新
> `docs/plans/2026-09-20-bootstrap-cache-audit/README.md` 状态行。
>
> **漂移检查（先跑）**：
> `git diff --stat 82d2203..HEAD -- src/bin/cli/drift_check.rs crates/emukc_bootstrap/assets/.sync-fingerprint.json main-decoder/src/io.ts`

## 状态

- **优先级**: P2
- **工作量**: M
- **风险**: LOW（只读调研 + 文档）
- **依赖**: 012（先把最小版本校验跑通，再谈统一）
- **类别**: direction
- **计划基线**: commit `82d2203`, 2026-09-20

## 为什么这件事重要

这条数据管线里有**四份互不比较的客户端版本记录**。缺少一份权威记录，是
「资产过期检测」「drift-check 复活」「`make update` 可信」这三件事共同的前置条件——
没有它，每个修复都退化成又一个临时的点对点检查（计划 012 就是其中一个）。

这份计划的目标不是立刻实现，而是**把选择摊开**，让维护者决定要不要收敛、收敛到
什么程度。

## 当前状态：四份记录

**记录 A — `z/cache/kcs2/version.json`**
逐子系统的资源版本（例如 `"title": "6.1.7.0"`、`"common": "6.3.2.1"`）。
`crates/emukc_bootstrap/src/make_list/source/kcs2/versioned/mod.rs:19-44` 解析它，
每次 make-list 都远程重取，**从不持久化用于比较**。

**记录 B — `kcs_const.js` 的 `scriptVesion`**（上游拼写如此）
客户端脚本版本。被**两种语言的两个正则**各解析一次：
`main-decoder/src/io.ts:7` 和
`crates/emukc_bootstrap/src/make_list/source/kcs2/plain.rs:19-20`。
产出 `main-decoder/out/version.txt`（被 gitignore）以及 9 个 decoder 资产里 8 个的
`scriptVersion` 字段。

**记录 C — `crates/emukc_bootstrap/assets/.sync-fingerprint.json`**
唯一提交进仓库的版本记录，当前内容开头是：

```json
{
  "version": "6.3.0.0",
  "assets": {
    "battle_module_index": "EKyKuciE7ZQ8rUYXhj1tn3eSa4N8RP4kAivvvgRF4hbQ",
```

只有 `src/bin/cli/drift_check.rs:247-258` 读写它。**它记的是 `6.3.0.0`，而资产
已经同步到 `6.3.5.0`**——中间三次同步都没跑过 `--accept`。

**记录 D — codex**：`.data/codex/` 下没有任何客户端版本字段。
`crates/emukc_bootstrap/src/parser/mod.rs:121-136` 从第三方数据构建 `Codex`，
不含版本信息。

**全仓库唯一的跨记录比较**是 `src/bin/cli/drift_check.rs:166`（C 与 B 比）。
A 与 B、A 与 C 从不比较——尽管 make-list **在同一个进程里同时持有 A 和 B**
（`plain.rs:36` 和 `versioned/mod.rs:57`）。

**drift-check 的现状**：只挂在 `battle drift-check` 子命令下
（`src/bin/cli/battle.rs:23-24, 41, 111`）。`Makefile` 和 `.pre-commit-config.yaml`
都没有引用它，仓库也没有 `.github/workflows/`。它跟踪 6 个资产
（4 个 battle + `wikiwiki_map_catalog` + `public_map_catalog_overlays`，
见 `drift_check.rs:190-199`），**其中没有任何一个 cache-list 输入**——
`resource_manifest.json`、`cache_rules.json`、`resource_categories.json`、
`resource_id_sets.json`、`audio_resources.json`、`ui_resources.json`、
`resource_templates.json` 全部不在跟踪范围内。

它还读 `main-decoder/out/version.txt`（`drift_check.rs:214-216`），而该路径被
gitignore（`main-decoder/.gitignore:5-8` 只放行 `out/battle/*.json`），
所以干净 clone 上它只能走 `VERSION_ABSENT` 分支。

**已有的相关制度知识**（阅读，不要在本计划中改写）：
- `docs/solutions/architecture-patterns/drift-check-baseline-refresh-boundary.md`
  记录的流程是「review `git diff` → `--accept` → 资产和基线一起提交」。
  代码已经三次绕过这个流程——按审计规则，**这是一条已经过期的决策记录**，
  要么流程改，要么代码改，本计划要给出建议。
- `docs/solutions/architecture-patterns/drift-check-sync-loop.md`
- `PROJECT_MEMORY.md:57-59` 记录了两个版本轴的关系：
  「`kcs_const.js` 的 `scriptVesion` 是客户端脚本版本，驱动 `out/version.txt` 和
  每个同步资产的 `scriptVersion`；`kcs2/version.json` 存逐子系统的资源版本，
  独立移动。只发布 main.js 的版本会动前者，不动后者。」

## 需要用到的命令

| 用途 | 命令 | 成功时的表现 |
|------|------|--------------|
| 看 drift-check 现状 | `cargo run --release -- battle drift-check` | 预期**非零退出**（基线已过期） |
| 查引用 | `grep -rn 'drift-check\|drift_check' Makefile .pre-commit-config.yaml src/ --include='*'` | 只在 `battle.rs` 命中 |
| 资产版本 | `grep -h '"scriptVersion"' crates/emukc_bootstrap/assets/*.json \| sort -u` | 列出资产侧版本 |
| 基线版本 | `head -3 crates/emukc_bootstrap/assets/.sync-fingerprint.json` | 显示 `"version"` |

## 范围

**范围内**（本计划只产出这些）：

- `docs/plans/2026-09-20-bootstrap-cache-audit/013-findings.md`（新建，设计文档）
- 一个**只读**的验证脚本，放 `/tmp`，不进工作树

**范围外**（本计划一行都不改）：

- 任何生产代码
- `crates/emukc_bootstrap/assets/` 下的任何文件，**包括** `.sync-fingerprint.json`
  ——刷新基线是一个需要人 review diff 的动作，不能在调研计划里顺手做
- `docs/solutions/**` 下的任何文件——本计划**建议**如何更新它们，
  由后续实现计划执行
- `Makefile`、`.pre-commit-config.yaml`

## Git 工作流

- 分支：`docs/client-version-authority-spike`
- 提交信息：`docs: record the client version authority findings`
- 不 push，不开 PR，除非操作者要求。

## 步骤

### 步骤 1：核实四份记录的现状

逐条跑「需要用到的命令」里的四条，把实际输出记下来。特别确认：

- `.sync-fingerprint.json` 的 `version` 与资产的 `scriptVersion` 差几个版本
- `cargo run --release -- battle drift-check` 的实际退出码和输出

**验证**：四条命令的输出都已记录

### 步骤 2：回答三个设计问题

在设计文档中，每个问题都要给出**带证据的结论**，不要只列选项：

**问题 1：权威记录应该记什么？**
候选字段：客户端 `scriptVersion`、`main.js` 的内容哈希、`kcs2/version.json` 的
内容哈希、各资产的哈希。

要处理的一个已知问题：decoder 把 `scriptVersion` 从 **`kcs_const.js`** 里取出来，
盖在从 **`main.js`** 解码出来的内容上（`main-decoder/src/io.ts:35-61`、
`pipeline.ts:58`）。两个文件走**不同的 CDN 组**
（`crates/emukc_bootstrap/src/download.rs:292-306`：`kcs_const.js` 走 `gadgets_cdn`，
`main.js` 走 `game_cdn`），可以一个成功一个失败。
`PROJECT_MEMORY.md:47` 记着确实发生过手工替换 `main.js` 的情况。
所以「资产标着版本 X」并不能保证「资产是从版本 X 的 main.js 解码出来的」。
**建议评估：记录 `main.js` 的内容哈希，让「同版本不同内容」可被发现。**

**问题 2：记录放在哪里？**
候选：扩展 `.sync-fingerprint.json`；新建一个 `client-state.json`；
放进 codex。要考虑：干净 clone 上能不能读到（`out/version.txt` 被 gitignore
这个坑不要再踩一遍）；谁写、谁读。

**问题 3：drift-check 怎么处理？**
三条路，选一条并说明理由：
- (a) **复活**：刷新基线、把 7 个 cache-list 资产加进
  `drift_check.rs:190-199` 的跟踪列表、加 `make drift-check`、接进 pre-commit
- (b) **缩小**：明确它只管 battle 资产，cache-list 侧用计划 012 的版本校验，
  两套机制各管各的
- (c) **替换**：用新的权威记录取代它

选 (a) 要面对 `drift-check-baseline-refresh-boundary.md` 记录的流程已被绕过三次
这个事实——如果流程本身太重，复活它只会被第四次绕过。**这一点必须在文档里正面
回答。**

**验证**：三个问题在文档里都有结论和依据

### 步骤 3：写一个只读的一致性检查脚本

写一个脚本（放 `/tmp`），读取四份记录并报告它们是否一致。这是设计的**最小可验证
原型**——如果连脚本都难写清楚，说明设计还没想明白。

脚本要输出：各记录的当前值、两两比较结果、以及「如果这个检查在 CI 里跑，今天会不会
红」。

**验证**：脚本能跑，输出四份记录的实际值和比较结果

### 步骤 4：写设计文档

在 `docs/plans/2026-09-20-bootstrap-cache-audit/013-findings.md` 里写：

1. 四份记录的现状（步骤 1 的实测输出）
2. 三个设计问题的结论（步骤 2）
3. 推荐方案，以及**明确的不推荐项**和理由
4. 落地拆解：如果采纳，拆成几个实现计划，每个的范围和验证方式
5. 成本估计：改动面、要动的文件、风险点
6. **不采纳的后果**：如果什么都不做，会继续发生什么（引用本次审计的证据）

文档要能让一个没参与本次审计的人读完就能决策。

**验证**：文档写完，六个部分齐全

### 步骤 5：确认没有副作用

**验证**：
- `git status` → 只有新建的 `013-findings.md` 和 README 的状态行改动
- 没有任何 `crates/`、`src/`、`main-decoder/src/` 下的文件被修改
- `crates/emukc_bootstrap/assets/.sync-fingerprint.json` **未被修改**

## 测试计划

本计划不产出生产代码，因此没有测试。步骤 3 的脚本是设计的可行性验证，
不进仓库，不需要测试。

## 完成标准

- [ ] `013-findings.md` 存在，六个部分齐全
- [ ] 三个设计问题各有带证据的结论，不是选项罗列
- [ ] 文档对「drift-check 的流程已被绕过三次」这一点有正面回答
- [ ] 步骤 3 的脚本跑过，实测输出已写进文档
- [ ] `git status` 确认零生产代码改动，`.sync-fingerprint.json` 未被修改
- [ ] README.md 状态行已更新

## STOP 条件

- 计划 012 未完成——先把最小版本校验跑通。它会暴露实际的使用摩擦，
  那些摩擦正是本计划要设计的输入。
- 调研中发现四份记录之外还有第五份——汇报，重新评估范围。
- 步骤 3 的脚本写不出来，因为某两份记录根本没有可比的公共字段——
  这本身是重要发现，写进文档并汇报。

## 维护须知

- 本计划**刻意不实现**。判断依据：四份记录的收敛方式会决定 drift-check 的存废、
  `make update` 的形态、以及资产版本字段的语义——这些是需要维护者拍板的结构性
  决定，不该由执行者在实现过程中顺手定下来。
- 如果维护者读完文档决定「不收敛，维持现状 + 计划 012 的点对点校验」，
  那也是一个有效结论。把它记进 `docs/solutions/`，下次就不用再调研一遍。
- 后续实现计划的编号从 014 起。
