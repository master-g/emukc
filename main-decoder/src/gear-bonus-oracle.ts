// Checks the converted equipment bonus table against the game client's own bonus code.
//
// The client shows a ship's equipment bonus with `SlotItemEffectUtil`, a few hundred
// hand-written functions. The server works the same numbers out from the table converted from
// KC3Kai (`gear-bonus.ts`) with a port of KC3Kai's reader. This runs both over the same ships
// and loadouts and compares the seven stats the server applies. A difference is either a
// mistake in the conversion or the port, or KC3Kai and the client disagreeing. The entries known
// to disagree are listed in `gear-bonus-known-diffs.json` with how many probes differ; a new
// entry or a changed count fails. `--accept` rewrites the list from the current run, keeping
// the reasons already written down.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { loadClient } from "./client-runtime";
import type { GearBonusDocument } from "./gear-bonus";

/** Client stat name to the name used in the asset and by the server. */
const STATS = { houg: "houg", raig: "raig", tyku: "tyku", souk: "souk", kaih: "houk", tais: "tais", saku: "saku" } as const;

/** Equipment id and improvement level. */
type Loadout = [number, number][];

interface KnownDiffs {
	/** What the list was last accepted against. */
	against: { kc3kai: string; client: string };
	/** Entry key to the number of differing probes and why they differ, once someone looked. */
	gears: Record<string, { probes: number; reason: string }>;
}

interface Probe {
	ship: number;
	gears: Loadout;
}

function repoPath(...segments: string[]): string {
	return resolve(import.meta.dir, "../..", ...segments);
}

/** The loadouts that exercise one entry: alone in several counts, and with what its rules name. */
export function loadoutsFor(ids: number[], companions: number[]): Loadout[] {
	const loadouts: Loadout[] = [];
	for (const id of ids) {
		for (const stars of [0, 10]) {
			for (const count of [1, 2, 3]) loadouts.push(Array(count).fill([id, stars]));
		}
		loadouts.push([[id, 10], [id, 0]]);
		for (const other of companions) {
			if (other === id) continue;
			loadouts.push([[id, 10], [other, 0]], [[id, 0], [other, 10]], [[id, 10], [id, 10], [other, 0]], [[id, 0], [other, 0], [other, 0]], [[id, 10], [other, 10], [other, 10]]);
		}
		const all = companions.filter((other) => other !== id);
		if (all.length > 1) loadouts.push([[id, 10], ...all.slice(0, 5).map((other): [number, number] => [other, 10])]);
	}
	return loadouts;
}

/** Everything an entry's rules look at besides the entry's own equipment. */
export function companionsOf(document: GearBonusDocument, key: string): number[] {
	const found = new Set<number>();
	for (const rule of document.gears.find((gear) => gear.key === key)?.rules ?? []) {
		for (const id of rule.distinctGears ?? []) found.add(id);
		for (const synergy of rule.synergy ?? []) {
			for (const name of [...synergy.flags, synergy.byCount?.gear ?? ""]) {
				const ids = document.synergyGears[name.replace(/Nonexist$/, "")];
				if (ids?.[0] !== undefined) found.add(ids[0]);
			}
			if (synergy.byStars) found.add(Number(synergy.byStars.gearId));
		}
	}
	return [...found];
}

