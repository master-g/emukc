---
title: "feat: Debian 12 VPS 部署方案 —— 容器内构建 + Makefile 部署 + Caddy 反代"
type: feat
date: 2026-07-02
status: in-progress
depth: deep
origin: session "将 EmuKC 部署到 VPS" (2026-07-02)
---

# feat: Debian 12 VPS 部署方案

## Summary

建立一条从 macOS arm64 本地机到 Debian 12 x86_64 VPS 的可重复部署流水线，覆盖**编译 → 打包 → 传输 → systemd 运行 → Caddy 终止 TLS** 全链路，并附带一次性环境初始化与运维 runbook。

核心做法：

- **本地用 Docker 容器（`debian:12-slim`）原生构建** linux amd64 二进制，借助 Rosetta 2 运行 amd64 容器，规避全部 C 依赖（mimalloc / SQLite / OpenSSL）的交叉编译坑，glibc 与目标 VPS 完全一致。
- **C 依赖保持动态链接**，不改任何 `Cargo.toml` feature（遵守 surgical-change 与 Balance Defaults Policy）。VPS 装 `libsqlite3-0` / `libssl3`（Debian 12 默认自带）。
- **emukcd 只跑 loopback HTTP**，TLS 由前端 **Caddy 反向代理**终止。
- **游戏数据在 VPS 上直接 `emukcd bootstrap`** 生成（codex + 资源缓存），不经过本地上传。
- 交付通过 **Makefile target（`make deploy`）一条命令**完成构建→打包→scp→systemctl restart。

本计划是**纯基础设施/部署工具**计划：不修改任何 Rust 源码、不修改 `Cargo.toml` / `Cross.toml`、不改变游戏行为。

---

## Problem Frame

当前仓库**没有任何生产部署工具**：

- 无 `Dockerfile` / `docker-compose` / 容器配置。
- `Cross.toml` 仅有 Windows MSVC 目标，无 Linux x86_64 目标。
- 无 `.cargo/config.toml`（无 linker/target 配置）。
- 无 systemd unit / supervisord / pm2 等进程管理配置。
- 无 `deploy/` / `ops/` / `infra/` 目录，无 CI（`.github/` 仅放 agent prompts）。
- `Makefile` 仅有 dev 便利 target（build/run/serve/test/bootstrap/cache/battle），无 install/deploy/package/release。
- `docs/solutions/` 全为 bug/设计知识，无部署/运维文档；`README.md` / `HTTPS.md` 仅给本地指引。

直接 `cargo build` 在 macOS 上无法产出 Linux 二进制；而 emukcd 有三个 **C 语言原生依赖**让交叉编译变难：

| 依赖 | crate | 链接方式 | 交叉编译难度 |
|---|---|---|---|
| **mimalloc** | `libmimalloc-sys` (via `emukc_app`) | 从源码编译（需 cmake + cc） | 中 |
| **SQLite** | `libsqlite3-sys` (via sea-orm `sqlx-sqlite`) | **无 `bundled` feature → 链系统 `libsqlite3.so`** | 高（需目标平台 dev 库或加 bundled） |
| **OpenSSL** | `openssl-sys` (via sea-orm `runtime-tokio-native-tls` + reqwest `default-tls`) | 链系统 `libssl.so` | 高（同上） |
| redb | 纯 Rust | 静态 | 无 |
| axum-server TLS | rustls | 静态 | 无 |

`libsqlite3-sys` 与 `openssl-sys` 均**未启用 bundled/vendored**，故跨平台构建要么装目标平台的 dev 库，要么改 Cargo feature。本计划**选择不改 feature**（保持 surgical），转而用**容器内原生构建**绕开整个交叉编译问题域。

---

## Requirements

