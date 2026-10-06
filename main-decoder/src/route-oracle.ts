// Checks the converted routing rules against the code they were converted from.
//
// For a deterministic set of fleets it asks three things where each fleet goes from each
// node: the compass simulator's own branch functions (imported straight from the pinned
// source), the neutral rule document evaluated here, and the server's router through
// `route-rules dist`. The first two disagreeing means the parser is wrong; the first and
// third disagreeing means the conversion or the router is.
//
// This runs third-party code and needs the downloaded source, so it is a manually run
// diagnostic, not a test.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { type Expr, loadSourceData, pinnedSource, type RouteRulesDocument, type Rule, type SourceData, type Term } from "./route-rules.ts";

interface ProbeShip {
	id: number;
	stype: number;
	speed: number;
	slow: boolean;
	equips: number[];
}

interface Probe {
	map: string;
	variant: string;
	node: string | null;
	visited: string[];
	fleet: { ships: ProbeShip[]; los_ship: number; los_equip: number };
}

type Distribution = Record<string, number>;

const SAMPLES_PER_NODE = 400;
const EXTRA_ROUNDS = 12;
const TOLERANCE = 0.01;

/** The variant of ours each source phase is compared against; phases left out have none. */
const VARIANTS: Record<string, Record<string, string>> = {
	"7-3": { "1": "pre_p_unlock", "2": "post_p_unlock" },
	"5-6": { "3": "" },
};

function repoPath(...segments: string[]): string {
	return resolve(import.meta.dir, "../..", ...segments);
}

/** Small deterministic generator, so every run probes the same fleets. */
export function mulberry32(seed: number): () => number {
	let state = seed >>> 0;
	return () => {
		state = (state + 0x6d2b79f5) >>> 0;
		let value = Math.imul(state ^ (state >>> 15), 1 | state);
		value = (value + Math.imul(value ^ (value >>> 7), 61 | value)) ^ value;
		return ((value ^ (value >>> 14)) >>> 0) / 4294967296;
	};
}

/** Reading a key the probe never set would silently compare `undefined`; make it loud. */
export function guarded<T extends object>(target: T, label: string): T {
	return new Proxy(target, {
		get(object, key, receiver) {
			if (typeof key === "string" && !(key in object)) throw new Error(`${label}.${key} is not provided by the probe`);
			return Reflect.get(object, key, receiver);
		},
	});
}

function floor2(value: number): number {
	return Math.floor(value * 100) / 100;
}

function speedClass(ships: ProbeShip[]): number {
	return Math.min(...ships.map((ship) => ship.speed)) / 5;
}

function compare(left: number, op: string, right: number): boolean {
	switch (op) {
		case "==":
			return left === right;
		case "!=":
			return left !== right;
		case ">=":
			return left >= right;
		case "<=":
			return left <= right;
		case "<":
			return left < right;
		default:
			return left > right;
	}
}

function countTerm(term: Term, probe: Probe, data: SourceData): number {
	const { ships } = probe.fleet;
	switch (term.kind) {
		case "ship_types":
			return ships.filter((ship) => term.types.includes(data.ships[ship.id]?.type ?? "")).length;
		case "ships":
			return ships.filter((ship) => term.ids.includes(ship.id)).length;
		case "fleet_size":
			return ships.length;
		case "equip_carriers":
			return ships.filter((ship) => ship.equips.some((id) => term.ids.includes(id))).length;
		case "slow_ships":
			return ships.filter((ship) => ship.slow && term.types.includes(data.ships[ship.id]?.type ?? "")).length;
	}
}

function evaluate(expr: Expr, probe: Probe, data: SourceData): boolean {
	if ("and" in expr) return expr.and.every((part) => evaluate(part, probe, data));
	if ("or" in expr) return expr.or.some((part) => evaluate(part, probe, data));
	if ("not" in expr) return !evaluate(expr.not, probe, data);
	if ("cmp" in expr) {
		const sum = expr.cmp.lhs.reduce((total, { coef, term }) => total + coef * countTerm(term, probe, data), 0);
		return compare(sum, expr.cmp.op, expr.cmp.rhs);
	}
	if ("los" in expr) {
		return compare(floor2(probe.fleet.los_ship + expr.los.cn * probe.fleet.los_equip), expr.los.op, expr.los.value);
	}
	if ("speed" in expr) return compare(speedClass(probe.fleet.ships), expr.speed.op, expr.speed.value);
	if ("visited" in expr) return probe.visited.includes(expr.visited) || probe.node === expr.visited;
	const flagship = probe.fleet.ships[0];
	return flagship !== undefined && expr.flagship_types.includes(data.ships[flagship.id]?.type ?? "");
}

