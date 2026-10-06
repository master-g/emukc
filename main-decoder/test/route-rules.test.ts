import { describe, expect, test } from "bun:test";

import { parseBranchFile, RouteRuleSyntaxError, type Expr, type SourceData } from "../src/route-rules.ts";

const DATA: SourceData = {
	baseTypes: ["BB", "BBV", "CV", "CL", "CT", "DD", "DE"],
	groups: { BBs: ["BB", "BBV"], CLE: ["CL", "CT"], Ds: ["DD", "DE"] },
	ships: {
		1: { name: "睦月", type: "DD", base: 1 },
		254: { name: "睦月改", type: "DD", base: 1 },
		2: { name: "如月", type: "DD", base: 2 },
	},
	countHelpers: { count_Mutsuki_class: ["睦月", "如月"] },
	phases: { "9-2": [1, 2] },
};

/** Wraps case bodies in the shape every source file has. */
function source(cases: string, name = "calc_9_1"): string {
	return `
export const ${name} = (node, sim_fleet, option) => {
	const { speed, seek, route, DD, CL, Ds } = destructuring_assignment_helper(sim_fleet);
	const { phase: phase_string } = option;
	const phase = Number(phase_string);
	switch (node) {
${cases}
	}
	omission_of_conditions(node, sim_fleet);
}`;
}

function rules(cases: string, label = "A") {
	const phase = parseBranchFile(source(cases), "9-1.ts", "9-1", DATA)[""];
	return phase?.nodes[label]?.rules.map(({ cond, targets }) => ({ cond, targets }));
}

const count = (types: string[], op: string, rhs: number): Expr => ({
	cmp: { lhs: [{ coef: 1, term: { kind: "ship_types", types } }], op, rhs },
});

