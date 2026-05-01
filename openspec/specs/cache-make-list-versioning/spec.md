# cache-make-list-versioning

## Requirements

- make-list 生成的 version 必须不低于已缓存 version
- version.json 必须通过 force remote 获取，不使用本地缓存（根因：`NoVersion` 导致返回旧缓存）
- rollback case 必须在 make-list 阶段检测并处理（使用缓存 version 替代），而非等到 populate 阶段
- category 在 version.json 中不存在时，使用已缓存 version（如有）或记录 warn
- populate 遇到 `InvalidFileVersion` 时跳过而非重试（版本回退不是下载失败）
- kache 必须暴露公共 API `get_cached_version(path)` 供 make-list 查询已缓存 version
