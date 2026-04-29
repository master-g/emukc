import type { ModuleArtifact, ModuleGraph, ResourceCoverageMode, CacheRulesAsset } from "./types.ts";
import type { ResourceManifest } from "./resource-manifest.ts";
import type { ResourceCategoriesAsset } from "./types.ts";

interface ExtractedCacheRules {
	shipRules: CacheRulesAsset["shipRules"];
	slotRules: CacheRulesAsset["slotRules"];
	soundRules: CacheRulesAsset["soundRules"];
}

function emptyProvenance(): { moduleIds: Set<string>; moduleNames: Set<string> } {
	return { moduleIds: new Set(), moduleNames: new Set() };
}

function moduleName(module: ModuleArtifact): string {
	return module.readableName ?? module.fileName;
}

function addProvenance(provenance: { moduleIds: Set<string>; moduleNames: Set<string> }, module: ModuleArtifact): void {
	provenance.moduleIds.add(module.id);
	provenance.moduleNames.add(moduleName(module));
}

function sortedProvenance(provenance: { moduleIds: Set<string>; moduleNames: Set<string> }) {
	return {
		moduleIds: [...provenance.moduleIds].sort(),
		moduleNames: [...provenance.moduleNames].sort(),
	};
}

function coverageModeFromCount(count: number, complete = false): ResourceCoverageMode {
	if (count === 0) {
		return "unresolved";
	}
	return complete ? "observed-complete" : "partial";
}

function numericLiterals(source: string): number[] {
	return [...source.matchAll(/\b(?:0x[0-9a-fA-F]+|\d+)\b/g)]
		.map(match => Number.parseInt(match[0] ?? "0", 0))
		.filter(value => Number.isInteger(value) && value > 0);
}

function parseObjectNumberMap(source: string, exportName: string): Record<string, number> {
	const match = new RegExp(`${exportName}\\s*=\\s*\\{([\\s\\S]*?)\\};`).exec(source);
	if (match === null) {
		return {};
	}
	const entries = [...(match[1] ?? "").matchAll(/(\d+)\s*:\s*(0x[0-9a-fA-F]+|\d+)/g)];
	return Object.fromEntries(entries.map(entry => [entry[1] ?? "", Number.parseInt(entry[2] ?? "0", 0)]));
}

function parseEnemySlotBorder(source: string): number | undefined {
	const match = /ENEMY_SLOT_BORDER\s*=\s*(0x[0-9a-fA-F]+|\d+)/.exec(source);
	return match === null ? undefined : Number.parseInt(match[1] ?? "0", 0);
}

