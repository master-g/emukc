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

export function extractUiResources(moduleGraph: ModuleGraph): ExtractedUiResources {
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

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (
			!source.includes("resources/map/")
			&& !source.includes("resources/furniture/")
			&& !source.includes("resources/useitem/")
			&& !source.includes("resources/area/")
			&& !source.includes("worldselect/")
		) {
			continue;
		}

		for (const match of source.matchAll(/resources\/furniture\/([A-Za-z0-9_./-]+)/g)) {
			extracted.furnitureExplicitPaths.add(match[0]);
			const category = match[1]?.split("/")[0];
			if (category !== undefined && category !== "wd") {
				extracted.furnitureCategories.add(category);
			}
		}

		for (const match of source.matchAll(/resources\/useitem\/card\/([0-9]{3})\.png/g)) {
			extracted.useItemCardIds.add(match[1] ?? "");
		}
		for (const match of source.matchAll(/resources\/useitem\/card_\/([0-9]{3})\.png/g)) {
			extracted.useItemUnderlineIds.add(match[1] ?? "");
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

		for (const match of source.matchAll(/worldselect\/([A-Za-z0-9_.-]+)/g)) {
			extracted.worldSelectFiles.add(match[1] ?? "");
		}
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
