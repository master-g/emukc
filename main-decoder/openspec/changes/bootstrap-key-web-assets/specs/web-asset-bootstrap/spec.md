## ADDED Requirements

### Requirement: Bootstrap downloads kcs_const.js

系统 SHALL 在 bootstrap 流程中将 `gadget_html5/js/kcs_const.js` 从配置的 gadgets CDN 下载到 `z/cache/gadget_html5/js/kcs_const.js`。

#### Scenario: kcs_const.js not present in cache
- **WHEN** 用户执行 `cargo run -- bootstrap` 且 `z/cache/gadget_html5/js/kcs_const.js` 不存在
- **THEN** 系统 SHALL 从 `gadgets_cdn` 配置的第一个可用 CDN 下载该文件到目标路径

#### Scenario: kcs_const.js already exists and overwrite is false
- **WHEN** 目标文件已存在且未传 `--overwrite`
- **THEN** 系统 SHALL 跳过下载并记录 debug 日志

### Requirement: Bootstrap downloads main.js

系统 SHALL 在 bootstrap 流程中将 `kcs2/js/main.js` 从配置的 game CDN 下载到 `z/cache/kcs2/js/main.js`。

#### Scenario: main.js not present in cache
- **WHEN** 用户执行 `cargo run -- bootstrap` 且 `z/cache/kcs2/js/main.js` 不存在
- **THEN** 系统 SHALL 从 `game_cdn` 配置的 CDN 下载该文件到目标路径

#### Scenario: main.js already exists and overwrite is false
- **WHEN** 目标文件已存在且未传 `--overwrite`
- **THEN** 系统 SHALL 跳过下载并记录 debug 日志

### Requirement: Bootstrap downloads version.json

系统 SHALL 在 bootstrap 流程中将 `kcs2/version.json` 从配置的 game CDN 下载到 `z/cache/kcs2/version.json`。

#### Scenario: version.json not present in cache
- **WHEN** 用户执行 `cargo run -- bootstrap` 且 `z/cache/kcs2/version.json` 不存在
- **THEN** 系统 SHALL 从 `game_cdn` 配置的 CDN 下载该文件到目标路径

#### Scenario: version.json already exists and overwrite is false
- **WHEN** 目标文件已存在且未传 `--overwrite`
- **THEN** 系统 SHALL 跳过下载并记录 debug 日志

### Requirement: CDN configuration not set gracefully handled

系统 SHALL 在 CDN 未配置时优雅降级而非报错。

#### Scenario: gadgets_cdn is empty
- **WHEN** `gadgets_cdn` 配置为空列表
- **THEN** 系统 SHALL 跳过 kcs_const.js 下载，输出 warn 级别日志说明需要配置 gadgets_cdn

#### Scenario: game_cdn is empty
- **WHEN** `game_cdn` 配置为空列表
- **THEN** 系统 SHALL 跳过 main.js 和 version.json 下载，输出 warn 级别日志说明需要配置 game_cdn

### Requirement: Force update restores web assets

系统 SHALL 在 `--force-update` 删除版本文件后重新下载恢复。

#### Scenario: force-update with CDN configured
- **WHEN** 用户执行 `cargo run -- bootstrap --force-update` 且 CDN 已配置
- **THEN** 系统 SHALL 先删除旧的 kcs_const.js 和 version.json，然后重新下载这三个文件

### Requirement: Skip web assets flag

系统 SHALL 提供 `--skip-web-assets` CLI 标志。

#### Scenario: skip-web-assets flag provided
- **WHEN** 用户执行 `cargo run -- bootstrap --skip-web-assets`
- **THEN** 系统 SHALL 跳过所有 Web 资源下载，仅执行原有 Codex 构建流程
