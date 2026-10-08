// Holds the cache list against the client that will ask for what is on it.
//
// The list is expanded from rules the decoder recognises in `main.js`, and a way of building
// an address that it does not recognise is left out without a trace. Checks from the
// client's side:
//
// - every directory the client names in a string literal (`"resources/ship/"`, …) must have
//   something listed under it;
// - every resource type a ship or equipment loader is called with, as the decoder recorded
//   the call sites in `resource_manifest.json`, must have something listed under it;
// - the suffix in every listed ship and equipment address must be the one the client's own
//   `SuffixUtil` computes for that id and resource type.
//
// - for every ship that has a resource type listed, every damaged and broken state a call
//   site can ask of that type is asked of the client's own `ShipLoader.getPath`, and the
//   address it answers must be listed at the version it answers;
// - the families whose ids are kept by hand (area banners, map files, use item cards) must
//   have every id of the master data, or the hole recorded in `cache-list-known-holes.json`.
//
// What it cannot judge it says: call sites whose type is not a literal, listed addresses in
// a form it does not parse, and id groups the decoder could not read from the client.
// Run after `make decode-main` and `make cache-make-list`; a manual diagnostic, not a test.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { loadClient } from "./client-runtime";

const repoPath = (...parts: string[]) => resolve(import.meta.dir, "../..", ...parts);

/** Directories the client names that the list is right to have nothing under. */
const KNOWN_EMPTY: Record<string, string> = {
	"resources/friendly_panel/e": "the panel of the event running at the time; none is",
	"resources/friendly_panel/ship/": "ships of the friendly fleet request of an event; the ids come with the event",
	"resources/setsubun_panel/ship/": "ships of the Setsubun mini game; the ids come with it",
	"resources/world/": "the server names; this server draws them itself",
};

/** Hand-listed id groups that are swept against the master data below, so they are judged. */
const SWEPT = ["map.defaultFiles", "useItem.cardIds", "useItem.underlineIds", "area.sallyIds"];

