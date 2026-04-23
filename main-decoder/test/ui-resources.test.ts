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
