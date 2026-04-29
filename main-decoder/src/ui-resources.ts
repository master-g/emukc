import type { ModuleGraph, ResourceCoverageMode, UiResourcesAsset } from "./types.ts";

interface ExtractedUiResources {
	mapDefaultFiles: Set<string>;
	mapEventFiles: Set<string>;
	furnitureCategories: Set<string>;
	furnitureExplicitPaths: Set<string>;
	useItemCardIds: Set<string>;
	useItemUnderlineIds: Set<string>;
	areaSallyIds: Set<string>;
	areaAirunitIds: Set<string>;
	areaAirunitExtendConfirmIds: Set<string>;
	worldSelectFiles: Set<string>;
}

function coverageMode(values: Set<unknown>): ResourceCoverageMode {
	return values.size > 0 ? "partial" : "unresolved";
}

function zeroPad3(value: number): string {
	return value.toString().padStart(3, "0");
}

function parseIntegerLiteral(value: string | undefined): number | undefined {
	if (value === undefined) {
		return undefined;
	}
	const radix = /^0x/i.test(value) ? 16 : 10;
	const parsed = Number.parseInt(value, radix);
	return Number.isInteger(parsed) ? parsed : undefined;
}

function addUseItemId(cardIds: Set<string>, underlineIds: Set<string>, id: number | undefined, type: number | undefined): void {
	if (id === undefined || type === undefined) {
		return;
	}
	if (!Number.isInteger(id) || id <= 0) {
		return;
	}
	if (type === 1) {
		cardIds.add(zeroPad3(id));
	} else if (type === 2) {
		underlineIds.add(zeroPad3(id));
	}
}

function addFurnitureExplicitPath(extracted: ExtractedUiResources, rawPath: string): void {
	const normalized = rawPath.replace(/^\.\//, "").split("?")[0] ?? rawPath;
	const category = /^resources\/furniture\/([^/]+)\//.exec(normalized)?.[1];
	if (category !== undefined && category !== "wd") {
		extracted.furnitureCategories.add(category);
	}
	if (!/^resources\/furniture\/[^/]+\/.+\.[A-Za-z0-9]+$/.test(normalized)) {
		return;
	}
	extracted.furnitureExplicitPaths.add(normalized);
}

function addFurnitureCategoryMatch(target: Set<string>, source: string): void {
	const regexCategoryMatch = source.match(/resources\/furniture\/\(([^)]+)\)\/\.\+/);
	if (regexCategoryMatch !== null) {
		for (const value of regexCategoryMatch[1]?.split("|") ?? []) {
			const category = value.trim();
			if (category.length > 0 && category !== "wd") {
				target.add(category);
			}
		}
	}
}

const CONCAT_ACCESS = String.raw`(?:\s*\.\s*concat|\s*\[\s*["']concat["']\s*\])\s*\(`;
const NUMBER_LITERAL = String.raw`(?:0x[0-9a-fA-F]+|\d+)`;

function addMapFileMatch(
	defaultFiles: Set<string>,
	eventFiles: Set<string>,
	area: string,
	file: string,
): void {
	const normalized = `${area}/${file}`;
	if (/^00[1-7]$/.test(area)) {
		defaultFiles.add(normalized);
	} else if (/^\d{3}$/.test(area)) {
		eventFiles.add(normalized);
	}
}

