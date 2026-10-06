// Converts the compass simulator's per-map branch functions into a neutral JSON form.
//
// The source is one TypeScript function per map (`src/core/branch/world{N}/{N-M}.ts`): a
// `switch (node)` whose cases are ordered `if (...) return ...` chains. This module only
// translates syntax; turning the vocabulary into ids the server understands happens on the
// Rust side. Anything outside the constructs handled here fails with `file:line` instead of
// being guessed at, so a source upgrade that introduces a new idiom is noticed.

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

import { parse } from "@babel/parser";
import * as t from "@babel/types";

/** One counted quantity on the left-hand side of a comparison. */
export type Term =
	| { kind: "ship_types"; types: string[] }
	| { kind: "ships"; ids: number[] }
	| { kind: "fleet_size" }
	/** Ships carrying at least one equipment with one of the ids. */
	| { kind: "equip_carriers"; ids: number[] }
	/** Ships of the given types whose own (unequipped) speed is slow. */
	| { kind: "slow_ships"; types: string[] };

export type Expr =
	| { and: Expr[] }
	| { or: Expr[] }
	| { not: Expr }
	| { cmp: { lhs: { coef: number; term: Term }[]; op: string; rhs: number } }
	| { los: { cn: number; op: string; value: number } }
	| { speed: { op: string; value: number } }
	| { visited: string }
	| { flagship_types: string[] };

export interface Target {
	node: string;
	rate: number | null;
}

export interface Rule {
	/** `null` when the rule is unconditional. */
	cond: Expr | null;
	targets: Target[];
	line: number;
}

export interface NodeRules {
	/** The player picks the next cell; the source returns `option.<node>`. */
	active: boolean;
	rules: Rule[];
}

export interface PhaseRules {
	start: Rule[];
	nodes: Record<string, NodeRules>;
}

export interface SourceShip {
	name: string;
	type: string;
	base: number;
}

export interface SourceData {
	/** Composition keys counted per ship type, e.g. `DD`. */
	baseTypes: string[];
	/** Derived composition keys expanded to base keys, e.g. `Ds` -> `DD`, `DE`. */
	groups: Record<string, string[]>;
	ships: Record<number, SourceShip>;
	/** Helper functions counting ships by a fixed list of base names. */
	countHelpers: Record<string, string[]>;
	/** Precomputed per-fleet counters such as `drum_carrier_count`, as the term they stand for. */
	fields: Record<string, Term>;
	/** Phase option values per map; maps without a phase option are absent. */
	phases: Record<string, number[]>;
}

const SPEED_HELPERS: Record<string, { op: string; value: number }> = {
	is_fleet_speed_slow: { op: "==", value: 1 },
	is_fleet_speed_fast_or_more: { op: ">=", value: 2 },
	is_fleet_speed_faster_or_more: { op: ">=", value: 3 },
	is_fleet_speed_fastest: { op: "==", value: 4 },
};

/** Source comparison operators and the spelling used in the output. */
const COMPARISONS: Record<string, string> = { "===": "==", "!==": "!=", ">=": ">=", "<=": "<=", "<": "<", ">": ">" };

export class RouteRuleSyntaxError extends Error {}

type Folded = Expr | boolean;

interface Linear {
	terms: Map<string, { coef: number; term: Term }>;
	los: Map<number, number>;
	constant: number;
}

interface Scope {
	file: string;
	data: SourceData;
	phase: number | null;
	functions: Map<string, t.ArrowFunctionExpression>;
	/** Local boolean `const`s, inlined where they are used. */
	locals: Map<string, t.Expression>;
}

function fail(scope: Pick<Scope, "file">, node: t.Node | null | undefined, message: string): never {
	throw new RouteRuleSyntaxError(`${scope.file}:${node?.loc?.start.line ?? "?"}: ${message}`);
}

function and(parts: Folded[]): Folded {
	const kept: Expr[] = [];
	for (const part of parts) {
		if (part === false) return false;
		if (part === true) continue;
		if ("and" in part) kept.push(...part.and);
		else kept.push(part);
	}
	if (kept.length === 0) return true;
	return kept.length === 1 ? (kept[0] as Expr) : { and: kept };
}

