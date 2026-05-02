# material-concurrency

## Requirements

- material 记录的 read-modify-write 操作必须有并发保护
- 并发的 material 写入不得丢失更新（last-write-wins）
- bucket/torch/devmat/screw 的值不得因 replenish 操作被意外覆盖
