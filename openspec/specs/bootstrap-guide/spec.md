## ADDED Requirements

### Requirement: 文档包含完整的初始化流程说明
BOOTSTRAP.md SHALL 按顺序说明以下步骤：
1. 环境准备（Rust 工具链、项目克隆）
2. 配置文件创建（从 emukc.config.example.toml 复制）
3. bootstrap 命令执行（下载清单 + 构建 Codex）
4. cache make-list 命令执行（生成缓存列表）
5. cache populate 命令执行（下载资源文件）
6. 启动服务器

#### Scenario: 新用户按文档完成首次初始化
- **WHEN** 新用户克隆项目后阅读 BOOTSTRAP.md
- **THEN** 能够按文档步骤完成从零到服务器启动的全部操作

#### Scenario: 一键模式替代手动分步
- **WHEN** 用户执行 `cargo run`（无子命令）
- **THEN** 文档说明此命令会自动完成 bootstrap、cache list、下载资源的全部流程

### Requirement: 文档包含配置文件字段说明
BOOTSTRAP.md SHALL 说明 emukc.config.toml 中每个必填字段的作用：
- `workspace_root`：用户数据存储位置
- `cache_root`：游戏缓存目录
- `bind`：服务器监听地址
- `proxy`：代理设置（用于下载资源）
- `game_cdn` / `gadgets_cdn`：CDN 地址列表

#### Scenario: 用户根据说明填写配置文件
- **WHEN** 用户打开 emukc.config.example.toml
- **THEN** 文档中对应说明能帮助用户理解每个字段的含义和填写方式

### Requirement: 文档包含命令参数速查
BOOTSTRAP.md SHALL 列出 bootstrap 和 cache 子命令的常用参数：
- `--overwrite`、`--force-update`、`--proxy`（bootstrap）
- `--output`、`--overwrite`、`--greedy`、`--manifest`、`--concurrent`（cache make-list）
- `--src`、`--concurrent`（cache populate）

#### Scenario: 用户需要自定义命令参数
- **WHEN** 用户想要覆盖已有数据或调整并发数
- **THEN** 文档中的参数速查表提供足够信息

### Requirement: 文档包含常见问题排查
BOOTSTRAP.md SHALL 包含以下常见问题的排查指引：
- 配置文件找不到
- bootstrap 下载失败（网络/代理问题）
- Codex 加载失败
- 缓存资源下载不完整

#### Scenario: 用户遇到下载失败
- **WHEN** bootstrap 或 populate 命令因网络问题失败
- **THEN** 文档中的排查指引帮助用户检查代理配置和重试方法
