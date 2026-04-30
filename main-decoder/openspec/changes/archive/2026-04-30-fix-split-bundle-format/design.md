## Context

`splitBundle` in `split.ts` splits `main.js` into sections for decoding. It supports two patterns:

- **Pattern 1**: decoder function in prefix, string-table function at end of file
- **Pattern 2** (legacy): decoder helper after the UMD wrapper

The current main.js (post April 2026 KanColle update) has a new layout:

```
function _0x30a5(){...}   // string array function (returns encoded array)
function _0x5d1e(...){...} // decoder lookup (base64 decode + array index lookup)
}(_0x30a5,0xbb11b)        // IIFE that shuffles the array
,!function(a,b){...}(...)  // UMD wrapper with all game modules
```

`findDecoderInPrefix` matches `_0x30a5` (the array function, index 0) instead of `_0x5d1e` (the actual decoder). There is no string-table function after the wrapper — the decoder runtime is the entire prefix. `findStringTableFunction` picks a random `function _0x...() {}` deep inside the UMD wrapper, producing a `decoderRuntimeSource` with `prefix);randomInnerCode` that has a syntax error.

## Goals / Non-Goals

**Goals:**
- `splitBundle` correctly handles the new bundle format where decoder + array are both in prefix
- `createDecoder` gets a syntactically valid `decoderRuntimeSource` that evaluates to the decoder function
- Backward-compatible with old bundle formats (Pattern 1 string-table, Pattern 2 legacy)
- Existing tests continue to pass

**Non-Goals:**
- Not changing the overall decode pipeline or module extraction
- Not handling arbitrary obfuscation patterns — only the known formats (old + new)
- Not changing `createDecoder`'s interface

## Decisions

### D1: Identify decoder by base64 alphabet + return pattern

**Choice**: `findDecoderInPrefix` should find the function in the prefix that contains the base64 alphabet literal AND a `return` statement with array indexing — the actual lookup function, not the array holder.

**Rationale**: `_0x30a5` returns a static array. `_0x5d1e` contains the base64 alphabet string and performs decode + lookup. The base64 alphabet is already used as a discriminator in Pattern 2. Matching on `return _0x...[_0x...]` distinguishes the decoder from the array function.

### D2: Detect prefix-only decoder runtime (no string table)

**Choice**: When `findStringTableFunction` returns a function whose position is inside the UMD wrapper body (between `wrapperBangIndex` and end of file), treat it as "no string table found." The `decoderRuntimeSource` should be just the prefix, and `helperSource` should be empty.

**Rationale**: The old format had a real string table after the wrapper. In the new format, all decoder infrastructure is in the prefix. There's nothing to append. Injecting `);` + random inner code creates the syntax error.

### D3: Smaller search window for string table

**Choice**: Reduce `findStringTableFunction` search window from last 500k chars to last 50k chars.

**Rationale**: In old formats the string table was truly at the very end. 500k is too large — it picks up functions deep inside the UMD wrapper body. 50k is sufficient for the genuine string table while avoiding false positives.

## Risks / Trade-offs

- **Old format regression**: Changing `findDecoderInPrefix` could break old bundle parsing. → Mitigation: keep both detection paths, test with old fixtures if available.
- **Future format changes**: The obfuscator could shift patterns again. → Mitigation: clear error messages with diagnostic context (function names found, positions, etc.).
