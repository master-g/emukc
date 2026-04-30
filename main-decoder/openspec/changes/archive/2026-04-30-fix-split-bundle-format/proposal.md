## Why

KanColle's latest `main.js` uses a new obfuscation layout: the string-array function (`_0x30a5`) and decoder lookup function (`_0x5d1e`) are both in the prefix before the UMD wrapper, and there is no separate string-table function at the end of the file. `splitBundle` in `split.ts` picks the wrong decoder (`_0x30a5`, the array function) and locates a false "string table" function inside the wrapper body, producing a `decoderRuntimeSource` with a syntax error (`SyntaxError: Parser error`). This blocks `bun run decode` entirely.

## What Changes

- `findDecoderInPrefix` must identify the actual decoder lookup function (the one containing base64 decode logic and `return`) rather than the first `function _0x...()` in the prefix
- `splitBundle` Pattern 1 must handle bundles where no string-table function exists after the UMD wrapper — the decoder runtime is the prefix itself
- `decoderRuntimeSource` construction must not inject an unmatched `)` when there is no trailing string table

## Capabilities

### New Capabilities
- `bundle-split-lookup-prefix`: split main.js bundles where the decoder lookup function (not just the array function) lives entirely in the prefix, with no separate string table at end of file

### Modified Capabilities

## Impact

- `main-decoder/src/split.ts` — `findDecoderInPrefix`, `findStringTableFunction`, and `splitBundle` Pattern 1
- `main-decoder/src/decode.ts` — `createDecoder` (may need to handle both old and new `decoderRuntimeSource` formats)
- `main-decoder/test/` — test fixtures and split tests need new bundle samples
