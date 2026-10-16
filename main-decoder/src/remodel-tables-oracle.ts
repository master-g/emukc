// Checks `remodel_tables.json` against the client's own functions it was read from.
//
// `remodel-tables.ts` reads switch statements; this runs them. For every ship id, with and without
// a blueprint and across the steel tiers, the client's `_getRequiredDevkitNum` and
// `_getRequiredBuildKitNum` must give what the asset gives, and the three `ShipUpgradeModel`
// getters must too for every remodel target. Any difference fails.

import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { loadClient } from "./client-runtime";
import type { RemodelTableEntry, RemodelTablesAsset } from "./remodel-tables";

const MAX_SHIP_ID = 1500;

export function devkitFromAsset(asset: RemodelTablesAsset, ship: number, blueprint: number, steel: number): number {
	const listed = asset.devkit.byShip[String(ship)];
	if (listed !== undefined) return listed;
	if (blueprint !== 0 && !asset.devkit.blueprintGroup.includes(ship)) return 0;
	return asset.devkit.steelTiers.find((tier) => steel < tier.below)?.count ?? asset.devkit.steelTop;
}

export function entryCount(entries: RemodelTableEntry[], before: number, after: number): number {
	return entries.find((entry) => entry.after === after && (entry.before === undefined || entry.before === before))?.count ?? 0;
}

if (import.meta.main) {
	const asset = JSON.parse(
		readFileSync(resolve(import.meta.dir, "../../crates/emukc_bootstrap/assets/remodel_tables.json"), "utf8"),
	) as RemodelTablesAsset;
	const client = loadClient(resolve(import.meta.dir, "../out/main.decoded.js"));
	const holder = new (client.require(client.moduleExporting("ShipUpgradeModelHolder")).ShipUpgradeModelHolder)();
	const { ShipUpgradeModel } = client.require(client.moduleExporting("ShipUpgradeModel"));

	const steels = [0, ...asset.devkit.steelTiers.flatMap((tier) => [tier.below - 1, tier.below]), 99999];
	const befores = [0, ...new Set([...asset.artillery, ...asset.screw, ...asset.arsenal].flatMap((entry) => entry.before ?? []))];
	const diffs: string[] = [];
	let probes = 0;
	for (let ship = 1; ship <= MAX_SHIP_ID; ship++) {
		for (const blueprint of [0, 1]) {
			for (const steel of steels) {
				probes++;
				const want = holder._getRequiredDevkitNum(ship, blueprint, steel);
				const got = devkitFromAsset(asset, ship, blueprint, steel);
				if (want !== got) diffs.push(`devkit ship ${ship} blueprint ${blueprint} steel ${steel}: client ${want}, asset ${got}`);
			}
		}
		probes++;
		const buildkit = holder._getRequiredBuildKitNum(ship);
		if (buildkit !== (asset.buildkit.byShip[String(ship)] ?? 0)) diffs.push(`buildkit ship ${ship}: client ${buildkit}, asset ${asset.buildkit.byShip[String(ship)] ?? 0}`);
		for (const before of befores) {
			const row = new ShipUpgradeModel({ api_id: ship, api_current_ship_id: before });
			for (const [getter, entries] of [["newhokohesosizai", asset.artillery], ["revkit", asset.screw], ["arsenalmaterial", asset.arsenal]] as const) {
				probes++;
				const want = row[getter];
				const got = entryCount(entries, before, ship);
				if (want !== got) diffs.push(`${getter} ${before} -> ${ship}: client ${want}, asset ${got}`);
			}
		}
	}
	console.log(`${probes} probes over ship ids 1..${MAX_SHIP_ID}: ${diffs.length} differ`);
	for (const diff of diffs.slice(0, 20)) console.log(`  ${diff}`);
	if (diffs.length > 0) process.exit(1);
}
