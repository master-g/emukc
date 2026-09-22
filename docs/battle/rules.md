# EmuKC Battle Rules Register

> 这份文档只记录已经准备实现或已经实现到 battle core 的规则，不记录“可能存在但尚未成文”的猜测。

## Source Priority

规则提取按以下顺序建立证据：

1. 真实战报 / API 观测 / 可复现实例
2. `wikiwiki.jp`、`en.kancollewiki.net`
3. KC3Kai / poi 等成熟实现
4. 仓库内既有调研和测试

证据等级：

- `A`: 多信源一致，且能被实战或日志复现
- `B`: Wiki 与成熟实现一致，但缺少本地复现
- `C`: 单一社区来源，先实现并标记待复核

## Implemented

| Rule ID | Phase | Statement | Confidence | Sources | Status |
|---------|-------|-----------|------------|---------|--------|
| `targeting.surface_asw_prefers_submarine.day` | `DayShelling` | 能在昼战进行对潜攻击的水面舰，面对混编舰队时先只在潜水目标中选取目标 | `B` | `en.kancollewiki.net/Shooting_Order_and_Targeting`, `wikiwiki.jp` 对潜页, KC3Kai/poi 行为对照 | Implemented |
| `targeting.non_asw_surface_ignores_submarine.day` | `DayShelling` | 不能在昼战对潜的攻击舰，不会把潜水舰加入合法目标集合 | `B` | 同上 | Implemented |
| `targeting.torpedo_ignores_submarine` | `OpeningTorpedo`, `ClosingTorpedo` | 雷击阶段只在水面目标集合中选取目标，不会攻击潜水舰 | `B` | `en.kancollewiki.net/Shooting_Order_and_Targeting`, 社区实现对照 | Implemented |
| `targeting.surface_asw_prefers_submarine.night` | `NightShelling` | 具备夜间对潜能力的轻型水面舰，面对混编舰队时优先选潜水目标 | `C` | `en.kancollewiki.net/Shooting_Order_and_Targeting` 夜战条目, 社区资料 | Implemented |
| `damage.submarine_targeted_by_night_shelling_is_scratch` | `NightShelling` | 夜战普通炮击命中潜水舰时按 scratch damage 处理，而非普通夜战火力公式 | `C` | `en.kancollewiki.net/Shooting_Order_and_Targeting` | Implemented |
| `display.day_shelling_excludes_non_attack_equipment` | `DayShelling` | 昼战炮击的 `api_si_list` 只暴露与当前攻击直接相关的展示装备，不再回退到“前两个槽位” | `B` | decoded `PhaseHougeki`, 事故样例 `102 -> btxt_flat`, 本地回归测试 | Implemented |
| `display.day_asw_names_no_equipment` | `DayShelling`, `OpeningTaisen` | 对潜攻击的 `api_si_list` 为 `[-1]`。客户端把它交给 `PhaseAttackBakurai` / `PhaseAttackKansaiki`，两者以 `showTelop = false` 构造、根本不读 `api_si_list`；而声纳 / 爆雷 / 水上機在上游都没有 `btxt_flat`（2026-09-22 探测 46/47/132/25 全 404，对照 41/220 为 200） | `A` | decoded `PhasePreAntiSubmarine` / `PhaseAttackBakurai` / `PhaseAttackKansaiki`, CDN 单路径探测, 本地回归测试 | Implemented（取代早先的 `display.day_asw_prefers_asw_equipment`） |
| `display.night_attack_matches_attack_type` | `NightShelling` | 夜战 `api_si_list` 按 `NightAttackType` 返回主炮 / 副炮 / 鱼雷 / 雷达组合，不再沿用昼战展示逻辑 | `B` | decoded `PhaseHougeki`, `wikiwiki.jp`, 本地 CI/连击测试 | Implemented |
| `enemy.manifest_unknown_slot_is_dropped` | `SortieEnemyBuild` | 敌舰 bootstrap 装备若不存在于当前 manifest，则在 runtime 中丢弃并清零对应 onslot | `B` | 本地回归测试, runtime warning 设计 | Implemented |
| `enemy.manifest_only_fallback_zeroes_onslot_when_slots_missing` | `SortieEnemyBuild` | manifest-only fallback 敌舰在没有装备数据时必须返回 `api_onslot = [0; 5]`，保持响应内部自洽 | `B` | 本地回归测试, decoded consumer constraints | Implemented |
| `survival.sortie_non_taiha_sinking_protection` | `DamageApplication` | 出击战中，非大破入场的己方舰若受到致死伤害，不会在该战中被击沉；旗舰始终受保护 | `A` | `wikiwiki.jp` 轟沈ストッパー条目, 本地回归测试 | Implemented |
| `survival.practice_and_enemy_do_not_use_sinking_protection` | `DamageApplication` | 沉船保护只对 sortie friendly side 生效；演习与敌方结算不应用该保护 | `A` | `wikiwiki.jp`, 本地回归测试 | Implemented |
| `result.friendly_sinking_downgrades_win_rank` | `BattleResult` | 友军沉舰会影响胜利评级；己方沉舰时不能判定为 `S`，并按沉舰数量降到 `A/D/E` | `B` | `wikiwiki.jp`, 本地 battle result 回归测试 | Implemented |
| `result.sunk_friendly_ship_gets_no_exp` | `BattleResult` | 已沉友军舰不会继续获得 ship EXP，也不应以正常存活舰语义参与 sortie settlement | `A` | `wikiwiki.jp`, 本地回归测试 | Implemented |
| `payload.torpedo_damage_fields_are_directional` | `OpeningTorpedo`, `ClosingTorpedo` | 友军造成的雷击伤害写入 `api_fydam*`，敌军造成的雷击伤害写入 `api_eydam*`，二者不可互换 | `A` | decoded client behavior, 本地 torpedo payload 回归测试 | Implemented |
| `shelling.bb_class_gate_for_second_round` | `DayShelling` (Shelling2) | Shelling2 仅当任一方在战斗开始时拥有 BB 级舰船 (FBB/BB/BBV/XBB) 时执行 | `B` | `wikiwiki.jp`, `en.kancollewiki.net`, 本地测试 | Implemented |
| `torpedo.closing_rejects_chuha` | `ClosingTorpedo` | 中破 (HP ≤ 50%) 舰船无法参与闭幕雷击阶段 | `B` | `wikiwiki.jp`, `en.kancollewiki.net`, 本地测试 | Implemented |
| `attack_type.asw_is_normal` (R1) | `OpeningTaisen`, `DayShelling` | 对潜攻击上报 `api_at_type = 0`。7 是客户端的空母カットイン：开幕对潜里它落到 `PhaseAttackDanchaku` 的 `throw new Error()`，昼战里它会把驱逐舰投爆雷播成空母切入 | `A` | decoded `module-29177-phase-pre-anti-submarine.js` / `module-90992-phase-attack-danchaku.js`, `docs/apilist.txt:2260`, `crates/emukc_battle` 回归测试 | Implemented |
| `attack_type.within_consumer_acceptance` (R2) | `DayShelling`, `NightShelling`, `OpeningTaisen` | 每个 `api_at_type` / `api_sp_list` 取值都要落在该阶段消费模块的接受集内。三个阶段接受集不同：昼战炮击收 7，开幕对潜不收，夜战读 `api_sp_list` 且编号与昼战冲突 | `A` | `crates/emukc_bootstrap/assets/battle_attack_type_acceptance.json`（随 `main.js` 再生）, `sim_validation_gate` 反向测试 | Implemented |
| `display.names_only_equipment_with_a_name_plate` (R3) | `DayShelling`, `NightShelling`, `OpeningTaisen` | `api_si_list` 只放客户端画得出名牌的装备：連撃与炮击类切入走炮械集合，昼战普通攻击只放主炮 / 副炮。舰载机与水上機被排除——整个舰载机队里只有夜间机型有 `btxt_flat`（2026-09-22 探测 16/20/23/102 全 404） | `A` | decoded `module-19362-cutin-attack.js`（`showTelop`）, `module-482-preload-cutin-kubo.js`, CDN 单路径探测, `crates/emukc_battle` 回归测试 | Implemented |
| `resource.derived_paths_are_generated` (R4) | `DayShelling`, `NightShelling` | 战斗响应推导出的每条资源路径都必须落在 `make_list` 的生成覆盖内，未覆盖即错误级发现。名牌只从展示装备推导，不从 `api_eSlot` 推导；昼战空母切入不推导名牌（`PreloadCutinKubo` 只在 `night == 1` 时取） | `A` | decoded `module-482-preload-cutin-kubo.js`, `BTXT_FLAT_IDS` + `cache_rules.json`, `sim_validation_gate` 反向测试 | Implemented |

