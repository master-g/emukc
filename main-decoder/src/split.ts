import type { BundleSections } from "./types.ts";

const WRAPPER_START_RE = /,\s*!\s*function\(/;
const HELPER_FUNCTION_RE = /function\s+(_0x[0-9a-fA-F]+)\s*\(/g;
const BASE64_ALPHABET = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789+/=";
const STRING_TABLE_FUNCTION_RE = /function\s+(_0x[0-9a-fA-F]+)\s*\(\)\s*\{/g;

function findDecoderInPrefix(prefixSource: string): { decoderFunctionName: string; helperFunctionNames: string[]; hasBase64: boolean } | undefined {
	// Find all function _0x... definitions in the prefix (not just at line start)
	const allFnMatches = [...prefixSource.matchAll(/\bfunction\s+(_0x[0-9a-fA-F]+)\s*\(/g)];

	// Identify the decoder: the function whose body contains the base64 alphabet
	let decoderFunctionName: string | undefined;
	let hasBase64 = false;
	for (const match of allFnMatches) {
		const name = match[1];
		if (name === undefined) continue;
		const window = prefixSource.slice(match.index, match.index + 100000);
		if (window.includes(BASE64_ALPHABET)) {
			decoderFunctionName = name;
			hasBase64 = true;
			break;
		}
	}

	// Fallback: first function if prefix contains base64 somewhere
	if (decoderFunctionName === undefined && allFnMatches.length > 0 && prefixSource.includes(BASE64_ALPHABET)) {
		decoderFunctionName = allFnMatches[0][1];
		hasBase64 = false;
	}

	if (decoderFunctionName === undefined) {
		return undefined;
	}

	const helperFunctionNames = [decoderFunctionName, ...allFnMatches
		.map(m => m[1])
		.filter((name): name is string => name !== undefined && name !== decoderFunctionName)];

	return { decoderFunctionName, helperFunctionNames, hasBase64 };
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

	// Pattern 1: decoder in prefix
	const prefixDecoder = findDecoderInPrefix(prefixSource);
	if (prefixDecoder !== undefined) {
		// New format: decoder has base64 alphabet embedded — self-contained prefix, no string table
		if (prefixDecoder.hasBase64) {
			return {
				wrapperBangIndex,
				helperStartIndex: mainSource.length,
				decoderFunctionName: prefixDecoder.decoderFunctionName,
				helperFunctionNames: prefixDecoder.helperFunctionNames,
				prefixSource,
				helperSource: "",
				wrapperSource: `(${mainSource.slice(wrapperBangIndex + 1)}`,
				decoderRuntimeSource: `${prefixSource})`,
			};
		}

		// Old format: decoder in prefix but needs string table at end of file
		const stringTableName = findStringTableFunction(mainSource, Math.max(0, mainSource.length - 50000));
		const stringTableStart = stringTableName !== undefined
			? mainSource.lastIndexOf(`function ${stringTableName}()`)
			: -1;

		if (stringTableStart === -1 || stringTableStart <= wrapperBangIndex) {
			return {
				wrapperBangIndex,
				helperStartIndex: mainSource.length,
				decoderFunctionName: prefixDecoder.decoderFunctionName,
				helperFunctionNames: prefixDecoder.helperFunctionNames,
				prefixSource,
				helperSource: "",
				wrapperSource: `(${mainSource.slice(wrapperBangIndex + 1)}`,
				decoderRuntimeSource: `${prefixSource})`,
			};
		}

		const helperFunctionNames = stringTableName !== undefined
			? [...prefixDecoder.helperFunctionNames, stringTableName]
			: prefixDecoder.helperFunctionNames;

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
