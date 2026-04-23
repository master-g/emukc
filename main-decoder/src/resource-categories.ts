import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type {
	ModuleArtifact,
	ModuleGraph,
	ResourceCategoriesAsset,
	ResourceCategoryEntry,
} from "./types.ts";

type CategorySource =
	| "resources.getShip"
	| "ShipLoader.add"
	| "resources.getSlotitem"
	| "SlotLoader.add"
	| "explicit-path";

type CategoryMap = Map<string, ResourceCategoryEntry>;

interface ExtractedResourceCategories {
	shipTargetTypes: ResourceCategoryEntry[];
	slotTargetTypes: ResourceCategoryEntry[];
	shipGenerationGroups: ResourceCategoriesAsset["shipGenerationGroups"];
	slotGenerationGroups: ResourceCategoriesAsset["slotGenerationGroups"];
	spRemodelSubcategories: string[];
}

const SHIP_GENERATION_GROUP_CANDIDATES: ResourceCategoriesAsset["shipGenerationGroups"] = {
	defaultFriendly: [
		"album_status",
		"banner",
		"banner2",
		"banner2_dmg",
		"banner2_g_dmg",
		"banner_dmg",
		"banner_g_dmg",
		"card",
		"card_dmg",
		"power_up",
		"remodel",
		"remodel_dmg",
		"supply_character",
		"supply_character_dmg",
	],
	defaultAbyssal: ["banner", "banner3", "banner3_g_dmg"],
	friendGraph: ["full", "full_dmg", "character_full", "character_full_dmg", "character_up", "character_up_dmg"],
	enemyGraph: ["full", "full_dmg"],
};

const SLOT_GENERATION_GROUP_CANDIDATES: ResourceCategoriesAsset["slotGenerationGroups"] = {
	default: ["card", "card_t", "item_on", "item_up", "remodel", "statustop_item"],
	baga: ["card", "card_t", "item_on", "remodel", "statustop_item"],
	airunit: ["airunit_banner", "airunit_fairy", "airunit_name"],
};

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
	if (t.isBooleanLiteral(node)) {
		return String(node.value);
	}
	if (t.isMemberExpression(node)) {
		return memberExpressionToString(node);
	}

	return undefined;
}

function memberExpressionToString(node: t.MemberExpression): string | undefined {
	const objectSource = t.isMemberExpression(node.object)
		? memberExpressionToString(node.object)
		: expressionToSource(node.object);
	if (objectSource === undefined) {
		return undefined;
	}

	let propertySource: string | undefined;
	if (t.isIdentifier(node.property) && !node.computed) {
		propertySource = node.property.name;
	} else if (t.isStringLiteral(node.property)) {
		propertySource = node.property.value;
	} else if (t.isNumericLiteral(node.property)) {
		propertySource = String(node.property.value);
	} else {
		propertySource = expressionToSource(node.property);
	}

	if (propertySource === undefined) {
		return undefined;
	}

	return node.computed && !t.isIdentifier(node.property)
		? `${objectSource}[${propertySource}]`
		: `${objectSource}.${propertySource}`;
}

function getCallExpressionChain(node: t.Expression | t.V8IntrinsicIdentifier): string | undefined {
	if (!t.isMemberExpression(node)) {
		return undefined;
	}

	return memberExpressionToString(node);
}

function addCategory(
	target: CategoryMap,
	source: CategorySource,
	targetType: string,
	module: ModuleArtifact,
): void {
	const moduleName = module.readableName ?? module.fileName;
	const key = `${source}:${targetType}`;
	const existing = target.get(key);
	if (existing !== undefined) {
		if (!existing.moduleIds.includes(module.id)) {
			existing.moduleIds.push(module.id);
		}
		if (!existing.moduleNames.includes(moduleName)) {
			existing.moduleNames.push(moduleName);
		}
		return;
	}

	target.set(key, {
		source,
		targetType,
		moduleIds: [module.id],
		moduleNames: [moduleName],
	});
}

