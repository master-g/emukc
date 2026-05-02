## Why

项目缺少面向新用户的初始化指南。新用户不知道如何从零开始配置 EmuKC：创建配置文件、下载游戏清单数据、构建 Codex、生成缓存列表、下载资源文件。当前 README 只列出了命令，没有完整的端到端流程说明。需要一份简体中文的 BOOTSTRAP.md 引导用户完成全部初始化步骤。

## What Changes

- 新增 `BOOTSTRAP.md` 文件，使用简体中文撰写
- 内容涵盖：环境准备、配置文件创建、bootstrap 命令（下载清单 + 构建 Codex）、cache make-list 命令（生成缓存列表）、cache populate 命令（下载资源文件）、serve 命令（启动服务器）、一键模式（auto 命令）
- 包含完整的命令示例和输出预期
- 包含常见问题排查

## Capabilities

### New Capabilities
- `bootstrap-guide`: 新增 BOOTSTRAP.md 文档，用简体中文介绍项目初始化流程

### Modified Capabilities

（无）

## Non-goals

- 不修改任何代码逻辑
- 不修改现有 README 或 CLAUDE.md
- 不涉及 battle、gameplay 等高级功能的使用说明
- 不包含开发指南或架构说明（这些已在 CLAUDE.md 中）

## Impact

- 仅新增文档文件，无代码变更
- 影响范围：项目根目录新增 BOOTSTRAP.md
