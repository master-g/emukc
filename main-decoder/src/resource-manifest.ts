import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type { ModuleArtifact, ModuleGraph } from "./types.ts";

// --- Types ---

export interface ResourceManifestShipEntry {
	kind: "ship";
	source: string;
	targetType: string;
	shipMstIdSource: string;
	damagedSource: string;
	moduleIds: string[];
	moduleNames: string[];
}

export interface ResourceManifestSlotitemEntry {
	kind: "slotitem";
	source: string;
	targetType: string;
	slotMstIdSources: string[];
	moduleIds: string[];
	moduleNames: string[];
}

export interface ResourceManifestTextureProviderEntry {
	kind: "texture-provider";
	provider: string;
	textureIds: number[];
	moduleIds: string[];
	moduleNames: string[];
}

export interface ResourceManifestExplicitPathEntry {
	kind: "explicit-path";
	paths: string[];
	moduleIds: string[];
	moduleNames: string[];
}

export type ResourceManifestEntry =
	| ResourceManifestShipEntry
	| ResourceManifestSlotitemEntry
	| ResourceManifestTextureProviderEntry
	| ResourceManifestExplicitPathEntry;

export interface ResourceManifest {
	version: 1;
	generatedAt: string;
	summary: {
		totalEntries: number;
		shipEntryCount: number;
		slotitemEntryCount: number;
		textureProviderEntryCount: number;
		explicitPathEntryCount: number;
		totalExplicitPaths: number;
		modulesCovered: number;
	};
	entries: ResourceManifestEntry[];
}

// --- Helpers ---

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

// --- Dedup key builders ---

function shipDedupKey(source: string, targetType: string, shipMstIdSource: string, damagedSource: string): string {
	return `ship:${source}:${targetType}:${shipMstIdSource}:${damagedSource}`;
}

function slotitemDedupKey(source: string, targetType: string, slotMstIdSources: string[]): string {
	return `slotitem:${source}:${targetType}:${slotMstIdSources.join(",")}`;
}

// --- Module-level extraction ---

interface ModuleResourceFindings {
	shipEntries: Map<string, ResourceManifestShipEntry>;
	slotEntries: Map<string, ResourceManifestSlotitemEntry>;
	textureEntries: Map<string, ResourceManifestTextureProviderEntry>;
	explicitPaths: Set<string>;
}

