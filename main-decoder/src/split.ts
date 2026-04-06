import type { BundleSections } from "./types.ts";

const WRAPPER_START_RE = /,\s*!\s*function\(/;
const HELPER_FUNCTION_RE = /function\s+(_0x[0-9a-fA-F]+)\s*\(/g;
const BASE64_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";

export function splitBundle(mainSource: string): BundleSections {
  const wrapperMatch = WRAPPER_START_RE.exec(mainSource);
  if (wrapperMatch === null) {
    throw new Error("Failed to locate the start of the UMD wrapper");
  }

  const bangOffset = wrapperMatch[0].indexOf("!");
  if (bangOffset === -1) {
    throw new Error("Failed to locate the wrapper bang token");
  }

  const wrapperBangIndex = wrapperMatch.index + bangOffset;
  const lastDecodeURIComponentIndex = mainSource.lastIndexOf("decodeURIComponent");
  if (lastDecodeURIComponentIndex === -1) {
    throw new Error("Failed to locate the decoder helper via decodeURIComponent");
  }

  const helperStartIndex = mainSource.lastIndexOf("function _0x", lastDecodeURIComponentIndex);
  if (helperStartIndex === -1) {
    throw new Error("Failed to locate the start of the helper tail");
  }
  if (wrapperBangIndex >= helperStartIndex) {
    throw new Error("Detected invalid bundle boundaries");
  }

  const helperSource = mainSource.slice(helperStartIndex);
  if (!helperSource.includes(BASE64_ALPHABET)) {
    throw new Error("Helper tail does not contain the expected base64 alphabet literal");
  }

  const decoderNameMatch = /^function\s+(_0x[0-9a-fA-F]+)\s*\(/.exec(helperSource);
  const decoderFunctionName = decoderNameMatch?.[1];
  if (decoderFunctionName === undefined) {
    throw new Error("Failed to determine the decoder function name");
  }

  const helperFunctionNames = [...helperSource.matchAll(HELPER_FUNCTION_RE)]
    .map(match => match[1])
    .filter((name): name is string => name !== undefined);

  return {
    wrapperBangIndex,
    helperStartIndex,
    decoderFunctionName,
    helperFunctionNames,
    prefixSource: mainSource.slice(0, wrapperMatch.index),
    helperSource,
    wrapperSource: `(${mainSource.slice(wrapperBangIndex + 1, helperStartIndex)}`,
    decoderRuntimeSource: `${mainSource.slice(0, wrapperMatch.index)});${helperSource}`,
  };
}
