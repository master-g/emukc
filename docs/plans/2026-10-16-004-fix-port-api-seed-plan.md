---
title: "PORT_API_SEED from the Client - Plan"
type: fix
date: 2026-10-16
status: implemented
execution: code
---

# PORT_API_SEED from the Client - Plan

Issue #185。客户端 6.3.6.0 的 `PORT_API_SEED` 换成了 `[5560, 8863, 7748, 8399, 9742, 7258, 3474, 6103, 7002, 2282]`，仓库三处还是旧表：排名戦果的加密（`api_req_ranking`）、`emukc_crypto::PortApiKey`、`scripts/fetch_live_api.py`。当前客户端因此解错排名戦果；旧表的 `api_port` 签名被线上服务器拒绝（2026-10-16 实测 100，新表 1）。

**范围**：U1–U3，一个 PR。
**非目标**：`create-serial-id`（客户端另一处用 seed，仓库没有对应实现）。

**基线**：`main` `52f07599`。

## Implementation Units

### U1 decoder 抽取 `port_api_seed`

- `main-decoder/src/port-api-seed.ts`：按导出名 `PORT_API_SEED` 找模块，babel 读出 10 个数的数组字面量，形状不对就报错。
- `pipeline.ts` 写 `out/resources/port_api_seed.json`，`--sync-assets` 同步到 `crates/emukc_bootstrap/assets/`。不写 `generatedAt`。
- bun 单测喂合成模块。

### U2 Rust 读资产

- `assets.rs` 登记 `port_api_seed`（embedded），`emukc_bootstrap` 提供 `port_api_seed()`，读一次。
- 排名的 `encode_war_result` 用它；排名测试按客户端写法解码时也用它，不再自带一份表。
- `PortApiKey::create` 改为接收 seed 表；测试向量用客户端自己的 `_createKey` 在新表下重新生成。
- drift 基线加入新资产。

### U3 脚本与文档

- `fetch_live_api.py` 从资产读 seed；selftest 与 Rust 同一组向量。
- `live-api-investigation.md` 写明 seed 来自同步资产。
