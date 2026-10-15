---
title: "A Relocating Plane Cannot Be Deployed Again - Plan"
type: fix
date: 2026-10-15
status: implemented
execution: code
---

# A Relocating Plane Cannot Be Deployed Again - Plan

Issue #146。配置转换中（配置転換中）的装备，可以通过 `api_req_air_corps/set_plane` 再次部署，只有客户端会拦。分诊时用户定为服务器拒绝。

**范围**：`set_airbase_plane` 拒绝自身还在配置转换的装备；替换钉住旧行为的测试；更新 `air-corps.md`。
**基线**：`main` `34cdd584`，分支 `fix/relocating-plane`。

## Evidence

- `Ctx::set_airbase_plane` 把配置转换中的行（`plane_db::Status::Reassigning`）当作一次新部署：删掉这一行，重新部署并扣铝土，部署到同一个或另一个基地都行。
  测试 `a_relocating_squadron_can_be_reassigned`（2026-09-22，随两段式移除加入）钉住了这个行为。
- `Ctx::change_deployment_base` 已经会拒绝：`WrongType("slot item {id} is relocating")`。
- 客户端用 `api_port/port` 的 `api_plane_info.api_base_convert_slot` 记录配置转换中的装备，不会把它们列出来；配置转换只在母港结算（`settle_relocations_impl`）。

## Decision

装备自身的行还处于 `Reassigning` 时，`set_airbase_plane` 返回 `WrongType`，并且在扣任何东西之前就返回，事务回滚。
等待时间已到、但母港还没结算的，同样算配置转换中，和客户端的列表一致。
向一个仍留着别的装备配置转换行的格子部署，行为不变。

## Implementation Units

- **U1（先写测试）**：用 `a_relocating_squadron_waits_for_the_port_to_deploy_again` 替换旧测试，覆盖以下三点：
  - 配置转换中部署到另一个基地或原基地都失败；
  - 铝土不变；
  - 母港结算后，可以部署并扣铝土。

  然后修改 `set_airbase_plane`。
- **U2**：从 `air-corps.md` 的 Known gaps 删掉这一条，在配置转换一节补上这条规则。