function or(parts: Folded[]): Folded {
	const kept: Expr[] = [];
	for (const part of parts) {
		if (part === true) return true;
		if (part === false) continue;
		if ("or" in part) kept.push(...part.or);
		else kept.push(part);
	}
	if (kept.length === 0) return false;
	return kept.length === 1 ? (kept[0] as Expr) : { or: kept };
}

function not(part: Folded): Folded {
	if (typeof part === "boolean") return !part;
	return "not" in part ? part.not : { not: part };
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

function unwrap(node: t.Expression): t.Expression {
	let current = node;
	while (current.type === "ParenthesizedExpression" || current.type === "TSAsExpression" || current.type === "TSNonNullExpression") {
		current = current.expression;
	}
	return current;
}

function stringArgument(scope: Scope, node: t.Node | undefined): string {
	if (node?.type !== "StringLiteral") fail(scope, node, "expected a string literal");
	return node.value;
}

function shipIdsByBaseNames(scope: Scope, node: t.Node, names: string[]): number[] {
	const ids: number[] = [];
	for (const name of names) {
		const bases = Object.entries(scope.data.ships).filter(([id, ship]) => ship.name === name && ship.base === Number(id));
		if (bases.length === 0) fail(scope, node, `unknown base ship ${name}`);
		const baseIds = new Set(bases.map(([id]) => Number(id)));
		for (const [id, ship] of Object.entries(scope.data.ships)) {
			if (baseIds.has(ship.base)) ids.push(Number(id));
		}
	}
	return [...new Set(ids)].sort((a, b) => a - b);
}

function shipIdsByName(scope: Scope, node: t.Node, name: string): number[] {
	const ids = Object.entries(scope.data.ships)
		.filter(([, ship]) => ship.name === name)
		.map(([id]) => Number(id));
	if (ids.length === 0) fail(scope, node, `unknown ship ${name}`);
	return ids.sort((a, b) => a - b);
}

function termKey(term: Term): string {
	return JSON.stringify(term);
}

function addTerm(linear: Linear, coef: number, term: Term): void {
	const key = termKey(term);
	const existing = linear.terms.get(key);
	const next = (existing?.coef ?? 0) + coef;
	if (next === 0) linear.terms.delete(key);
	else linear.terms.set(key, { coef: next, term });
}

function stringArray(scope: Scope, node: t.Node | undefined): string[] {
	if (node?.type !== "ArrayExpression") fail(scope, node, "expected an array of names");
	return node.elements.map((element) => stringArgument(scope, element ?? undefined));
}

/** Reads `a + b - c`-style arithmetic into coefficients over counted quantities. */
function linearize(scope: Scope, raw: t.Expression, sign: number, into: Linear): void {
	const node = unwrap(raw);
	switch (node.type) {
		case "NumericLiteral":
			into.constant += sign * node.value;
			return;
		case "BinaryExpression": {
			if (node.operator !== "+" && node.operator !== "-") break;
			if (node.left.type === "PrivateName") break;
			linearize(scope, node.left, sign, into);
			linearize(scope, node.right, node.operator === "+" ? sign : -sign, into);
			return;
		}
		case "Identifier": {
			const { name } = node;
			if (name === "phase") {
				if (scope.phase === null) fail(scope, node, "phase is used but this map has no phase option");
				into.constant += sign * scope.phase;
				return;
			}
			if (name === "ships_length") {
				addTerm(into, sign, { kind: "fleet_size" });
				return;
			}
			const field = scope.data.fields[name];
			if (field !== undefined) {
				addTerm(into, sign, field);
				return;
			}
			if (scope.data.baseTypes.includes(name)) {
				addTerm(into, sign, { kind: "ship_types", types: [name] });
				return;
			}
			const group = scope.data.groups[name];
			if (group !== undefined) {
				for (const type of group) addTerm(into, sign, { kind: "ship_types", types: [type] });
				return;
			}
			break;
		}
		case "MemberExpression": {
			const object = unwrap(node.object);
			if (object.type === "Identifier" && object.name === "seek" && node.property.type === "Identifier") {
				const match = /^c([1-4])$/.exec(node.property.name);
				if (match !== null) {
					const cn = Number(match[1]);
					into.los.set(cn, (into.los.get(cn) ?? 0) + sign);
					return;
				}
			}
			break;
		}
		case "CallExpression": {
			if (node.callee.type !== "Identifier") break;
			const callee = node.callee.name;
			if (callee === "count_ships_by_base_names") {
				const names = stringArray(scope, node.arguments[0]);
				addTerm(into, sign, { kind: "ships", ids: shipIdsByBaseNames(scope, node, names) });
				return;
			}
			const names = scope.data.countHelpers[callee];
			if (names !== undefined) {
				addTerm(into, sign, { kind: "ships", ids: shipIdsByBaseNames(scope, node, names) });
				return;
			}
			break;
		}
		default:
			break;
	}
	fail(scope, node, `unsupported arithmetic: ${node.type}`);
}

function comparison(scope: Scope, node: t.BinaryExpression): Folded {
	if (node.left.type === "PrivateName") fail(scope, node, "unsupported comparison");
	const linear: Linear = { terms: new Map(), los: new Map(), constant: 0 };
	linearize(scope, node.left, 1, linear);
	linearize(scope, node.right, -1, linear);
	const op = COMPARISONS[node.operator] as string;
	const rhs = 0 - linear.constant;

	if (linear.los.size > 0) {
		const [entry] = [...linear.los.entries()];
		if (linear.los.size !== 1 || linear.terms.size !== 0 || entry === undefined || entry[1] !== 1) {
			fail(scope, node, "a LoS score can only be compared with a constant");
		}
		return { los: { cn: entry[0], op, value: rhs } };
	}
	if (linear.terms.size === 0) return compare(0, op, rhs);

	const lhs = [...linear.terms.values()];
	// Merge plain ship-type terms that share a coefficient: `CL + CT` reads as one count.
	const merged: { coef: number; term: Term }[] = [];
	for (const entry of lhs) {
		const previous = merged.find((other) => other.coef === entry.coef && other.term.kind === "ship_types");
		if (entry.term.kind === "ship_types" && previous !== undefined && previous.term.kind === "ship_types") {
			previous.term.types.push(...entry.term.types);
		} else {
			merged.push({ coef: entry.coef, term: structuredClone(entry.term) });
		}
	}
	return { cmp: { lhs: merged, op, rhs } };
}

function condition(scope: Scope, raw: t.Expression): Folded {
	const node = unwrap(raw);
	switch (node.type) {
		case "LogicalExpression":
			if (node.operator === "&&") return and([condition(scope, node.left), condition(scope, node.right)]);
			if (node.operator === "||") return or([condition(scope, node.left), condition(scope, node.right)]);
			break;
		case "UnaryExpression":
			if (node.operator === "!") return not(condition(scope, node.argument));
			break;
		case "BinaryExpression":
			if (node.operator in COMPARISONS) return comparison(scope, node);
			break;
		case "Identifier": {
			const local = scope.locals.get(node.name);
			if (local !== undefined) return condition(scope, local);
			break;
		}
		case "CallExpression": {
			const { callee } = node;
			if (callee.type === "Identifier") {
				const speed = SPEED_HELPERS[callee.name];
				if (speed !== undefined) return { speed };
				if (callee.name === "includes_base_ship") {
					const ids = shipIdsByBaseNames(scope, node, [stringArgument(scope, node.arguments[0])]);
					return { cmp: { lhs: [{ coef: 1, term: { kind: "ships", ids } }], op: ">=", rhs: 1 } };
				}
				if (callee.name === "includes_ship_name") {
					const ids = shipIdsByName(scope, node, stringArgument(scope, node.arguments[1]));
					return { cmp: { lhs: [{ coef: 1, term: { kind: "ships", ids } }], op: ">=", rhs: 1 } };
				}
				if (callee.name === "is_flagship_CL") return { flagship_types: ["CL"] };
			}
			if (
				callee.type === "MemberExpression" &&
				callee.object.type === "Identifier" &&
				callee.object.name === "route" &&
				callee.property.type === "Identifier" &&
				callee.property.name === "includes"
			) {
				return { visited: stringArgument(scope, node.arguments[0]) };
			}
			break;
		}
		default:
			break;
	}
	fail(scope, node, `unsupported condition: ${node.type}`);
}

function targets(scope: Scope, raw: t.Expression): Target[] | "active" {
	const node = unwrap(raw);
	if (node.type === "StringLiteral") return [{ node: node.value, rate: null }];
	if (node.type === "MemberExpression" && node.object.type === "Identifier" && node.object.name === "option") {
		return "active";
	}
	if (node.type === "ArrayExpression") {
		return node.elements.map((element) => {
			if (element?.type !== "ObjectExpression") fail(scope, element, "expected { node, rate }");
			let target: string | undefined;
			let rate: number | undefined;
			for (const property of element.properties) {
				if (property.type !== "ObjectProperty" || property.key.type !== "Identifier") {
					fail(scope, property, "expected { node, rate }");
				}
				if (property.key.name === "node" && property.value.type === "StringLiteral") target = property.value.value;
				else if (property.key.name === "rate" && property.value.type === "NumericLiteral") rate = property.value.value;
				else fail(scope, property, "expected { node, rate }");
			}
			if (target === undefined || rate === undefined) fail(scope, element, "expected { node, rate }");
			return { node: target, rate };
		});
	}
	fail(scope, node, `unsupported return value: ${node.type}`);
}

interface CaseOutput {
	active: boolean;
	rules: Rule[];
}

/**
 * Flattens a statement list into ordered rules. Returns whether control can no longer reach
 * the statement after the list.
 *
 * An `if` without a matching inner rule falls through to the statements after it, which is
 * exactly what trying the later (lower-priority) rules does, so the outer condition is only
 * carried into the branch itself. The `else` branch additionally needs the negated test.
 */
function flatten(scope: Scope, statements: t.Statement[], guard: Folded, out: CaseOutput): boolean {
	if (guard === false) return false;
	for (const statement of statements) {
		switch (statement.type) {
			case "BlockStatement":
				if (flatten(scope, statement.body, guard, out)) return true;
				break;
			case "IfStatement": {
				const test = condition(scope, statement.test);
				const consequent = statement.consequent.type === "BlockStatement" ? statement.consequent.body : [statement.consequent];
				const thenEnds = flatten(scope, consequent, and([guard, test]), out);
				let elseEnds = false;
				if (statement.alternate != null) {
					const alternate = statement.alternate.type === "BlockStatement" ? statement.alternate.body : [statement.alternate];
					elseEnds = flatten(scope, alternate, and([guard, not(test)]), out);
				}
				if ((test === true && thenEnds) || (test === false && elseEnds) || (thenEnds && elseEnds)) return true;
				break;
			}
			case "ReturnStatement": {
				if (statement.argument == null) fail(scope, statement, "empty return");
				const result = targets(scope, statement.argument);
				if (result === "active") {
					if (guard !== true || out.rules.length > 0) fail(scope, statement, "a player-chosen branch must be unconditional");
					out.active = true;
				} else {
					out.rules.push({ cond: guard === true ? null : guard, targets: result, line: statement.loc?.start.line ?? 0 });
				}
				return true;
			}
			case "BreakStatement":
				return true;
			case "SwitchStatement": {
				// `switch (Ds) { case 0: ... }` nested in a node's case: each arm is an
				// `if (Ds === 0)`, and a `break` only leaves the arm.
				const seen: Folded[] = [];
				for (const arm of statement.cases) {
					if (arm.consequent.length === 0) fail(scope, arm, "case fallthrough is not supported");
					const test =
						arm.test == null
							? not(or(seen))
							: condition(scope, { ...t.binaryExpression("===", statement.discriminant, arm.test), loc: arm.loc });
					seen.push(test);
					flatten(scope, arm.consequent, and([guard, test]), out);
				}
				break;
			}
			case "VariableDeclaration": {
				for (const declaration of statement.declarations) {
					if (statement.kind !== "const" || declaration.id.type !== "Identifier" || declaration.init == null) {
						fail(scope, statement, "unsupported declaration");
					}
					scope.locals.set(declaration.id.name, declaration.init);
				}
				break;
			}
			default:
				fail(scope, statement, `unsupported statement: ${statement.type}`);
		}
	}
	return false;
}

function isCall(node: t.Node | null | undefined, name: string): node is t.CallExpression {
	return node?.type === "CallExpression" && node.callee.type === "Identifier" && node.callee.name === name;
}

function evaluateFunction(scope: Scope, fn: t.ArrowFunctionExpression): PhaseRules {
	if (fn.body.type !== "BlockStatement") fail(scope, fn, "expected a block body");
	const result: PhaseRules = { start: [], nodes: {} };

	for (const statement of fn.body.body) {
		if (statement.type === "VariableDeclaration") {
			for (const declaration of statement.declarations) {
				const init = declaration.init == null ? null : unwrap(declaration.init);
				const fromHelper = isCall(init, "destructuring_assignment_helper");
				const fromOption = init?.type === "Identifier" && init.name === "option";
				const phaseNumber = isCall(init, "Number") && declaration.id.type === "Identifier" && declaration.id.name === "phase";
				if (!fromHelper && !fromOption && !phaseNumber) fail(scope, statement, "unsupported declaration");
			}
			continue;
		}
		if (statement.type === "ExpressionStatement" && isCall(statement.expression, "omission_of_conditions")) continue;
		if (statement.type === "ReturnStatement" && statement.argument != null) {
			// `return phase === 1 ? calc_phase_1(node, sim_fleet) : calc_phase_2(node, sim_fleet);`
			const argument = unwrap(statement.argument);
			if (argument.type !== "ConditionalExpression") fail(scope, statement, "unsupported return");
			const test = condition(scope, argument.test);
			if (typeof test !== "boolean") fail(scope, statement, "a delegating return must depend on the phase only");
			const chosen = unwrap(test ? argument.consequent : argument.alternate);
			if (chosen.type !== "CallExpression" || chosen.callee.type !== "Identifier") fail(scope, statement, "unsupported return");
			const target = scope.functions.get(chosen.callee.name);
			if (target === undefined) fail(scope, statement, `unknown function ${chosen.callee.name}`);
			return evaluateFunction(scope, target);
		}
		if (statement.type !== "SwitchStatement") fail(scope, statement, `unsupported statement: ${statement.type}`);
		if (statement.discriminant.type !== "Identifier" || statement.discriminant.name !== "node") {
			fail(scope, statement, "expected switch (node)");
		}

		for (const switchCase of statement.cases) {
			if (switchCase.consequent.length === 0) fail(scope, switchCase, "case fallthrough is not supported");
			const out: CaseOutput = { active: false, rules: [] };
			scope.locals = new Map();
			flatten(scope, switchCase.consequent, true, out);
			const test = switchCase.test;
			if (test?.type === "NullLiteral") {
				if (out.active) fail(scope, switchCase, "the start cannot be player-chosen");
				result.start = out.rules;
			} else if (test?.type === "StringLiteral") {
				result.nodes[test.value] = out;
			} else {
				fail(scope, switchCase, "expected case null or a node label");
			}
		}
	}
	return result;
}

/** Converts one map's source file. The result is keyed by phase, `""` when the map has none. */
export function parseBranchFile(code: string, file: string, area: string, data: SourceData): Record<string, PhaseRules> {
	const ast = parse(code, { sourceType: "module", plugins: ["typescript"] });
	const functions = new Map<string, t.ArrowFunctionExpression>();
	let exported: t.ArrowFunctionExpression | undefined;

	for (const statement of ast.program.body) {
		const declaration = statement.type === "ExportNamedDeclaration" ? statement.declaration : statement;
		if (declaration?.type !== "VariableDeclaration") continue;
		for (const declarator of declaration.declarations) {
			if (declarator.id.type !== "Identifier" || declarator.init?.type !== "ArrowFunctionExpression") continue;
			functions.set(declarator.id.name, declarator.init);
			if (statement.type === "ExportNamedDeclaration") exported = declarator.init;
		}
	}
	if (exported === undefined) throw new RouteRuleSyntaxError(`${file}: no exported branch function`);

	const phases = data.phases[area];
	const result: Record<string, PhaseRules> = {};
	for (const phase of phases ?? [null]) {
		const scope: Scope = { file, data, phase, functions, locals: new Map() };
		result[phase === null ? "" : String(phase)] = evaluateFunction(scope, exported);
	}
	return result;
}

function requireMatch(source: string, pattern: RegExp, label: string): RegExpExecArray {
	const match = pattern.exec(source);
	if (match === null) throw new RouteRuleSyntaxError(`compass source: cannot find ${label}`);
	return match;
}

/** Reads the vocabulary the branch functions rely on from the source tree itself. */
export function loadSourceData(sourceDir: string): SourceData {
	const read = (...segments: string[]) => readFileSync(join(sourceDir, "src", ...segments), "utf8");

	const composition = read("models", "Composition.ts");
	const initial = requireMatch(composition, /const INITIAL: CompositionBase = \{([^}]*)\}/, "the composition keys")[1] ?? "";
	const baseTypes = [...initial.matchAll(/(\w+):\s*0/g)].map((match) => match[1] as string);
	const utilBody = requireMatch(composition, /const calc_composition_util = [\s\S]*?const util:/, "the composition groups")[0];
	const groups: Record<string, string[]> = {};
	for (const match of utilBody.matchAll(/const (\w+) = (\w+(?:\s*\+\s*\w+)*);/g)) {
		const members = (match[2] as string).split("+").map((member) => member.trim());
		groups[match[1] as string] = members.flatMap((member) => groups[member] ?? [member]);
	}
	for (const [group, members] of Object.entries(groups)) {
		const unknown = members.filter((member) => !baseTypes.includes(member));
		if (unknown.length > 0) throw new RouteRuleSyntaxError(`compass source: group ${group} uses unknown types ${unknown.join(", ")}`);
	}

	const ships: Record<number, SourceShip> = {};
	for (const match of read("data", "ship.ts").matchAll(/^[ ,]*(\d+):\{name:"([^"]+)",type:ST\.(\w+),[^}]*\bbase:(\d+)\}/gm)) {
		ships[Number(match[1])] = { name: match[2] as string, type: match[3] as string, base: Number(match[4]) };
	}
	if (Object.keys(ships).length === 0) throw new RouteRuleSyntaxError("compass source: cannot read the ship table");

	const adoptFleet = read("models", "fleet", "AdoptFleet.ts");
	const nameLists: Record<string, string[]> = {};
	for (const match of adoptFleet.matchAll(/const (\w+_BASE_NAMES): BaseShipName\[\]\s*=\s*\[([^\]]*)\]/g)) {
		nameLists[match[1] as string] = [...(match[2] as string).matchAll(/'([^']+)'/g)].map((name) => name[1] as string);
	}
	const countHelpers: Record<string, string[]> = {};
	for (const match of adoptFleet.matchAll(/export function (count_\w+)\([^)]*\)[^{]*\{\s*return count_ships_by_base_names\(\s*(\w+_BASE_NAMES),/g)) {
		const names = nameLists[match[2] as string];
		if (names !== undefined) countHelpers[match[1] as string] = names;
	}

	// The counters `FleetComponent` and `EquippedShip` precompute. Their definitions are short
	// enough to pin with a pattern each, so a changed definition fails here instead of
	// silently keeping the old meaning.
	const equipIds: Record<string, number> = {};
	const equipSource = read("data", "equip.ts");
	const radarIds: number[] = [];
	for (const match of equipSource.matchAll(/^[ ,]*(\d+):\{[^}]*\btype:EquipType\.(\w+),\s*name:'([^']+)'/gm)) {
		equipIds[match[3] as string] = Number(match[1]);
		// The source files radars under its own two types, which is not the game's
		// equipment type for every one of them, so the ids are what is exact.
		if (match[2] === "RadarS" || match[2] === "RadarL") radarIds.push(Number(match[1]));
	}
	const equipId = (name: string): number => {
		const id = equipIds[name];
		if (id === undefined) throw new RouteRuleSyntaxError(`compass source: unknown equipment ${name}`);
		return id;
	};
	const equippedShip = read("models", "ship", "EquippedShip.ts");
	const craftNames = requireMatch(equippedShip, /ROUTING_CRAFT_NAMES: EquipName\[\]\s*=\s*\[([^\]]*)\]/, "ROUTING_CRAFT_NAMES")[1] ?? "";
	requireMatch(equippedShip, /name === 'ドラム缶\(輸送用\)'\) acc\.drum_count\+\+/, "the drum canister counter");
	requireMatch(equippedShip, /name === '北方迷彩\(\+北方装備\)'\) acc\.has_arBulge = true/, "the arctic bulge flag");
	requireMatch(equippedShip, /\[EquipType\.RadarS, EquipType\.RadarL\]\.includes\(equip\.type\)\) \{\s*acc\.has_radar = true/, "the radar flag");
	if (radarIds.length === 0) throw new RouteRuleSyntaxError("compass source: no radars in the equipment table");
	requireMatch(
		read("models", "fleet", "FleetComponent.ts"),
		/ship\.type === ShipType\.BB\s*&& ship\.speed_group >= SLOW_THRESHOLD\s*\) acc\.SBB_count\+\+/,
		"the slow battleship counter",
	);
	const fields: Record<string, Term> = {
		drum_carrier_count: { kind: "equip_carriers", ids: [equipId("ドラム缶(輸送用)")] },
		arBulge_carrier_count: { kind: "equip_carriers", ids: [equipId("北方迷彩(+北方装備)")] },
		craft_carrier_count: {
			kind: "equip_carriers",
			ids: [...craftNames.matchAll(/'([^']+)'/g)].map((name) => equipId(name[1] as string)).sort((a, b) => a - b),
		},
		radar_carrier_count: { kind: "equip_carriers", ids: radarIds.sort((a, b) => a - b) },
		SBB_count: { kind: "slow_ships", types: ["BB"] },
	};

	const phases: Record<string, number[]> = {};
	const options = read("data", "options.ts");
	for (const match of options.matchAll(/'(\d+-\d+)':\s*\{\s*'phase':\s*\{[\s\S]*?options:\s*\[([\s\S]*?)\]/g)) {
		phases[match[1] as string] = [...(match[2] as string).matchAll(/value:\s*'(\d+)'/g)].map((value) => Number(value[1]));
	}

	return { baseTypes, groups, ships, countHelpers, fields, phases };
}

export interface RouteRulesDocument {
	source: { repo: string; commit: string; license: string; copyright: string };
	maps: Record<string, Record<string, PhaseRules>>;
}

/** Converts every regular map (worlds 1-7) of an unpacked source tree. */
export function convertSource(sourceDir: string, repo: string, commit: string): RouteRulesDocument {
	const data = loadSourceData(sourceDir);
	const license = readFileSync(join(sourceDir, "LICENSE"), "utf8").split("\n");
	const index = readFileSync(join(sourceDir, "src", "core", "branch", "index.ts"), "utf8");
	const areas = [...index.matchAll(/^\s*'([1-7]-\d+)':\s*\{/gm)].map((match) => match[1] as string);
	if (areas.length === 0) throw new RouteRuleSyntaxError("compass source: no regular maps registered");

	const maps: RouteRulesDocument["maps"] = {};
	for (const area of areas) {
		const world = area.split("-")[0] as string;
		const file = join("src", "core", "branch", `world${world}`, `${area}.ts`);
		maps[area] = parseBranchFile(readFileSync(join(sourceDir, file), "utf8"), file, area, data);
	}
	return {
		source: {
			repo,
			commit,
			license: (license[0] ?? "").trim(),
			copyright: (license.find((line) => line.startsWith("Copyright")) ?? "").trim(),
		},
		maps,
	};
}

function repoPath(...segments: string[]): string {
	return resolve(import.meta.dir, "../..", ...segments);
}

/** The pinned source lives in the Rust crate that downloads it; read it from there. */
export function pinnedSource(): { repo: string; commit: string } {
	const rust = readFileSync(repoPath("crates/emukc_bootstrap/src/compass_source.rs"), "utf8");
	return {
		repo: requireMatch(rust, /COMPASS_SOURCE_REPO: &str = "([^"]+)"/, "COMPASS_SOURCE_REPO")[1] as string,
		commit: requireMatch(rust, /COMPASS_SOURCE_COMMIT: &str = "([^"]+)"/, "COMPASS_SOURCE_COMMIT")[1] as string,
	};
}

if (import.meta.main) {
	const { repo, commit } = pinnedSource();
	const sourceDir = repoPath(".data/temp/x20a_compass", commit);
	const output = process.argv[2] ?? repoPath(".data/temp/x20a_compass", `${commit}.route_rules.json`);
	const document = convertSource(sourceDir, repo, commit);

	let conditions = 0;
	let rates = 0;
	for (const phases of Object.values(document.maps)) {
		for (const phase of Object.values(phases)) {
			for (const rule of [...phase.start, ...Object.values(phase.nodes).flatMap((node) => node.rules)]) {
				if (rule.cond !== null) conditions += 1;
				rates += rule.targets.filter((target) => target.rate !== null).length;
			}
		}
	}
	mkdirSync(dirname(output), { recursive: true });
	writeFileSync(output, `${JSON.stringify(document, null, 2)}\n`);
	console.log(`converted ${Object.keys(document.maps).length} maps (${conditions} conditional rules, ${rates} weighted targets) -> ${output}`);
}
