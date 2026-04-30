## 1. Fix findDecoderInPrefix

- [x] 1.1 Rewrite `findDecoderInPrefix` to scan prefix for the function containing base64 alphabet + return-with-index pattern, instead of matching the first `function _0x...()`

## 2. Fix splitBundle Pattern 1

- [x] 2.1 Reduce `findStringTableFunction` search window from 500k to 50k
- [x] 2.2 Add prefix-only branch: when no string table found (or found inside wrapper), set `decoderRuntimeSource = prefixSource`, `helperSource = ""`, `wrapperSource` extends to file end

## 3. Verify

- [x] 3.1 `bun run decode --sync-battle-assets` succeeds without SyntaxError
- [x] 3.2 `bun test` passes