- **R1.** 在 macOS arm64 上，用一条 `make build-linux` 命令产出可在 Debian 12 x86_64 直接运行的 `emukcd` ELF 二进制，无需手写交叉链接器配置。
- **R2.** 构建产物与目标 VPS（Debian 12, glibc 2.36）二进制兼容，运行时仅需 `libsqlite3-0` / `libssl3`（默认已装）。
- **R3.** 不修改任何 Rust 源码、`Cargo.toml`、`Cross.toml`；改动严格限于新增文件（`deploy/`）与 `Makefile` 追加 target。
- **R4.** 提供 `make deploy` 一条命令完成：构建 → 打包（含版本号） → scp 到 VPS → `systemctl restart emukcd`。
- **R5.** 提供 systemd unit，使 emukcd 以非 root 用户 `emukc` 运行，开机自启，崩溃自动重启，带基本 hardening。
- **R6.** emukcd 在 VPS 上只监听 loopback HTTP，TLS 由 Caddy 在前端终止。
- **R7.** 提供一次性 VPS 环境初始化脚本（装 caddy + 运行库、建用户、建目录、装 unit/Caddyfile）。
- **R8.** 游戏数据（codex + 资源缓存）在 VPS 上用 `emukcd bootstrap` / `cache populate` 直接生成，不从本地上传。
- **R9.** 提供完整 runbook（`deploy/README.md`）：首次部署、日常更新、客户端配置（hosts + Caddy CA）、备份/查日志。
- **R10.** 构建过程利用 cargo registry / target 缓存，增量构建可接受（首次慢，后续快）。
- **R11.** `build.rs` 的 git 版本号在容器构建中正常嵌入（容器挂载含 `.git` 的 repo）。

---

## Scope Boundaries

### In Scope
- 新增 `deploy/` 目录与其下全部文件。
- `Makefile` 追加 `build-linux` / `package` / `deploy` / `vps-setup` target。
- VPS 侧 systemd + Caddy 部署模型。

### Out of Scope（不做）
- **不修改任何 Rust 源码**（不动 `src/`、不动任何 `crates/*/src/`）。
- **不修改 `Cargo.toml`**（不加 `bundled`/`vendored` feature，无 balance 变更）。
- **不修改 `Cross.toml`**（Windows MSVC 目标与本计划无关）。
- 不引入 CI（仓库约定无 CI server，质量门本地 + review 时执行）。
- 不做 Docker 镜像发布/容器化运行（用户已选裸 systemd + scp 方案；容器仅用于本地构建）。
- 不实现自动滚动更新 / 蓝绿部署（单实例 VPS，systemd restart 即可）。
- 不固化 KanColle 客户端的具体 hosts/CA 接入方式（该部分高度依赖用户接入工具，runbook 给标准做法并标为可调）。

### Deferred to Follow-Up
- 若未来需要多实例 / 蓝绿 / 自动扩缩，再立独立计划。
- 若需要把 `runtime-tokio-native-tls` 切到 `runtime-tokio-rustls` 以彻底消除 OpenSSL 依赖，属独立 Cargo 改动计划（本计划刻意不动 Cargo）。
- macOS arm64 native（Apple Silicon）VPS 构建路径（当前目标明确是 x86_64 VPS）。

---

## Context & Research

### 相关代码与配置事实

