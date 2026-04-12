import type { BundleSections } from "./types.ts";

const WRAPPER_START_RE = /,\s*!\s*function\(/;
const HELPER_FUNCTION_RE = /function\s+(_0x[0-9a-fA-F]+)\s*\(/g;
const BASE64_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
const DECODER_FUNCTION_START_RE = /^function\s+(_0x[0-9a-fA-F]+)\s*\(/;
const STRING_TABLE_FUNCTION_RE = /function\s+(_0x[0-9a-fA-F]+)\s*\(\)\s*\{/g;

function findDecoderInPrefix(prefixSource: string): { decoderFunctionName: string; helperFunctionNames: string[] } | undefined {
	const decoderMatch = DECODER_FUNCTION_START_RE.exec(prefixSource);
	if (decoderMatch === null) {
		return undefined;
	}

	const decoderFunctionName = decoderMatch[1];
	if (decoderFunctionName === undefined) {
		return undefined;
	}

	if (!prefixSource.includes(BASE64_ALPHABET)) {
		return undefined;
	}

	const helperFunctionNames = [decoderFunctionName, ...prefixSource.matchAll(HELPER_FUNCTION_RE)
		.map(match => match[1])
		.filter((name): name is string => name !== undefined && name !== decoderFunctionName)];

	return { decoderFunctionName, helperFunctionNames };
}

function findStringTableFunction(mainSource: string, searchFromEnd: number): string | undefined {
	const tail = mainSource.slice(searchFromEnd);
	const matches = [...tail.matchAll(STRING_TABLE_FUNCTION_RE)];
	if (matches.length === 0) {
		return undefined;
	}

	const lastName = matches[matches.length - 1]?.[1];
	return lastName;
}

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
	const prefixSource = mainSource.slice(0, wrapperMatch.index);

	// Pattern 1 (current): decoder in prefix, string table at end of file
	const prefixDecoder = findDecoderInPrefix(prefixSource);
	if (prefixDecoder !== undefined) {
		const stringTableName = findStringTableFunction(mainSource, Math.max(0, mainSource.length - 500000));
		const helperFunctionNames = stringTableName !== undefined
			? [...prefixDecoder.helperFunctionNames, stringTableName]
			: prefixDecoder.helperFunctionNames;

		// The wrapper source is between the bang and the string table (or end of file)
		const stringTableStart = stringTableName !== undefined
			? mainSource.lastIndexOf(`function ${stringTableName}()`)
			: mainSource.length;
		if (stringTableStart === -1 || stringTableStart <= wrapperBangIndex) {
			throw new Error("Failed to locate the string table function boundaries");
		}

		return {
			wrapperBangIndex,
			helperStartIndex: stringTableStart,
			decoderFunctionName: prefixDecoder.decoderFunctionName,
			helperFunctionNames,
			prefixSource,
			helperSource: mainSource.slice(stringTableStart),
			wrapperSource: `(${mainSource.slice(wrapperBangIndex + 1, stringTableStart)}`,
			decoderRuntimeSource: `${prefixSource});${mainSource.slice(stringTableStart)}`,
		};
	}

	// Pattern 2 (legacy): decoder helper after the UMD wrapper, containing base64 alphabet
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
