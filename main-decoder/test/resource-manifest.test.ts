import { describe, expect, test } from "bun:test";

import { extractResourceManifest } from "../src/resource-manifest.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function wrapModule(body: string): string {
	return `function(module, exports) { ${body} }`;
}

function makeModule(overrides: Partial<ModuleArtifact> & { id: string; source: string }): ModuleArtifact {
	return {
		displayId: overrides.id,
		fileName: `${overrides.id}.js`,
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
		readableName: overrides.id,
		...overrides,
	};
}

function makeGraph(modules: ModuleArtifact[]): ModuleGraph {
	return {
		modules,
		summary: {} as ModuleGraphSummary,
	};
}

describe("extractResourceManifest — ship resources", () => {
	test("extracts resources.getShip call", () => {
		const source = wrapModule(`resources.getShip(vo.ship.api_id, false, "full")`);
		const graph = makeGraph([makeModule({ id: "m1", source })]);
		const manifest = extractResourceManifest(graph);

		expect(manifest.version).toBe(2);
		expect(manifest.pathRules.shipStandardCategories).toContain("banner");
		expect(manifest.entries).toHaveLength(1);
		expect(manifest.entries[0]!.kind).toBe("ship");
		const entry = manifest.entries.find((e): e is Extract<typeof e, { kind: "ship" }> => e.kind === "ship");
		expect(entry).toBeDefined();
		if (entry === undefined) {
			return;
		}
		expect(entry.source).toBe("resources.getShip");
		expect(entry.targetType).toBe("full");
		expect(entry.shipMstIdSource).toBe("vo.ship.api_id");
		expect(entry.damagedSource).toBe("false");
		expect(entry.moduleIds).toEqual(["m1"]);
	});

	test("extracts ShipLoader.add call", () => {
		const source = wrapModule(`var loader = new ShipLoader(); loader.add(shipId, true, "banner")`);
		const graph = makeGraph([makeModule({ id: "m2", source })]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "ship");
		expect(entry).toBeDefined();
		if (entry?.kind !== "ship") {
			return;
		}
		expect(entry.source).toBe("ShipLoader.add");
		expect(entry.targetType).toBe("banner");
		expect(entry.damagedSource).toBe("true");
	});

	test("skips getShip with spread arguments", () => {
		const source = wrapModule(`resources.getShip(...args)`);
		const graph = makeGraph([makeModule({ id: "m3", source })]);
		const manifest = extractResourceManifest(graph);

		expect(manifest.entries).toHaveLength(0);
	});
});

describe("extractResourceManifest — slotitem resources", () => {
	test("extracts resources.getSlotitem call", () => {
		const source = wrapModule(`resources.getSlotitem(eq.api_id, "card")`);
		const graph = makeGraph([makeModule({ id: "s1", source })]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "slotitem");
		expect(entry).toBeDefined();
		if (entry?.kind !== "slotitem") {
			return;
		}
		expect(entry.source).toBe("resources.getSlotitem");
		expect(entry.targetType).toBe("card");
		expect(entry.slotMstIdSources).toEqual(["eq.api_id"]);
	});

	test("extracts SlotLoader.add call", () => {
		const source = wrapModule(`var sl = new SlotLoader(); sl.add(itemId, "item_on")`);
		const graph = makeGraph([makeModule({ id: "s2", source })]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "slotitem");
		expect(entry).toBeDefined();
		if (entry?.kind !== "slotitem") {
			return;
		}
		expect(entry.source).toBe("SlotLoader.add");
		expect(entry.targetType).toBe("item_on");
	});
});

describe("extractResourceManifest — texture provider", () => {
	test("extracts getTexture call with numeric IDs", () => {
		const source = wrapModule(`COMMON_MISC.getTexture(1, 2, 5)`);
		const graph = makeGraph([makeModule({ id: "t1", source })]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "texture-provider");
		expect(entry).toBeDefined();
		if (entry?.kind !== "texture-provider") {
			return;
		}
		expect(entry.provider).toBe("COMMON_MISC");
		expect(entry.textureIds).toEqual([1, 2, 5]);
	});

	test("merges getTexture calls with same provider", () => {
		const source = wrapModule(`
			FOO.getTexture(1);
			FOO.getTexture(2, 3);
		`);
		const graph = makeGraph([makeModule({ id: "t2", source })]);
		const manifest = extractResourceManifest(graph);

		const entries = manifest.entries.filter(e => e.kind === "texture-provider");
		expect(entries).toHaveLength(1);
		if (entries[0]?.kind !== "texture-provider") {
			return;
		}
		expect(entries[0].textureIds).toEqual([1, 2, 3]);
	});
});

describe("extractResourceManifest — explicit paths", () => {
	test("extracts resources/ paths from source", () => {
		const source = wrapModule(`var url = "resources/battle/banner/001_abc.png"`);
		const graph = makeGraph([makeModule({ id: "p1", source })]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "explicit-path");
		expect(entry).toBeDefined();
		if (entry?.kind !== "explicit-path") {
			return;
		}
		expect(entry.paths).toContain("resources/battle/banner/001_abc.png");
	});

	test("deduplicates same path across modules", () => {
		const source1 = wrapModule(`"resources/ui/bg.png"`);
		const source2 = wrapModule(`"resources/ui/bg.png"`);
		const graph = makeGraph([
			makeModule({ id: "p2a", source: source1 }),
			makeModule({ id: "p2b", source: source2 }),
		]);
		const manifest = extractResourceManifest(graph);

		const entry = manifest.entries.find(e => e.kind === "explicit-path");
		expect(entry).toBeDefined();
		if (entry?.kind !== "explicit-path") {
			return;
		}
		expect(entry.paths).toEqual(["resources/ui/bg.png"]);
	});
});

describe("extractResourceManifest — deduplication", () => {
	test("merges same ship pattern from multiple modules", () => {
		const source1 = wrapModule(`resources.getShip(id, false, "full")`);
		const source2 = wrapModule(`resources.getShip(id, false, "full")`);
		const graph = makeGraph([
			makeModule({ id: "d1", source: source1 }),
			makeModule({ id: "d2", source: source2 }),
		]);
		const manifest = extractResourceManifest(graph);

		const shipEntries = manifest.entries.filter(e => e.kind === "ship");
		expect(shipEntries).toHaveLength(1);
		if (shipEntries[0]?.kind !== "ship") {
			return;
		}
		expect(shipEntries[0].moduleIds).toEqual(["d1", "d2"]);
	});
});

describe("extractResourceManifest — empty modules", () => {
	test("returns empty manifest for modules with no resources", () => {
		const source = wrapModule(`var x = 42; console.log("hello")`);
		const graph = makeGraph([makeModule({ id: "empty", source })]);
		const manifest = extractResourceManifest(graph);

		expect(manifest.entries).toHaveLength(0);
		expect(manifest.pathRules.btxtFlatSlotIds.length).toBeGreaterThan(300);
		expect(manifest.summary.totalEntries).toBe(0);
		expect(manifest.summary.modulesCovered).toBe(0);
	});
});
