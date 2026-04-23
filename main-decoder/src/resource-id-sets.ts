import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type {
	ModuleArtifact,
	ModuleGraph,
	ResourceCoverageMode,
	ResourceIdSetEntry,
	ResourceIdSetsAsset,
} from "./types.ts";

type ShipIdSetKey =
	| "specialShips"
	| "spRemodelShips"
	| "spRemodelMessageShips"
	| "cardRoundShips"
	| "rewardShips";

type SlotitemIdSetKey = "btxtFlatIds" | "itemUpIds";

interface MutableIdSetEntry {
	coverageMode: ResourceCoverageMode;
	ids: Set<number>;
	moduleIds: Set<string>;
	moduleNames: Set<string>;
}

interface ExtractedResourceIdSets {
	shipIdSets: Record<ShipIdSetKey, MutableIdSetEntry>;
	slotitemIdSets: Record<SlotitemIdSetKey, MutableIdSetEntry>;
}

const SHIP_TARGET_TO_KEY: Partial<Record<string, ShipIdSetKey>> = {
	special: "specialShips",
	"sp_remodel/full_x2": "spRemodelShips",
	"sp_remodel/silhouette": "spRemodelShips",
	"sp_remodel/text_class": "spRemodelShips",
	"sp_remodel/text_name": "spRemodelShips",
	"sp_remodel/text_remodel_mes": "spRemodelMessageShips",
	card_round: "cardRoundShips",
	icon_box: "cardRoundShips",
	reward_card: "rewardShips",
	reward_icon: "rewardShips",
};

const SLOT_TARGET_TO_KEY: Partial<Record<string, SlotitemIdSetKey>> = {
	btxt_flat: "btxtFlatIds",
	item_up: "itemUpIds",
};

function createIdSetEntry(): MutableIdSetEntry {
	return {
		coverageMode: "unresolved",
		ids: new Set(),
		moduleIds: new Set(),
		moduleNames: new Set(),
	};
}

function parseFactorySource(source: string): t.File {
	return parse(`(${source});`, {
		sourceType: "script",
		allowReturnOutsideFunction: true,
	});
}

function expressionToSource(node: t.Node | null | undefined): string | undefined {
	if (node == null) {
		return undefined;
	}
	if (t.isIdentifier(node)) {
		return node.name;
	}
	if (t.isThisExpression(node)) {
		return "this";
	}
	if (t.isStringLiteral(node)) {
		return node.value;
	}
	if (t.isNumericLiteral(node)) {
		return String(node.value);
	}
	if (t.isMemberExpression(node)) {
		const objectSource = t.isMemberExpression(node.object)
			? expressionToSource(node.object)
			: expressionToSource(node.object);
		if (objectSource === undefined) {
			return undefined;
		}
		if (t.isIdentifier(node.property) && !node.computed) {
			return `${objectSource}.${node.property.name}`;
		}
		if (t.isStringLiteral(node.property)) {
			return `${objectSource}.${node.property.value}`;
		}
		if (t.isNumericLiteral(node.property)) {
			return `${objectSource}[${node.property.value}]`;
		}
	}
	return undefined;
}

function isShipIdLikeSource(source: string | undefined): boolean {
	return source !== undefined && (source.endsWith(".mst_id") || source.endsWith(".mstID"));
}

function isNumericLiteralLike(expr: t.Expression | t.PrivateName | t.V8IntrinsicIdentifier | null | undefined): expr is t.NumericLiteral {
	return expr != null && t.isNumericLiteral(expr);
}

function addObservedId(entry: MutableIdSetEntry, id: number, module: ModuleArtifact): void {
	if (!Number.isInteger(id) || id <= 0) {
		return;
	}
	entry.ids.add(id);
	entry.moduleIds.add(module.id);
	entry.moduleNames.add(module.readableName ?? module.fileName);
	entry.coverageMode = "partial";
}

function finalizeEntry(entry: MutableIdSetEntry): ResourceIdSetEntry {
	return {
		coverageMode: entry.coverageMode,
		ids: [...entry.ids].sort((left, right) => left - right),
		moduleIds: [...entry.moduleIds].sort(),
		moduleNames: [...entry.moduleNames].sort(),
	};
}