- **二进制名 `emukcd`**：`src/bin/emukcd.rs`（无显式 `[[bin]]`，按文件名产出）。edition 2024，`rust-version = "1.96.0"`（`Cargo.toml`），`rust-toolchain.toml` 锁 `channel = "stable"`。
- **release profile 激进**（`Cargo.toml`）：`opt-level=3`、`lto=true`、`codegen-units=1`、`panic="abort"`、`strip="debuginfo"` → release 链接慢，但产物小。
- **mimalloc**：`crates/emukc_app/Cargo.toml` `mimalloc = { version = "0.1.47", default-features = false }`，`crates/emukc_app/src/mem.rs` 以 `#[global_allocator]` 注册（linux 走 `_` 分支）。`libmimalloc-sys` 经 cmake + cc 从源码编译。
- **SQLite**：`Cargo.toml` `sea-orm = { features = ["debug-print", "runtime-tokio-native-tls", "sqlx-sqlite"] }`；`libsqlite3-sys 0.30.1` 仅 dep `cc`/`pkg-config`/`vcpkg`，**无 bundled** → 链系统 `libsqlite3.so`。DB 文件 `{workspace_root}/emukc.db`（`src/bin/state/mod.rs` `DB_NAME`）。
- **OpenSSL**：经 sea-orm `runtime-tokio-native-tls` 与 reqwest `default-tls` 引入 `openssl-sys` → 链系统 `libssl.so`。
- **redb**：`redb = "4.1.0"` 纯 Rust（仅 dep `libc`），文件 `{cache_root}/kache.redb`。
- **TLS**：`axum-server` 启用 `tls-rustls`；`src/bin/net/mod.rs:84-97` —— `tls_cert`+`tls_key` 同设则 rustls HTTPS，否则 plain HTTP。本计划令 VPS 走 plain HTTP（Caddy 终止 TLS），故配置不设这两项。
- **配置加载**：`src/bin/cfg/mod.rs` `AppConfig`（lines 17-45）；相对路径**相对配置文件父目录**解析并 `canonicalize`；自动建子目录。字段：`workspace_root` / `cache_root` / `mods_root`(opt) / `bind` / `tls_cert`(opt) / `tls_key`(opt) / `proxy`(opt) / `gadgets_cdn` / `game_cdn`。
- **信号处理**：`src/bin/net/signal.rs` 在 unix 监听 `SIGHUP/SIGINT/SIGQUIT/SIGTERM`，优雅关停 → 与 systemd 兼容。
- **`build.rs`** 读 `.git` 短 hash 写入 `OUT_DIR/git_version.rs`；无 `.git` 则 fallback `"unknown"`。
- **bootstrap**（`src/bin/cli/bootstrap.rs`）：4 阶段——下载 kc_data/kc3kai/kcanotify/kcwiki → 解析 → `codex.save()` 写 14 个 JSON 到 `{workspace_root}/codex/` → 下 `kcs_const.js`/`version.json` 到 `{cache_root}/`。需访问 `gadgets_cdn`/`game_cdn`（真 KanColle CDN），可用 `proxy`。
- **缓存填充**：`cache make-list` + `cache populate -n <N>`（`src/bin/cli/cache/`），写 `{cache_root}/kache.redb` 等。

### 决策依据

- **为何容器内原生构建而非 cargo-zigbuild / cross**：emukcd 的 mimalloc（cmake 交叉）+ sqlite/openssl（目标 dev 库）三重 C 依赖使纯交叉编译出错率高；而 `debian:12-slim` amd64 容器（Docker Desktop on Apple Silicon 经 Rosetta 2）默认 triple 即 `x86_64-unknown-linux-gnu`，`cargo build --release` 直接产出与 VPS 同 glibc(2.36) 的 ELF，零交叉配置。这是最可靠的路径。
- **为何不改 Cargo feature 加 bundled/vendored**：遵守 PR Review Rules 的 surgical-change（每行改动可追溯到请求）与 Balance Defaults Policy（任何 Cargo 改动需独立 commit + ce-plan）。VPS 装 `libsqlite3-0`/`libssl3` 零成本（Debian 12 默认自带），无理由为打包整洁度引入 Cargo 改动。
- **为何 Caddy 而非 nginx**：Caddy 自动证书（`tls internal` 自签 CA，免交互）+ 单文件配置，远比 nginx 手动证书管理简单；KanColle 客户端要访问的固定域名（`osapi.dmm.com` 等）需自签 + hosts 映射，Caddy `tls internal` 正合此用例。
- **为何 VPS 上 bootstrap 而非本地上传**：用户已选；资源缓存可达数 GB，上传慢；VPS（通常日本 IP）直连 KanColle CDN 更顺，无需本地 socks5 代理。

---

## Key Technical Decisions

