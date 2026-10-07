// Converts KC3Kai's table of visible equipment bonuses into a neutral JSON asset.
//
// The source (`src/library/objects/GearBonus.js`) is one function returning an object literal,
// keyed by equipment id, with the conditions a ship has to meet and the stats it then gains.
// `KC3Gear.equipmentTotalStatsOnShipBonus` is what reads it; the Rust side ports that reader.
// This module only reshapes the literal: aliases between entries are resolved, "one or a list"
// becomes a list, and the nation table the rules refer to is taken from `Meta.js` of the same
// commit. A key it does not know fails the conversion instead of being dropped, so a source
// upgrade that introduces a new qualifier is noticed.
//
// Where the table disagrees with the game client, `gear-bonus-corrections.json` holds the fix
// and is merged in here; `gear-bonus-oracle.ts` is what finds the disagreements.

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

import { parse } from "@babel/parser";
import type * as t from "@babel/types";

const STATS = ["houg", "raig", "tyku", "souk", "houk", "tais", "saku", "houm", "leng", "soku", "baku"] as const;
const LIST_QUALIFIERS = ["ids", "excludes", "origins", "classes", "excludeClasses", "stypes", "excludeStypes", "distinctGears"] as const;
const NUMBER_QUALIFIERS = ["remodel", "remodelCap", "minStars", "minCount", "countCap", "speedCap"] as const;

export type Stats = Partial<Record<(typeof STATS)[number], number>>;

export interface Synergy {
	/** Counters that all have to be above zero. `<name>Nonexist` is above zero when `<name>` is zero. */
	flags: string[];
	/** Corrections only: other equipment that has to be carried as well, `minCount` copies (one by default) with `minStars` or more. */
	requires?: { gears: number[]; minStars?: number; minCount?: number }[];
	single?: Stats;
	multiple?: Stats;
	/** Index into `flags` of the counter `multiple` scales with; the equipment's own count otherwise. */
	countFlag?: number;
	countCap?: number;
	/** Granted once per stat evaluation, however many rules name the same flags. */
	distinct?: Stats;
	byCount?: { gear: string; distinct?: boolean; table: Record<string, Stats> };
	byStars?: { gearId: string; noStarsLessThan?: number; isMultiple?: boolean; table: { minStars: number; stats: Stats }[] };
}

export interface Rule {
	/** The ship class (`api_ctype`) the rule was listed under, if any. */
	class?: number;
	/** The nation the rule was listed under, if any; see `nations`. */
	nation?: string;
	ids?: number[];
	excludes?: number[];
	origins?: number[];
	classes?: number[];
	excludeClasses?: number[];
	stypes?: number[];
	excludeStypes?: number[];
	distinctGears?: number[];
	remodel?: number;
	remodelCap?: number;
	minStars?: number;
	minCount?: number;
	countCap?: number;
	speedCap?: number;
	single?: Stats;
	multiple?: Stats;
	synergy?: Synergy[];
}

export interface GearEntry {
	/**
	 * An equipment id, or `t2_<n>` / `t3_<n>` for every equipment of that `api_type[2]` / `api_type[3]`.
	 * Corrections may join ids with `+` into one entry that counts the copies of all of them together.
	 */
	key: string;
	rules: Rule[];
}

export interface GearBonusDocument {
	source: { repo: string; commit: string; files: string[] };
	/** Nation name to ship classes. A class in none of the lists counts as `Japan`. */
	nations: Record<string, number[]>;
	/** Counter name to the equipment ids that raise it. */
	synergyGears: Record<string, number[]>;
	/** In the order the source's reader visits them; some qualifiers only grant on first visit. */
	gears: GearEntry[];
}

export class GearBonusSyntaxError extends Error {}

function fail(message: string): never {
	throw new GearBonusSyntaxError(`gear bonus source: ${message}`);
}

type Plain = number | string | boolean | Plain[] | { [key: string]: Plain };

/** The value of a literal expression. Anything that would need evaluating fails. */
function literalValue(node: t.Node, code: string): Plain {
	switch (node.type) {
		case "NumericLiteral":
		case "StringLiteral":
		case "BooleanLiteral":
			return node.value;
		case "UnaryExpression": {
			const value = literalValue(node.argument, code);
			if (node.operator === "-" && typeof value === "number") return -value;
			break;
		}
		case "ArrayExpression":
			return node.elements.map((element) => (element === null ? fail(`hole in an array at line ${node.loc?.start.line}`) : literalValue(element, code)));
		case "ObjectExpression": {
			const object: Record<string, Plain> = {};
			for (const property of node.properties) {
				if (property.type !== "ObjectProperty" || property.computed) break;
				const key = property.key.type === "Identifier" ? property.key.name : property.key.type === "StringLiteral" || property.key.type === "NumericLiteral" ? String(property.key.value) : null;
				if (key === null) break;
				if (key in object) fail(`key ${key} appears twice at line ${property.loc?.start.line}`);
				object[key] = literalValue(property.value, code);
			}
			if (Object.keys(object).length === node.properties.length) return object;
			break;
		}
	}
	return fail(`not a plain literal at line ${node.loc?.start.line}: ${code.slice(node.start ?? 0, (node.start ?? 0) + 60)}`);
}