function parseExcludeResources(source: string): Array<{ type: string; mstId: number }> {
	const blocks = [...source.matchAll(/\{[^{}]*(?:['"]?type['"]?)\s*:\s*['"]([^'"]+)['"][^{}]*(?:['"]?mst_id['"]?)\s*:\s*(0x[0-9a-fA-F]+|\d+)[^{}]*\}/g)];
	return blocks.map(block => ({
		type: block[1] ?? "",
		mstId: Number.parseInt(block[2] ?? "0", 0),
	})).filter(entry => entry.type.length > 0 && entry.mstId > 0);
}

function parseStaticNumberArray(source: string, pattern: RegExp): number[] {
	const match = pattern.exec(source);
	if (match === null) {
		return [];
	}
	return [...(match[1] ?? "").matchAll(/0x[0-9a-fA-F]+|\d+/g)]
		.map(entry => Number.parseInt(entry[0] ?? "0", 0))
		.filter(value => Number.isInteger(value) && value > 0);
}

function parseAlbumAddImageSlots(source: string): number[] {
	return parseStaticNumberArray(source, /ADD_IMAGE_SLOTS\s*=\s*\[([^\]]*)\]/);
}

function parseVoiceDiffsFromSource(source: string): number[] {
	for (const marker of ['"voice":[', 'voice\\":[', "'voice':["]) {
		const start = source.indexOf(marker);
		if (start < 0) {
			continue;
		}
		const valuesStart = start + marker.length;
		const valuesEnd = source.indexOf("]", valuesStart);
		if (valuesEnd < 0) {
			continue;
		}
		const values = source
			.slice(valuesStart, valuesEnd)
			.split(",")
			.map(value => Number.parseInt(value.trim(), 10))
			.filter(value => Number.isInteger(value) && value > 0);
		if (values.length >= 20) {
			return values;
		}
	}
	return [];
}

function parseShipVoiceFormula(source: string): CacheRulesAsset["soundRules"]["shipVoices"]["formula"] | undefined {
	const maxMatch = source.match(/<=\s*(\d+)/);
	const formulaMatch = source.match(/(\d+)\s*\*\s*\([^)]*\+\s*(\d+)\)\s*\*\s*.*?%\s*(\d+)\s*\+\s*(\d+)/s);
	if (maxMatch === null || formulaMatch === null) {
		return undefined;
	}
	return {
		maxFormulaVoiceId: Number.parseInt(maxMatch[1] ?? "0", 10),
		multiplier: Number.parseInt(formulaMatch[1] ?? "0", 10),
		shipIdOffset: Number.parseInt(formulaMatch[2] ?? "0", 10),
		modulo: Number.parseInt(formulaMatch[3] ?? "0", 10),
		base: Number.parseInt(formulaMatch[4] ?? "0", 10),
		voiceDiffs: [],
	};
}

function classifyVoiceTarget(expression: string): "ship" | "9997" | "9998" | "9999" | "other" {
	const normalized = expression.replace(/\s+/g, "");
	if (normalized.includes('"9997"') || normalized.includes("'9997'") || normalized === "9997") {
		return "9997";
	}
	if (normalized.includes('"9998"') || normalized.includes("'9998'") || normalized === "9998") {
		return "9998";
	}
	if (normalized.includes('"9999"') || normalized.includes("'9999'") || normalized === "9999") {
		return "9999";
	}
	if (
		normalized.includes('"titlecall_')
		|| normalized.includes("'titlecall_")
		|| normalized.includes('"tutorial"')
		|| normalized.includes("'tutorial'")
	) {
		return "other";
	}
	return "ship";
}

function addNumericList(target: Set<number>, values: string): void {
	for (const value of values.split(",")) {
		const parsed = Number.parseInt(value.trim(), 10);
		if (Number.isInteger(parsed) && parsed > 0) {
			target.add(parsed);
		}
	}
}

function addVoicePlayObservations(
	source: string,
	shipVoiceIds: Set<number>,
	bucketIds: Record<"9997" | "9998" | "9999", Set<number>>,
	bucketDynamic: Record<"9997" | "9998" | "9999", boolean>,
): boolean {
	let found = false;

	for (const match of source.matchAll(/sound\.voice\.play\(([^,]+),\s*([^,)]+)[^)]*\)/g)) {
		const targetKind = classifyVoiceTarget(match[1] ?? "");
		const rawVoice = (match[2] ?? "").trim();
		const numericVoice = Number.parseInt(rawVoice, 10);
		if (targetKind === "other") {
			continue;
		}
		if (Number.isInteger(numericVoice) && numericVoice > 0) {
			found = true;
			if (targetKind === "ship") {
				shipVoiceIds.add(numericVoice);
			} else {
				bucketIds[targetKind].add(numericVoice);
			}
		} else if (targetKind !== "ship") {
			bucketDynamic[targetKind] = true;
			found = true;
		}
	}

	for (const match of source.matchAll(/sound\.voice\.playAtRandom\(([^,]+),\s*\[([^\]]+)\]/g)) {
		const targetKind = classifyVoiceTarget(match[1] ?? "");
		if (targetKind === "other") {
			continue;
		}
		const values = new Set<number>();
		addNumericList(values, match[2] ?? "");
		if (values.size === 0) {
			if (targetKind !== "ship") {
				bucketDynamic[targetKind] = true;
			}
			continue;
		}
		found = true;
		if (targetKind === "ship") {
			for (const value of values) {
				shipVoiceIds.add(value);
			}
		} else {
			for (const value of values) {
				bucketIds[targetKind].add(value);
			}
		}
	}

	return found;
}

function addLocalVoicePlayObservations(
	source: string,
	bucketIds: Record<"9997" | "9998" | "9999", Set<number>>,
	bucketDynamic: Record<"9997" | "9998" | "9999", boolean>,
): boolean {
	let found = false;
	for (const match of source.matchAll(/\b_?playVoice\(\s*(999[789])\s*,\s*([^,)]+)[^)]*\)/g)) {
		const bucket = match[1] as "9997" | "9998" | "9999" | undefined;
		if (bucket === undefined) {
			continue;
		}
		const rawVoice = (match[2] ?? "").trim();
		const numericVoice = Number.parseInt(rawVoice, 10);
		if (Number.isInteger(numericVoice) && numericVoice > 0) {
			bucketIds[bucket].add(numericVoice);
		} else {
			bucketDynamic[bucket] = true;
		}
		found = true;
	}
	return found;
}

