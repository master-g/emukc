import { describe, expect, test } from "bun:test";

import { extractResourceIdSets, toResourceIdSetsAsset } from "../src/resource-id-sets.ts";
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

describe("extractResourceIdSets", () => {
	test("captures ship ids from special-attack branch conditions", () => {
		const graph = makeGraph([
			makeModule({
				id: "m1",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					if (573 == shipMstId) {
						resources.getShip(shipMstId, false, "special");
					}
				`),
			}),
		]);

		const extracted = extractResourceIdSets(graph);
		expect(extracted.shipIdSets.specialShips.ids.has(573)).toBe(true);
		expect(extracted.shipIdSets.specialShips.coverageMode).toBe("partial");
	});

	test("captures ship ids from ternary special-case branches", () => {
		const graph = makeGraph([
			makeModule({
				id: "m-ternary",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					var damaged = attacker.isDamaged();
					(571 != shipMstId && 576 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
		]);

		const extracted = extractResourceIdSets(graph);
		expect(extracted.shipIdSets.specialShips.ids.has(571)).toBe(true);
		expect(extracted.shipIdSets.specialShips.ids.has(576)).toBe(true);
		expect(extracted.shipIdSets.specialShips.coverageMode).toBe("partial");
	});

	test("captures direct numeric ship ids for reward-style categories", () => {
		const graph = makeGraph([
			makeModule({
				id: "m2",
				source: wrapModule(`
					var shipId = 900;
					resources.getShip(shipId, false, "reward_card");
				`),
			}),
		]);

		const extracted = extractResourceIdSets(graph);
		expect(extracted.shipIdSets.rewardShips.ids.has(900)).toBe(true);
	});

	test("leaves btxt_flat unresolved when only runtime slot ids appear", () => {
		const graph = makeGraph([
			makeModule({
				id: "m3",
				source: wrapModule(`resources.getSlotitem(slotId, "btxt_flat")`),
			}),
		]);

		const extracted = extractResourceIdSets(graph);
		expect(extracted.slotitemIdSets.btxtFlatIds.ids.size).toBe(0);
		expect(extracted.slotitemIdSets.btxtFlatIds.coverageMode).toBe("unresolved");
	});

	test("converts extracted sets into an asset shape", () => {
		const graph = makeGraph([
			makeModule({
				id: "m4",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					if (541 == shipMstId) {
						resources.getShip(shipMstId, false, "special");
					}
				`),
			}),
		]);

		const asset = toResourceIdSetsAsset("6.2.8.0", extractResourceIdSets(graph));
		expect(asset.version).toBe(1);
		expect(asset.coverageMode).toBe("mainjs-observed");
		expect(asset.shipIdSets.specialShips.ids).toEqual([541]);
		expect(asset.unresolvedKeys).toContain("btxtFlatIds");
	});
});