describe("parseBranchFile", () => {
	test("an if chain becomes ordered rules ending in an unconditional one", () => {
		expect(rules(`case 'A': if (DD >= 2) { return 'B'; } return 'C';`)).toEqual([
			{ cond: count(["DD"], ">=", 2), targets: [{ node: "B", rate: null }] },
			{ cond: null, targets: [{ node: "C", rate: null }] },
		]);
	});

	test("a weighted return keeps every target on one rule", () => {
		expect(rules(`case 'A': return [{ node: 'B', rate: 0.45 }, { node: 'C', rate: 0.55 }];`)).toEqual([
			{ cond: null, targets: [{ node: "B", rate: 0.45 }, { node: "C", rate: 0.55 }] },
		]);
	});

	test("a nested if carries the outer condition and falls through without negation", () => {
		expect(rules(`case 'A': if (DD >= 2) { if (CL >= 1) { return 'B'; } } return 'C';`)).toEqual([
			{ cond: { and: [count(["DD"], ">=", 2), count(["CL"], ">=", 1)] }, targets: [{ node: "B", rate: null }] },
			{ cond: null, targets: [{ node: "C", rate: null }] },
		]);
	});

	test("an else branch is guarded by the negated test", () => {
		expect(rules(`case 'A': if (DD >= 2) { return 'B'; } else { return 'C'; }`)).toEqual([
			{ cond: count(["DD"], ">=", 2), targets: [{ node: "B", rate: null }] },
			{ cond: { not: count(["DD"], ">=", 2) }, targets: [{ node: "C", rate: null }] },
		]);
	});

	test("sums of ship types merge, groups expand, and operators are normalised", () => {
		expect(rules(`case 'A': if (CL + Ds === 3) { return 'B'; } if (BBs < 2) { return 'C'; } break;`)).toEqual([
			{ cond: count(["CL", "DD", "DE"], "==", 3), targets: [{ node: "B", rate: null }] },
			{ cond: count(["BB", "BBV"], "<", 2), targets: [{ node: "C", rate: null }] },
		]);
	});

	test("subtraction and a variable right-hand side stay on the left as coefficients", () => {
		expect(rules(`case 'A': if (BBs - SBB_count >= 2) { return 'B'; } if (DD === ships_length) { return 'C'; } break;`)).toEqual([
			{
				cond: {
					cmp: {
						lhs: [
							{ coef: 1, term: { kind: "ship_types", types: ["BB", "BBV"] } },
							{ coef: -1, term: { kind: "field", name: "SBB_count" } },
						],
						op: ">=",
						rhs: 2,
					},
				},
				targets: [{ node: "B", rate: null }],
			},
			{
				cond: {
					cmp: {
						lhs: [
							{ coef: 1, term: { kind: "ship_types", types: ["DD"] } },
							{ coef: -1, term: { kind: "fleet_size" } },
						],
						op: "==",
						rhs: 0,
					},
				},
				targets: [{ node: "C", rate: null }],
			},
		]);
	});

	test("LoS, speed, visited cells and named ships have their own atoms", () => {
		const result = rules(`case 'A':
			if (seek.c4 < 45) { return 'B'; }
			if (is_fleet_speed_slow(speed) && !route.includes('D')) { return 'C'; }
			if (includes_base_ship('睦月', base_ship_names)) { return 'D'; }
			if (count_Mutsuki_class(fleet) >= 2) { return 'E'; }
			break;`);
		expect(result?.map((rule) => rule.cond)).toEqual([
			{ los: { cn: 4, op: "<", value: 45 } },
			{ and: [{ speed: { op: "==", value: 1 } }, { not: { visited: "D" } }] },
			{ cmp: { lhs: [{ coef: 1, term: { kind: "ships", ids: [1, 254] } }], op: ">=", rhs: 1 } },
			{ cmp: { lhs: [{ coef: 1, term: { kind: "ships", ids: [1, 2, 254] } }], op: ">=", rhs: 2 } },
		]);
	});

	test("a nested switch reads as one if per arm", () => {
		expect(rules(`case 'A': switch (Ds) { case 0: return 'B'; case 1: if (CL >= 1) { return 'C'; } break; default: return 'D'; } break;`)).toEqual([
			{ cond: count(["DD", "DE"], "==", 0), targets: [{ node: "B", rate: null }] },
			{ cond: { and: [count(["DD", "DE"], "==", 1), count(["CL"], ">=", 1)] }, targets: [{ node: "C", rate: null }] },
			{
				cond: { not: { or: [count(["DD", "DE"], "==", 0), count(["DD", "DE"], "==", 1)] } },
				targets: [{ node: "D", rate: null }],
			},
		]);
	});

	test("a local boolean const is inlined", () => {
		expect(rules(`case 'A': { const flag = DD >= 2 || CL >= 1; if (flag && seek.c1 >= 30) { return 'B'; } return 'C'; }`)?.[0]?.cond).toEqual({
			and: [{ or: [count(["DD"], ">=", 2), count(["CL"], ">=", 1)] }, { los: { cn: 1, op: ">=", value: 30 } }],
		});
	});

	test("the start, a player-chosen branch and an unreachable case are told apart", () => {
		const phase = parseBranchFile(
			source(`case null: if (DD >= 2) { return '1'; } return '2'; case 'A': return option.A; case 'B': break;`),
			"9-1.ts",
			"9-1",
			DATA,
		)[""];
		expect(phase?.start.map((rule) => rule.targets[0]?.node)).toEqual(["1", "2"]);
		expect(phase?.nodes.A).toEqual({ active: true, rules: [] });
		expect(phase?.nodes.B).toEqual({ active: false, rules: [] });
	});

	test("phase conditions fold away, once per phase value", () => {
		const phases = parseBranchFile(
			source(`case 'A': if (phase === 1) { return 'B'; } if (phase < 3 && DD >= 2) { return 'C'; } return 'D';`, "calc_9_2"),
			"9-2.ts",
			"9-2",
			DATA,
		);
		expect(phases["1"]?.nodes.A?.rules.map((rule) => [rule.cond, rule.targets[0]?.node])).toEqual([[null, "B"]]);
		expect(phases["2"]?.nodes.A?.rules.map((rule) => [rule.cond, rule.targets[0]?.node])).toEqual([
			[count(["DD"], ">=", 2), "C"],
			[null, "D"],
		]);
	});

	test("a function delegating by phase resolves to the chosen function", () => {
		const code = `
const calc_phase_1 = (node, sim_fleet) => { switch (node) { case 'A': return 'B'; } }
const calc_phase_2 = (node, sim_fleet) => { switch (node) { case 'A': return 'C'; } }
export const calc_9_2 = (node, sim_fleet, option) => {
	const { phase: phase_string } = option;
	const phase = Number(phase_string);
	return phase === 1 ? calc_phase_1(node, sim_fleet) : calc_phase_2(node, sim_fleet);
}`;
		const phases = parseBranchFile(code, "9-2.ts", "9-2", DATA);
		expect(phases["1"]?.nodes.A?.rules[0]?.targets[0]?.node).toBe("B");
		expect(phases["2"]?.nodes.A?.rules[0]?.targets[0]?.node).toBe("C");
	});

	test.each([
		["a loop", `case 'A': while (DD >= 2) { return 'B'; }`, "unsupported statement: WhileStatement"],
		["a ternary", `case 'A': return DD >= 2 ? 'B' : 'C';`, "unsupported return value: ConditionalExpression"],
		["an unknown name", `case 'A': if (torpedo_count >= 2) { return 'B'; } break;`, "unsupported arithmetic: Identifier"],
		["an unknown ship", `case 'A': if (includes_base_ship('大和', x)) { return 'B'; } break;`, "unknown base ship 大和"],
		["LoS against a count", `case 'A': if (seek.c4 >= DD) { return 'B'; } break;`, "a LoS score can only be compared with a constant"],
		["case fallthrough", `case 'A': case 'B': return 'C';`, "case fallthrough is not supported"],
	])("%s fails with the file and line", (_label, cases, message) => {
		expect(() => parseBranchFile(source(cases), "9-1.ts", "9-1", DATA)).toThrow(RouteRuleSyntaxError);
		expect(() => parseBranchFile(source(cases), "9-1.ts", "9-1", DATA)).toThrow(new RegExp(`^9-1\\.ts:\\d+: ${message}`));
	});
});
