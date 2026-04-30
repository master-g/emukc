## ADDED Requirements

### Requirement: findDecoderInPrefix identifies the lookup function, not the array function

系统 SHALL 从 prefix 中识别包含 base64 字母表和 `return` + 数组索引的函数作为 decoder，而非第一个 `function _0x...()`。

#### Scenario: New format with array function first, lookup function second
- **WHEN** prefix 包含 `_0x30a5`（数组函数，仅返回数组）和 `_0x5d1e`（包含 base64 字母表和 `return _0x...[_0x...]` 的 lookup 函数）
- **THEN** `findDecoderInPrefix` SHALL 返回 `decoderFunctionName = "_0x5d1e"`

#### Scenario: Old format with single decoder in prefix
- **WHEN** prefix 仅包含一个函数且该函数同时持有数组和 lookup 逻辑
- **THEN** `findDecoderInPrefix` SHALL 返回该函数名（向后兼容）

### Requirement: splitBundle handles prefix-only decoder without string table

系统 SHALL 在无独立 string-table 函数时正确构建 `decoderRuntimeSource`。

#### Scenario: No string table function found after UMD wrapper
- **WHEN** `findStringTableFunction` 未在 wrapper 之后找到合法的 string-table 函数
- **THEN** `decoderRuntimeSource` SHALL 等于 `prefixSource`
- **THEN** `helperSource` SHALL 为空字符串
- **THEN** `wrapperSource` SHALL 为从 `wrapperBangIndex+1` 到文件末尾

#### Scenario: String table function exists at end of file (old format)
- **WHEN** 存在合法的 string-table 函数位于 UMD wrapper 之后
- **THEN** `splitBundle` SHALL 使用现有 Pattern 1 逻辑拆分（向后兼容）

### Requirement: decoderRuntimeSource is syntactically valid JavaScript

系统 SHALL 保证 `decoderRuntimeSource` 是可被 `new Function()` 解析的合法 JavaScript。

#### Scenario: Prefix-only runtime
- **WHEN** 使用 prefix-only 模式
- **THEN** `new Function(decoderRuntimeSource + "; return " + decoderFunctionName)` SHALL 不抛出 SyntaxError

#### Scenario: createDecoder returns a callable function
- **WHEN** `createDecoder` 被调用
- **THEN** 返回值 SHALL 为 `(value: number | string) => string` 类型的函数