function sortEntries(entries: CategoryMap): ResourceCategoryEntry[] {
	return [...entries.values()].map(entry => ({
		...entry,
		moduleIds: [...entry.moduleIds].sort(),
		moduleNames: [...entry.moduleNames].sort(),
	})).sort((left, right) => {
		const typeCompare = left.targetType.localeCompare(right.targetType);
		return typeCompare !== 0 ? typeCompare : left.source.localeCompare(right.source);
	});
}

function extractExplicitShipCategory(pathTail: string): string | undefined {
	const segments = pathTail.split("/").filter(Boolean);
	if (segments.length === 0) {
		return undefined;
	}

	if (segments[0] === "sp_remodel") {
		return segments[1] !== undefined ? `sp_remodel/${segments[1]}` : undefined;
	}

	return segments[0];
}

function extractExplicitSlotCategory(pathTail: string): string | undefined {
	const segments = pathTail.split("/").filter(Boolean);
	return segments[0];
}

function collectExplicitPathCategories(
	module: ModuleArtifact,
	shipCategories: CategoryMap,
	slotCategories: CategoryMap,
): void {
	for (const match of module.source.matchAll(/(?:kcs2\/)?resources\/ship\/([A-Za-z0-9_./-]+)/g)) {
		const category = extractExplicitShipCategory(match[1] ?? "");
		if (category !== undefined) {
			addCategory(shipCategories, "explicit-path", category, module);
		}
	}

	for (const match of module.source.matchAll(/(?:kcs2\/)?resources\/slot\/([A-Za-z0-9_./-]+)/g)) {
		const category = extractExplicitSlotCategory(match[1] ?? "");
		if (category !== undefined) {
			addCategory(slotCategories, "explicit-path", category, module);
		}
	}
}

function buildShipGenerationGroups(shipTargetTypes: Set<string>): ResourceCategoriesAsset["shipGenerationGroups"] {
	return {
		defaultFriendly: SHIP_GENERATION_GROUP_CANDIDATES.defaultFriendly.filter(targetType => shipTargetTypes.has(targetType)),
		defaultAbyssal: SHIP_GENERATION_GROUP_CANDIDATES.defaultAbyssal.filter(targetType => shipTargetTypes.has(targetType)),
		friendGraph: SHIP_GENERATION_GROUP_CANDIDATES.friendGraph.filter(targetType => shipTargetTypes.has(targetType)),
		enemyGraph: SHIP_GENERATION_GROUP_CANDIDATES.enemyGraph.filter(targetType => shipTargetTypes.has(targetType)),
	};
}

function buildSlotGenerationGroups(slotTargetTypes: Set<string>): ResourceCategoriesAsset["slotGenerationGroups"] {
	return {
		default: SLOT_GENERATION_GROUP_CANDIDATES.default.filter(targetType => slotTargetTypes.has(targetType)),
		baga: SLOT_GENERATION_GROUP_CANDIDATES.baga.filter(targetType => slotTargetTypes.has(targetType)),
		airunit: SLOT_GENERATION_GROUP_CANDIDATES.airunit.filter(targetType => slotTargetTypes.has(targetType)),
	};
}

function normalizeShipCategoryTargets(
	rawTargetType: string,
	damagedArg: t.CallExpression["arguments"][1] | undefined,
): string[] {
	if (rawTargetType.startsWith("sp_remodel/")) {
		return [rawTargetType];
	}

	if (rawTargetType === "album_status") {
		return [rawTargetType];
	}

	const forcedDamaged = rawTargetType === "banner_g" || rawTargetType === "banner2_g" || rawTargetType === "banner3_g";
	if (forcedDamaged) {
		return [`${rawTargetType}_dmg`];
	}

	if (damagedArg !== undefined && !t.isSpreadElement(damagedArg) && t.isBooleanLiteral(damagedArg)) {
		return [damagedArg.value ? `${rawTargetType}_dmg` : rawTargetType];
	}

	return [rawTargetType, `${rawTargetType}_dmg`];
}

