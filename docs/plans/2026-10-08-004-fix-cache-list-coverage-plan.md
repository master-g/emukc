---
title: "Cache List Coverage - Plan"
type: fix
date: 2026-10-08
status: implemented
execution: code
---

# Cache List Coverage - Plan

## Problem

缓存清单由解码器从 `main.js` 抽出的规则加 start2 数据展开而成，但它不知道自己漏了什么：
`cache_rules.json` 自报"未解决规则 0"，而 2026-10-08 的无头运行（`make headless-check SCENARIO=fresh_1_1`，
211 个资源请求）发现两件事：

- 清单里没有 `kcs2/resources/ship/full_animation/` 这一类（0 条）。客户端在母港旗舰的
  `api_mst_shipgraph.api_sp_flag` 为 1 时加载它，每艘三个文件（`_data.json`、`.json`、`.png`），中破另有一组
  `full_animation_dmg`。start2 里现有 10 个带该标记的条目。文件之所以没报错，是平时游玩回源补进了缓存。
- 客户端向站外请求库文件，共 8 个地址：仓库自带的 `assets/www/emukc/game/index.php` 引 cdnjs 的 axios 0.19.2、
  tweenjs 0.6.2、pixi.js 4.8.8、howler 2.2.0；缓存里的官方 `kcs2/world.html` 引 cdnjs 的 axios 0.19.0、
  pixi.js 4.5.1、howler 2.0.9 和 `code.createjs.com` 的 tweenjs 0.6.2。断网时游戏起不来。
  `assets/www/emukc/game/assets/js/libs/` 下已有这些库的本地副本，只缺 axios 0.19.0。

用户 2026-10-08 定：不走"把地图和编成枚举一遍"的路线，从解码后的代码怎样用版本号和 start2 拼地址出发改进生成；
站外库随本计划一并处理。

## Decision

- **补规则的依据是客户端代码，不是抓到的请求。** 无头运行只用来发现漏项和事后确认，每个漏项都回到
  `main.decoded.js` 读出拼地址的条件，写成由 start2 展开的规则。
- **让漏项自己暴露。** 客户端的资源地址都经过少数几个出口（`ShipLoader`、`SlotLoader`、`SuffixUtil.create`、
  地图与血条的加载器、音频、`UIImageLoader`）。解码器列出所有调用这些出口的位置和它们用的资源类型字面量，
  与已有规则对账，对不上的写进 `unresolvedRules`。"未解决 0"从此表示对过账，而不是没查。
- **后缀与版本号同客户端对拍，不改生成方式。** Rust 侧保留自己的实现；新增对拍脚本用 `client-runtime.ts` 加载客户端，
  让它的 `SuffixUtil` / `VersionUtil` 对 start2 的每个舰、装备 ID 算地址，与生成的清单比较。做法同
  `gear-bonus-oracle`：手动跑，`make update` 之后不阻断地跑一次。
- **站外库由本服务器提供。** `index.php` 是仓库自己的模板，直接改成指向本地副本；`world.html` 是缓存里的官方文件，
  不改缓存，在提供它的时候把那四个 `<script src>` 换成本地地址。
- 不做：运行时数据才能决定的资源（活动图、家具范围等）的枚举方式不变；不动缓存层的回源逻辑；
  不把无头检查变成质量门。

## Implementation Units

### U1 舰船动画进清单

清单生成为每个 `api_sp_flag == 1` 的 shipgraph 条目产出 `full_animation` 与 `full_animation_dmg` 各三个文件，
后缀与版本号用现有的舰船路径函数。先读 `main.decoded.js` 第 33400 行附近的 `_loadFlagShipAnimation`
确认 `_dmg` 的文件名形态和 `_data.json` 的版本参数。10 个条目里只有 951 是己方舰，其余 ID ≥ 2000，
是否在官服存在由 `cache populate` 的结果说明；不存在的按 `ABYSSAL_ITEM_UP_HOLES` 的先例记成已知空洞。

完成标志：`make cache-make-list` 后清单含 `0951_8344_uocopczppbln` 的三个文件；单元测试断言一个带标记的条目
产出六条路径、不带标记的产出零条。

