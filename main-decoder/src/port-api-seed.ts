// Reads the client's `PORT_API_SEED`, the ten numbers keyed by `member_id % 10` that sign
// `api_port/port` and scale the 戦果 in the ranking list. The client changes it between
// builds (6.3.6.0 did), so the server takes it from this asset rather than a copy.

import type { ModuleGraph } from "./types.ts";

export interface PortApiSeedAsset {
	version: 1;
	scriptVersion: string;
	seed: number[];
}

export function extractPortApiSeed(graph: ModuleGraph): number[] {
	const module = graph.modules.find((candidate) => candidate.exportNames.includes("PORT_API_SEED"));
	const literal = module?.source.match(/exports\.PORT_API_SEED = \[([\d,\s]*)\]/)?.[1];
	const seed = literal?.split(",").map((value) => Number(value.trim())) ?? [];
	if (seed.length !== 10 || !seed.every(Number.isInteger)) {
		throw new Error("port api seed: no ten-number PORT_API_SEED export found");
	}
	return seed;
}

/** No timestamp: decoding the same client twice gives the same file. */
export function toPortApiSeedAsset(scriptVersion: string, seed: number[]): PortApiSeedAsset {
	return { version: 1, scriptVersion, seed };
}
