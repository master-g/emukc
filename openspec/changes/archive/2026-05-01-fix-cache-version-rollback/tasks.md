## 1. 修复 version.json 获取方式

- [x] 1.1 在 `versioned/mod.rs` 中将 `NoVersion` 替换为 force remote 获取（使用 `GetOption` 配置 `enable_local: false` 或等效方案）
- [x] 1.2 验证 force remote 方案是否需要新增 `GetOption` 构造方法（如 `new_remote_only()`），若需要则先添加
- [x] 1.3 添加 trace 日志：记录 version.json 获取来源（local/remote）和内容 hash

## 2. 暴露 kache version 查询 API

- [x] 2.1 在 `kache.rs` 中添加公共方法 `pub async fn get_cached_version(&self, path: &str) -> Result<Option<String>>`
- [x] 2.2 封装现有的 `read_version_from_db` 逻辑

## 3. make-list 添加 rollback 检测

- [x] 3.1 在 `img.rs` 的 `make()` 中，对每个 LIST entry 调用 `cache.get_cached_version(path)` 比较版本
- [x] 3.2 如果缓存 version > version.json version → warn 日志 + 使用缓存 version
- [x] 3.3 如果 category 在 version.json 中不存在（`versions.get(category)` 返回 None）→ warn 日志 + 使用缓存 version（如有）

## 4. populate 改进 InvalidFileVersion 处理

- [x] 4.1 在 `populate.rs` 中区分 `InvalidFileVersion` 与其他下载错误
- [x] 4.2 `InvalidFileVersion` 不重试，直接跳过并记录 warn 日志

## 5. 测试

- [x] 5.1 测试：make-list 使用 force remote 获取 version.json（mock 远程响应）
- [x] 5.2 测试：make-list 检测到 rollback 时使用缓存 version
- [x] 5.3 测试：category 缺失时的行为（warn + 使用缓存 version）
- [x] 5.4 测试：populate 遇到 InvalidFileVersion 时跳过而非重试
- [x] 5.5 cargo test 相关模块通过
