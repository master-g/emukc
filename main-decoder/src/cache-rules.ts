import type { ModuleArtifact, ModuleGraph, ResourceCoverageMode, CacheRulesAsset } from "./types.ts";
import type { ResourceManifest } from "./resource-manifest.ts";
import type { ResourceCategoriesAsset } from "./types.ts";

interface ExtractedCacheRules {
	shipRules: CacheRulesAsset["shipRules"];
	slotRules: CacheRulesAsset["slotRules"];
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
	const itemUpProvenance = emptyProvenance();
	const btxtFlatProvenance = emptyProvenance();

	let itemUpReplaceMap: Record<string, number> = {};
	let enemySlotBorder: number | undefined;
	let excludes: Array<{ type: string; mstId: number }> = [];
	let hasItemUpNormalization = false;
	let hasBtxtFlatNonEnemyRule = false;

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (!source.includes("special") && !source.includes("ITEMUP_REPLACE") && !source.includes("ENEMY_SLOT_BORDER") && !source.includes("btxt_flat")) {
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

	return {
		shipRules: {
			special: {
				kind: "special_cases",
				coverageMode: coverageModeFromCount(specialCaseList.length, specialCaseList.length > 0),
				cases: specialCaseList,
				...sortedProvenance(specialProvenance),
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
		},
	};
}

function countModes(extracted: ExtractedCacheRules, mode: ResourceCoverageMode): number {
	const rules = [
		extracted.shipRules.special,
		extracted.slotRules.itemUp,
		extracted.slotRules.btxtFlat,
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
	if (extracted.slotRules.itemUp.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.itemUp");
	}
	if (extracted.slotRules.btxtFlat.coverageMode === "unresolved") {
		unresolvedRules.push("slotRules.btxtFlat");
	}

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			shipRuleCount: 1,
			slotRuleCount: 2,
			observedCompleteRuleCount: countModes(extracted, "observed-complete"),
			partialRuleCount: countModes(extracted, "partial"),
			unresolvedRuleCount: countModes(extracted, "unresolved"),
		},
		resourceManifest: options.resourceManifest ?? defaultResourceManifest(),
		resourceCategories: options.resourceCategories ?? defaultResourceCategories(scriptVersion),
		shipRules: extracted.shipRules,
		slotRules: extracted.slotRules,
		unresolvedRules,
	};
}
