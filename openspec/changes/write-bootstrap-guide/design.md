## Context

EmuKC 是 KanColle 服务端模拟器。新用户需要从零配置环境、下载游戏清单数据、构建 Codex、生成缓存列表、下载资源文件后才能启动服务器。当前仅有 CLAUDE.md 面向开发者，缺少面向用户的初始化指南。

现有命令流程：
1. 创建 `emukc.config.toml`（从示例文件复制）
2. `cargo run -- bootstrap` — 下载游戏清单 + 构建 Codex 到 `.data/codex/`
3. `cargo run -- cache make-list` — 生成缓存资源列表文件 `cache_resources.nedb`
4. `cargo run -- cache populate` — 根据列表下载资源到缓存目录
5. `cargo run -- serve` 或 `cargo run -- new-session` — 启动服务器

也支持一键模式：`cargo run`（无子命令）自动完成上述全部步骤。

## Goals / Non-Goals

**Goals:**
- 创建 `BOOTSTRAP.md`，简体中文，覆盖完整初始化流程
- 包含两种路径：手动分步执行 + 一键自动模式
- 包含配置文件说明、命令参数、常见问题

**Non-Goals:**
- 不修改任何代码
- 不涉及高级功能（battle 诊断、gameplay 测试等）
- 不替代 README 或 CLAUDE.md

## Decisions

- **文档放在项目根目录**：`BOOTSTRAP.md` 是用户第一个看到的文件之一，与 README 同级
- **简体中文撰写**：目标用户群为中文社区
- **两种初始化路径并列**：新手用一键模式，有经验用户用手动分步模式获取更多控制
- **命令示例使用 `cargo run --`**：与 CLAUDE.md 保持一致

## Risks / Trade-offs

- [文档可能随命令变更而过时] → 标注命令版本，提醒用户查看 `--help`
- [配置文件字段可能变化] → 引用示例文件作为权威来源