### U2 调用点对账

`main-decoder` 新增一步：找出所有调用资源出口的位置，取出资源类型字面量（如 `"full_animation"`、`"banner_g_dmg"`），
与 `cache_rules.json` 已覆盖的类型集合相减，差集写入 `unresolvedRules`（类型、所在模块号、出口名）。
同步进 `crates/emukc_bootstrap/assets/`。类型由变量传入而取不到字面量的调用点单独列为"无法静态判定"，不计入差集。

完成标志：在 U1 之前的规则上跑，报告里出现 `full_animation`；U1 之后它消失。报告里剩下的每一项要么补规则，
要么在计划的 Outcome 里写明为什么不需要。

### U3 地址对拍

`main-decoder/src/cache-list-oracle.ts` 加 `make cache-list-oracle`：对 start2 里每个舰与装备 ID、每个已覆盖的资源类型，
用客户端函数算出地址，与清单比较，差异写到 `.data/temp/`。接进 `make update`，不阻断。

完成标志：对当前清单差异为 0，或每条差异都有解释并修掉。

### U4 站外库本地化

补 axios 0.19.0 的本地副本；`index.php` 的四个地址改为本地；`kcs2/world.html` 在提供时改写那四个地址
（只匹配这四个已知地址，匹配不到时原样返回并记一条警告，官方改了页面能看见）。

完成标志：单元测试覆盖改写；无头报告的 `off_site` 为空。

### U5 确认与沉淀

`make headless-check` 跑 `fresh_1_1` 与 `transport_5_6`，`not_in_cache_list` 与 `off_site` 均为空。
更新 `docs/solutions/best-practices/decoder-first-cachelist-pipeline.md`（对账与对拍）、
`headless-client-check.md`（去掉已修的发现）、`CLAUDE.md` 命令节、`PROJECT_MEMORY.md`。

## Open Questions

- U2 报告会列出多少未覆盖的类型，现在不知道；工作量由它决定，可能需要拆后续计划。
- ID ≥ 2000 的九个动画条目在官服是否真有文件。
- `world.html` 之外，缓存里的其他官方页面（`gadget_html5`）是否也引站外地址；U4 时用无头报告和 grep 一并查。

## Verification

- 三道质量门（clippy 基线 17）；`cd main-decoder && bun run check && bun test`。
- `make cache-make-list` 前后清单的条目数差异在 PR 里说明；再生的 `cache_rules.json` 等资产同样说明。
- `make cache-list-oracle` 的结果；两个无头场景的资源报告。
- 注意：`make-list` / `populate` 与无头检查都要先停掉占着 `z/cache` 锁的服务器；`populate` 会向官服下载新增条目，
  执行前告知用户条目数。

## Outcome

- U1：动画进清单。带标记的 10 个条目里 9 个是深海舰（不会当旗舰，官服也没有文件），规则收紧到玩家能拥有的舰，
  现在只有 951 的 6 个文件。951 的三条与无头运行里客户端请求的地址一致。
- U2 与 U3 合成一个脚本 `cache-list-oracle`，没有写进 `cache_rules.json` 的 `unresolvedRules`：对账的对象改成
  "客户端源码里的目录字面量 × 生成出的清单"，比对规则集合更直接。28 个目录里 4 个为空：三个登记为合理
  （活动与节分的面板、服务器自己画的镇守府名），一个是真漏项 `area/airunit_extend_confirm`，已补（常规海域 6、7 各 2 文件；
  活动海域 58 官服没有）。后缀对拍 32,519 条，0 差异。版本号没有对拍（客户端的 `VersionUtil` 要先装载主数据模型）。
- U4：`index.php` 指向已内嵌的副本；`world.html` 在提供时改写。axios 0.19.0 用 0.19.2 的副本顶替，没有下载新文件。
  无头运行的 `off_site` 从 8 降到 0。
- 清单 73,082 → 73,092 条（+10），10 条都已从官服取到。第一版多出的 56 条在官服 404，按上面的原因从规则里排除。
- U5：两个无头场景的 `off_site`、`not_in_cache_list`、`fetched_from_origin`、`missing_on_origin` 全为空。
