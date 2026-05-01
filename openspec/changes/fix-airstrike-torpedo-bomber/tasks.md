## 1. 修改 BattleKoukuStage3 结构体

- [ ] 1.1 在 `crates/emukc_model/src/kc2/` 中找到 BattleKoukuStage3 定义，将 `api_frai_flag`/`api_erai_flag`/`api_fbak_flag`/`api_ebak_flag` 重命名为 `api_frai`/`api_erai`/`api_fbak`/`api_ebak`，语义改为 per-attacker 目标索引（-1 = 未攻击）
- [ ] 1.2 更新所有引用旧字段名的代码（全局搜索 `api_frai_flag`/`api_fbak_flag`/`api_erai_flag`/`api_ebak_flag`）

## 2. 修改 AirstrikeOutput 结构

- [ ] 2.1 在 `crates/emukc_gameplay/src/game/battle/core.rs` 修改 `AirstrikeOutput`：将 `bak_flag: &'a mut [i64]` 和 `rai_flag: &'a mut [i64]` 改为 `bak_targets: &'a mut [i64]` 和 `rai_targets: &'a mut [i64]`
- [ ] 2.2 初始化时填充 -1 而非 0

## 3. 修改 execute_airstrike_phase

- [ ] 3.1 Phase 1 (舰爆): 改为 `output.bak_targets[ship_idx] = target_idx as i64`（当前是 `output.bak_flag[target_idx] = 1`）
- [ ] 3.2 Phase 2 (舰攻): 改为 `output.rai_targets[ship_idx] = target_idx as i64`（当前是 `output.rai_flag[target_idx] = 1`）
- [ ] 3.3 damage 累积保持不变（per-defender 正确）

## 4. 修改 simulate_kouku

- [ ] 4.1 初始化 `api_frai`/`api_fbak` 为 `vec![-1i64; friendly.len()]`，`api_erai`/`api_ebak` 为 `vec![-1i64; enemy.len()]`
- [ ] 4.2 传递给 `execute_airstrike_phase` 的新 AirstrikeOutput 结构
- [ ] 4.3 构建 BattleKoukuStage3 时使用新字段名

## 5. 验证和测试

- [ ] 5.1 更新 `crates/emukc_gameplay/src/game/battle/core.rs` 中的现有测试用例
- [ ] 5.2 添加测试：验证单 CV 舰爆+舰攻同时出击时，`api_erai` 和 `api_ebak` 正确记录 attacker→target 映射
- [ ] 5.3 添加测试：验证多 CV 时每个 CV 的 target 独立记录
- [ ] 5.4 cargo test 全量通过