export function extractResourceCategories(moduleGraph: ModuleGraph): ExtractedResourceCategories {
	const shipCategories: CategoryMap = new Map();
	const slotCategories: CategoryMap = new Map();

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (typeof source !== "string") {
			continue;
		}

		if (
			!source.includes("getShip")
			&& !source.includes("getSlotitem")
			&& !source.includes("ShipLoader")
			&& !source.includes("SlotLoader")
			&& !source.includes("TaskLoadShipResource")
			&& !source.includes("TaskLoadSlotResource")
			&& !source.includes("resources/ship/")
			&& !source.includes("resources/slot/")
			&& !source.includes("kcs2/resources/ship/")
			&& !source.includes("kcs2/resources/slot/")
		) {
			continue;
		}

		collectExplicitPathCategories(module, shipCategories, slotCategories);

		let ast: t.File;
		try {
			ast = parseFactorySource(source);
		} catch {
			continue;
		}

		const shipLoaderBindings = new Set<string>();
		const slotLoaderBindings = new Set<string>();
		const stringBindings = new Map<string, string>();

		traverse(ast, {
			VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
				if (!t.isIdentifier(path.node.id) || path.node.init == null || !t.isNewExpression(path.node.init)) {
					if (t.isIdentifier(path.node.id) && t.isStringLiteral(path.node.init)) {
						stringBindings.set(path.node.id.name, path.node.init.value);
					}
					return;
				}

				const callee = path.node.init.callee;
				const calleeName = getCallExpressionChain(callee)
					?? (t.isIdentifier(callee) ? callee.name : undefined);
				if (calleeName === undefined) {
					return;
				}

				if (calleeName.endsWith("ShipLoader")) {
					shipLoaderBindings.add(path.node.id.name);
				}
				if (calleeName.endsWith("SlotLoader")) {
					slotLoaderBindings.add(path.node.id.name);
				}
			},
		});

		traverse(ast, {
			CallExpression(path: NodePath<t.CallExpression>) {
				const calleeChain = getCallExpressionChain(path.node.callee);
				const memberCallee = t.isMemberExpression(path.node.callee) ? path.node.callee : undefined;
				const isAliasedLoaderAdd = memberCallee !== undefined
					&& t.isIdentifier(memberCallee.object)
					&& t.isIdentifier(memberCallee.property, { name: "add" });
				const loaderAliasName = isAliasedLoaderAdd && memberCallee !== undefined && t.isIdentifier(memberCallee.object)
					? memberCallee.object.name
					: undefined;

				let normalizedCalleeChain = calleeChain;
				if (normalizedCalleeChain !== undefined && isAliasedLoaderAdd) {
					if (shipLoaderBindings.has(loaderAliasName ?? "")) {
						normalizedCalleeChain = "ShipLoader.add";
					} else if (slotLoaderBindings.has(loaderAliasName ?? "")) {
						normalizedCalleeChain = "SlotLoader.add";
					}
				}
				if (normalizedCalleeChain === undefined) {
					normalizedCalleeChain = shipLoaderBindings.has(loaderAliasName ?? "")
						? "ShipLoader.add"
						: slotLoaderBindings.has(loaderAliasName ?? "")
							? "SlotLoader.add"
							: undefined;
				}
				if (normalizedCalleeChain === undefined) {
					return;
				}

				if (normalizedCalleeChain.endsWith("resources.getShip") || normalizedCalleeChain.endsWith("ShipLoader.add")) {
					const damagedArg = path.node.arguments[1];
					const typeArg = path.node.arguments[2];
					const targetType = typeArg !== undefined && !t.isSpreadElement(typeArg)
						? t.isStringLiteral(typeArg)
							? typeArg.value
							: t.isIdentifier(typeArg)
								? stringBindings.get(typeArg.name)
								: undefined
						: undefined;
					if (targetType !== undefined) {
						for (const normalizedTargetType of normalizeShipCategoryTargets(targetType, damagedArg)) {
							addCategory(
								shipCategories,
								normalizedCalleeChain.endsWith("resources.getShip") ? "resources.getShip" : "ShipLoader.add",
								normalizedTargetType,
								module,
							);
						}
					}
					return;
				}

				if (normalizedCalleeChain.endsWith("resources.getSlotitem") || normalizedCalleeChain.endsWith("SlotLoader.add")) {
					const typeArg = path.node.arguments[1];
					const targetType = typeArg !== undefined && !t.isSpreadElement(typeArg)
						? t.isStringLiteral(typeArg)
							? typeArg.value
							: t.isIdentifier(typeArg)
								? stringBindings.get(typeArg.name)
								: undefined
						: undefined;
					if (targetType !== undefined) {
						addCategory(
							slotCategories,
							normalizedCalleeChain.endsWith("resources.getSlotitem") ? "resources.getSlotitem" : "SlotLoader.add",
							targetType,
							module,
						);
					}
				}
			},
			NewExpression(path: NodePath<t.NewExpression>) {
				const calleeName = getCallExpressionChain(path.node.callee)
					?? (t.isIdentifier(path.node.callee) ? path.node.callee.name : undefined);
				if (calleeName === undefined) {
					return;
				}

				const targetArg = path.node.arguments[0];
				const targetType = targetArg !== undefined && !t.isSpreadElement(targetArg)
					? t.isStringLiteral(targetArg)
						? targetArg.value
						: t.isIdentifier(targetArg)
							? stringBindings.get(targetArg.name)
							: undefined
					: undefined;
				if (targetType === undefined) {
					return;
				}

				if (calleeName.endsWith("TaskLoadShipResource")) {
					addCategory(shipCategories, "explicit-path", targetType, module);
				}
				if (calleeName.endsWith("TaskLoadSlotResource")) {
					addCategory(slotCategories, "explicit-path", targetType, module);
				}
			},
		});
	}

	const shipTargetTypes = sortEntries(shipCategories);
	const slotTargetTypes = sortEntries(slotCategories);
	const shipTargetTypeSet = new Set(shipTargetTypes.map(entry => entry.targetType));
	const slotTargetTypeSet = new Set(slotTargetTypes.map(entry => entry.targetType));
	const spRemodelSubcategories = [...shipTargetTypeSet]
		.filter(targetType => targetType.startsWith("sp_remodel/"))
		.map(targetType => targetType.slice("sp_remodel/".length))
		.sort();

	return {
		shipTargetTypes,
		slotTargetTypes,
		shipGenerationGroups: buildShipGenerationGroups(shipTargetTypeSet),
		slotGenerationGroups: buildSlotGenerationGroups(slotTargetTypeSet),
		spRemodelSubcategories,
	};
}

export function toResourceCategoriesAsset(
	scriptVersion: string,
	extracted: ExtractedResourceCategories,
): ResourceCategoriesAsset {
	const shipTargetTypeSet = new Set(extracted.shipTargetTypes.map(entry => entry.targetType));
	const slotTargetTypeSet = new Set(extracted.slotTargetTypes.map(entry => entry.targetType));

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			shipTargetTypeCount: shipTargetTypeSet.size,
			slotTargetTypeCount: slotTargetTypeSet.size,
			spRemodelSubcategoryCount: extracted.spRemodelSubcategories.length,
			shipGenerationGroupCount: Object.values(extracted.shipGenerationGroups).reduce((sum, group) => sum + group.length, 0),
			slotGenerationGroupCount: Object.values(extracted.slotGenerationGroups).reduce((sum, group) => sum + group.length, 0),
		},
		shipTargetTypes: extracted.shipTargetTypes,
		slotTargetTypes: extracted.slotTargetTypes,
		shipGenerationGroups: extracted.shipGenerationGroups,
		slotGenerationGroups: extracted.slotGenerationGroups,
		spRemodelSubcategories: extracted.spRemodelSubcategories,
	};
}
