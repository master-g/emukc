## Context

`cache make-list` 生成 cache list 时，部分资源的 version 比已缓存 version 更旧。回滚触发流程：

```
make-list (versioned/mod.rs:21)
  ├── GetOption::new_non_mod().get(cache, "kcs2/version.json", NoVersion)
  ├── NoVersion → find_in_local 跳过版本检查，返回本地缓存的旧 version.json
  └── 旧 version.json → 过时的 category→version 映射

make-list (versioned/img.rs:552-557)
  ├── 对 LIST 中每个 path: category = path.split('/').next()
  └── list.add("kcs2/img/{path}", versions.get(category))

populate → kache.rs find_in_local (line ~403)
  ├── cmp_version(stored_version, requested_version)
  ├── Greater → InvalidFileVersion (rollback)
  └── 日志: "{path} the required version {requested} is older than the local version {stored}"
```

### 具体案例

`kcs2/img/common/bg_map/bg_y.png`:
- LIST 中 path: `common/bg_map/bg_y.png` (img.rs line 131)
- category: `common`
- version.json 最新版 `common: "6.2.9.0"`
- 旧 version.json 中 `common: "6.2.0.0"`
- make-list 用 NoVersion 获取到旧 version.json → 生成 6.2.0.0 → populate 时与缓存 6.2.9.0 冲突 → InvalidFileVersion

### 根因确认

1. **`NoVersion` 是根因**: `versioned/mod.rs:21` 用 `NoVersion` 获取 version.json，导致永远返回本地缓存版本，不检查时效性
2. **两种策略均受影响**: Manifest 和 Default/Greedy 策略都通过 `kcs2::make_manifest_support()` / `kcs2::make()` 调用 `versioned::make()`，共享同一问题
3. **category 匹配无误**: `path.split('/').next()` 与 version.json 的 key 对应关系正确

### Edge Cases

- **category 缺失**: `versions.get(category)` 返回 None 时，entry 无版本号。当前代码会传 None 给 `list.add()`，后续行为未定义
- **populate 重试无效**: populate 遇到 `InvalidFileVersion` 时重试下载，但版本回退不是下载失败，重试不会修复

## Goals / Non-Goals

**Goals:**
- 确保 make-list 使用最新 version.json（精确机制：替换 NoVersion）
- make-list 检测并处理 rollback case
- populate 不对 `InvalidFileVersion` 做无意义重试
- 处理 category 缺失 edge case

**Non-Goals:**
- 不改变 version 四段式格式
- 不修改 redb 存储结构

## Decisions

### Decision 1: version.json 使用 force remote 获取

将 `NoVersion` 替换为强制远程获取。选项：

- **Option A**: 使用 `GetOption::new_remote_only()` 或等效配置（`enable_local: false, enable_remote: true`），每次从 CDN 获取最新 version.json
- **Option B**: 在获取前先删除本地 version.json 缓存，然后正常获取

推荐 Option A。version.json 仅几 KB，强制远程获取的延迟可忽略。

### Decision 2: 暴露 kache version 查询 API 并在 make-list 检测 rollback

**前置**: 在 kache 中添加公共方法 `get_cached_version(&self, path: &str) -> Option<String>`，封装 `read_version_from_db`。

在 `img.rs` 的 `make()` 函数中，对每个 entry：
1. 调用 `cache.get_cached_version(path)` 获取已缓存 version
2. 如果已缓存 version > version.json 中对应 version → warn 日志 + 使用缓存 version
3. 如果 category 在 version.json 中不存在 → warn 日志 + 使用缓存 version（如有）

### Decision 3: populate 跳过 InvalidFileVersion

在 populate 的错误处理中，对 `InvalidFileVersion` 做特殊处理：
- 不重试，直接跳过
- 记录 warn 日志（version rollback 已在 make-list 阶段处理，此处为兜底）

## Risks / Trade-offs

- [Risk] 强制远程获取 version.json 可能增加延迟 → Mitigation: 仅几 KB，可忽略
- [Risk] 使用缓存 version 而非 version.json version 可能导致资源不一致 → Mitigation: 缓存 version 来自上次成功下载，应该可信
- [Risk] 暴露 `get_cached_version` 增加 kache 公共 API 面积 → Mitigation: 只读查询，风险低
