---
title: "Headless Client Check - Plan"
type: feat
date: 2026-10-08
status: implemented
execution: code
---

# Headless Client Check - Plan

## Problem

凡是"客户端会不会照这个包正确演出"的问题，现在只能靠用户开浏览器打一遍。5-6 输送血条留下三处这样的推断
（`api_landing_hp` 的解析、揚陸点 `(9, 1, 9)` 的分支、类型 3 的血条图片）。用户 2026-10-08 要求改成无头验证，
不再需要他介入。

## Evidence

2026-10-08 在草稿目录探测过（独立工作区、端口 27777、复用 codex 与 `z/cache`）：

- Playwright 1.58（`~/.local/pipx/venvs/playwright`）加本机 Chrome（`channel="chrome"`，无头）能把真实客户端
  渲染出来：选服、GAME START、教程命名画面依次到达，无 JS 异常，无失败请求。
- 按坐标点击画布有效，服务器收到对应的 KCSAPI 请求。
- 用请求拦截把 `/kcs2/js/main.js` 换成 `main-decoder/out/main.decoded.js`，并在入口语句前插入
  `globalThis.__clientRequire = <require>`（与 `client-runtime.ts` 同一条正则），游戏照常启动，页面里能调用内部模块。
- 会话令牌走 REST：`/api/v1/auth/{sign-up,new-profile,start-game}`。`auto` 与 `new-session` 会调 `open::that`
  弹出用户的浏览器，不能用。

没验证：新档进教程之后的流程；时间加速。

## Decision

分两层，各管各的问题。

- **A 层（Bun，已有基础）**：`client-runtime.ts` 加载解码包，把服务器的响应喂给客户端自己的模型类，断言读出的值。
  秒级、确定性。回答"客户端把这个包读成什么"。
- **B 层（无头浏览器，新增）**：真服务器加真客户端，脚本驱动一段流程。判定靠四个信号：页面异常、控制台报错、
  状态码 ≥400 的请求、KCSAPI 请求序列及其中的字段；截图只在失败时留证。回答"场景衔接、资源加载、动画阶段出不出错"。
- **档案准备**做成 CLI 命令直接写库（用户 2026-10-08 确认），不加调试 REST 接口，运行中的服务器上不出现这个入口。
- **隔离**：B 层每次在 `.data/temp/headless/<run>/` 建工作区（拷 codex，新库），独立端口，不带 TLS；
  永不打开有界面的浏览器，永不读写 `.data/emukc.db`。
- **不进质量门**：B 层依赖 7 GB 资源缓存和已解码的 `main.js`，是改完玩法后手动跑的 `make` 目标。A 层的检查同
  `gear-bonus-oracle` 一样是手动诊断，不进 `cargo test`。
- 不做：通用的 UI 自动化框架、截图比对、把现有玩法测试搬到浏览器里。

## Implementation Units

### U1 A 层关掉 5-6 的两处推断

新增 `main-decoder/src/sortie-replay.ts`（脚本 `bun run sortie-replay`）：读 `EMUKC_DUMP_DIR` 里一次 5-6 出击的转储
（由 `map_5_6_empties_its_transport_gauge_by_what_the_fleet_lands` 一类的玩法测试产生），用客户端的模型类读
mapinfo、`next` 和 `battleresult`，报告客户端读到的血条类型与长度、揚陸点格子的类型、`api_landing_hp` 三个值。
模块按导出名找（`moduleExporting`），不写死模块号。

完成标志：脚本对 5-6 转储输出的值与服务器结算一致；任一处读不到则报出字段名。两处推断在
`docs/solutions/architecture-patterns/map-gauge-phases.md` 的 Known gaps 里改成已验证或记下差异。

### U2 档案准备命令

`cargo run -- dev seed-profile --name <n> --pass <p> --scenario <s>`：建账号与档案，按场景文件写入舰船、装备、
编成、已通关地图与当前血条阶段，标记教程已完成，最后打印会话令牌（不开浏览器）。场景是
`tests/headless/scenarios/*.toml`，第一份是 `map_5_6_transport.toml`。写库全部走 `Ctx` 上已有的操作与 `_impl`。

完成标志：对新库执行后，`api_port/port` 返回的舰队与场景一致，`api_get_member/mapinfo` 里 5-6 可出击且
`api_gauge_type` 为 3。

### U3 B 层驱动脚本

`tests/headless/run.py`（Playwright Python）加 `make headless-check SCENARIO=map_5_6_transport`：
建工作区 → 起服务器 → `seed-profile` → 无头 Chrome 加载（换入解码包并挂 `__clientRequire`）→ 到母港 →
出击到 boss 战结果 → 关服务器。流程里的每一步优先调用客户端内部代码，调不动的地方才点坐标。
工作区的 game config 打开 `god_mode` 与 `one_hit_kill`，战斗短且必 S 胜。
报告写到工作区：四个信号、KCSAPI 序列、失败时的截图；有异常或失败请求则退出码非 0。

完成标志：`make headless-check SCENARIO=map_5_6_transport` 无人值守跑完，报告里有 `api_req_sortie/battleresult`
且带 `api_landing_hp`，无页面异常，无失败请求；故意把 `api_landing_hp` 去掉重跑能看到差别（异常，或确认客户端静默
跳过并记录）。

### U4 沉淀

`docs/solutions/best-practices/headless-client-check.md` 记两层各自能回答什么、怎么加场景、已知限制；
`CLAUDE.md` 命令节加一行；`PROJECT_MEMORY.md` 记前置条件与"不进质量门"的决定。

## Open Questions

- 教程能否只靠库里的标记跳过，还是要让脚本走完教程（U2 时确认）。
- 能否加速时间（注入脚本放大 `performance.now` 与 `requestAnimationFrame` 的步长）；不行则一次出击按真实动画时长跑，
  预计几分钟，可以接受。
- 从母港到出击这段，客户端的场景切换能否直接调用；不行就点坐标（1200×720 画布，坐标固定）。

## Verification

- U1：`cd main-decoder && bun run check && bun test`，`bun run sortie-replay` 对 5-6 转储的输出。
- U2：一条玩法测试断言场景写入后的 port 与 mapinfo。
- U3：`make headless-check SCENARIO=map_5_6_transport` 的报告。
- 三道质量门不变（clippy 基线 17）。

## Outcome

- U1：三处推断读客户端代码与资源就全部确认，回放脚本没有写（结论记在 `map-gauge-phases.md` 的 Known gaps）。
- U2：没有新命令也没有场景文件；`new-session` 加了 `--scenario`（复用 `scenario::PRESETS`）与 `--no-open`，
  新增预设 `transport_5_6`。教程靠 `firstflag` 加 `tutorial_progress` 100 跳过。
- U3：`tests/headless/run.py` 与 `make headless-check`；全程点坐标，没用到 `__clientRequire`（已挂好）。
  `transport_5_6` 无人值守跑通：A、C2、D、揚陸点 E、boss G，结算 `api_landing_hp` 为 280 中卸 40。
  没做"去掉 `api_landing_hp` 再跑"的反向试验；时间加速没试。
