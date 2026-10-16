// Reads the remodel numbers the client hard-codes instead of taking from start2.
//
// `ShipUpgradeModelHolder` works out 開発資材 and 高速建造材 from switch tables keyed by the ship
// before the remodel, with a steel-based fallback for 開発資材. `ShipUpgradeModel` (one
// `api_mst_shipupgrade` row) adds 新型砲熕兵装資材 (75), 改修資材 (4) and 工廠資材 (104) by the
// ship after it. The server charges the same numbers from this asset. Any shape this does not
// recognise is an error, so a client change cannot quietly drop an entry.

import { parse } from "@babel/parser";
import * as t from "@babel/types";

import type { ModuleGraph } from "./types.ts";

/** A count that applies to the remodel into `after`, or only to the one from `before`. */
export interface RemodelTableEntry {
	after: number;
	before?: number;
	count: number;
}

export interface RemodelTables {
	/** 開発資材, keyed by the ship before the remodel. */
	devkit: {
		byShip: Record<string, number>;
		/** Ships outside the table that pay the steel-based amount even when a blueprint is needed. */
		blueprintGroup: number[];
		/** The first tier whose bound the steel cost is below gives the count; `steelTop` otherwise. */
		steelTiers: { below: number; count: number }[];
		steelTop: number;
	};
	/** 高速建造材, keyed by the ship before the remodel; 0 outside the table. */
	buildkit: { byShip: Record<string, number> };
	/** 新型砲熕兵装資材 (useitem 75). */
	artillery: RemodelTableEntry[];
	/** 改修資材 (useitem 4). */
	screw: RemodelTableEntry[];
	/** 工廠資材 (useitem 104). */
	arsenal: RemodelTableEntry[];
}

export interface RemodelTablesAsset extends RemodelTables {
	version: 1;
	scriptVersion: string;
}

function fail(message: string): never {
	throw new Error(`remodel tables: ${message}`);
}

function findNode<T>(root: t.Node, wanted: (node: t.Node) => T | null): T | null {
	const stack: unknown[] = [root];
	while (stack.length > 0) {
		const current = stack.pop();
		if (current === null || typeof current !== "object") continue;
		if (Array.isArray(current)) {
			stack.push(...current);
			continue;
		}
		if (typeof (current as t.Node).type === "string") {
			const found = wanted(current as t.Node);
			if (found !== null) return found;
		}
		for (const [key, child] of Object.entries(current)) {
			if (key !== "loc" && child !== null && typeof child === "object") stack.push(child);
		}
	}
	return null;
}

function moduleExporting(graph: ModuleGraph, name: string): t.Node {
	const module = graph.modules.find((candidate) => candidate.exportNames.includes(name)) ?? fail(`no module exports ${name}`);
	return parse(`(${module.source})`, { sourceType: "script" });
}

function method(root: t.Node, name: string): t.ClassMethod {
	return (
		findNode(root, (node) => (t.isClassMethod(node) && t.isIdentifier(node.key, { name }) ? node : null)) ??
		fail(`no method ${name}`)
	);
}

function number(node: t.Node | null | undefined, what: string): number {
	return t.isNumericLiteral(node) ? node.value : fail(`${what} is not a number`);
}

function onlyReturn(body: t.Statement[], what: string): t.Expression {
	const [statement] = body;
	return body.length === 1 && t.isReturnStatement(statement) && statement.argument ? statement.argument : fail(`${what} is not a single return`);
}

/** `N == <subject>` or `N === <subject>`, either way round: N. */
function comparedNumber(test: t.Node, subject: (node: t.Node) => boolean, what: string): number {
	if (t.isBinaryExpression(test) && (test.operator === "==" || test.operator === "===")) {
		if (t.isNumericLiteral(test.left) && subject(test.right)) return test.left.value;
		if (t.isNumericLiteral(test.right) && subject(test.left as t.Node)) return test.right.value;
	}
	return fail(`${what} does not compare a number`);
}

/** `switch (<subject>) { case a: case b: return <value>; ... default: <rest> }` */
function switchCases<T>(
	statement: t.Statement | undefined,
	subject: (node: t.Node) => boolean,
	value: (returned: t.Expression, id: number) => T,
	what: string,
): { cases: [number, T][]; fallback: t.Statement[] } {
	if (!t.isSwitchStatement(statement) || !subject(statement.discriminant)) fail(`${what} is not a switch on the expected value`);
	const cases: [number, T][] = [];
	let pending: number[] = [];
	let fallback: t.Statement[] | undefined;
	for (const clause of statement.cases) {
		if (clause.test === null || clause.test === undefined) {
			if (pending.length > 0) fail(`${what} falls through into default`);
			fallback = clause.consequent;
			continue;
		}
		pending.push(number(clause.test, `${what} case`));
		if (clause.consequent.length === 0) continue;
		const returned = onlyReturn(clause.consequent, `${what} case ${pending.join(",")}`);
		for (const id of pending) cases.push([id, value(returned, id)]);
		pending = [];
	}
	if (pending.length > 0 || fallback === undefined) fail(`${what} has no default`);
	return { cases, fallback };
}

function isParam(fn: t.ClassMethod, index: number): (node: t.Node) => boolean {
	const param = fn.params[index];
	if (!t.isIdentifier(param)) fail(`parameter ${index} is not a plain name`);
	return (node) => t.isIdentifier(node, { name: param.name });
}

function isThisMember(name: string): (node: t.Node) => boolean {
	return (node) => t.isMemberExpression(node) && t.isThisExpression(node.object) && t.isIdentifier(node.property, { name });
}