function collectComparisonNumbers(
	expression: t.Expression,
	trackedShipIds: Set<string>,
	trackedSlotIds: Set<string>,
): { shipIds: number[]; slotIds: number[] } {
	if (t.isLogicalExpression(expression)) {
		const left = collectComparisonNumbers(expression.left, trackedShipIds, trackedSlotIds);
		const right = collectComparisonNumbers(expression.right, trackedShipIds, trackedSlotIds);
		return {
			shipIds: [...left.shipIds, ...right.shipIds],
			slotIds: [...left.slotIds, ...right.slotIds],
		};
	}

	if (!t.isBinaryExpression(expression)) {
		return { shipIds: [], slotIds: [] };
	}

	const leftSource = expressionToSource(expression.left);
	const rightSource = expressionToSource(expression.right);
	const leftIsShip = leftSource !== undefined && trackedShipIds.has(leftSource);
	const rightIsShip = rightSource !== undefined && trackedShipIds.has(rightSource);
	const leftIsSlot = leftSource !== undefined && trackedSlotIds.has(leftSource);
	const rightIsSlot = rightSource !== undefined && trackedSlotIds.has(rightSource);

	if ((leftIsShip || rightIsShip) && isNumericLiteralLike(leftIsShip ? expression.right : expression.left)) {
		return { shipIds: [(leftIsShip ? expression.right : expression.left).value], slotIds: [] };
	}

	if ((leftIsSlot || rightIsSlot) && isNumericLiteralLike(leftIsSlot ? expression.right : expression.left)) {
		return { shipIds: [], slotIds: [(leftIsSlot ? expression.right : expression.left).value] };
	}

	return { shipIds: [], slotIds: [] };
}

function isShipCallForTarget(path: NodePath<t.CallExpression>, targetType: string): boolean {
	const { node } = path;
	const calleeSource = expressionToSource(node.callee);
	if (calleeSource === undefined) {
		return false;
	}
	if (!calleeSource.endsWith("resources.getShip") && !calleeSource.endsWith("ShipLoader.add")) {
		return false;
	}
	const typeArg = node.arguments[2];
	return typeArg !== undefined && !t.isSpreadElement(typeArg) && t.isStringLiteral(typeArg, { value: targetType });
}

function isSlotCallForTarget(path: NodePath<t.CallExpression>, targetType: string): boolean {
	const { node } = path;
	const calleeSource = expressionToSource(node.callee);
	if (calleeSource === undefined) {
		return false;
	}
	if (!calleeSource.endsWith("resources.getSlotitem") && !calleeSource.endsWith("SlotLoader.add")) {
		return false;
	}
	const typeArg = node.arguments[1];
	return typeArg !== undefined && !t.isSpreadElement(typeArg) && t.isStringLiteral(typeArg, { value: targetType });
}