function addWorldSelectRange(source: string, files: Set<string>): void {
	if (!source.includes("btn_chinjyufu") || !/for\s*\([^;]+;\s*[^;]+<\s*20\s*;/.test(source)) {
		return;
	}
	for (let id = 1; id <= 20; id += 1) {
		files.add(`btn_chinjyufu${id}.png`);
		if (source.includes("_off.png")) {
			files.add(`btn_chinjyufu${id}_off.png`);
		}
	}
}

export function extractUiResources(moduleGraph: ModuleGraph, supplementalSources: string[] = []): ExtractedUiResources {
	const extracted: ExtractedUiResources = {
		mapDefaultFiles: new Set(),
		mapEventFiles: new Set(),
		furnitureCategories: new Set(),
		furnitureExplicitPaths: new Set(),
		useItemCardIds: new Set(),
		useItemUnderlineIds: new Set(),
		areaSallyIds: new Set(),
		areaAirunitIds: new Set(),
		areaAirunitExtendConfirmIds: new Set(),
		worldSelectFiles: new Set(),
	};

	const sources = [
		...moduleGraph.modules.map(module => module.source),
		...supplementalSources,
	];

	const literalConcat = CONCAT_ACCESS;

	for (const source of sources) {
		if (
			!source.includes("resources/map/")
			&& !source.includes("resources/furniture/")
			&& !source.includes("getFurniture(")
			&& !source.includes("resources/useitem/")
			&& !source.includes("getUseitem(")
			&& !source.includes(".add(")
			&& !source.includes("resources/area/")
			&& !source.includes("worldselect/")
		) {
			continue;
		}

		for (const match of source.matchAll(/resources\/furniture\/([A-Za-z0-9_./-]+)/g)) {
			addFurnitureExplicitPath(extracted, match[0]);
		}
		addFurnitureCategoryMatch(extracted.furnitureCategories, source);
		for (const match of source.matchAll(/getFurniture\([^,]+,\s*["']([A-Za-z_]+)["']\)/g)) {
			const category = match[1] ?? "";
			if (category.length > 0) {
				extracted.furnitureCategories.add(category);
			}
		}

		const furnitureConcatRegex = new RegExp(
			String.raw`resources/furniture/["']\s*${literalConcat}\s*["']([^"']+)["']\s*,\s*["']/["']\s*\)` +
				String.raw`${literalConcat}\s*["']([^"']+)["']\s*,\s*["']/["']\s*\)` +
				String.raw`${literalConcat}\s*["']([^"']+)["']\s*\)`,
			"g",
		);
		for (const match of source.matchAll(furnitureConcatRegex)) {
			const category = match[1];
			const subdir = match[2];
			const file = match[3];
			if (category !== undefined && subdir !== undefined && file !== undefined) {
				addFurnitureExplicitPath(extracted, `resources/furniture/${category}/${subdir}/${file}`);
			}
		}

		for (const match of source.matchAll(/resources\/useitem\/card\/([0-9]{3})\.png/g)) {
			extracted.useItemCardIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/useitem\/card_\/([0-9]{3})\.png/g)) {
			extracted.useItemUnderlineIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/useitem\/card\/"\s*\+\s*"([0-9]{3})"\s*\+\s*"\.png"/g)) {
			extracted.useItemCardIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/useitem\/card_\/"\s*\+\s*"([0-9]{3})"\s*\+\s*"\.png"/g)) {
			extracted.useItemUnderlineIds.add(match[1] ?? "");
		}
		const useitemAddRegex = new RegExp(String.raw`\badd\(\s*(${NUMBER_LITERAL})\s*,\s*(${NUMBER_LITERAL})(?:\s*[),])`, "g");
		for (const match of source.matchAll(useitemAddRegex)) {
			addUseItemId(
				extracted.useItemCardIds,
				extracted.useItemUnderlineIds,
				parseIntegerLiteral(match[1]),
				parseIntegerLiteral(match[2]),
			);
		}
		const getUseitemRegex = new RegExp(String.raw`\bgetUseitem\(\s*(${NUMBER_LITERAL})\s*,\s*(${NUMBER_LITERAL})(?:\s*[),])`, "g");
		for (const match of source.matchAll(getUseitemRegex)) {
			addUseItemId(
				extracted.useItemCardIds,
				extracted.useItemUnderlineIds,
				parseIntegerLiteral(match[1]),
				parseIntegerLiteral(match[2]),
			);
		}

		for (const match of source.matchAll(/resources\/area\/sally\/([0-9_]+)\.png/g)) {
			extracted.areaSallyIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/area\/airunit\/([0-9_]+)\.png/g)) {
			extracted.areaAirunitIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/area\/airunit_extend_confirm\/([0-9_]+)\.png/g)) {
			extracted.areaAirunitExtendConfirmIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/area\/airunit_extend_confirm\/"\s*\.concat\(\s*"([0-9_]+)"\s*,\s*"([0-9_]+)\.png"\s*\)/g)) {
			const prefix = match[1] ?? "";
			const suffix = match[2] ?? "";
			if (prefix.length > 0) {
				extracted.areaAirunitExtendConfirmIds.add(`${prefix}${suffix}`);
			}
		}
		const areaConcatRegex = new RegExp(
			String.raw`resources/area/(sally|airunit|airunit_extend_confirm)/["']\s*${literalConcat}\s*["']([0-9_]+)["']\s*,\s*["']([0-9_]*\.png)["']\s*\)`,
			"g",
		);
		for (const match of source.matchAll(areaConcatRegex)) {
			const family = match[1];
			const id = `${match[2] ?? ""}${(match[3] ?? "").replace(/\.png$/, "")}`;
			if (id.length === 0) {
				continue;
			}
			if (family === "sally") {
				extracted.areaSallyIds.add(id);
			} else if (family === "airunit") {
				extracted.areaAirunitIds.add(id);
			} else if (family === "airunit_extend_confirm") {
				extracted.areaAirunitExtendConfirmIds.add(id);
			}
		}

		for (const match of source.matchAll(/resources\/map\/([0-9]{3}\/[0-9]{2}[_A-Za-z0-9.]*)/g)) {
			const file = match[1];
			if (file === undefined) {
				continue;
			}
			if (file.startsWith("0")) {
				extracted.mapDefaultFiles.add(file);
			} else {
				extracted.mapEventFiles.add(file);
			}
		}
		for (const match of source.matchAll(/resources\/map\/"\s*\.concat\(\s*"(\d{3})"\s*,\s*"\/"\s*\)\.concat\(\s*"([^"]+)"\s*\)/g)) {
			const area = match[1];
			const file = match[2];
			if (area !== undefined && file !== undefined) {
				addMapFileMatch(extracted.mapDefaultFiles, extracted.mapEventFiles, area, file);
			}
		}
		for (const match of source.matchAll(/resources\/map\/([0-9]{3})\/([0-9]{2}[_A-Za-z0-9.]+)/g)) {
			const area = match[1];
			const file = match[2];
			if (area !== undefined && file !== undefined) {
				addMapFileMatch(extracted.mapDefaultFiles, extracted.mapEventFiles, area, file);
			}
		}
		const mapConcatRegex = new RegExp(
			String.raw`resources/map/["']\s*${literalConcat}\s*["'](\d{3})["']\s*,\s*["']/["']\s*\)${literalConcat}\s*["']([^"']+)["']\s*\)`,
			"g",
		);
		for (const match of source.matchAll(mapConcatRegex)) {
			const area = match[1];
			const file = match[2];
			if (area !== undefined && file !== undefined) {
				addMapFileMatch(extracted.mapDefaultFiles, extracted.mapEventFiles, area, file);
			}
		}

		for (const match of source.matchAll(/worldselect\/([A-Za-z0-9_.-]+)/g)) {
			const file = match[1] ?? "";
			if (/\.[A-Za-z0-9]+$/.test(file)) {
				extracted.worldSelectFiles.add(file);
			}
		}
		addWorldSelectRange(source, extracted.worldSelectFiles);
	}

	return extracted;
}

export function toUiResourcesAsset(
	scriptVersion: string,
	extracted: ExtractedUiResources,
): UiResourcesAsset {
	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			mapDefaultFileCount: extracted.mapDefaultFiles.size,
			mapEventFileCount: extracted.mapEventFiles.size,
			furnitureCategoryCount: extracted.furnitureCategories.size,
			useItemCardIdCount: extracted.useItemCardIds.size,
			useItemUnderlineIdCount: extracted.useItemUnderlineIds.size,
			areaSallyIdCount: extracted.areaSallyIds.size,
			areaAirunitIdCount: extracted.areaAirunitIds.size,
			worldSelectFileCount: extracted.worldSelectFiles.size,
		},
		map: {
			defaultFiles: {
				coverageMode: coverageMode(extracted.mapDefaultFiles),
				files: [...extracted.mapDefaultFiles].sort(),
			},
			eventFiles: {
				coverageMode: coverageMode(extracted.mapEventFiles),
				files: [...extracted.mapEventFiles].sort(),
			},
		},
		furniture: {
			categories: [...extracted.furnitureCategories].sort(),
			explicitPaths: [...extracted.furnitureExplicitPaths].sort(),
		},
		useItem: {
			cardIds: {
				coverageMode: coverageMode(extracted.useItemCardIds),
				ids: [...extracted.useItemCardIds].sort(),
			},
			underlineIds: {
				coverageMode: coverageMode(extracted.useItemUnderlineIds),
				ids: [...extracted.useItemUnderlineIds].sort(),
			},
		},
		area: {
			sallyIds: {
				coverageMode: coverageMode(extracted.areaSallyIds),
				ids: [...extracted.areaSallyIds].sort(),
			},
			airunitIds: {
				coverageMode: coverageMode(extracted.areaAirunitIds),
				ids: [...extracted.areaAirunitIds].sort(),
			},
			airunitExtendConfirmIds: {
				coverageMode: coverageMode(extracted.areaAirunitExtendConfirmIds),
				ids: [...extracted.areaAirunitExtendConfirmIds].sort(),
			},
		},
		worldSelect: {
			files: [...extracted.worldSelectFiles].sort(),
		},
	};
}