1. **构建器镜像 `deploy/Dockerfile.build`** 基于 `debian:12-slim`，装 `build-essential cmake pkg-config libssl-dev libsqlite3-dev git ca-certificates curl`，rustup 装 stable（与 `rust-toolchain.toml` 一致）。该镜像**仅用于本地构建**，不发布。
2. **构建命令无需 `--target`**：容器默认 triple 即目标 triple，`cargo build --release` 直接产出正确 ELF。
3. **构建缓存**：用 docker named volume `emukc-cargo-registry` 挂 `~/.cargo/registry`；源码挂 host repo；`target/` 落 host（增量编译）。
4. **`build.rs` 版本号**：容器构建时挂载整个 repo（含 `.git`），版本号正常嵌入。
5. **systemd unit** `deploy/emukcd.service`：非 root 用户 `emukc`，`WorkingDirectory=/var/lib/emukc`，`ExecStart=/usr/local/bin/emukcd --config /etc/emukc/emukc.config.toml serve`，`Restart=on-failure`，加 `NoNewPrivileges`/`ProtectSystem=strict`/`ReadWritePaths=/var/lib/emukc` 等 hardening。
6. **Caddy** `deploy/Caddyfile`：`tls internal` 为 KanColle 固定域名终止 TLS，`reverse_proxy 127.0.0.1:27666`。
7. **生产配置模板** `deploy/emukc.config.production.example.toml`：`bind=127.0.0.1:27666`（HTTP loopback），绝对路径 `workspace_root`/`cache_root` 指向 `/var/lib/emukc`，不设 `tls_cert`/`tls_key`，`proxy` 留空（VPS 直连）。
8. **Makefile** 追加 4 个 target（`build-linux`/`package`/`deploy`/`vps-setup`），变量 `VPS_HOST`/`VPS_PATH`/`IMAGE` 可覆盖。

---

## Implementation Units

### U1. `deploy/Dockerfile.build` —— 容器构建器镜像

**Goal:** 提供一个可在 macOS arm64 上经 Rosetta 2 运行的 linux/amd64 构建环境，原生编译出 Debian 12 兼容的 `emukcd`。

**Requirements:** R1, R2, R3, R10, R11

**Files:**
- New: `deploy/Dockerfile.build`

**Approach:**
- `FROM --platform=linux/amd64 debian:12-slim`（显式指定 amd64，Docker Desktop on Apple Silicon 自动用 Rosetta 2 运行）。
- `apt-get install`: `build-essential cmake pkg-config libssl-dev libsqlite3-dev git ca-certificates curl`（覆盖 mimalloc 的 cmake/cc、sqlite/openssl 的 dev 头）。
- 装 rustup：`curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain stable --profile minimal`（匹配 `rust-toolchain.toml` 的 stable channel）。装 `rustfmt`/`clippy` 组件以兼容 `rust-toolchain.toml` 声明。
- 设 `WORKDIR /workspace`；`ENTRYPOINT` 留空（由 Makefile 传 `cargo build` 命令）。
- 不 `COPY` 源码（构建时挂载 host repo，保证增量缓存与 `.git` 可见）。

**Verification:**
- `docker build -f deploy/Dockerfile.build -t emukc-builder deploy/` 成功。
- 容器内 `rustc -vV` 报 `host: x86_64-unknown-linux-gnu`。
- 容器内 `cargo build --release` 产出 `target/release/emukcd`；`file target/release/emukcd` 报 `ELF 64-bit LSB ... x86-64 ... dynamically linked`；`ldd` 仅依赖 `libsqlite3.so.0` / `libssl.so.3` / `libgcc_s` / `libc` / `ld-linux` 等系统库。

---

### U2. `deploy/emukcd.service` —— systemd unit

**Goal:** 让 emukcd 以非特权用户开机自启、崩溃自重启。

**Requirements:** R5, R6

**Files:**
- New: `deploy/emukcd.service`