if (import.meta.main) {
	const document = JSON.parse(readFileSync(repoPath("crates/emukc_bootstrap/assets/gear_bonus.json"), "utf8")) as GearBonusDocument;
	const knownFile = resolve(import.meta.dir, "../gear-bonus-known-diffs.json");
	const known = JSON.parse(readFileSync(knownFile, "utf8")) as KnownDiffs;
	const start2 = JSON.parse(readFileSync(repoPath(".data/codex/start2.json"), "utf8"));
	// Abyssal ships and equipment start at 1501.
	const items = new Map<number, any>(start2.api_mst_slotitem.filter((item: any) => item.api_id <= 1500).map((item: any) => [item.api_id, item]));
	const ships: any[] = start2.api_mst_ship.filter((ship: any) => ship.api_id <= 1500);

	const client = loadClient(resolve(import.meta.dir, "../out/main.decoded.js"));
	const paramModule = client.moduleExporting("SlotItemEffectParamModel");
	// The parameter model looks equipment master data up on the application singleton, the
	// first module it imports. Nothing loaded any master data, so answer from the codex.
	const singleton = client.moduleSource(paramModule).match(/\(_0x[0-9a-f]+\((\d+)\)\)/)?.[1];
	if (singleton === undefined) throw new Error("cannot find the application singleton the parameter model imports");
	client.require(Number(singleton)).default.model.slot.getMst = (id: number | string) => {
		const item = items.get(Number(id));
		return { equipType: item.api_type[2], sakuteki: item.api_saku, meichu: item.api_houm, taiku: item.api_tyku };
	};
	const { SlotItemEffectUtil } = client.require(client.moduleExporting("SlotItemEffectUtil"));

	// Bonus rules are written for the ships that can carry the equipment; what either side
	// says about a loadout the game does not allow means nothing.
	const stypes = new Map<number, any>(start2.api_mst_stype.map((stype: any) => [stype.api_id, stype]));
	const canEquip = (ship: any, id: number): boolean => {
		const type = String(items.get(id).api_type[2]);
		const own = start2.api_mst_equip_ship[ship.api_id]?.api_equip_type;
		const inSlot = own ? type in own && (own[type] === null || own[type].includes(id)) : stypes.get(ship.api_stype)?.api_equip_type[type] === 1;
		const extra = start2.api_mst_equip_exslot_ship[id];
		const inExtraSlot = start2.api_mst_equip_exslot.includes(Number(type)) || extra?.api_ship_ids?.[ship.api_id] === 1 || extra?.api_stypes?.[ship.api_stype] === 1 || extra?.api_ctypes?.[ship.api_ctype] === 1;
		return inSlot || inExtraSlot;
	};

	const probes: Probe[] = [];
	const origins: string[] = [];
	for (const gear of document.gears) {
		const typed = gear.key.match(/^t([23])_(\d+)$/);
		const ids = typed ? [...items.values()].filter((item) => item.api_type[Number(typed[1])] === Number(typed[2])).map((item) => item.api_id as number).slice(0, 3) : [Number(gear.key)];
		for (const gears of loadoutsFor(ids.filter((id) => items.has(id)), companionsOf(document, gear.key).filter((id) => items.has(id)))) {
			for (const ship of ships) {
				if (!gears.every(([id]) => canEquip(ship, id))) continue;
				probes.push({ ship: ship.api_id, gears });
				origins.push(gear.key);
			}
		}
	}

	const probeFile = repoPath(".data/temp/gear_bonus_probes.json");
	mkdirSync(repoPath(".data/temp"), { recursive: true });
	writeFileSync(probeFile, JSON.stringify(probes));
	const server = Bun.spawnSync(["cargo", "run", "-q", "--release", "--", "gear-bonus", "probe", "--input", probeFile], { cwd: repoPath(), stdout: "pipe", stderr: "inherit" });
	if (server.exitCode !== 0) throw new Error(`gear-bonus probe failed with exit code ${server.exitCode}`);
	const actual = JSON.parse(server.stdout.toString()) as Record<string, number>[];

	const shipById = new Map(ships.map((ship) => [ship.api_id as number, ship]));
	const differing = new Map<string, { count: number; first: string }>();
	/** Every differing probe, for working out what a correction has to say. */
	const details: string[] = [];
	probes.forEach((probe, index) => {
		const mst = shipById.get(probe.ship);
		const ship = { mstID: mst.api_id, yomi: mst.api_yomi, shipTypeID: mst.api_stype, getClassType: () => mst.api_ctype };
		const slots = probe.gears.map(([id, level]) => ({ mstID: id, equipType: items.get(id).api_type[2], level }));
		const want = SlotItemEffectUtil.getSlotitemEffect(ship, slots);
		const diffs = Object.entries(STATS).filter(([theirs, ours]) => (want?.[theirs] ?? 0) !== (actual[index]![ours] ?? 0));
		if (diffs.length === 0) return;
		details.push(JSON.stringify({ key: origins[index], ship: probe.ship, gears: probe.gears, delta: Object.fromEntries(diffs.map(([theirs, ours]) => [ours, (want?.[theirs] ?? 0) - (actual[index]![ours] ?? 0)])) }));
		const entry = differing.get(origins[index]!) ?? { count: 0, first: "" };
		entry.count += 1;
		entry.first ||= `${mst.api_name} (${mst.api_id}, class ${mst.api_ctype}) with ${probe.gears.map(([id, stars]) => `${id}★${stars}`).join(" ")}: ${diffs.map(([theirs, ours]) => `${theirs} client ${want?.[theirs] ?? 0} / server ${actual[index]![ours] ?? 0}`).join(", ")}`;
		differing.set(origins[index]!, entry);
	});

	const report: string[] = [];
	const unlisted: string[] = [];
	for (const [key, { count, first }] of differing) {
		const listed = known.gears[key];
		if (listed?.probes !== count) unlisted.push(key);
		report.push(`${listed?.probes === count ? "listed" : "UNLISTED"} ${key} ${items.get(Number(key))?.api_name ?? ""}: ${count} probes differ${listed ? ` (listed: ${listed.probes}; ${listed.reason})` : ""}\n  e.g. ${first}`);
	}
	const stale = Object.keys(known.gears).filter((key) => !differing.has(key));
	const reportFile = repoPath(".data/temp/gear_bonus_oracle.txt");
	writeFileSync(reportFile, `${report.join("\n")}\n`);
	writeFileSync(repoPath(".data/temp/gear_bonus_diffs.jsonl"), `${details.join("\n")}\n`);
	console.log(`${probes.length} probes over ${document.gears.length} entries and ${ships.length} ships: ${differing.size} entries differ, ${unlisted.length} of them not as listed; report at ${reportFile}`);
	if (stale.length > 0) console.log(`listed but no longer differing: ${stale.join(", ")}`);
	if (process.argv.includes("--accept")) {
		const accepted: KnownDiffs = {
			against: { kc3kai: document.source.commit, client: readFileSync(resolve(import.meta.dir, "../out/version.txt"), "utf8").trim() },
			gears: Object.fromEntries([...differing].map(([key, { count }]) => [key, { probes: count, reason: known.gears[key]?.reason ?? "not looked into" }])),
		};
		writeFileSync(knownFile, `${JSON.stringify(accepted, null, 2)}\n`);
		console.log(`accepted ${differing.size} entries into ${knownFile}`);
	} else if (unlisted.length > 0 || stale.length > 0) {
		process.exit(1);
	}
}