function main() {
	const listFile = resolve(process.argv[2] ?? repoPath("z/cache/cache_resources.nedb"));
	const items = readFileSync(listFile, "utf8")
		.split("\n")
		.filter(Boolean)
		.map((line) => JSON.parse(line) as { path: string; version?: string });
	const listed = items.map((item) => item.path);
	const versionOf = new Map(items.map((item) => [item.path, item.version ?? ""]));
	const bundle = resolve(import.meta.dir, "../out/main.decoded.js");
	const source = readFileSync(bundle, "utf8").split("\n");

	const named = new Map<string, number>();
	source.forEach((line, index) => {
		for (const [, directory] of line.matchAll(/"((?:resources|img)\/[a-z0-9_/]*)"/g)) {
			if (!named.has(directory!)) named.set(directory!, index + 1);
		}
	});
	const empty = [...named].filter(([directory]) => !(directory in KNOWN_EMPTY) && !listed.some((path) => path.startsWith(`kcs2/${directory}`)));

	// `ShipLoader.getPath` turns the grey banners into their damaged directory whatever it is asked.
	const directoryOf = (kind: string, type: string) => `kcs2/resources/${kind === "ship" ? "ship" : "slot"}/${/^banner\d?_g$/.test(type) ? `${type}_dmg` : type}/`;
	const manifest = JSON.parse(readFileSync(repoPath("crates/emukc_bootstrap/assets/resource_manifest.json"), "utf8")).entries as any[];
	const callSites = manifest.filter((entry) => entry.kind === "ship" || entry.kind === "slotitem");
	const untyped = callSites.filter((entry) => typeof entry.targetType !== "string");
	const typed = new Map<string, string>();
	for (const entry of callSites) {
		if (typeof entry.targetType === "string") typed.set(directoryOf(entry.kind, entry.targetType), `${entry.kind} ${entry.targetType} (${entry.moduleNames?.[0] ?? "?"})`);
	}
	const unlistedTypes = [...typed].filter(([directory]) => !listed.some((path) => path.startsWith(directory)));

	const ui = JSON.parse(readFileSync(repoPath("crates/emukc_bootstrap/assets/ui_resources.json"), "utf8"));
	const byHand: string[] = [];
	const walk = (node: any, name: string) => {
		if (node === null || typeof node !== "object") return;
		if (typeof node.coverageMode === "string" && node.coverageMode !== "observed-complete" && !SWEPT.includes(name)) byHand.push(`${name} (${node.coverageMode})`);
		for (const [key, child] of Object.entries(node)) walk(child, name ? `${name}.${key}` : key);
	};
	walk(ui, "");

	const client = loadClient(bundle);
	const { SuffixUtil } = client.require(client.moduleExporting("SuffixUtil"));
	const wrong: string[] = [];
	let checked = 0;
	const unparsed = new Map<string, number>();
	for (const path of listed) {
		if (!/^kcs2\/resources\/(ship|slot)\//.test(path)) continue;
		// `_b` after the id is the broken look of an abyssal ship; it does not enter the suffix.
		const found = path.match(/^kcs2\/resources\/(ship|slot)\/(.+)\/(\d+)(?:_b)?_(\d+)[_.]/);
		if (!found) {
			const directory = path.slice(0, path.lastIndexOf("/"));
			unparsed.set(directory, (unparsed.get(directory) ?? 0) + 1);
			continue;
		}
		const [, family, type, id, suffix] = found;
		checked += 1;
		const want = SuffixUtil.create(Number(id), `${family}_${type}`);
		if (want !== suffix) wrong.push(`${path}: the client computes ${want}`);
	}

	// The client's own `ShipLoader.getPath`, answering from the codex what it would ask the
	// master data models. For every ship that has a resource type listed, every damaged and
	// broken state a call site can ask of that type must be listed too, at the same version.
	const start2 = JSON.parse(readFileSync(repoPath(".data/codex/start2.json"), "utf8"));
	const graphs = new Map<number, any>(start2.api_mst_shipgraph.map((graph: any) => [graph.api_id, graph]));
	const loaderModule = client.moduleExporting("ShipLoader");
	const singleton = client.moduleSource(loaderModule).match(/\(_0x[0-9a-f]+\((\d+)\)\)/)?.[1];
	if (singleton === undefined) throw new Error("cannot find the application singleton the ship loader imports");
	const app = client.require(Number(singleton)).default;
	app.model.ship_graph.get = (id: number | string) => {
		const graph = graphs.get(Number(id));
		return graph ? { unique_key: graph.api_filename, version: graph.api_version[0], getSpFlag: () => graph.api_sp_flag ?? 0 } : null;
	};
	Object.defineProperty(app.settings, "path_root", { get: () => "kcs2/" });
	const { ShipLoader } = client.require(loaderModule);

	const states = new Map<string, Set<boolean>>();
	for (const entry of callSites) {
		if (entry.kind !== "ship" || typeof entry.targetType !== "string") continue;
		const asked = states.get(entry.targetType) ?? new Set<boolean>();
		for (const damaged of entry.damagedSource === "false" ? [false] : entry.damagedSource === "true" ? [true] : [false, true]) asked.add(damaged);
		states.set(entry.targetType, asked);
	}
	// Types some call site passes the fourth argument, the broken look, as `true`.
	const breakable = new Set([...readFileSync(bundle, "utf8").matchAll(/"([a-z0-9_]+)", true\)/g)].map((match) => match[1]!).filter((type) => states.has(type)));
	const listedSet = new Set(listed);
	// What the origin is known not to have: the generator's own hole tables, which travel in
	// the manifest's path rules, and the ones recorded beside this script.
	const pathRules = JSON.parse(readFileSync(repoPath("crates/emukc_bootstrap/assets/resource_manifest.json"), "utf8")).pathRules;
	const recorded = JSON.parse(readFileSync(resolve(import.meta.dir, "../cache-list-known-holes.json"), "utf8"));
	const holeTables: Record<string, number[]> = {
		"character_full/": pathRules.eventShipHoles.full,
		"character_full_dmg/": pathRules.eventShipHoles.fullDmg,
		"character_up/": pathRules.eventShipHoles.up,
		"character_up_dmg/": pathRules.eventShipHoles.upDmg,
		"full/": pathRules.enemyShipHoles.full,
		"full_dmg/": pathRules.enemyShipHoles.fullDmg,
	};
	for (const [directory, hole] of Object.entries<any>(recorded)) {
		if (typeof hole === "object" && directory.startsWith("kcs2/resources/ship/")) holeTables[directory.replace("kcs2/resources/ship/", "")] = hole.ids;
	}
	const isKnownHole = (path: string, id: number) => Object.entries(holeTables).some(([directory, ids]) => path.startsWith(`kcs2/resources/ship/${directory}`) && ids.includes(id));

	// Families whose ids are listed by hand, held against the ids the master data has: an id
	// that is neither listed nor a recorded hole is new, and has to be asked of the origin.
	const pad = (id: number, width: number) => String(id).padStart(width, "0");
	const sweeps: [string, number][] = [
		...start2.api_mst_maparea.map((area: any) => [`kcs2/resources/area/sally/${pad(area.api_id, 3)}.png`, area.api_id]),
		...start2.api_mst_mapinfo.flatMap((map: any) => ["_image.png", "_image.json", "_info.json"].map((file) => [`kcs2/resources/map/${pad(map.api_maparea_id, 3)}/${pad(map.api_no, 2)}${file}`, map.api_id])),
		...start2.api_mst_useitem.filter((item: any) => item.api_name).flatMap((item: any) => ["card", "card_"].map((kind) => [`kcs2/resources/useitem/${kind}/${pad(item.api_id, 3)}.png`, item.api_id])),
	];
	const unswept = sweeps.filter(([path, id]) => !listedSet.has(path) && !recorded[path.slice(0, path.lastIndexOf("/") + 1)]?.ids?.includes(id)).map(([path]) => `NOT LISTED ${path} (its id is in the master data; ask the origin, then list it or record the hole)`);
	let holes = 0;
	const variants: string[] = [];
	let asked = 0;
	for (const [type, damagedStates] of states) {
		const home = directoryOf("ship", type);
		const ids = new Set(listed.filter((path) => path.startsWith(home)).map((path) => Number(path.slice(home.length).match(/^\d+/)?.[0])));
		for (const id of ids) {
			for (const damaged of damagedStates) {
				for (const broken of breakable.has(type) ? [false, true] : [false]) {
					const [path, version = ""] = (ShipLoader.getPath(id, damaged, type, broken) as string).split("?version=");
					asked += 1;
					if (!listedSet.has(path!) && isKnownHole(path!, id)) {
						holes += 1;
						continue;
					}
					if (!listedSet.has(path!)) variants.push(`NOT LISTED ${path} (${type}, damaged ${damaged}, broken ${broken})`);
					else if ((versionOf.get(path!) || "") !== (version === "1" ? "" : version)) variants.push(`WRONG VERSION ${path}: listed ${versionOf.get(path!) || "none"}, the client asks ${version || "none"}`);
				}
			}
		}
	}

	const report = [
		...empty.map(([directory, line]) => `NOTHING LISTED under ${directory} (main.decoded.js:${line})`),
		...unlistedTypes.map(([directory, site]) => `NOTHING LISTED under ${directory}, asked for by ${site}`),
		...wrong.map((line) => `WRONG SUFFIX ${line}`),
		...variants,
		...unswept,
	];
	mkdirSync(repoPath(".data/temp"), { recursive: true });
	const reportFile = repoPath(".data/temp/cache_list_oracle.txt");
	const notJudged = [
		...untyped.map((entry) => `call site without a literal type: ${entry.kind} in ${entry.moduleNames?.[0] ?? "?"}`),
		...[...unparsed].map(([directory, count]) => `${count} addresses under ${directory} are in a form whose suffix is not checked`),
		...byHand.map((group) => `ids listed by hand and in no master data to sweep them against: ${group}`),
	];
	writeFileSync(reportFile, `${[...report, ...notJudged.map((line) => `NOT JUDGED ${line}`)].join("\n")}\n`);
	console.log(`${listed.length} listed paths: ${named.size} directories named by the client, ${empty.length} with nothing listed; ${typed.size} resource types at ${callSites.length} loader call sites, ${unlistedTypes.length} with nothing listed; ${checked} suffixes checked, ${wrong.length} wrong; ${asked} ship addresses asked of the client's loader, ${variants.length} not listed as it asks (${holes} more are known holes); ${sweeps.length} addresses of hand-listed families swept against the master data, ${unswept.length} unaccounted for; ${notJudged.length} things not judged; report at ${reportFile}`);
	for (const line of report.slice(0, 20)) console.log(line);
	for (const line of notJudged) console.log(`not judged: ${line}`);
	if (report.length > 0) process.exit(1);
}

main();
