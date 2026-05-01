# quest-factory-events

## Requirements

- 批量开发 (api_multiple_flag=1) 必须正确推进开发类任务进度
- 每个成功开发的 item 必须触发独立的 quest progress update
- 开发类任务的 condition 必须正确匹配 SlotItemConstructed 事件
- 如有需要，开发失败也应有对应事件类型
