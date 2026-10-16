import { describe, expect, test } from "bun:test";

import { extractRemodelTables } from "../src/remodel-tables.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function makeModule(id: string, exportNames: string[], source: string): ModuleArtifact {
	return {
		id,
		displayId: id,
		fileName: `${id}.js`,
		moduleKind: "game",
		cleanupTier: "none",
		exportNames,
		hasDefaultExport: false,
		canonicalParameterNames: [],
		rawObfuscatedIdentifierCount: 0,
		transformedObfuscatedIdentifierCount: 0,
		obfuscatedIdentifierDelta: 0,
		shellMetrics: { namespaceShellCount: 0, normalizedNamespaceShellCount: 0, classShellCount: 0, normalizedClassShellCount: 0, structuralTransformCount: 0 },
		lineCount: 1,
		dependencies: [],
		readableName: id,
		source: `function(module, exports, require) { ${source} }`,
	};
}

const HOLDER = `
	class ShipUpgradeModelHolder {
		constructor() {
			this._USE_DEVKIT_GROUP_ = [503, 504];
		}
		_getRequiredDevkitNum(_0xa, _0xb, _0xc) {
			switch (_0xa) {
				case 214:
				case 242:
					return 15;
				case 88:
					return 181;
				default:
					return 0 != _0xb && -1 == this._USE_DEVKIT_GROUP_.indexOf(_0xa) || _0xc < 4500 ? 0 : _0xc < 5500 ? 10 : _0xc < 6500 ? 15 : 20;
			}
		}
		_getRequiredBuildKitNum(_0xa) {
			switch (_0xa) {
				case 214:
					return 5;
				case 503:
				case 504:
					return 20;
				default:
					return 0;
			}
		}
	}
	exports.ShipUpgradeModelHolder = ShipUpgradeModelHolder;
`;

const MODEL = `
	class ShipUpgradeModel {
		get newhokohesosizai() {
			switch (this.mst_id_after) {
				case 546:
					return 3;
				case 591:
				case 592:
					return 2;
				case 911:
					return 136 == this.mst_id_before ? 3 : 0;
				default:
					return 0;
			}
		}
		get revkit() {
			return 975 === this.mst_id_after ? 5 : 0;
		}
		get arsenalmaterial() {
			switch (this.mst_id_after) {
				case 1040:
					return 5;
				case 902:
					return 2;
				default:
					return 0;
			}
		}
	}
	exports.ShipUpgradeModel = ShipUpgradeModel;
`;

function graph(holder = HOLDER, model = MODEL): ModuleGraph {
	return {
		modules: [
			makeModule("1", ["ShipUpgradeModel"], model),
			makeModule("2", ["ShipUpgradeModelHolder", "ShipUpgradeModelHolderEdit"], holder),
		],
		summary: {} as ModuleGraphSummary,
	};
}

describe("extractRemodelTables", () => {
	test("reads the client's hard-coded remodel tables", () => {
		const tables = extractRemodelTables(graph());
		expect(tables.devkit).toEqual({
			byShip: { "88": 181, "214": 15, "242": 15 },
			blueprintGroup: [503, 504],
			steelTiers: [
				{ below: 4500, count: 0 },
				{ below: 5500, count: 10 },
				{ below: 6500, count: 15 },
			],
			steelTop: 20,
		});
		expect(tables.buildkit).toEqual({ byShip: { "214": 5, "503": 20, "504": 20 } });
		expect(tables.artillery).toEqual([
			{ after: 546, count: 3 },
			{ after: 591, count: 2 },
			{ after: 592, count: 2 },
			{ after: 911, before: 136, count: 3 },
		]);
		expect(tables.screw).toEqual([{ after: 975, count: 5 }]);
		expect(tables.arsenal).toEqual([
			{ after: 1040, count: 5 },
			{ after: 902, count: 2 },
		]);
	});

	test("fails on a shape it does not know", () => {
		expect(() => extractRemodelTables(graph(HOLDER.replace("_0xc < 4500", "_0xc <= 4500")))).toThrow();
		expect(() => extractRemodelTables(graph(HOLDER, MODEL.replace("return 0;\n\t\t\t}\n\t\t}\n\t\tget revkit", "return 1;\n\t\t\t}\n\t\t}\n\t\tget revkit")))).toThrow();
		expect(() => extractRemodelTables({ modules: [], summary: {} as ModuleGraphSummary })).toThrow();
	});
});