function extractModuleResources(module: ModuleArtifact): ModuleResourceFindings {
	const findings: ModuleResourceFindings = {
		shipEntries: new Map(),
		slotEntries: new Map(),
		textureEntries: new Map(),
		explicitPaths: new Set(),
	};

	// Quick pre-check: skip modules with no resource-related code at all
	const source = module.source;
	if (
		!source.includes("getShip")
		&& !source.includes("getSlotitem")
		&& !source.includes("getTexture")
		&& !source.includes("ShipLoader")
		&& !source.includes("SlotLoader")
		&& !source.includes("resources/")
	) {
		return findings;
	}

	// Extract explicit paths via regex (fast, no AST needed)
	for (const match of source.matchAll(/resources\/[A-Za-z0-9_./-]+/g)) {
		findings.explicitPaths.add(match[0]);
	}

	// AST-based extraction
	let ast: t.File;
	try {
		ast = parseFactorySource(source);
	} catch {
		// Source may not be valid standalone JS; keep regex-extracted explicit paths
		return findings;
	}
	const moduleName = module.readableName ?? module.fileName;

	// Track loader aliases
	const shipLoaderBindings = new Set<string>();
	const slotLoaderBindings = new Set<string>();

	traverse(ast, {
		VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
			if (!t.isIdentifier(path.node.id) || path.node.init == null || !t.isNewExpression(path.node.init)) {
				return;
			}

			const callee = path.node.init.callee;
			const calleeChain = getCallExpressionChain(callee);
			// Handle both MemberExpression (a.b.ShipLoader) and Identifier (ShipLoader) callees
			const calleeName = calleeChain
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

			// Resolve aliased loader calls (e.g., var sl = new ShipLoader(); sl.add(...))
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

			// Ship: resources.getShip or ShipLoader.add
			if (normalizedCalleeChain.endsWith("resources.getShip") || normalizedCalleeChain.endsWith("ShipLoader.add")) {
				const [idArg, damagedArg, typeArg] = path.node.arguments;
				if (idArg === undefined || damagedArg === undefined || typeArg === undefined) {
					return;
				}
				if (t.isSpreadElement(idArg) || t.isSpreadElement(damagedArg) || t.isSpreadElement(typeArg) || !t.isStringLiteral(typeArg)) {
					return;
				}

				const shipMstIdSource = expressionToSource(idArg);
				const damagedSource = expressionToSource(damagedArg);
				if (shipMstIdSource === undefined || damagedSource === undefined) {
					return;
				}

				const action = normalizedCalleeChain.endsWith("resources.getShip") ? "resources.getShip" : "ShipLoader.add";
				const key = shipDedupKey(action, typeArg.value, shipMstIdSource, damagedSource);
				findings.shipEntries.set(key, {
					kind: "ship",
					source: action,
					targetType: typeArg.value,
					shipMstIdSource,
					damagedSource,
					moduleIds: [module.id],
					moduleNames: [moduleName],
				});
				return;
			}

			// Slotitem: resources.getSlotitem or SlotLoader.add
			if (normalizedCalleeChain.endsWith("resources.getSlotitem") || normalizedCalleeChain.endsWith("SlotLoader.add")) {
				const [idArg, typeArg] = path.node.arguments;
				if (idArg === undefined || typeArg === undefined) {
					return;
				}
				if (t.isSpreadElement(idArg) || t.isSpreadElement(typeArg) || !t.isStringLiteral(typeArg)) {
					return;
				}

				const slotMstIdSource = expressionToSource(idArg);
				if (slotMstIdSource === undefined) {
					return;
				}

				const action = normalizedCalleeChain.endsWith("resources.getSlotitem") ? "resources.getSlotitem" : "SlotLoader.add";
				const sources = [slotMstIdSource];
				const key = slotitemDedupKey(action, typeArg.value, sources);
				findings.slotEntries.set(key, {
					kind: "slotitem",
					source: action,
					targetType: typeArg.value,
					slotMstIdSources: sources,
					moduleIds: [module.id],
					moduleNames: [moduleName],
				});
				return;
			}

			// Texture provider: getTexture
			if (normalizedCalleeChain.endsWith("getTexture")) {
				const provider = normalizedCalleeChain.split(".").at(-2);
				if (provider === undefined) {
					return;
				}

				const numericIds = path.node.arguments
					.filter((arg): arg is t.NumericLiteral => !t.isSpreadElement(arg) && t.isNumericLiteral(arg))
					.map(arg => arg.value);

				const existing = findings.textureEntries.get(provider);
				if (existing !== undefined) {
					existing.textureIds = [...new Set([...existing.textureIds, ...numericIds])].sort((a, b) => a - b);
				} else {
					findings.textureEntries.set(provider, {
						kind: "texture-provider",
						provider,
						textureIds: [...new Set(numericIds)].sort((a, b) => a - b),
						moduleIds: [module.id],
						moduleNames: [moduleName],
					});
				}
			}
		},
	});

	return findings;
}

// --- Cross-module deduplication ---

interface AggregatedFindings {
	shipEntries: Map<string, ResourceManifestShipEntry>;
	slotEntries: Map<string, ResourceManifestSlotitemEntry>;
	textureEntries: Map<string, ResourceManifestTextureProviderEntry>;
	explicitPaths: Map<string, { path: string; moduleIds: string[]; moduleNames: string[] }>;
}

