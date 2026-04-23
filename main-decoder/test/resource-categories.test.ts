import { describe, expect, test } from "bun:test";

import { extractResourceCategories, toResourceCategoriesAsset } from "../src/resource-categories.ts";
import { runDecodePipeline } from "../src/pipeline.ts";
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
		shellMetrics: {
			namespaceShellCount: 0,
			normalizedNamespaceShellCount: 0,
			classShellCount: 0,
			normalizedClassShellCount: 0,
			structuralTransformCount: 0,
		},
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

describe("extractResourceCategories", () => {
	test("extracts ship and slot categories from API calls and loader aliases", () => {
		const graph = makeGraph([
			makeModule({
				id: "m1",
				source: wrapModule(`
					resources.getShip(shipId, false, "full");
					resources.getSlotitem(slotId, "item_up");
					var shipLoader = new ShipLoader();
					shipLoader.add(shipId, false, "banner");
					var slotLoader = new SlotLoader();
					slotLoader.add(slotId, "card");
				`),
			}),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipTargetTypes.map(entry => [entry.source, entry.targetType])).toEqual([
			["ShipLoader.add", "banner"],
			["resources.getShip", "full"],
		]);
		expect(extracted.slotTargetTypes.map(entry => [entry.source, entry.targetType])).toEqual([
			["SlotLoader.add", "card"],
			["resources.getSlotitem", "item_up"],
		]);
	});

	test("extracts explicit-path categories and sp_remodel subcategories", () => {
		const graph = makeGraph([
			makeModule({
				id: "m2",
				source: wrapModule(`
					var p1 = "resources/ship/sp_remodel/animation_key/0502_remodel.json";
					var p2 = "resources/ship/special/0640_1234.png";
					var p3 = "resources/slot/airunit_banner/0500_9999.png";
				`),
			}),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipTargetTypes.map(entry => entry.targetType)).toEqual(["sp_remodel/animation_key", "special"]);
		expect(extracted.slotTargetTypes.map(entry => entry.targetType)).toEqual(["airunit_banner"]);
		expect(extracted.spRemodelSubcategories).toEqual(["animation_key"]);
	});

	test("deduplicates provenance across modules", () => {
		const graph = makeGraph([
			makeModule({ id: "m3a", source: wrapModule(`resources.getShip(id, false, "full")`) }),
			makeModule({ id: "m3b", source: wrapModule(`resources.getShip(id, false, "full")`) }),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipTargetTypes).toHaveLength(1);
		expect(extracted.shipTargetTypes[0]?.moduleIds).toEqual(["m3a", "m3b"]);
		expect(extracted.shipTargetTypes[0]?.moduleNames).toEqual(["m3a", "m3b"]);
	});

	test("builds Rust-facing generation groups from discovered categories", () => {
		const graph = makeGraph([
			makeModule({
				id: "m4",
				source: wrapModule(`
					resources.getShip(shipId, false, "album_status");
					resources.getShip(shipId, false, "banner");
					resources.getShip(shipId, false, "full");
					resources.getShip(shipId, false, "full_dmg");
					resources.getSlotitem(slotId, "card");
					resources.getSlotitem(slotId, "item_up");
					resources.getSlotitem(slotId, "airunit_banner");
				`),
			}),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipGenerationGroups.defaultFriendly).toEqual(["album_status", "banner"]);
		expect(extracted.shipGenerationGroups.defaultAbyssal).toEqual(["banner"]);
		expect(extracted.shipGenerationGroups.friendGraph).toEqual(["full", "full_dmg"]);
		expect(extracted.slotGenerationGroups.default).toEqual(["card", "item_up"]);
		expect(extracted.slotGenerationGroups.airunit).toEqual(["airunit_banner"]);
	});

	test("normalizes ship damaged variants to final resource categories", () => {
		const graph = makeGraph([
			makeModule({
				id: "m4b",
				source: wrapModule(`
					resources.getShip(shipId, damaged, "full");
					resources.getShip(shipId, false, "banner_g");
				`),
			}),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipTargetTypes.map(entry => entry.targetType)).toEqual(["banner_g_dmg", "full", "full_dmg"]);
	});

	test("resolves target types through string literal bindings", () => {
		const graph = makeGraph([
			makeModule({
				id: "m4c",
				source: wrapModule(`
					var shipTarget = "power_up";
					var slotTarget = "card_t";
					resources.getShip(shipId, damaged, shipTarget);
					resources.getSlotitem(slotId, slotTarget);
				`),
			}),
		]);

		const extracted = extractResourceCategories(graph);
		expect(extracted.shipTargetTypes.some(entry => entry.targetType === "power_up")).toBe(true);
		expect(extracted.slotTargetTypes.some(entry => entry.targetType === "card_t")).toBe(true);
	});

	test("converts extracted data into a synced asset shape", () => {
		const graph = makeGraph([
			makeModule({ id: "m5", source: wrapModule(`resources.getShip(shipId, false, "special")`) }),
		]);

		const asset = toResourceCategoriesAsset("6.2.8.0", extractResourceCategories(graph));
		expect(asset.version).toBe(1);
		expect(asset.scriptVersion).toBe("6.2.8.0");
		expect(asset.shipTargetTypes[0]?.targetType).toBe("special");
		expect(asset.summary.shipTargetTypeCount).toBe(1);
	});
});

test("extracts resource categories from the current decoded main.js", async () => {
	const result = await runDecodePipeline({ writeOutputs: false });
	const asset = result.resourceCategories;

	expect(asset.summary.shipTargetTypeCount).toBeGreaterThan(10);
	expect(asset.summary.slotTargetTypeCount).toBeGreaterThan(8);
	expect(asset.shipTargetTypes.some(entry => entry.targetType === "full")).toBe(true);
	expect(asset.shipTargetTypes.some(entry => entry.targetType === "banner")).toBe(true);
	expect(asset.shipTargetTypes.some(entry => entry.targetType === "special")).toBe(true);
	expect(asset.shipTargetTypes.some(entry => entry.targetType === "sp_remodel/full_x2")).toBe(true);
	expect(asset.slotTargetTypes.some(entry => entry.targetType === "btxt_flat")).toBe(true);
	expect(asset.slotTargetTypes.some(entry => entry.targetType === "airunit_banner")).toBe(true);
	expect(asset.spRemodelSubcategories).toEqual(
		expect.arrayContaining(["animation_key", "full_x2", "silhouette", "text_class", "text_name", "text_remodel_mes"]),
	);
	expect(asset.shipGenerationGroups.defaultFriendly).toEqual(expect.arrayContaining(["album_status", "banner", "remodel"]));
	expect(asset.shipGenerationGroups.friendGraph).toEqual(expect.arrayContaining(["full", "full_dmg"]));
	expect(asset.slotGenerationGroups.default).toEqual(expect.arrayContaining(["card", "item_up", "remodel"]));
});