function toDistribution(targets: { node: string; rate: number | null }[]): Distribution {
	const distribution: Distribution = {};
	for (const target of targets) distribution[target.node] = (distribution[target.node] ?? 0) + (target.rate ?? 1);
	return distribution;
}

/** The index of the rule the document says fires, or -1 when none does. */
function firingRule(rules: Rule[], probe: Probe, data: SourceData): number {
	return rules.findIndex((rule) => rule.cond === null || evaluate(rule.cond, probe, data));
}

export function sameDistribution(left: Distribution, right: Distribution): boolean {
	const labels = new Set([...Object.keys(left), ...Object.keys(right)]);
	return [...labels].every((label) => Math.abs((left[label] ?? 0) - (right[label] ?? 0)) <= TOLERANCE);
}

function collect(expr: Expr | null, visit: (expr: Expr) => void): void {
	if (expr === null) return;
	visit(expr);
	if ("and" in expr) for (const part of expr.and) collect(part, visit);
	else if ("or" in expr) for (const part of expr.or) collect(part, visit);
	else if ("not" in expr) collect(expr.not, visit);
}

interface Generator {
	/** A fleet for the node; with `target`, one shaped after that rule's own counts. */
	next(target?: Rule): Probe;
}

/**
 * Fleets aimed at one node's rules: ship types and named ships the rules mention are drawn
 * more often, and `LoS` scores land on either side of the node's thresholds.
 */
function probeGenerator(
	area: string,
	variant: string,
	node: string | null,
	rules: Rule[],
	data: SourceData,
	stypes: Record<number, number>,
	equipChoices: number[][],
	random: () => number,
): Generator {
	const mentionedTypes = new Set<string>();
	const mentionedShips = new Set<number>();
	const losPoints: { cn: number; value: number }[] = [];
	const visitedLabels = new Set<string>();
	for (const rule of rules) {
		collect(rule.cond, (expr) => {
			if ("cmp" in expr) {
				for (const { term } of expr.cmp.lhs) {
					if (term.kind === "ship_types" || term.kind === "slow_ships") for (const type of term.types) mentionedTypes.add(type);
					if (term.kind === "ships") for (const id of term.ids.slice(0, 2)) mentionedShips.add(id);
				}
			} else if ("los" in expr) losPoints.push({ cn: expr.los.cn, value: expr.los.value });
			else if ("visited" in expr) visitedLabels.add(expr.visited);
			else if ("flagship_types" in expr) for (const type of expr.flagship_types) mentionedTypes.add(type);
		});
	}

	// A handful of hulls per type, one per base ship so a fleet never holds two forms of
	// the same ship. Some types (航戦, 雷巡, 装甲空母) only exist as remodels.
	const byType = new Map<string, number[]>();
	const basesByType = new Map<string, Set<number>>();
	for (const [id, ship] of Object.entries(data.ships)) {
		const list = byType.get(ship.type) ?? [];
		const bases = basesByType.get(ship.type) ?? new Set<number>();
		if (!bases.has(ship.base) && list.length < 8) {
			list.push(Number(id));
			bases.add(ship.base);
		}
		byType.set(ship.type, list);
		basesByType.set(ship.type, bases);
	}
	const allTypes = [...byType.keys()];
	const focusTypes = mentionedTypes.size > 0 ? [...mentionedTypes] : allTypes;
	const pick = <T>(items: T[]): T => items[Math.floor(random() * items.length)] as T;

	return {
		next(target?: Rule): Probe {
			let size = random() < 0.15 ? 1 + Math.floor(random() * 3) : 4 + Math.floor(random() * 3);
			const ids = new Set<number>();
			const bases = new Set<number>();
			// A fleet cannot hold two forms of one ship.
			const add = (id: number): void => {
				const base = data.ships[id]?.base ?? id;
				if (bases.has(base) || ids.size >= 6) return;
				bases.add(base);
				ids.add(id);
			};
			if (target !== undefined) {
				// Give every count the rule tests a value at or next to its bound, so rules
				// such as `AS === 1 && Ss === 3 && DD === 2` are met on purpose, not by luck.
				size = 0;
				collect(target.cond, (expr) => {
					if (!("cmp" in expr)) return;
					const [first] = expr.cmp.lhs;
					if (expr.cmp.lhs.length !== 1 || first === undefined || first.coef !== 1) return;
					const wanted = Math.max(0, expr.cmp.rhs + (random() < 0.7 ? 0 : pick([-1, 1])));
					const { term } = first;
					for (let count = 0; count < wanted; count += 1) {
						if (term.kind === "ships") add(pick(term.ids));
						else if (term.kind === "ship_types" || term.kind === "slow_ships") add(pick(byType.get(pick(term.types)) ?? [1]));
					}
				});
				size = Math.min(6, ids.size + (random() < 0.5 ? 0 : Math.floor(random() * 3)));
			}
			const fleetFocus = [pick(focusTypes), pick(focusTypes), pick(allTypes)];
			for (let attempts = 0; ids.size < size && attempts < 80; attempts += 1) {
				const roll = random();
				add(roll < 0.15 && mentionedShips.size > 0 ? pick([...mentionedShips]) : pick(byType.get(roll < 0.8 ? pick(fleetFocus) : pick(allTypes)) ?? [1]));
			}
			if (ids.size === 0) add(pick(byType.get(pick(allTypes)) ?? [1]));
			// A fleet is no faster than its slowest hull unless equipment speeds it up.
			const bump = random() < 0.25 ? 1 + Math.floor(random() * 2) : 0;
			const ships = [...ids].map((id): ProbeShip => {
				const slow = slowIds.has(id);
				return { id, stype: stypes[id] ?? 0, speed: Math.min(4, (slow ? 1 : 2) + bump) * 5, slow, equips: random() < 0.3 ? pick(equipChoices) : [] };
			});
			if (random() < 0.3) ships.sort(() => random() - 0.5);

			let los_ship = 20 + Math.floor(random() * 60);
			let los_equip = Math.floor(random() * 10);
			if (losPoints.length > 0) {
				const point = pick(losPoints);
				los_equip = random() < 0.5 ? 0 : 3.25;
				los_ship = point.value + pick([-3, -1, -0.5, 0, 0.5, 2]) - point.cn * los_equip;
			}
			const visited = [...visitedLabels].filter(() => random() < 0.5);
			return { map: area, variant, node, visited, fleet: { ships, los_ship, los_equip } };
		},
	};
}