**Approach:**
- `[Unit]`：`Description=EmuKC KanColle Server Emulator`、`After=network-online.target`、`Wants=network-online.target`。
- `[Service]`：`Type=simple`、`User=emukc`、`Group=emukc`、`WorkingDirectory=/var/lib/emukc`、`ExecStart=/usr/local/bin/emukcd --config /etc/emukc/emukc.config.toml serve`、`Environment=EMUKC_LOG_LEVEL=info`、`Restart=on-failure`、`RestartSec=5s`、`TimeoutStopSec=30s`（配合 `signal.rs` 优雅关停）。
- Hardening：`NoNewPrivileges=true`、`ProtectSystem=strict`、`ProtectHome=true`、`ReadWritePaths=/var/lib/emukc`、`PrivateTmp=true`、`CapabilityBoundingSet=`（空）。
- `[Install]`：`WantedBy=multi-user.target`。

**Verification:**
- `systemctl daemon-reload && systemctl enable --now emukcd` 后 `systemctl status emukcd` active。
- `ss -ltnp | grep 27666` 显示监听 127.0.0.1:27666。
- `systemctl restart emukcd` 触发优雅关停（旧进程 SIGTERM 后退出，非 SIGKILL）。

---

### U3. `deploy/Caddyfile` —— 反向代理 + TLS 终止

**Goal:** 在前端为 KanColle 客户端要访问的固定域名终止 TLS，反代到 loopback 上的 emukcd。

**Requirements:** R6

**Files:**
- New: `deploy/Caddyfile`

**Approach:**
- 用 `tls internal`（Caddy 内置 CA，自动生成并信任根证书）。
- 为 KanColle 固定域名（如 `osapi.dmm.com`、`gadget_html5` 相关 CDN host 等；具体集合在 runbook 标注为可调）写 site block，`reverse_proxy 127.0.0.1:27666`。
- 顶部注释说明：客户端需 (a) 装 Caddy 根 CA 到信任库，(b) 配 hosts 把这些域名指向 VPS IP。这两步为 KanColle 接入的标准做法，**依赖用户具体接入工具**，runbook 给参考命令并标可调。

**Verification:**
- `caddy validate --config deploy/Caddyfile` 通过。
- `systemctl reload caddy` 后 `curl -k https://osapi.dmm.com/`(经 hosts 解析) 能拿到 emukcd 响应（带 `svdata=` 前缀的 KCSAPI 路径）。

---

### U4. `deploy/emukc.config.production.example.toml` —— 生产配置模板

**Goal:** 给 VPS 一份可直接 `cp` 成 `emukc.config.toml` 的生产配置样板。

**Requirements:** R2, R6, R8

**Files:**
- New: `deploy/emukc.config.production.example.toml`

**Approach:**
- 基于现有 `emukc.config.example.toml` 字段集，但：
  - `bind = "127.0.0.1:27666"`（loopback HTTP，由 Caddy 终止 TLS）。
  - `workspace_root = "/var/lib/emukc/data"`、`cache_root = "/var/lib/emukc/cache"`（绝对路径，避开相对路径歧义）。
  - `mods_root` 注释掉。
  - `tls_cert`/`tls_key` 注释掉（Caddy 终止）。
  - `proxy` 留空注释（VPS 直连 KanColle CDN；若 VPS 被墙再填）。
  - 保留 `gadgets_cdn` / `game_cdn` 列表（bootstrap 需要）。

**Verification:**
- `emukcd --config <this> version` 不报配置解析错。
- 字段集与 `AppConfig`（`src/bin/cfg/mod.rs:17-45`）一一对应，无遗漏无多余。

---

### U5. `deploy/vps-setup.sh` —— VPS 一次性环境初始化

**Goal:** 在干净的 Debian 12 VPS 上一键装好运行环境。

**Requirements:** R5, R6, R7

**Files:**
- New: `deploy/vps-setup.sh`