/** `0 != blueprint && -1 == this._USE_DEVKIT_GROUP_.indexOf(ship)` */
function isBlueprintOutsideGroup(node: t.Node, blueprint: (node: t.Node) => boolean, ship: (node: t.Node) => boolean): boolean {
	if (!t.isLogicalExpression(node, { operator: "&&" })) return false;
	const needsBlueprint = t.isBinaryExpression(node.left, { operator: "!=" }) && t.isNumericLiteral(node.left.left, { value: 0 }) && blueprint(node.left.right);
	const outside = node.right;
	const outsideGroup =
		t.isBinaryExpression(outside, { operator: "==" }) &&
		t.isUnaryExpression(outside.left, { operator: "-" }) &&
		t.isNumericLiteral(outside.left.argument, { value: 1 }) &&
		t.isCallExpression(outside.right) &&
		t.isMemberExpression(outside.right.callee) &&
		isThisMember("_USE_DEVKIT_GROUP_")(outside.right.callee.object) &&
		t.isIdentifier(outside.right.callee.property, { name: "indexOf" }) &&
		outside.right.arguments.length === 1 &&
		ship(outside.right.arguments[0] as t.Node);
	return needsBlueprint && outsideGroup;
}

function devkit(holder: t.Node): RemodelTables["devkit"] {
	const fn = method(holder, "_getRequiredDevkitNum");
	const [ship, blueprint, steel] = [isParam(fn, 0), isParam(fn, 1), isParam(fn, 2)];
	const { cases, fallback } = switchCases(fn.body.body[0], ship, (returned) => number(returned, "devkit"), "_getRequiredDevkitNum");

	// blueprint && outside group || steel < a ? 0 : steel < b ? x : steel < c ? y : z
	let node: t.Expression = onlyReturn(fallback, "devkit default");
	const steelTiers: { below: number; count: number }[] = [];
	let first = true;
	while (t.isConditionalExpression(node)) {
		let test: t.Node = node.test;
		if (first) {
			if (!t.isLogicalExpression(test, { operator: "||" }) || !isBlueprintOutsideGroup(test.left, blueprint, ship)) fail("devkit default does not start with the blueprint check");
			test = test.right;
			if (number(node.consequent, "devkit blueprint count") !== 0) fail("devkit blueprint check does not give 0");
		}
		if (!t.isBinaryExpression(test, { operator: "<" }) || !steel(test.left as t.Node)) fail("devkit default does not compare steel with <");
		steelTiers.push({ below: number(test.right, "steel bound"), count: number(node.consequent, "devkit tier") });
		node = node.alternate;
		first = false;
	}
	if (first) fail("devkit default is not a steel tier chain");

	const group = findNode(holder, (node) =>
		t.isAssignmentExpression(node) && isThisMember("_USE_DEVKIT_GROUP_")(node.left) && t.isArrayExpression(node.right) ? node.right : null,
	) ?? fail("no _USE_DEVKIT_GROUP_");
	return {
		byShip: Object.fromEntries(cases.map(([id, count]) => [String(id), count])),
		blueprintGroup: group.elements.map((element) => number(element, "_USE_DEVKIT_GROUP_ entry")),
		steelTiers,
		steelTop: number(node, "devkit top tier"),
	};
}

function buildkit(holder: t.Node): RemodelTables["buildkit"] {
	const fn = method(holder, "_getRequiredBuildKitNum");
	const { cases, fallback } = switchCases(fn.body.body[0], isParam(fn, 0), (returned) => number(returned, "buildkit"), "_getRequiredBuildKitNum");
	if (number(onlyReturn(fallback, "buildkit default"), "buildkit default") !== 0) fail("buildkit default is not 0");
	return { byShip: Object.fromEntries(cases.map(([id, count]) => [String(id), count])) };
}

/** A count that is either a number or `N == this.mst_id_before ? count : 0`. */
function entryValue(returned: t.Expression, after: number, what: string): RemodelTableEntry {
	if (t.isConditionalExpression(returned)) {
		if (number(returned.alternate, what) !== 0) fail(`${what} for ${after} does not fall back to 0`);
		const before = comparedNumber(returned.test, isThisMember("mst_id_before"), what);
		return { after, before, count: number(returned.consequent, what) };
	}
	return { after, count: number(returned, what) };
}

/** A getter on the row that gives a count by the ship after the remodel, 0 otherwise. */
function byShipAfter(model: t.Node, name: string): RemodelTableEntry[] {
	const getter = method(model, name);
	const after = isThisMember("mst_id_after");
	const [statement] = getter.body.body;
	if (t.isReturnStatement(statement) && t.isConditionalExpression(statement.argument)) {
		const returned = statement.argument;
		if (number(returned.alternate, name) !== 0) fail(`${name} does not fall back to 0`);
		return [{ after: comparedNumber(returned.test, after, name), count: number(returned.consequent, name) }];
	}
	const { cases, fallback } = switchCases(statement, after, (returned, id) => entryValue(returned, id, name), name);
	if (number(onlyReturn(fallback, `${name} default`), `${name} default`) !== 0) fail(`${name} default is not 0`);
	return cases.map(([, entry]) => entry).filter((entry) => entry.count !== 0);
}

export function extractRemodelTables(graph: ModuleGraph): RemodelTables {
	const holder = moduleExporting(graph, "ShipUpgradeModelHolder");
	const model = moduleExporting(graph, "ShipUpgradeModel");
	return {
		devkit: devkit(holder),
		buildkit: buildkit(holder),
		artillery: byShipAfter(model, "newhokohesosizai"),
		screw: byShipAfter(model, "revkit"),
		arsenal: byShipAfter(model, "arsenalmaterial"),
	};
}

/** No timestamp: decoding the same client twice gives the same file. */
export function toRemodelTablesAsset(scriptVersion: string, tables: RemodelTables): RemodelTablesAsset {
	return { version: 1, scriptVersion, ...tables };
}
