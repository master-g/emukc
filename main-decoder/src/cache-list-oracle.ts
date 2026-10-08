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
// What it cannot judge it says: call sites whose type is not a literal, listed addresses in
// a form it does not parse, and id groups the decoder could not read from the client, which
// are listed by hand. It does not check that a directory has every id it should: something
// listed under each is all it asks.
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

function main() {
	const listFile = resolve(process.argv[2] ?? repoPath("z/cache/cache_resources.nedb"));
	const listed = readFileSync(listFile, "utf8")
		.split("\n")
		.filter(Boolean)
		.map((line) => JSON.parse(line).path as string);
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
		if (typeof node.coverageMode === "string" && node.coverageMode !== "observed-complete") byHand.push(`${name} (${node.coverageMode})`);
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

	const report = [
		...empty.map(([directory, line]) => `NOTHING LISTED under ${directory} (main.decoded.js:${line})`),
		...unlistedTypes.map(([directory, site]) => `NOTHING LISTED under ${directory}, asked for by ${site}`),
		...wrong.map((line) => `WRONG SUFFIX ${line}`),
	];
	mkdirSync(repoPath(".data/temp"), { recursive: true });
	const reportFile = repoPath(".data/temp/cache_list_oracle.txt");
	const notJudged = [
		...untyped.map((entry) => `call site without a literal type: ${entry.kind} in ${entry.moduleNames?.[0] ?? "?"}`),
		...[...unparsed].map(([directory, count]) => `${count} addresses under ${directory} are in a form whose suffix is not checked`),
		...byHand.map((group) => `ids listed by hand, the decoder could not read them from the client: ${group}`),
	];
	writeFileSync(reportFile, `${[...report, ...notJudged.map((line) => `NOT JUDGED ${line}`)].join("\n")}\n`);
	console.log(`${listed.length} listed paths: ${named.size} directories named by the client, ${empty.length} with nothing listed; ${typed.size} resource types at ${callSites.length} loader call sites, ${unlistedTypes.length} with nothing listed; ${checked} suffixes checked, ${wrong.length} wrong; ${notJudged.length} things not judged; report at ${reportFile}`);
	for (const line of report.slice(0, 20)) console.log(line);
	for (const line of notJudged) console.log(`not judged: ${line}`);
	if (report.length > 0) process.exit(1);
}

main();