**Approach:**
- `set -euo pipefail`，幂等（重复跑不报错）。
- `apt-get update && apt-get install -y caddy libsqlite3-0 libssl3 ca-certificates curl`（caddy 用 Debian 12 官方源；`libsqlite3-0`/`libssl3` 通常已装，显式声明确保）。
- `useradd --system --no-create-home --shell /usr/sbin/nologin emukc`（`id emukc` 已存在则跳过）。
- `mkdir -p /var/lib/emukc/data /var/lib/emukc/cache /etc/emukc`，`chown -R emukc:emukc /var/lib/emukc`。
- 安装 Caddyfile：`cp deploy/Caddyfile /etc/caddy/Caddyfile`，`systemctl reload caddy || systemctl restart caddy`，`systemctl enable caddy`。
- **不**自动装 `emukcd.service`（需先有二进制），仅提示用户在 `make deploy` 后执行 `systemctl enable --now emukcd`。
- 末尾打印下一步指引（bootstrap / cache populate / 客户端配置）。

**Verification:**
- 在全新 Debian 12 容器/VM 跑一次：`caddy --version` 正常、`id emukc` 存在、`/var/lib/emukc` 属主正确、`systemctl is-enabled caddy` enabled。

---

### U6. `Makefile` 追加 deploy target

**Goal:** 提供一条命令的本地构建与远程部署。

**Requirements:** R1, R4, R10

**Files:**
- Modify: `Makefile`（追加变量与 4 个 target，不改现有 target）

**Approach:**
- 新增变量（可覆盖）：
  - `VPS_HOST ?= root@your-vps-ip`
  - `VPS_PATH ?= /var/lib/emukc`
  - `IMAGE ?= emukc-builder`
  - `GIT_SHA := $(shell git rev-parse --short HEAD)`
- `build-linux`：`docker build --platform linux/amd64 -f deploy/Dockerfile.build -t $(IMAGE) deploy/`；再 `docker run --rm --platform linux/amd64 -v $(PWD):/workspace -v emukc-cargo-registry:/root/.cargo/registry -w /workspace $(IMAGE) cargo build --release`。产物在 host `target/release/emukcd`。
- `package`：依赖 `build-linux`；`tar -czf emukc-$(GIT_SHA).tar.gz -C target/release emukcd`；`shasum -a 256 emukc-$(GIT_SHA).tar.gz > emukc-$(GIT_SHA).tar.gz.sha256`。
- `deploy`：依赖 `package`；`scp emukc-$(GIT_SHA).tar.gz $(VPS_HOST):/tmp/`；`ssh $(VPS_HOST) "tar -xzf /tmp/emukc-$(GIT_SHA).tar.gz -C /usr/local/bin && chmod +x /usr/local/bin/emukcd && sudo systemctl restart emukcd && sudo systemctl status emukcd --no-pager"`。
- `vps-setup`：`scp deploy/vps-setup.sh $(VPS_HOST):/tmp/ && ssh $(VPS_HOST) "sudo bash /tmp/vps-setup.sh"`。
- 全部加 `## ` 自文档注释，纳入现有 `help` target 的 grep。

**Verification:**
- `make build-linux` 在 macOS 上产出 `target/release/emukcd`（ELF x86-64）。
- `make package` 产出 `emukc-<sha>.tar.gz` + `.sha256`。
- `make deploy VPS_HOST=user@ip` 端到端跑通（需 VPS 已 vps-setup）。

---

### U7. `deploy/README.md` —— 部署 runbook

**Goal:** 让任何人照文档完成首次部署与日常更新。

**Requirements:** R7, R9

**Files:**
- New: `deploy/README.md`

**Approach:**
- **架构概览**图（ASCII）：macOS (Docker build) → tarball → scp → VPS (systemd emukcd loopback:27666 ← Caddy :443 ← 客户端)。
- **前置条件**：macOS 装 Docker Desktop（开启 Rosetta 2 for x86/am64 emulation）；VPS 为 Debian 12、有 root/sudo。
- **首次部署**（ordered）：
  1. `make vps-setup VPS_HOST=user@ip`（装 caddy + 运行库 + 建用户/目录）。
  2. `make deploy VPS_HOST=user@ip`（构建 + 传二进制 + 装 unit）。
  3. VPS 上 `cp /etc/emukc/emukc.config.production.example.toml /etc/emukc/emukc.config.toml`，按需调。
  4. VPS 上 `sudo -u emukc emukcd --config /etc/emukc/emukc.config.toml bootstrap --overwrite --force-update`（生成 codex）。
  5. VPS 上 `sudo -u emukc emukcd --config /etc/emukc/emukc.config.toml cache make-list --overwrite` + `cache populate -n 16`。
  6. `sudo systemctl enable --now emukcd`。
  7. 客户端配置（见下）。