let slowIds = new Set<number>();

async function main(): Promise<void> {
	const { commit } = pinnedSource();
	const sourceDir = repoPath(".data/temp/x20a_compass", commit);
	const src = (...segments: string[]) => join(sourceDir, "src", ...segments);
	const document = JSON.parse(readFileSync(repoPath(".data/temp/x20a_compass", `${commit}.route_rules.json`), "utf8")) as RouteRulesDocument;
	const data = loadSourceData(sourceDir);

	const { calc_next_node } = (await import(src("core", "branch", "index.ts"))) as {
		calc_next_node: (area: string, node: string | null, fleet: unknown, option: Record<string, string>) => string | { node: string; rate: number }[];
	};
	const { derive_composition } = (await import(src("models", "Composition.ts"))) as { derive_composition: (ships: unknown[]) => object };
	const sourceShips = ((await import(src("data", "ship.ts"))) as { default: Record<number, { name: string; type: number; sg: number; base: number }> }).default;

	// The source's slow hulls (speed group 低速A and below) must be the manifest's `api_soku < 10`,
	// since the server reads the latter.
	const manifest = JSON.parse(readFileSync(repoPath(".data/codex/start2.json"), "utf8")) as { api_mst_ship: { api_id: number; api_stype: number; api_soku?: number }[] };
	const stypes: Record<number, number> = {};
	const speedMismatches: number[] = [];
	slowIds = new Set(Object.entries(sourceShips).filter(([, ship]) => ship.sg >= 5).map(([id]) => Number(id)));
	for (const ship of manifest.api_mst_ship) {
		stypes[ship.api_id] = ship.api_stype;
		if (sourceShips[ship.api_id] !== undefined && slowIds.has(ship.api_id) !== (ship.api_soku ?? 10) < 10) speedMismatches.push(ship.api_id);
	}

	const carrierIds = (name: string): number[] => {
		const field = data.fields[name];
		return field?.kind === "equip_carriers" ? field.ids : [];
	};
	const equipChoices = ["drum_carrier_count", "craft_carrier_count", "radar_carrier_count"].map((name) => carrierIds(name).slice(0, 1));
	equipChoices.push(equipChoices.flat());

	const sourceAnswer = (probe: Probe, phase: string): Distribution | null => {
		const ships = probe.fleet.ships.map((ship) => sourceShips[ship.id] as { name: string; type: number; base: number });
		const carriers = (name: string) => probe.fleet.ships.filter((ship) => ship.equips.some((id) => carrierIds(name).includes(id))).length;
		const { los_ship, los_equip } = probe.fleet;
		const adopt_fleet = guarded(
			{
				fleets: [{ units: ships.map((ship) => ({ ship })) }],
				ship_names: ships.map((ship) => ship.name),
				base_ship_names: ships.map((ship) => sourceShips[ship.base]?.name),
				composition: guarded(derive_composition(ships), "composition"),
				fleet_type: 0,
				ships_length: ships.length,
				speed: speedClass(probe.fleet.ships),
				seek: { c1: floor2(los_ship + los_equip), c2: floor2(los_ship + 2 * los_equip), c3: floor2(los_ship + 3 * los_equip), c4: floor2(los_ship + 4 * los_equip) },
				drum_carrier_count: carriers("drum_carrier_count"),
				radar_carrier_count: carriers("radar_carrier_count"),
				craft_carrier_count: carriers("craft_carrier_count"),
				arBulge_carrier_count: 0,
				SBB_count: probe.fleet.ships.filter((ship) => ship.slow && data.ships[ship.id]?.type === "BB").length,
			},
			"adopt_fleet",
		);
		const start = probe.node === "2" ? "2" : "1";
		const route = probe.node === null ? [null] : [null, ...new Set([start, ...probe.visited, probe.node])];
		const log = console.log;
		console.log = () => {};
		try {
			const answer = calc_next_node(probe.map, probe.node, { adopt_fleet, route }, { phase });
			return typeof answer === "string" ? { [answer]: 1 } : toDistribution(answer);
		} catch (error) {
			// The source throws 「条件漏れ」 for fleets it considers unable to reach the node.
			if (error instanceof Error && error.constructor.name === "OmissionOfConditions") return null;
			throw error;
		} finally {
			console.log = log;
		}
	};

	const probes: Probe[] = [];
	const expected: Distribution[] = [];
	const origins: string[] = [];
	const parserDiffs: string[] = [];
	let unreachable = 0;
	let totalRules = 0;
	let coveredRules = 0;
	const uncovered: string[] = [];
	const random = mulberry32(20261006);

	for (const [area, phases] of Object.entries(document.maps)) {
		for (const [phase, rules] of Object.entries(phases)) {
			const variant = phase === "" ? "" : VARIANTS[area]?.[phase];
			if (variant === undefined) continue;
			const nodes: [string | null, Rule[]][] = [[null, rules.start], ...Object.entries(rules.nodes).filter(([, node]) => !node.active).map(([label, node]): [string, Rule[]] => [label, node.rules])];
			for (const [node, nodeRules] of nodes) {
				if (nodeRules.length === 0) continue;
				const generator = probeGenerator(area, variant, node, nodeRules, data, stypes, equipChoices, random);
				const fired = new Set<number>();
				for (let round = 0; round <= EXTRA_ROUNDS && fired.size < nodeRules.length; round += 1) {
					for (let sample = 0; sample < SAMPLES_PER_NODE; sample += 1) {
						const missing = round === 0 ? [] : nodeRules.filter((_, ruleIndex) => !fired.has(ruleIndex));
						const probe = generator.next(missing.length > 0 ? missing[Math.floor(random() * missing.length)] : undefined);
						const answer = sourceAnswer(probe, phase);
						const index = firingRule(nodeRules, probe, data);
						if (answer === null) {
							unreachable += 1;
							if (index >= 0) parserDiffs.push(`${area} ${node}: the source rejects a fleet the document routes by rule at line ${nodeRules[index]?.line}`);
							continue;
						}
						// After the first round only fleets reaching a new rule are worth keeping.
						if (round > 0 && (index < 0 || fired.has(index))) continue;
						if (index < 0) {
							parserDiffs.push(`${area} ${node}: the source answers ${JSON.stringify(answer)} but no document rule fires`);
							continue;
						}
						fired.add(index);
						const fromDocument = toDistribution(nodeRules[index]?.targets ?? []);
						if (!sameDistribution(answer, fromDocument)) {
							parserDiffs.push(`${area} ${node} line ${nodeRules[index]?.line}: source ${JSON.stringify(answer)}, document ${JSON.stringify(fromDocument)}`);
						}
						probes.push(probe);
						expected.push(answer);
						origins.push(`${area}${phase === "" ? "" : ` phase ${phase}`} ${node ?? "start"} (source line ${nodeRules[index]?.line})`);
					}
				}
				totalRules += nodeRules.length;
				coveredRules += fired.size;
				nodeRules.forEach((rule, index) => {
					if (!fired.has(index)) uncovered.push(`${area}${phase === "" ? "" : ` phase ${phase}`} ${node ?? "start"} line ${rule.line}`);
				});
			}
		}
	}

	const probeFile = repoPath(".data/temp/route_oracle_probes.json");
	mkdirSync(dirname(probeFile), { recursive: true });
	writeFileSync(probeFile, JSON.stringify(probes));
	const router = Bun.spawnSync(["cargo", "run", "-q", "--", "route-rules", "dist", "--input", probeFile], { cwd: repoPath(), stdout: "pipe", stderr: "inherit" });
	if (router.exitCode !== 0) throw new Error(`route-rules dist failed with exit code ${router.exitCode}`);
	const lines = router.stdout.toString().trim().split("\n");
	const actual = JSON.parse(lines[lines.length - 1] ?? "[]") as Distribution[];
	if (actual.length !== probes.length) throw new Error(`asked ${probes.length} probes, got ${actual.length} answers`);

	const routerDiffs: string[] = [];
	actual.forEach((distribution, index) => {
		const want = expected[index] as Distribution;
		if (!sameDistribution(want, distribution)) {
			const probe = probes[index] as Probe;
			const fleet = probe.fleet.ships.map((ship) => `${data.ships[ship.id]?.name}(${data.ships[ship.id]?.type}${ship.slow ? ",slow" : ""}${ship.equips.length > 0 ? `,equips ${ship.equips.join("/")}` : ""})`).join(" ");
			routerDiffs.push(
				`${origins[index]}: source ${JSON.stringify(want)}, router ${JSON.stringify(distribution)}\n  fleet: ${fleet}; speed ${speedClass(probe.fleet.ships)}; LoS ${probe.fleet.los_ship} + cn x ${probe.fleet.los_equip}; visited ${probe.visited.join(",") || "-"}`,
			);
		}
	});

	const report = [
		`# Route oracle report`,
		``,
		`Source commit ${commit}.`,
		``,
		`- probes compared with the router: ${probes.length}`,
		`- rules reached by a probe: ${coveredRules} of ${totalRules}`,
		`- fleets the source rejects as unable to reach the node: ${unreachable}`,
		`- ships whose slow flag differs between the source and the manifest: ${speedMismatches.length}${speedMismatches.length > 0 ? ` (${speedMismatches.join(", ")})` : ""}`,
		`- source vs document (parser) differences: ${parserDiffs.length}`,
		`- source vs router differences: ${routerDiffs.length}`,
		``,
		`## Source vs router`,
		``,
		...(routerDiffs.length > 0 ? routerDiffs.slice(0, 200).map((diff) => `- ${diff}`) : ["None."]),
		``,
		`## Source vs document`,
		``,
		...(parserDiffs.length > 0 ? [...new Set(parserDiffs)].slice(0, 200).map((diff) => `- ${diff}`) : ["None."]),
		``,
		`## Rules no probe reached`,
		``,
		...(uncovered.length > 0 ? uncovered.map((rule) => `- ${rule}`) : ["None."]),
		``,
	].join("\n");
	const reportFile = repoPath(".data/temp/route_oracle_report.md");
	writeFileSync(reportFile, report);
	console.log(report.split("\n").slice(4, 10).join("\n"));
	console.log(`report: ${reportFile}`);
	if (routerDiffs.length > 0 || parserDiffs.length > 0 || speedMismatches.length > 0) process.exit(1);
}

if (import.meta.main) await main();