function addEnemyVoiceConstObservations(source: string, target: Set<number>): boolean {
	let found = false;
	for (const match of source.matchAll(/['"][ad]['"]\s*:\s*\[([0-9,\s]+)\]/g)) {
		addNumericList(target, match[1] ?? "");
		found = true;
	}
	for (const match of source.matchAll(/return\s+([0-9]{6,})\s*;/g)) {
		const value = Number.parseInt(match[1] ?? "0", 10);
		if (Number.isInteger(value) && value > 0) {
			target.add(value);
			found = true;
		}
	}
	for (const match of source.matchAll(/\?\s*([0-9]{6,})\s*:\s*-?1\b/g)) {
		const value = Number.parseInt(match[1] ?? "0", 10);
		if (Number.isInteger(value) && value > 0) {
			target.add(value);
			found = true;
		}
	}
	return found;
}

const SHIP_TARGET_SEMANTIC_CASES: CacheRulesAsset["shipRules"]["targetSemantics"]["cases"] = [
	{ rawTargetType: "banner", selectorScope: "default-friendly", damagedState: "false", targetTypes: ["banner"] },
	{ rawTargetType: "banner", selectorScope: "default-friendly", damagedState: "true", targetTypes: ["banner_dmg"] },
	{ rawTargetType: "banner", selectorScope: "default-friendly", damagedState: "variable", targetTypes: ["banner", "banner_dmg"] },
	{ rawTargetType: "banner", selectorScope: "default-abyssal", damagedState: "false", targetTypes: ["banner"] },
	{ rawTargetType: "banner", selectorScope: "default-abyssal", damagedState: "true", targetTypes: ["banner"] },
	{ rawTargetType: "banner", selectorScope: "default-abyssal", damagedState: "variable", targetTypes: ["banner"] },
	{ rawTargetType: "banner_g", selectorScope: "default-friendly", damagedState: "true", targetTypes: ["banner_g_dmg"] },
	{ rawTargetType: "banner2", selectorScope: "default-friendly", damagedState: "false", targetTypes: ["banner2"] },
	{ rawTargetType: "banner2", selectorScope: "default-friendly", damagedState: "true", targetTypes: ["banner2_dmg"] },
	{ rawTargetType: "banner2", selectorScope: "default-friendly", damagedState: "variable", targetTypes: ["banner2", "banner2_dmg"] },
	{ rawTargetType: "banner2_g", selectorScope: "default-friendly", damagedState: "true", targetTypes: ["banner2_g_dmg"] },
	{ rawTargetType: "banner3", selectorScope: "default-abyssal", damagedState: "false", targetTypes: ["banner3"] },
	{ rawTargetType: "banner3", selectorScope: "default-abyssal", damagedState: "true", targetTypes: ["banner3"] },
	{ rawTargetType: "banner3", selectorScope: "default-abyssal", damagedState: "variable", targetTypes: ["banner3"] },
	{ rawTargetType: "banner3_g", selectorScope: "default-abyssal", damagedState: "true", targetTypes: ["banner3_g_dmg"] },
];

const SHIP_TARGET_SEMANTIC_REQUIRED_TARGETS = [
	"banner",
	"banner_g",
	"banner2",
	"banner2_g",
	"banner3",
	"banner3_g",
] as const;

function addShipTargetSemanticObservations(source: string, targetTypes: Set<string>): boolean {
	let found = false;
	for (const target of SHIP_TARGET_SEMANTIC_REQUIRED_TARGETS) {
		if (source.includes(`"${target}"`) || source.includes(`'${target}'`)) {
			targetTypes.add(target);
			found = true;
		}
	}
	return found;
}

function addSpecialCasesFromSource(
	source: string,
	casesByKey: Map<string, { damaged: boolean; shipIds: Set<number> }>,
): boolean {
	let found = false;
	for (const match of source.matchAll(/([^;{}]+?)\?\s*[^;{}]*["']full["'][^;{}]*:\s*[^;{}]*["']special["']/g)) {
		const condition = match[1] ?? "";
		const shipIds = numericLiterals(condition).filter(value => value !== 0);
		if (shipIds.length === 0) {
			continue;
		}
		const damaged = !/0\s*!=\s*[A-Za-z0-9_.$]+|[A-Za-z0-9_.$]+\s*!=\s*0/.test(condition);
		const key = String(damaged);
		const existing = casesByKey.get(key) ?? { damaged, shipIds: new Set<number>() };
		for (const shipId of shipIds) {
			existing.shipIds.add(shipId);
		}
		casesByKey.set(key, existing);
		found = true;
	}
	return found;
}

export function extractCacheRules(moduleGraph: ModuleGraph): ExtractedCacheRules {
	const specialProvenance = emptyProvenance();
	const specialCases = new Map<string, { damaged: boolean; shipIds: Set<number> }>();
	const shipTargetSemanticsProvenance = emptyProvenance();
	const itemUpProvenance = emptyProvenance();
	const btxtFlatProvenance = emptyProvenance();
	const itemUp2Provenance = emptyProvenance();
	const itemOn2Provenance = emptyProvenance();
	const shipVoiceProvenance = emptyProvenance();
	const kc9997Provenance = emptyProvenance();
	const kc9998Provenance = emptyProvenance();
	const kc9999Provenance = emptyProvenance();

	let itemUpReplaceMap: Record<string, number> = {};
	let enemySlotBorder: number | undefined;
	let excludes: Array<{ type: string; mstId: number }> = [];
	let hasItemUpNormalization = false;
	let hasBtxtFlatNonEnemyRule = false;
	const addImageSlots = new Set<number>();
	const shipTargetSemanticTypes = new Set<string>();
	let shipVoiceFormula = undefined as CacheRulesAsset["soundRules"]["shipVoices"]["formula"] | undefined;
	const shipVoiceIds = new Set<number>();
	const kc9997Ids = new Set<number>();
	const kc9998Ids = new Set<number>();
	const kc9999Ids = new Set<number>();
	const soundBucketDynamic = {
		"9997": false,
		"9998": false,
		"9999": false,
	} as Record<"9997" | "9998" | "9999", boolean>;
	let hasBeLeftVoice = false;
	let hasBeLeftTiredVoice = false;
	let hasTimeSignalVoice = false;
	const observedSpecialVoiceIds = new Set<number>();

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (
			module.readableName !== "EnemyVoiceConst"
			&& !source.includes("special")
			&& !source.includes("ITEMUP_REPLACE")
			&& !source.includes("ENEMY_SLOT_BORDER")
			&& !source.includes("btxt_flat")
			&& !source.includes("banner_g")
			&& !source.includes("banner2_g")
			&& !source.includes("banner3_g")
			&& !source.includes("ADD_IMAGE_SLOTS")
			&& !source.includes("item_up2")
			&& !source.includes("item_on2")
			&& !source.includes("sound.voice.")
			&& !source.includes('voice.play(')
			&& !source.includes('voice.playAtRandom(')
			&& !source.includes("availableBeLeftVoice")
			&& !source.includes("availableBeLeftVoices")
			&& !source.includes("availableTimeSignalVoice")
			&& !source.includes('voice\\":[')
			&& !source.includes('"voice":[')
			&& !source.includes("voice[")
			&& !source.includes("9997")
			&& !source.includes("9998")
			&& !source.includes("9999")
			&& !source.includes("EnemyVoiceConst")
		) {
			continue;
		}

		if (source.includes("special") && addSpecialCasesFromSource(source, specialCases)) {
			addProvenance(specialProvenance, module);
		}

		if (source.includes("ITEMUP_REPLACE")) {
			const parsedMap = parseObjectNumberMap(source, "ITEMUP_REPLACE");
			if (Object.keys(parsedMap).length > 0) {
				itemUpReplaceMap = { ...itemUpReplaceMap, ...parsedMap };
				addProvenance(itemUpProvenance, module);
			}
		}

		const parsedBorder = parseEnemySlotBorder(source);
		if (parsedBorder !== undefined) {
			enemySlotBorder = parsedBorder;
			addProvenance(itemUpProvenance, module);
		}

		const parsedExcludes = parseExcludeResources(source).filter(entry => entry.type === "item_up" || entry.type === "btxt_flat");
		if (parsedExcludes.length > 0) {
			excludes = [...excludes, ...parsedExcludes];
			addProvenance(itemUpProvenance, module);
		}

		if (source.includes("item_up") && source.includes("ITEMUP_REPLACE") && source.includes("ENEMY_SLOT_BORDER")) {
			hasItemUpNormalization = true;
			addProvenance(itemUpProvenance, module);
		}

		if (source.includes("btxt_flat") && source.includes("isEnemyItem")) {
			hasBtxtFlatNonEnemyRule = true;
			addProvenance(btxtFlatProvenance, module);
		}

		if (addShipTargetSemanticObservations(source, shipTargetSemanticTypes)) {
			addProvenance(shipTargetSemanticsProvenance, module);
		}

		const parsedVoiceFormula = parseShipVoiceFormula(source);
		if (parsedVoiceFormula !== undefined) {
			shipVoiceFormula = shipVoiceFormula === undefined
				? parsedVoiceFormula
				: {
					...parsedVoiceFormula,
					voiceDiffs: shipVoiceFormula.voiceDiffs,
				};
			addProvenance(shipVoiceProvenance, module);
		}

		const parsedVoiceDiffs = parseVoiceDiffsFromSource(source);
		if (parsedVoiceDiffs.length > 0) {
			if (shipVoiceFormula === undefined) {
				shipVoiceFormula = {
					base: 0,
					multiplier: 0,
					shipIdOffset: 0,
					modulo: 0,
					maxFormulaVoiceId: 0,
					voiceDiffs: parsedVoiceDiffs,
				};
			} else {
				shipVoiceFormula.voiceDiffs = parsedVoiceDiffs;
			}
			addProvenance(shipVoiceProvenance, module);
		}

		if (addVoicePlayObservations(source, shipVoiceIds, { "9997": kc9997Ids, "9998": kc9998Ids, "9999": kc9999Ids }, soundBucketDynamic)) {
			const has9997 = source.includes('"9997"') || source.includes("'9997'") || source.includes("(9997");
			const has9998 = source.includes('"9998"') || source.includes("'9998'") || source.includes("(9998");
			const has9999 = source.includes('"9999"') || source.includes("'9999'") || source.includes("(9999");
			if (has9997) addProvenance(kc9997Provenance, module);
			if (has9998) addProvenance(kc9998Provenance, module);
			if (has9999) addProvenance(kc9999Provenance, module);
			if (!has9997 && !has9998 && !has9999) addProvenance(shipVoiceProvenance, module);
		}

		if (addLocalVoicePlayObservations(source, { "9997": kc9997Ids, "9998": kc9998Ids, "9999": kc9999Ids }, soundBucketDynamic)) {
			if (source.includes("(9997")) addProvenance(kc9997Provenance, module);
			if (source.includes("(9998")) addProvenance(kc9998Provenance, module);
			if (source.includes("(9999")) addProvenance(kc9999Provenance, module);
		}

		if (source.includes("9997") && !source.includes('sound.voice.play("9997"') && !source.includes("sound.voice.play(9997")) {
			soundBucketDynamic["9997"] = true;
			addProvenance(kc9997Provenance, module);
		}

		if (source.includes("_enabled_029") && source.includes("play(this._mst_id.toString(), 29)")) {
			hasBeLeftVoice = true;
			addProvenance(shipVoiceProvenance, module);
		}

		if (source.includes("_enabled_129") && source.includes("play(this._mst_id.toString(), 129)")) {
			hasBeLeftTiredVoice = true;
			addProvenance(shipVoiceProvenance, module);
		}

		if (source.includes("_enabled_timeSignal") && source.includes("this._voicehour + 30")) {
			hasTimeSignalVoice = true;
			addProvenance(shipVoiceProvenance, module);
		}

		if ((source.includes('"9998"') || source.includes("'9998'")) && addEnemyVoiceConstObservations(source, kc9998Ids)) {
			addProvenance(kc9998Provenance, module);
		}
		if ((module.readableName === "EnemyVoiceConst" || source.includes("EnemyVoiceConst")) && addEnemyVoiceConstObservations(source, kc9998Ids)) {
			addProvenance(kc9998Provenance, module);
		}

		if (source.includes('sound.voice.play(') && /,\s*90[0-9]\b/.test(source)) {
			for (const match of source.matchAll(/,\s*(90[0-9])\b/g)) {
				const value = Number.parseInt(match[1] ?? "0", 10);
				if (Number.isInteger(value) && value > 0) {
					observedSpecialVoiceIds.add(value);
				}
			}
			addProvenance(shipVoiceProvenance, module);
		}

		const parsedAddImageSlots = parseAlbumAddImageSlots(source);
		if (parsedAddImageSlots.length > 0) {
			for (const slotId of parsedAddImageSlots) {
				addImageSlots.add(slotId);
			}
			addProvenance(itemUp2Provenance, module);
			addProvenance(itemOn2Provenance, module);
		}

		if (source.includes("item_up2")) {
			addProvenance(itemUp2Provenance, module);
		}

		if (source.includes("item_on2")) {
			addProvenance(itemOn2Provenance, module);
		}
	}

	const specialCaseList = [...specialCases.values()]
		.map(entry => ({
			damaged: entry.damaged,
			shipIds: [...entry.shipIds].sort((left, right) => left - right),
		}))
		.sort((left, right) => Number(left.damaged) - Number(right.damaged));
	const uniqueExcludes = new Map<string, { type: string; mstId: number }>();
	for (const entry of excludes) {
		uniqueExcludes.set(`${entry.type}:${entry.mstId}`, entry);
	}
	const addImageSlotIds = [...addImageSlots].sort((left, right) => left - right);
	const specialArtShipIds = specialCaseList
		.filter(entry => entry.damaged === false)
		.flatMap(entry => entry.shipIds)
		.sort((left, right) => left - right);
	const uniqueSpecialArtShipIds = [...new Set(specialArtShipIds)];
	const resolvedSpecialVoiceIds = [...observedSpecialVoiceIds]
		.filter(value => value === 900)
		.sort((left, right) => left - right);
	const unresolvedSpecialVoiceIds = [...observedSpecialVoiceIds]
		.filter(value => value !== 900)
		.sort((left, right) => left - right);
	const shipVoiceBaseIds = [...shipVoiceIds]
		.filter(value => value > 0 && value < 900 && value !== 29 && value !== 129 && (value < 30 || value > 53))
		.sort((left, right) => left - right);
	const shipVoiceCoverageSignals = [
		shipVoiceFormula !== undefined && shipVoiceFormula.base > 0 && shipVoiceFormula.voiceDiffs.length > 0,
		shipVoiceBaseIds.length > 0,
		hasBeLeftVoice,
		hasBeLeftTiredVoice,
		hasTimeSignalVoice,
	].filter(Boolean).length;
	const shipVoiceCoverageMode: ResourceCoverageMode = shipVoiceCoverageSignals === 0
		? "unresolved"
		: uniqueSpecialArtShipIds.length > 0 && resolvedSpecialVoiceIds.length > 0 && unresolvedSpecialVoiceIds.length === 0
			? "observed-complete"
			: "partial";
	const hasAnyShipTargetSemantics = shipTargetSemanticTypes.size > 0;
	const hasCompleteShipTargetSemantics = SHIP_TARGET_SEMANTIC_REQUIRED_TARGETS
		.every(target => shipTargetSemanticTypes.has(target));
	const shipTargetSemanticsCoverageMode: ResourceCoverageMode = hasCompleteShipTargetSemantics
		? "observed-complete"
		: hasAnyShipTargetSemantics
			? "partial"
			: "unresolved";

	return {
		shipRules: {
			special: {
				kind: "special_cases",
				coverageMode: coverageModeFromCount(specialCaseList.length, specialCaseList.length > 0),
				cases: specialCaseList,
				...sortedProvenance(specialProvenance),
			},
			targetSemantics: {
				kind: "ship_target_semantics",
				coverageMode: shipTargetSemanticsCoverageMode,
				cases: hasCompleteShipTargetSemantics ? SHIP_TARGET_SEMANTIC_CASES : [],
				...sortedProvenance(shipTargetSemanticsProvenance),
			},
		},
		slotRules: {
			itemUp: {
				kind: "item_up_normalization",
				coverageMode: coverageModeFromCount(
					Object.keys(itemUpReplaceMap).length + (enemySlotBorder === undefined ? 0 : 1) + (hasItemUpNormalization ? 1 : 0),
					hasItemUpNormalization && enemySlotBorder !== undefined,
				),
				replaceMap: Object.fromEntries(Object.entries(itemUpReplaceMap).sort(([left], [right]) => Number(left) - Number(right))),
				enemySlotBorder,
				exclude: [...uniqueExcludes.values()].filter(entry => entry.type === "item_up").sort((left, right) => left.mstId - right.mstId),
				...sortedProvenance(itemUpProvenance),
			},
			btxtFlat: {
				kind: "btxt_flat_non_enemy_runtime_slots",
				coverageMode: hasBtxtFlatNonEnemyRule ? "observed-complete" : "unresolved",
				excludeEnemyItems: hasBtxtFlatNonEnemyRule,
				...sortedProvenance(btxtFlatProvenance),
			},
			itemUp2: {
				kind: "observed_slot_subset",
				coverageMode: addImageSlotIds.length > 0 ? "observed-complete" : "unresolved",
				ids: addImageSlotIds,
				...sortedProvenance(itemUp2Provenance),
			},
			itemOn2: {
				kind: "observed_slot_subset",
				coverageMode: addImageSlotIds.length > 0 ? "observed-complete" : "unresolved",
				ids: addImageSlotIds,
				...sortedProvenance(itemOn2Provenance),
			},
		},
		soundRules: {
			shipVoices: {
				kind: "ship_voice_formula",
				coverageMode: shipVoiceCoverageMode,
				formula: shipVoiceFormula !== undefined && shipVoiceFormula.base > 0
					? shipVoiceFormula
					: undefined,
				requiredShipGraphFields: ["api_battle_n", "api_boko_d"],
				baseVoiceIds: shipVoiceBaseIds,
				beLeftVoiceIds: hasBeLeftVoice ? [29] : [],
				beLeftTiredVoiceIds: hasBeLeftTiredVoice ? [129] : [],
				timeSignalStartVoiceId: hasTimeSignalVoice ? 30 : undefined,
				timeSignalVoiceCount: hasTimeSignalVoice ? 24 : undefined,
				specialArtShipIds: uniqueSpecialArtShipIds,
				specialVoiceIds: resolvedSpecialVoiceIds,
				...sortedProvenance(shipVoiceProvenance),
			},
			kc9997: {
				kind: "sound_bucket",
				bucket: "9997",
				coverageMode: kc9997Ids.size > 0 ? (soundBucketDynamic["9997"] ? "partial" : "observed-complete") : (soundBucketDynamic["9997"] ? "partial" : "unresolved"),
				voiceIds: [...kc9997Ids].sort((left, right) => left - right),
				hasDynamicVoiceIds: soundBucketDynamic["9997"],
				...sortedProvenance(kc9997Provenance),
			},
			kc9998: {
				kind: "sound_bucket",
				bucket: "9998",
				coverageMode: kc9998Ids.size > 0 ? (soundBucketDynamic["9998"] ? "partial" : "observed-complete") : (soundBucketDynamic["9998"] ? "partial" : "unresolved"),
				voiceIds: [...kc9998Ids].sort((left, right) => left - right),
				hasDynamicVoiceIds: soundBucketDynamic["9998"],
				...sortedProvenance(kc9998Provenance),
			},
			kc9999: {
				kind: "sound_bucket",
				bucket: "9999",
				coverageMode: kc9999Ids.size > 0 ? (soundBucketDynamic["9999"] ? "partial" : "observed-complete") : (soundBucketDynamic["9999"] ? "partial" : "unresolved"),
				voiceIds: [...kc9999Ids].sort((left, right) => left - right),
				hasDynamicVoiceIds: soundBucketDynamic["9999"],
				...sortedProvenance(kc9999Provenance),
			},
		},
	};
}

function countModes(extracted: ExtractedCacheRules, mode: ResourceCoverageMode): number {
	const rules = [
		extracted.shipRules.special,
		extracted.shipRules.targetSemantics,
		extracted.slotRules.itemUp,
		extracted.slotRules.btxtFlat,
		extracted.slotRules.itemUp2,
		extracted.slotRules.itemOn2,
		extracted.soundRules.shipVoices,
		extracted.soundRules.kc9997,
		extracted.soundRules.kc9998,
		extracted.soundRules.kc9999,
	];
	return rules.filter(rule => rule.coverageMode === mode).length;
}

function defaultResourceManifest(): ResourceManifest {
	return {
		version: 2,
		generatedAt: new Date().toISOString(),
		pathRules: {
			shipDamageVariants: {},
			shipStandardCategories: [],
			shipFullCategories: [],
			slotStandardCategories: [],
			enemyPlaneIds: [],
			btxtFlatSlotIds: [],
			characterHoleIds: [],
			eventShipHoles: { full: [], fullDmg: [], up: [], upDmg: [] },
			enemyShipHoles: { full: [], fullDmg: [], up: [], upDmg: [] },
			specialShips: [],
			spRemodelShips: [],
			spRemodelMes: [],
			cardRounds: [],
			rewardShips: [],
		},
		summary: {
			totalEntries: 0,
			shipEntryCount: 0,
			slotitemEntryCount: 0,
			textureProviderEntryCount: 0,
			explicitPathEntryCount: 0,
			totalExplicitPaths: 0,
			modulesCovered: 0,
		},
		entries: [],
	};
}

function defaultResourceCategories(scriptVersion: string): ResourceCategoriesAsset {
	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			shipTargetTypeCount: 0,
			slotTargetTypeCount: 0,
			spRemodelSubcategoryCount: 0,
			shipGenerationGroupCount: 0,
			slotGenerationGroupCount: 0,
		},
		shipTargetTypes: [],
		slotTargetTypes: [],
		shipGenerationGroups: {
			defaultFriendly: [],
			defaultAbyssal: [],
			friendGraph: [],
			enemyGraph: [],
		},
		slotGenerationGroups: {
			default: [],
			baga: [],
			airunit: [],
		},
		spRemodelSubcategories: [],
	};
}

export function toCacheRulesAsset(
	scriptVersion: string,
	extracted: ExtractedCacheRules,
	options: {
		resourceManifest?: ResourceManifest;
		resourceCategories?: ResourceCategoriesAsset;
	} = {},
): CacheRulesAsset {
	const unresolvedRules: string[] = [];
	if (extracted.shipRules.special.coverageMode === "unresolved") {
		unresolvedRules.push("shipRules.special");
	}
	if (extracted.shipRules.targetSemantics.coverageMode === "unresolved") {
		unresolvedRules.push("shipRules.targetSemantics");
	}
	if (extracted.slotRules.itemUp.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.itemUp");
	}
	if (extracted.slotRules.btxtFlat.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.btxtFlat");
	}
	if (extracted.slotRules.itemUp2.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.itemUp2");
	}
	if (extracted.slotRules.itemOn2.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.itemOn2");
	}
	if (extracted.soundRules.shipVoices.coverageMode === "unresolved") {
		unresolvedRules.push("soundRules.shipVoices");
	}
	if (extracted.soundRules.kc9997.coverageMode === "unresolved") {
		unresolvedRules.push("soundRules.kc9997");
	}
	if (extracted.soundRules.kc9998.coverageMode === "unresolved") {
		unresolvedRules.push("soundRules.kc9998");
	}
	if (extracted.soundRules.kc9999.coverageMode === "unresolved") {
		unresolvedRules.push("soundRules.kc9999");
	}

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			shipRuleCount: 2,
			slotRuleCount: 4,
			soundRuleCount: 4,
			observedCompleteRuleCount: countModes(extracted, "observed-complete"),
			partialRuleCount: countModes(extracted, "partial"),
			unresolvedRuleCount: countModes(extracted, "unresolved"),
		},
		resourceManifest: options.resourceManifest ?? defaultResourceManifest(),
		resourceCategories: options.resourceCategories ?? defaultResourceCategories(scriptVersion),
		shipRules: extracted.shipRules,
		slotRules: extracted.slotRules,
		soundRules: extracted.soundRules,
		unresolvedRules,
	};
}
