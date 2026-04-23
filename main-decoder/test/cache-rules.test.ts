import { describe, expect, test } from "bun:test";

import { extractCacheRules, toCacheRulesAsset } from "../src/cache-rules.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function wrapModule(body: string): string {
	return `function(module, exports, require) { ${body} }`;
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

describe("extractCacheRules", () => {
	test("captures special ship cases from decoded conditional branches", () => {
		const graph = makeGraph([
			makeModule({
				id: "special-module",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					var damaged = attacker.isDamaged();
					(571 != shipMstId && 576 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.shipRules.special.coverageMode).toBe("observed-complete");
		expect(extracted.shipRules.special.cases).toEqual([
			{ damaged: false, shipIds: [571, 576] },
		]);
	});

	test("captures item_up normalization and btxt_flat non-enemy rule", () => {
		const graph = makeGraph([
			makeModule({
				id: "slot-loader",
				readableName: "SlotLoader",
				source: wrapModule(`
					exports.ITEMUP_REPLACE = { 1519: 1516, 1520: 1517 };
					function SlotLoader() {
						this.EXCLUDE_RES = [{ type: "item_character", mst_id: 42 }, { type: "item_up", mst_id: 496 }];
					}
					SlotLoader.prototype.add = function(mstId, type) {
						"item_up" == type && (1 == exports.ITEMUP_REPLACE.hasOwnProperty(mstId.toString())
							? mstId = exports.ITEMUP_REPLACE[mstId]
							: mstId > slotConstModule.SlotConst.ENEMY_SLOT_BORDER && (mstId -= slotConstModule.SlotConst.ENEMY_SLOT_BORDER));
						return mstId;
					};
				`),
			}),
			makeModule({
				id: "slot-const",
				readableName: "SlotConst",
				source: wrapModule(`
					SlotConst.ENEMY_SLOT_BORDER = 1500;
				`),
			}),
			makeModule({
				id: "btxt-module",
				source: wrapModule(`
					if (1 == this._night && 0 == slotUtilModule.SlotUtil.isEnemyItem(this._slot1.mstID)) {
						slotLoader.add(this._slot1.mstID, "btxt_flat");
					}
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.slotRules.itemUp.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.itemUp.enemySlotBorder).toBe(1500);
		expect(extracted.slotRules.itemUp.replaceMap).toEqual({
			"1519": 1516,
			"1520": 1517,
		});
		expect(extracted.slotRules.itemUp.exclude).toEqual([
			{ mstId: 496, type: "item_up" },
		]);
		expect(extracted.slotRules.btxtFlat.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.btxtFlat.excludeEnemyItems).toBe(true);
	});

	test("converts extracted rules into a unified cache-rules asset", () => {
		const graph = makeGraph([
			makeModule({
				id: "special-module",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					var damaged = attacker.isDamaged();
					(541 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
			makeModule({
				id: "slot-const",
				readableName: "SlotConst",
				source: wrapModule(`SlotConst.ENEMY_SLOT_BORDER = 1500;`),
			}),
			makeModule({
				id: "slot-loader",
				readableName: "SlotLoader",
				source: wrapModule(`
					exports.ITEMUP_REPLACE = { 1519: 1516 };
					function SlotLoader() {
						this.EXCLUDE_RES = [{ type: "item_up", mst_id: 496 }];
					}
				`),
			}),
		]);

		const asset = toCacheRulesAsset("6.2.8.0", extractCacheRules(graph));
		expect(asset.version).toBe(1);
		expect(asset.shipRules.special.cases).toEqual([{ damaged: false, shipIds: [541] }]);
		expect(asset.slotRules.itemUp.enemySlotBorder).toBe(1500);
		expect(asset.unresolvedRules).toContain("slotRules.btxtFlat");
	});
});