function findNode(root: t.Node, wanted: (node: t.Node) => t.Node | null): t.Node | null {
	const stack: unknown[] = [root];
	while (stack.length > 0) {
		const current = stack.pop();
		if (Array.isArray(current)) {
			stack.push(...current);
		} else if (current !== null && typeof current === "object" && typeof (current as t.Node).type === "string") {
			const found = wanted(current as t.Node);
			if (found !== null) return found;
			for (const [key, child] of Object.entries(current)) {
				if (key !== "loc" && child !== null && typeof child === "object") stack.push(child);
			}
		}
	}
	return null;
}

/** The object literal returned by `<anything>.<name> = function () { return {...}; }`. */
function returnedObject(code: string, name: string): Plain {
	const assigned = findNode(parse(code, { sourceType: "script" }), (node) =>
		node.type === "AssignmentExpression" && node.left.type === "MemberExpression" && node.left.property.type === "Identifier" && node.left.property.name === name && node.right.type === "FunctionExpression"
			? node.right.body
			: null,
	);
	const returned = assigned && findNode(assigned, (node) => (node.type === "ReturnStatement" && node.argument?.type === "ObjectExpression" ? node.argument : null));
	return returned ? literalValue(returned, code) : fail(`cannot find the object returned by ${name}`);
}

/** The object literal under the property `name`. */
function propertyObject(code: string, name: string): Plain {
	const value = findNode(parse(code, { sourceType: "script" }), (node) =>
		node.type === "ObjectProperty" && node.key.type === "Identifier" && node.key.name === name && node.value.type === "ObjectExpression" ? node.value : null,
	);
	return value ? literalValue(value, code) : fail(`cannot find the property ${name}`);
}

function isRecord(value: Plain | undefined): value is { [key: string]: Plain } {
	return typeof value === "object" && !Array.isArray(value);
}

function numbers(value: Plain | undefined, where: string): number[] {
	if (!Array.isArray(value) || value.some((item) => typeof item !== "number")) fail(`${where} is not a list of numbers`);
	return value as number[];
}

function number(value: Plain | undefined, where: string): number {
	return typeof value === "number" ? value : fail(`${where} is not a number`);
}

function stats(value: Plain | undefined, where: string): Stats {
	if (!isRecord(value)) fail(`${where} is not a stat object`);
	const result: Stats = {};
	for (const [key, amount] of Object.entries(value)) {
		if (!(STATS as readonly string[]).includes(key)) fail(`${where} names the unknown stat ${key}`);
		result[key as keyof Stats] = number(amount, `${where}.${key}`);
	}
	return result;
}

/** Splits the "named keys plus numeric keys" objects of `byCount` and `byStars`. */
function numericKeys(value: { [key: string]: Plain }, named: string[], where: string): [string, Stats][] {
	const rows: [string, Stats][] = [];
	for (const [key, entry] of Object.entries(value)) {
		if (/^\d+$/.test(key)) rows.push([key, stats(entry, `${where}.${key}`)]);
		else if (!named.includes(key)) fail(`${where} has the unknown key ${key}`);
	}
	return rows;
}

interface Context {
	counters: Set<string>;
	nonexist: Set<string>;
}

