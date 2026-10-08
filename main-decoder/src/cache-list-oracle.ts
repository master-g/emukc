// Holds the cache list against the client that will ask for what is on it.
//
// The list is expanded from rules the decoder recognises in `main.js`, and a way of building
// an address that it does not recognise is left out without a trace. Two checks from the
// client's side:
//
// - every directory the client names in a string literal (`"resources/ship/"`, …) must have
//   something listed under it;
// - the suffix in every listed ship and equipment address must be the one the client's own
//   `SuffixUtil` computes for that id and resource type.
//
// A directory the client only reaches through a variable is not seen by the first check.
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

	const client = loadClient(bundle);
	const { SuffixUtil } = client.require(client.moduleExporting("SuffixUtil"));
	const wrong: string[] = [];
	let checked = 0;
	for (const path of listed) {
		const found = path.match(/^kcs2\/resources\/(ship|slot)\/(.+)\/(\d+)_(\d+)[_.]/);
		if (!found) continue;
		const [, family, type, id, suffix] = found;
		checked += 1;
		const want = SuffixUtil.create(Number(id), `${family}_${type}`);
		if (want !== suffix) wrong.push(`${path}: the client computes ${want}`);
	}

	const report = [
		...empty.map(([directory, line]) => `NOTHING LISTED under ${directory} (main.decoded.js:${line})`),
		...wrong.map((line) => `WRONG SUFFIX ${line}`),
	];
	mkdirSync(repoPath(".data/temp"), { recursive: true });
	const reportFile = repoPath(".data/temp/cache_list_oracle.txt");
	writeFileSync(reportFile, `${report.join("\n")}\n`);
	console.log(`${listed.length} listed paths: ${named.size} directories named by the client, ${empty.length} with nothing listed; ${checked} suffixes checked, ${wrong.length} wrong; report at ${reportFile}`);
	for (const line of report.slice(0, 20)) console.log(line);
	if (report.length > 0) process.exit(1);
}

main();
