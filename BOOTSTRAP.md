# EmuKC 初始化指南

本文档介绍如何从零开始配置和初始化 EmuKC，包括下载游戏数据、构建缓存、启动服务器。

## 目录

- [环境准备](#环境准备)
- [配置文件](#配置文件)
- [快速开始（一键模式）](#快速开始一键模式)
- [手动分步初始化](#手动分步初始化)
- [命令参数速查](#命令参数速查)
- [常见问题](#常见问题)

---

## 环境准备

**必需：**

- Rust 工具链（edition 2024，最低版本 1.94.0）。安装方式：[rustup](https://rustup.rs/)
- 网络代理（用于下载 KanColle 服务器的游戏资源）

**步骤：**

```bash
# 克隆项目
git clone https://github.com/masterg0107/emukc.git
cd emukc

# 验证 Rust 版本
rustc --version   # 需要 >= 1.94.0

# 编译项目
cargo build
```

## 配置文件

从示例文件复制并编辑：

```bash
cp emukc.config.example.toml emukc.config.toml
```

### 配置字段说明

| 字段 | 必填 | 说明 |
|------|------|------|
| `workspace_root` | 是 | 用户数据存储目录，默认 `.data` |
| `cache_root` | 是 | 游戏缓存资源目录，默认 `./z/cache` |
| `mods_root` | 否 | 资源 mod 目录 |
| `bind` | 是 | 服务器监听地址，如 `0.0.0.0:27666` |
| `tls_cert` | 否 | TLS 证书文件路径（启用 HTTPS） |
| `tls_key` | 否 | TLS 私钥文件路径（启用 HTTPS） |
| `proxy` | 否 | 下载资源时使用的代理，如 `socks5://127.0.0.1:1086` |
| `gadgets_cdn` | 是 | HTML 组件 CDN 地址列表 |
| `game_cdn` | 是 | 游戏资源 CDN 地址列表 |

> **注意：** `proxy` 字段对于中国大陆用户通常是必需的，因为游戏资源托管在海外服务器上。

---

## 快速开始（一键模式）

最简单的方式——不带任何子命令直接运行：

```bash
cargo run
```

程序会自动执行以下步骤：

1. **检测状态** — 检查是否已有 Codex 和数据库
2. **引导配置** — 若无数据，提示是否自动创建
3. **下载清单** — 自动执行 bootstrap 流程
4. **构建 Codex** — 解析游戏数据到内存索引
5. **缓存资源** — 提示选择下载模式（最小化 / 完整 / 跳过）
6. **创建账户** — 自动创建默认账户（`admin` / `1234567`）
7. **启动服务器** — 自动打开浏览器访问游戏页面

首次运行时，程序会通过交互式提示引导你完成配置。

---

## 手动分步初始化

如果需要更多控制，可以手动执行每个步骤。

### 第 1 步：Bootstrap（下载清单 + 构建 Codex）

```bash
cargo run -- bootstrap --overwrite --force-update
```

此命令会：

- 下载游戏清单文件（舰船数据、装备数据、地图数据等）
- 解析并构建 Codex（游戏数据索引），保存到 `.data/codex/`
- `--overwrite`：覆盖已有数据
- `--force-update`：删除缓存中的版本文件，强制重新下载

### 第 2 步：生成缓存列表

```bash
cargo run -- cache make-list
```

此命令会：

- 根据 Codex 生成需要下载的资源文件列表
- 输出到 `<cache_root>/cache_resources.nedb`

### 第 3 步：下载资源文件

```bash
cargo run -- cache populate
```

此命令会：

- 读取上一步生成的缓存列表
- 并发下载所有资源文件到缓存目录
- 默认 16 并发

### 第 4 步：创建账户并启动

```bash
# 创建新账户并启动服务器
cargo run -- new-session -u admin -p 1234567
```

或者分开执行：

```bash
# 仅启动服务器（需已有账户）
cargo run -- serve
```

---

## 命令参数速查

### `bootstrap`

```bash
cargo run -- bootstrap [选项]
```

| 参数 | 说明 |
|------|------|
| `--overwrite` | 覆盖已有文件 |
| `--force-update` | 删除版本缓存文件，强制更新 |
| `--proxy <URL>` | 使用指定代理（覆盖配置文件中的设置） |
| `--output <DIR>` | 指定输出目录 |

### `cache make-list`

```bash
cargo run -- cache make-list [选项]
```

| 参数 | 说明 |
|------|------|
| `--output <FILE>` | 输出文件路径（默认 `<cache_root>/cache_resources.nedb`） |
| `--overwrite` | 覆盖已有列表文件 |
| `--greedy` | 贪婪模式：扫描所有可能资源（极慢，但最完整） |
| `--manifest` | 使用资源清单模式（基于 manifest 生成列表） |
| `--concurrent <N>` | 贪婪模式并发数（默认 16） |

### `cache populate`

```bash
cargo run -- cache populate [选项]
```

| 参数 | 说明 |
|------|------|
| `--src <FILE>` | 缓存列表文件路径（默认 `<cache_root>/cache_resources.nedb`） |
| `--concurrent <N>` | 并发下载任务数（默认 16） |

### `serve`

```bash
cargo run -- serve [选项]
```

| 参数 | 说明 |
|------|------|
| `--no-banner` | 隐藏启动 Banner |

### `new-session`

```bash
cargo run -- new-session -u <用户名> -p <密码> [选项]
```

创建新账户和档案，并自动启动服务器。

---

## 常见问题

### 配置文件找不到

**错误信息：** `Configuration file not found`

**解决方法：** 确保 `emukc.config.toml` 存在于以下位置之一：

1. 当前工作目录
2. 可执行文件所在目录
3. `EMUKC_CONFIG` 环境变量指定的路径

也可通过 `--config` 参数指定：

```bash
cargo run -- --config /path/to/emukc.config.toml serve
```

### Bootstrap 下载失败

**可能原因：**

- 网络无法访问 KanColle 服务器
- 代理配置错误

**解决方法：**

1. 确认 `emukc.config.toml` 中的 `proxy` 配置正确
2. 或在命令行直接指定代理：

```bash
cargo run -- bootstrap --proxy "socks5://127.0.0.1:1086"
```

3. 如果已有部分下载，加 `--overwrite` 重新下载

### Codex 加载失败

**错误信息：** `Failed to load app state` 或 Codex 相关错误

**解决方法：**

1. 确认 bootstrap 已成功执行（检查 `.data/codex/` 目录是否存在）
2. 重新执行 bootstrap：`cargo run -- bootstrap --overwrite`

### 缓存资源下载不完整

**解决方法：**

1. 重新运行 populate 命令，已下载的文件会自动跳过
2. 调整并发数以避免连接超时：`cargo run -- cache populate --concurrent 8`

### 端口被占用

**错误信息：** `Address already in use`

**解决方法：** 修改 `emukc.config.toml` 中的 `bind` 字段，使用其他端口：

```toml
bind = "0.0.0.0:27667"
```