- **客户端配置**（标"KanColle 专属，可调"）：hosts 映射 KanColle 固定域名 → VPS IP；信任 Caddy 根 CA（从 VPS `/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt` 取）。给出参考命令，注明依接入工具调整。
- **日常更新**：`make deploy VPS_HOST=user@ip`（一条命令）。
- **运维**：查日志 `journalctl -u emukcd -f`；备份 `sqlite3 /var/lib/emukc/data/emukc.db .backup /path/backup.db`；Caddy 日志 `journalctl -u caddy`。
- **故障排查**：常见错（glibc 版本、libsqlite3 缺失、端口占用、CA 不信任）的症状与解法。

**Verification:**
- 照 runbook 在全新 Debian 12 VPS 上从零跑通 emukcd + Caddy，客户端能进入游戏并收到 `svdata=` 响应。

---

## System-Wide Impact

- **新增目录** `deploy/`（7 个文件），对现有代码零侵入。
- **`Makefile`** 仅追加，不改现有 target 行为。
- **运行时**：VPS 多一个 systemd 服务（emukcd，loopback）+ Caddy（:443）；磁盘新增 `/var/lib/emukc`（codex + DB + cache，可达数 GB）。
- **无任何游戏行为变化**：本计划不动 gameplay / model / codex 数值。

---

## Risks & Dependencies

| 风险 | 缓解 |
|---|---|
| Docker Desktop on Apple Silicon 的 Rosetta 2 amd64 模拟构建慢 / 偶发 | 首次构建慢可接受；用 cargo registry + target 缓存增量；若 Rosetta 失败可回退 `--platform=linux/amd64` 下 QEMU（更慢但能跑）。 |
| `build.rs` 在容器内读 `.git` 失败 → 版本号 `"unknown"` | Makefile 挂载整个 repo 含 `.git`；runbook 记录该行为；fallback 不影响运行。 |
| Debian 12 caddy 版本旧（2.6.x）缺少某些指令 | Caddyfile 仅用稳定特性（`tls internal` / `reverse_proxy`），2.6 支持。 |
| KanColle 客户端接入（hosts + CA）依赖用户工具，无法标准化 | runbook 给参考做法并明确标"可调"；不固化进脚本。 |
| VPS 需能直连 KanColle CDN（日本 IP）跑 bootstrap | runbook 注明；若被墙，配置 `proxy` 字段。 |
| scp + ssh 需 VPS 免密或交互密码 | 假定用户已配 SSH key；runbook 前置条件说明。 |
| `libsqlite3-0`/`libssl3` 版本随 Debian 12 小版本变动影响动态链接 | 构建器与 VPS 同为 Debian 12，soname 一致（`libsqlite3.so.0`/`libssl.so.3`）；vps-setup 显式 apt 装。 |

---

## Execution Order

U1（构建器）→ U6（Makefile，依赖 U1 才能验证 build-linux）→ U4（配置模板，独立）→ U2（systemd unit，独立）→ U3（Caddyfile，独立）→ U5（vps-setup，依赖 U3）→ U7（runbook，汇总全部）。

实施时建议提交顺序（Conventional Commits，遵守 surgical 与无 AI attribution）：

1. `feat(deploy): add debian:12 builder image` —— U1
2. `feat(deploy): add systemd unit, Caddyfile, production config template` —— U2/U3/U4
3. `feat(deploy): add vps-setup script` —— U5
4. `feat(deploy): add make build-linux/package/deploy/vps-setup targets` —— U6
5. `docs(deploy): add VPS deployment runbook` —— U7

每步独立可验证。无任何 Rust/Cargo 改动，不触发 Balance Defaults Policy。
