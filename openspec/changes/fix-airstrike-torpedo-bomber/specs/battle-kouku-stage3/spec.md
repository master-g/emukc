# battle-kouku-stage3

## Requirements

- BattleKoukuStage3 的 `api_frai`/`api_erai` 字段必须是 per-attacker 目标索引数组（值 = 目标 position，-1 = 未攻击）
- BattleKoukuStage3 的 `api_fbak`/`api_ebak` 字段必须是 per-attacker 目标索引数组（值 = 目标 position，-1 = 未攻击）
- `api_fdam`/`api_edam` 保持 per-defender 累积伤害不变
- 每个 bomber slot 的攻击者 ship index 和目标 defender index 必须正确映射