export function extractResourceIdSets(moduleGraph: ModuleGraph): ExtractedResourceIdSets {
	const extracted: ExtractedResourceIdSets = {
		shipIdSets: {
			specialShips: createIdSetEntry(),
			spRemodelShips: createIdSetEntry(),
			spRemodelMessageShips: createIdSetEntry(),
			cardRoundShips: createIdSetEntry(),
			rewardShips: createIdSetEntry(),
		},
		slotitemIdSets: {
			btxtFlatIds: createIdSetEntry(),
			itemUpIds: createIdSetEntry(),
		},
	};

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (
			!source.includes("special")
			&& !source.includes("sp_remodel")
			&& !source.includes("card_round")
			&& !source.includes("reward_")
			&& !source.includes("icon_box")
			&& !source.includes("btxt_flat")
			&& !source.includes("item_up")
		) {
			continue;
		}

		let ast: t.File;
		try {
			ast = parseFactorySource(source);
		} catch {
			continue;
		}

		const numericBindings = new Map<string, number>();
		const shipIdAliases = new Set<string>();
		const slotIdAliases = new Set<string>();

		traverse(ast, {
			VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
				if (!t.isIdentifier(path.node.id) || path.node.init == null) {
					return;
				}
				const name = path.node.id.name;
				if (t.isNumericLiteral(path.node.init)) {
					numericBindings.set(name, path.node.init.value);
					return;
				}
				const sourceText = expressionToSource(path.node.init);
				if (isShipIdLikeSource(sourceText) || (sourceText !== undefined && shipIdAliases.has(sourceText))) {
					shipIdAliases.add(name);
					return;
				}
				if (
					sourceText !== undefined
					&& (sourceText.endsWith(".mst_id")
						|| sourceText.endsWith(".mstID")
						|| sourceText.includes("_slot_mst_id")
						|| sourceText.includes("_plane")
						|| sourceText === "slotitemMstID")
				) {
					slotIdAliases.add(name);
				}
			},
		});

		traverse(ast, {
			CallExpression(path: NodePath<t.CallExpression>) {
				const shipTypeArg = path.node.arguments[2];
				if (shipTypeArg !== undefined && !t.isSpreadElement(shipTypeArg) && t.isStringLiteral(shipTypeArg)) {
					const shipSetKey = SHIP_TARGET_TO_KEY[shipTypeArg.value];
					if (shipSetKey !== undefined) {
						const shipEntry = extracted.shipIdSets[shipSetKey];
						const idArg = path.node.arguments[0];
						if (idArg !== undefined && !t.isSpreadElement(idArg)) {
							if (t.isNumericLiteral(idArg)) {
								addObservedId(shipEntry, idArg.value, module);
							} else if (t.isIdentifier(idArg)) {
								const resolved = numericBindings.get(idArg.name);
								if (resolved !== undefined) {
									addObservedId(shipEntry, resolved, module);
								}
							}
						}

						const ifParent = path.findParent(parent => parent.isIfStatement());
						if (ifParent?.isIfStatement()) {
							const { shipIds } = collectComparisonNumbers(ifParent.node.test, shipIdAliases, slotIdAliases);
							for (const id of shipIds) {
								addObservedId(shipEntry, id, module);
							}
						}

						const switchParent = path.findParent(parent => parent.isSwitchCase());
						if (switchParent?.isSwitchCase() && t.isNumericLiteral(switchParent.node.test)) {
							addObservedId(shipEntry, switchParent.node.test.value, module);
						}
					}
				}

				const slotTypeArg = path.node.arguments[1];
				if (slotTypeArg !== undefined && !t.isSpreadElement(slotTypeArg) && t.isStringLiteral(slotTypeArg)) {
					const slotSetKey = SLOT_TARGET_TO_KEY[slotTypeArg.value];
					if (slotSetKey !== undefined) {
						const slotEntry = extracted.slotitemIdSets[slotSetKey];
						const idArg = path.node.arguments[0];
						if (idArg !== undefined && !t.isSpreadElement(idArg)) {
							if (t.isNumericLiteral(idArg)) {
								addObservedId(slotEntry, idArg.value, module);
							} else if (t.isIdentifier(idArg)) {
								const resolved = numericBindings.get(idArg.name);
								if (resolved !== undefined) {
									addObservedId(slotEntry, resolved, module);
								}
							}
						}

						const ifParent = path.findParent(parent => parent.isIfStatement());
						if (ifParent?.isIfStatement()) {
							const { slotIds } = collectComparisonNumbers(ifParent.node.test, shipIdAliases, slotIdAliases);
							for (const id of slotIds) {
								addObservedId(slotEntry, id, module);
							}
						}
					}
				}
			},
		});
	}

	return extracted;
}

export function toResourceIdSetsAsset(
	scriptVersion: string,
	extracted: ExtractedResourceIdSets,
): ResourceIdSetsAsset {
	const shipIdSets = {
		specialShips: finalizeEntry(extracted.shipIdSets.specialShips),
		spRemodelShips: finalizeEntry(extracted.shipIdSets.spRemodelShips),
		spRemodelMessageShips: finalizeEntry(extracted.shipIdSets.spRemodelMessageShips),
		cardRoundShips: finalizeEntry(extracted.shipIdSets.cardRoundShips),
		rewardShips: finalizeEntry(extracted.shipIdSets.rewardShips),
	};
	const slotitemIdSets = {
		btxtFlatIds: finalizeEntry(extracted.slotitemIdSets.btxtFlatIds),
		itemUpIds: finalizeEntry(extracted.slotitemIdSets.itemUpIds),
	};
	const allEntries = [...Object.values(shipIdSets), ...Object.values(slotitemIdSets)];
	const unresolvedKeys = [
		...Object.entries(shipIdSets).filter(([, entry]) => entry.coverageMode === "unresolved").map(([key]) => key),
		...Object.entries(slotitemIdSets).filter(([, entry]) => entry.coverageMode === "unresolved").map(([key]) => key),
	];

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		coverageMode: "mainjs-observed",
		summary: {
			shipCategoryCount: Object.keys(shipIdSets).length,
			slotitemCategoryCount: Object.keys(slotitemIdSets).length,
			resolvedCategoryCount: allEntries.filter(entry => entry.coverageMode !== "unresolved").length,
			unresolvedCategoryCount: unresolvedKeys.length,
		},
		shipIdSets,
		slotitemIdSets,
		unresolvedKeys,
	};
}
