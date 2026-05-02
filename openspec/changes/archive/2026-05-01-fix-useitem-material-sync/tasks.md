## 1. 修改 get_use_items_impl

- [x] 1.1 在 `use_item.rs` 的 `get_use_items_impl` 中增加 material 表查询，获取 bucket/torch/devmat/screw 当前值
- [x] 1.2 合并逻辑：遍历 use_item 结果，对 mst_id=1/2/3/4 用 material 值替换 count；若 use_item 表无对应记录，补充条目
- [x] 1.3 验证 `useitem` 端点返回的 api_id=1/2/3/4 条目 count 与 material 表一致
- [x] 1.4 验证 `require_info` 端点返回的 api_useitem 中 api_id=1/2/3/4 条目 count 与 material 表一致

## 2. 验证 incentive 数据分类

- [x] 2.1 检查 incentive 数据中是否存在 bucket/torch/devmat/screw 被标记为 `IncentiveType::UseItem` 的情况
- [x] 2.2 若存在，修改 incentive 写入路径使其同时更新 material 表

## 3. 测试

- [x] 3.1 添加测试：远征奖励 bucket 后 useitem 返回 material 表的值
- [x] 3.2 添加测试：即修消耗 bucket 后 useitem 返回 material 表的值
- [x] 3.3 添加测试：consume_use_item（奖章兑换 bucket）后 useitem 返回 material 表的值
- [x] 3.4 添加测试：新 profile（无 use_item 记录）useitem 仍返回 material 表的 bucket/torch/devmat/screw
- [x] 3.5 添加测试：require_info 的 api_useitem 中 bucket/torch/devmat/screw 与 material 表一致
- [x] 3.6 cargo test 全量通过