function convertSynergy(value: Plain, where: string, ctx: Context): Synergy {
	if (!isRecord(value)) fail(`${where} is not an object`);
	if (!Array.isArray(value.flags) || value.flags.some((flag) => typeof flag !== "string")) fail(`${where}.flags is not a list of names`);
	const flags = value.flags as string[];
	for (const flag of flags) {
		if (!ctx.counters.has(flag) && !ctx.nonexist.has(flag)) fail(`${where} names the unknown flag ${flag}`);
	}
	const synergy: Synergy = { flags };
	for (const [key, entry] of Object.entries(value)) {
		const at = `${where}.${key}`;
		if (key === "flags") continue;
		if (key === "single" || key === "multiple" || key === "distinct") {
			synergy[key] = stats(entry, at);
		} else if (key === "countFlag" || key === "countCap") {
			synergy[key] = number(entry, at);
			if (key === "countFlag" && !(synergy.countFlag! in flags)) fail(`${at} is outside flags`);
		} else if (key === "byCount" && isRecord(entry)) {
			const gear = entry.gear;
			if (typeof gear !== "string" || (gear !== "this" && !ctx.counters.has(gear))) fail(`${at}.gear is not a counter`);
			if ("distinct" in entry && typeof entry.distinct !== "boolean") fail(`${at}.distinct is not a boolean`);
			synergy.byCount = {
				gear: gear as string,
				...(entry.distinct === true && { distinct: true }),
				table: Object.fromEntries(numericKeys(entry, ["gear", "distinct"], at)),
			};
		} else if (key === "byStars" && isRecord(entry)) {
			const gearId = String(number(entry.gearId, `${at}.gearId`));
			if ("isMultiple" in entry && typeof entry.isMultiple !== "boolean") fail(`${at}.isMultiple is not a boolean`);
			synergy.byStars = {
				gearId,
				...("noStarsLessThan" in entry && { noStarsLessThan: number(entry.noStarsLessThan, `${at}.noStarsLessThan`) }),
				...(entry.isMultiple === true && { isMultiple: true }),
				table: numericKeys(entry, ["gearId", "noStarsLessThan", "isMultiple"], at).map(([minStars, amount]) => ({ minStars: Number(minStars), stats: amount })),
			};
		} else {
			fail(`${where} has the unknown key ${key}`);
		}
	}
	return synergy;
}

function convertRule(value: Plain, scope: Pick<Rule, "class" | "nation">, where: string, ctx: Context): Rule {
	if (!isRecord(value)) fail(`${where} is not an object`);
	const rule: Rule = { ...scope };
	for (const [key, entry] of Object.entries(value)) {
		const at = `${where}.${key}`;
		if ((LIST_QUALIFIERS as readonly string[]).includes(key)) {
			rule[key as (typeof LIST_QUALIFIERS)[number]] = numbers(entry, at);
		} else if ((NUMBER_QUALIFIERS as readonly string[]).includes(key)) {
			rule[key as (typeof NUMBER_QUALIFIERS)[number]] = number(entry, at);
		} else if (key === "single" || key === "multiple") {
			rule[key] = stats(entry, at);
		} else if (key === "synergy") {
			rule.synergy = (Array.isArray(entry) ? entry : [entry]).map((synergy, index) => convertSynergy(synergy, `${at}[${index}]`, ctx));
		} else {
			fail(`${where} has the unknown key ${key}`);
		}
	}
	return rule;
}

/**
 * Hand-written fixes where the source disagrees with the game client, by entry key. The rules
 * are in the converted form. `replace` stands in for the source's rules, `append` follows them
 * (or the replacement).
 */
export type Corrections = Record<string, { why: string; replace?: Rule[]; append?: Rule[] }>;

