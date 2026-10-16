import { describe, expect, test } from "bun:test";

import { extractPortApiSeed } from "../src/port-api-seed.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function graph(exportNames: string[], source: string): ModuleGraph {
	const module = { id: "1", exportNames, source: `function(module, exports) { ${source} }` } as ModuleArtifact;
	return { modules: [module], summary: {} as ModuleGraphSummary };
}

describe("extractPortApiSeed", () => {
	test("reads the seed table the client exports", () => {
		const seed = extractPortApiSeed(
			graph(["PORT_API_SEED"], `exports.PORT_API_SEED = undefined;\nexports.PORT_API_SEED = [5560, 8863, 7748, 8399, 9742, 7258, 3474, 6103, 7002, 2282];`),
		);
		expect(seed).toEqual([5560, 8863, 7748, 8399, 9742, 7258, 3474, 6103, 7002, 2282]);
	});

	test("fails when the table is missing or not ten numbers", () => {
		expect(() => extractPortApiSeed(graph([], ""))).toThrow();
		expect(() => extractPortApiSeed(graph(["PORT_API_SEED"], "exports.PORT_API_SEED = [1, 2, 3];"))).toThrow();
		expect(() => extractPortApiSeed(graph(["PORT_API_SEED"], "exports.PORT_API_SEED = makeSeed();"))).toThrow();
	});
});