function aggregateFindings(allFindings: Array<{ module: ModuleArtifact; findings: ModuleResourceFindings }>): AggregatedFindings {
	const result: AggregatedFindings = {
		shipEntries: new Map(),
		slotEntries: new Map(),
		textureEntries: new Map(),
		explicitPaths: new Map(),
	};

	for (const { module, findings } of allFindings) {
		const moduleName = module.readableName ?? module.fileName;

		// Merge ship entries
		for (const [key, entry] of findings.shipEntries) {
			const existing = result.shipEntries.get(key);
			if (existing !== undefined) {
				existing.moduleIds.push(...entry.moduleIds);
				existing.moduleNames.push(...entry.moduleNames);
			} else {
				result.shipEntries.set(key, { ...entry });
			}
		}

		// Merge slot entries
		for (const [key, entry] of findings.slotEntries) {
			const existing = result.slotEntries.get(key);
			if (existing !== undefined) {
				existing.moduleIds.push(...entry.moduleIds);
				existing.moduleNames.push(...entry.moduleNames);
			} else {
				result.slotEntries.set(key, { ...entry });
			}
		}

		// Merge texture entries
		for (const [provider, entry] of findings.textureEntries) {
			const existing = result.textureEntries.get(provider);
			if (existing !== undefined) {
				existing.textureIds = [...new Set([...existing.textureIds, ...entry.textureIds])].sort((a, b) => a - b);
				existing.moduleIds.push(...entry.moduleIds);
				existing.moduleNames.push(...entry.moduleNames);
			} else {
				result.textureEntries.set(provider, { ...entry });
			}
		}

		// Merge explicit paths
		for (const path of findings.explicitPaths) {
			const existing = result.explicitPaths.get(path);
			if (existing !== undefined) {
				existing.moduleIds.push(module.id);
				existing.moduleNames.push(moduleName);
			} else {
				result.explicitPaths.set(path, {
					path,
					moduleIds: [module.id],
					moduleNames: [moduleName],
				});
			}
		}
	}

	return result;
}

// --- Main extractor ---

export function extractResourceManifest(moduleGraph: ModuleGraph): ResourceManifest {
	// Extract from ALL modules
	const allFindings = moduleGraph.modules
		.map(module => ({ module, findings: extractModuleResources(module) }))
		.filter(({ findings }) =>
			findings.shipEntries.size > 0
			|| findings.slotEntries.size > 0
			|| findings.textureEntries.size > 0
			|| findings.explicitPaths.size > 0
		);

	const aggregated = aggregateFindings(allFindings);

	// Build entries array
	const entries: ResourceManifestEntry[] = [
		...aggregated.shipEntries.values(),
		...aggregated.slotEntries.values(),
		...aggregated.textureEntries.values(),
	];

	// Group explicit paths into a single entry
	if (aggregated.explicitPaths.size > 0) {
		const allModuleIds = [...new Set([...aggregated.explicitPaths.values()].flatMap(e => e.moduleIds))];
		const allModuleNames = [...new Set([...aggregated.explicitPaths.values()].flatMap(e => e.moduleNames))];
		entries.push({
			kind: "explicit-path",
			paths: [...aggregated.explicitPaths.keys()].sort(),
			moduleIds: allModuleIds,
			moduleNames: allModuleNames,
		});
	}

	const shipEntries = entries.filter((e): e is ResourceManifestShipEntry => e.kind === "ship");
	const slotEntries = entries.filter((e): e is ResourceManifestSlotitemEntry => e.kind === "slotitem");
	const textureEntries = entries.filter((e): e is ResourceManifestTextureProviderEntry => e.kind === "texture-provider");
	const pathEntries = entries.filter((e): e is ResourceManifestExplicitPathEntry => e.kind === "explicit-path");

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		summary: {
			totalEntries: entries.length,
			shipEntryCount: shipEntries.length,
			slotitemEntryCount: slotEntries.length,
			textureProviderEntryCount: textureEntries.length,
			explicitPathEntryCount: pathEntries.length,
			totalExplicitPaths: pathEntries.reduce((sum, e) => sum + e.paths.length, 0),
			modulesCovered: allFindings.length,
		},
		entries,
	};
}