export function convertGearBonus(gearBonusCode: string, metaCode: string, source: GearBonusDocument["source"], corrections: Corrections = {}): GearBonusDocument {
	const table = returnedObject(gearBonusCode, "explicitStatsBonusGears");
	const nationTable = propertyObject(metaCode, "countryCtypeMap");
	if (!isRecord(table) || !isRecord(table.synergyGears) || !isRecord(nationTable)) fail("the tables are not objects");

	const nations: Record<string, number[]> = {};
	for (const [name, classes] of Object.entries(nationTable)) nations[name] = numbers(classes, `nation ${name}`);

	const synergyGears: Record<string, number[]> = {};
	const ctx: Context = { counters: new Set(), nonexist: new Set() };
	for (const [key, value] of Object.entries(table.synergyGears)) {
		if (key.endsWith("Ids")) synergyGears[key.slice(0, -3)] = numbers(value, `synergyGears.${key}`);
		else if (key.endsWith("Nonexist") && value === 1) ctx.nonexist.add(key);
		else if (value !== 0) fail(`synergyGears.${key} is neither a counter, an id list nor a Nonexist flag`);
	}
	for (const key of Object.keys(table.synergyGears)) {
		const base = key.endsWith("Nonexist") ? key.slice(0, -8) : key.endsWith("Ids") ? key.slice(0, -3) : key;
		if (!(base in synergyGears)) fail(`synergyGears.${key} has no id list`);
	}
	for (const name of Object.keys(synergyGears)) ctx.counters.add(name);

	const entries = Object.entries(table).filter(([key]) => key !== "synergyGears");
	for (const [key, entry] of entries) {
		if (!/^(\d+|t[23]_\d+)$/.test(key) || !isRecord(entry)) fail(`unknown entry ${key}`);
		if (entry.count !== 0) fail(`entry ${key} does not start counting at zero`);
		// The source only looks at stars for an entry declaring this record, so that a `minStars`
		// rule elsewhere counts every copy. The client never does; stars are always honoured here.
		if ("starsDist" in entry && (!Array.isArray(entry.starsDist) || entry.starsDist.length > 0)) fail(`entry ${key} has a preset star record`);
	}

	const gears = entries.map(([key, entry]): GearEntry => {
		if (!isRecord(entry)) fail(`unknown entry ${key}`);
		const unknown = Object.keys(entry).find((name) => !["count", "starsDist", "byClass", "byNation", "byShip"].includes(name));
		if (unknown !== undefined) fail(`entry ${key} has the unknown key ${unknown}`);
		const byClass = entry.byClass ?? {};
		const byNation = entry.byNation ?? {};
		if (!isRecord(byClass) || !isRecord(byNation)) fail(`entry ${key} has a malformed byClass or byNation`);

		const rules: Rule[] = [];
		const add = (value: Plain, scope: Pick<Rule, "class" | "nation">, where: string) => {
			if (typeof value !== "object") fail(`${where} refers to something that is not a rule`);
			(Array.isArray(value) ? value : [value]).forEach((rule, index) => rules.push(convertRule(rule, scope, `${where}[${index}]`, ctx)));
		};
		for (const [shipClass, value] of Object.entries(byClass)) {
			// Anything but a rule stands for "the same as that class".
			add(typeof value === "object" ? value : (byClass[String(value)] ?? fail(`${key}.byClass.${shipClass} refers to a missing class`)), { class: Number(shipClass) }, `${key}.byClass.${shipClass}`);
		}
		for (const [nation, value] of Object.entries(byNation)) {
			if (!(nation in nations)) fail(`${key}.byNation names the unknown nation ${nation}`);
			// A string stands for another nation, a number for a ship class.
			const target = typeof value === "string" ? byNation[value] : typeof value === "number" ? byClass[value] : value;
			add(target ?? fail(`${key}.byNation.${nation} refers to a missing entry`), { nation }, `${key}.byNation.${nation}`);
		}
		if (entry.byShip !== undefined) add(entry.byShip, {}, `${key}.byShip`);
		return { key, rules };
	});

	for (const [key, correction] of Object.entries(corrections)) {
		if (!/^(\d+(\+\d+)*|t[23]_\d+)$/.test(key)) fail(`correction for the unknown entry ${key}`);
		let gear = gears.find((candidate) => candidate.key === key);
		if (gear === undefined) {
			// Numbered entries come first and in order, as in the source's reader.
			gear = { key, rules: [] };
			const after = gears.findIndex((candidate) => !/^\d+$/.test(candidate.key) || (/^\d+$/.test(key) && Number(candidate.key) > Number(key)));
			gears.splice(after < 0 ? gears.length : after, 0, gear);
		}
		if (correction.replace) gear.rules = [...correction.replace];
		if (correction.append) gear.rules.push(...correction.append);
		// A correction's synergy may consist of `requires` alone.
		for (const rule of gear.rules) for (const synergy of rule.synergy ?? []) synergy.flags ??= [];
	}
	return { source, nations, synergyGears, gears };
}

function repoPath(...segments: string[]): string {
	return resolve(import.meta.dir, "../..", ...segments);
}

/** The pinned source lives in the Rust crate that downloads it; read it from there. */
export function pinnedSource(): GearBonusDocument["source"] {
	const rust = readFileSync(repoPath("crates/emukc_bootstrap/src/kc3kai_source.rs"), "utf8");
	const constant = (name: string) => rust.match(new RegExp(`${name}: &str = "([^"]+)"`))?.[1] ?? fail(`cannot find ${name}`);
	return {
		repo: constant("KC3KAI_SOURCE_REPO"),
		commit: constant("KC3KAI_SOURCE_COMMIT"),
		files: [constant("KC3KAI_GEAR_BONUS_PATH"), constant("KC3KAI_META_PATH")],
	};
}

if (import.meta.main) {
	const source = pinnedSource();
	const [gearBonus, meta] = source.files.map((file) => readFileSync(repoPath(".data/temp/kc3kai", source.commit, file), "utf8")) as [string, string];
	const output = process.argv[2] ?? repoPath("crates/emukc_bootstrap/assets/gear_bonus.json");
	const corrections = JSON.parse(readFileSync(resolve(import.meta.dir, "../gear-bonus-corrections.json"), "utf8")).gears as Corrections;
	const document = convertGearBonus(gearBonus, meta, source, corrections);
	writeFileSync(output, `${JSON.stringify(document, null, 2)}\n`);
	const rules = document.gears.reduce((sum, gear) => sum + gear.rules.length, 0);
	console.log(`converted ${document.gears.length} equipment entries (${rules} rules, ${Object.keys(document.synergyGears).length} synergy counters, ${Object.keys(corrections).length} corrected entries) -> ${output}`);
}
