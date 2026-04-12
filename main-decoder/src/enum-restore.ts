import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

/**
 * Unique enum values used in KanColle battle system comparisons.
 *
 * Only values that are unambiguous across all known enum categories
 * (attackType, formation, engagement, airState) are included.
 * Low values (0-5) are excluded because they collide across categories.
 * Values 11-14 are excluded because they appear in both attackType and combined formation.
 */
const ENUM_NAMES: ReadonlyMap<number, string> = new Map([
	// Attack types (unique values 6-10)
	[6, "KUBO_CI"],
	[7, "SP_RDJ"],
	[8, "SP_SRD"],
	[9, "SP_SSS"],
	[10, "SP_SUIRAI"],
	// Attack types (named cutins, values ≥ 100 — all unique)
	[100, "NELSON_TOUCH"],
	[101, "NAGATO"],
	[102, "MUTSU"],
	[103, "COLORADO"],
	[104, "KONGO"],
	[105, "RICHELIEU"],
	[106, "QE"],
	// Attack types (special cutins)
	[200, "ZUIUN_CUTIN"],
	[300, "SS_CUTIN_A"],
	[301, "SS_CUTIN_B"],
	[302, "SS_CUTIN_C"],
	[400, "YAMATO_A"],
	[401, "YAMATO_B"],
	[1000, "SP_TYPE4"],
]);

const EQUALITY_OPERATORS = new Set(["==", "===", "!=", "!=="]);

/**
 * Adds trailing inline comments to numeric literals in comparison contexts
 * when the value matches a known enum entry. For instance, the literal 100
 * in a comparison becomes annotated with the attack type name NELSON_TOUCH.
 */
export function annotateEnumLiterals(ast: t.File): number {
	let count = 0;

	traverse(ast, {
		BinaryExpression(path: NodePath<t.BinaryExpression>) {
			if (!EQUALITY_OPERATORS.has(path.node.operator)) {
				return;
			}

			for (const side of [path.node.left, path.node.right] as t.Expression[]) {
				if (!t.isNumericLiteral(side)) {
					continue;
				}

				const name = ENUM_NAMES.get(side.value);
				if (name === undefined) {
					continue;
				}

				t.addComment(side, "trailing", ` ${name}`);
				count += 1;
			}
		},
	});

	return count;
}
