import { describe, expect, test } from "bun:test";

import { extractUiResources, toUiResourcesAsset } from "../src/ui-resources.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function makeModule(id: string, source: string): ModuleArtifact {
	return {
		id,
		displayId: id,
		fileName: `${id}.js`,
		moduleKind: "game",
		cleanupTier: "none",
		exportNames: [],
		hasDefaultExport: false,
		canonicalParameterNames: [],
		rawObfuscatedIdentifierCount: 0,
		transformedObfuscatedIdentifierCount: 0,
		obfuscatedIdentifierDelta: 0,
		shellMetrics: { namespaceShellCount: 0, normalizedNamespaceShellCount: 0, classShellCount: 0, normalizedClassShellCount: 0, structuralTransformCount: 0 },
		lineCount: 1,
		dependencies: [],
		readableName: id,
		source: `function(module, exports) { ${source} }`,
	};
}

function makeGraph(modules: ModuleArtifact[]): ModuleGraph {
	return { modules, summary: {} as ModuleGraphSummary };
}

describe("extractUiResources", () => {
	test("extracts furniture categories, useitem ids, and area ids", () => {
		const graph = makeGraph([
			makeModule("u1", `
				var a = "resources/furniture/normal/001_0001.png";
				var b = "resources/useitem/card/090.png";
				var c = "resources/useitem/card_/091.png";
				var d = "resources/area/sally/057_2.png";
				var e = "resources/area/airunit/006.png";
			`),
		]);

		const extracted = extractUiResources(graph);
		expect(extracted.furnitureCategories.has("normal")).toBe(true);
		expect(extracted.useItemCardIds.has("090")).toBe(true);
		expect(extracted.useItemUnderlineIds.has("091")).toBe(true);
		expect(extracted.areaSallyIds.has("057_2")).toBe(true);
		expect(extracted.areaAirunitIds.has("006")).toBe(true);
	});

	test("captures concatenated map, useitem, area, and worldselect paths from decoded modules", () => {
		const graph = makeGraph([
			makeModule("u-dynamic", `
				var mapPath = settings.path_root + "resources/map/" ["concat"]("057", "/").concat("03_info.json");
				var useitem = settings.path_root + "resources/useitem/card/" + "090" + ".png";
				var underline = settings.path_root + "resources/useitem/card_/" + "091" + ".png";
				var area = settings.path_root + "resources/area/airunit_extend_confirm/" ["concat"]("006", "_.png");
				var world = "kcs2/resources/worldselect/btn_chinjyufu_on.png";
			`),
		]);

		const asset = toUiResourcesAsset("6.2.8.0", extractUiResources(graph));
		expect(asset.map.eventFiles.files).toContain("057/03_info.json");
		expect(asset.useItem.cardIds.ids).toContain("090");
		expect(asset.useItem.underlineIds.ids).toContain("091");
		expect(asset.area.airunitExtendConfirmIds.ids).toContain("006_");
		expect(asset.worldSelect.files).toContain("btn_chinjyufu_on.png");
	});

	test("extracts useitem ids from real loader callers and worldselect ranges from supplemental world.js", () => {
		const graph = makeGraph([
			makeModule("u-useitem", `
				var loader = new UseitemLoader();
				loader.add(73, 1);
				loader.add(75, 2);
				var texture = resources.getUseitem(90, 1);
				var hexCard = resources.getUseitem(0x2a, 0x1);
				var hexUnderline = resources.getUseitem(0x5b, 0x2);
			`),
		]);
		const worldSource = `
			this.IMG_BG = "resources/worldselect/bg.jpg";
			r.add("world_on", "resources/worldselect/btn_chinjyufu_on.png");
			r.add("gauge_bg", "resources/worldselect/gauge20_gray.png");
			for (var o = 0; o < 20; o++) {
				r.add("world" + (o + 1), "resources/worldselect/btn_chinjyufu" + (o + 1) + "_off.png");
			}
		`;

		const asset = toUiResourcesAsset("6.2.8.0", extractUiResources(graph, [worldSource]));
		expect(asset.useItem.cardIds.ids).toEqual(expect.arrayContaining(["042", "073", "090"]));
		expect(asset.useItem.underlineIds.ids).toEqual(expect.arrayContaining(["075", "091"]));
		expect(asset.worldSelect.files).toEqual(expect.arrayContaining([
			"bg.jpg",
			"btn_chinjyufu1.png",
			"btn_chinjyufu1_off.png",
			"btn_chinjyufu20.png",
			"btn_chinjyufu20_off.png",
			"btn_chinjyufu_on.png",
			"gauge20_gray.png",
		]));
	});

	test("captures furniture regex categories from loader modules", () => {
		const graph = makeGraph([
			makeModule("u-furniture", `
				var pattern = new RegExp("^resources/furniture/(movable|normal|thumbnail|picture|outside|card)/.+");
				var scriptPath = settings.path_root + "resources/furniture/" ["concat"]("movable", "/").concat("wd", "/").concat("001_1234.json");
				var dynamicDirectory = settings.path_root + "resources/furniture/outside/" + fileName;
				var reward = resources.getFurniture(mst_id, "reward");
			`),
		]);

		const extracted = extractUiResources(graph);
		expect(extracted.furnitureCategories.has("movable")).toBe(true);
		expect(extracted.furnitureCategories.has("normal")).toBe(true);
		expect(extracted.furnitureCategories.has("thumbnail")).toBe(true);
		expect(extracted.furnitureCategories.has("picture")).toBe(true);
		expect(extracted.furnitureCategories.has("outside")).toBe(true);
		expect(extracted.furnitureCategories.has("card")).toBe(true);
		expect(extracted.furnitureCategories.has("reward")).toBe(true);
		expect([...extracted.furnitureExplicitPaths]).toEqual(expect.arrayContaining(["resources/furniture/movable/wd/001_1234.json"]));
		expect([...extracted.furnitureExplicitPaths]).not.toContain("resources/furniture/outside/");
	});

	test("converts extracted UI resources into an asset shape", () => {
		const graph = makeGraph([
			makeModule("u2", `
				var path = "resources/furniture/outside/window_bg_1-1.png";
				var area = "resources/area/airunit_extend_confirm/006_.png";
			`),
		]);

		const asset = toUiResourcesAsset("6.2.8.0", extractUiResources(graph));
		expect(asset.version).toBe(1);
		expect(asset.furniture.categories).toContain("outside");
		expect(asset.area.airunitExtendConfirmIds.ids).toContain("006_");
	});
});