## First-Batch Ship Capability Model

当前已经进入规则层的首批舰种判定：

- 昼战天然对潜：`DE`, `DD`, `CL`, `CLT`, `CT`, `AO`
- 昼战装备触发对潜：`BBV`, `CAV`, `AV`, `LHA`, `CVL`
- 夜战天然对潜：`DE`, `DD`, `CL`, `CLT`, `CT`, `AO`
- 夜战装备触发对潜：当前仅对支持夜战的 `CV`, `CVL`, `CVB` 预留入口

当前作为潜水目标处理的舰种：

- `SS`
- `SSV`

`AS`（潜水母舰）当前按水面舰处理，不进入潜水目标类。

当前已显式落地的目标分类（safe slice）：

- `SurfaceShip`
- `Installation`
- `PT`
- `Submarine`

当前 legality 仍保持安全默认：`Installation` 与 `PT` 先并入 surface-like bucket，继续沿用现有 day / night / torpedo 的分桶；后续再单独补齐对陆 / 对海合法目标差异。

## Follow-up

- 引入真正 battle-ready 的 enemy master-data source，减少 manifest-only fallback
- 将 day/night 展示装备规则继续迁移为更正式的结构化规则表，减少 battle core 中的硬编码分支
- 特殊攻击（`api_at_type = 100`）的包形状：当前按每个参战舰发一条记录，官方是一条记录带三个目标。取值合法所以 R2 抓不到，需要重做参战舰索引与伤害分配并重新冻结黄金基线，见 `docs/plans/2026-09-22-1435-fix-battle-protocol-semantics-gate-plan.md` 的 Scope Boundaries
- 为 `api_si_list` 与 phase payload 建立更大的 incident corpus，沉淀更多“客户端会崩”的黄金样例
- 复核夜间对潜优先级与 scratch damage 的精确公式，提升到 `B/A`
- 基于已落地的 `Installation` / `PT` taxonomy，继续补齐对陆 / 对海合法目标区分
- 扩展 combined battle、support expedition、LBAS 等 advanced topology
