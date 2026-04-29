import type {
	ModuleGraph,
	ResourceCoverageMode,
	ResourceTemplateCompletenessBlocker,
	ResourceTemplateDomain,
	ResourceTemplateFamily,
	ResourceTemplateInput,
	ResourceTemplateSegment,
	ResourceTemplatesAsset,
} from "./types.ts";

interface TemplateDraft {
	key: string;
	domain: ResourceTemplateDomain;
	outputPrefix: string;
	pathTemplate: ResourceTemplateSegment[];
	requiredInputs: ResourceTemplateInput[];
	coverageMode: ResourceCoverageMode;
	completenessBlockers?: ResourceTemplateCompletenessBlocker[];
	moduleIds: Set<string>;
	moduleNames: Set<string>;
	range?: { start: number; end: number; pad?: number };
}

interface ExtractedResourceTemplates {
	families: TemplateDraft[];
}

interface TemplateSource {
	id: string;
	fileName: string;
	readableName?: string;
	source: string;
}

function moduleName(module: TemplateSource): string {
	return module.readableName ?? module.fileName;
}

function literal(value: string): ResourceTemplateSegment {
	return { kind: "literal", value };
}

function placeholder(
	name: string,
	format: "number" | "pad2" | "pad3" | "raw" = "raw",
): ResourceTemplateSegment {
	return { kind: "placeholder", name, format };
}

function addTemplate(
	templates: Map<string, TemplateDraft>,
	module: TemplateSource,
	template: Omit<TemplateDraft, "moduleIds" | "moduleNames">,
): void {
	const existing = templates.get(template.key);
	if (existing !== undefined) {
		existing.moduleIds.add(module.id);
		existing.moduleNames.add(moduleName(module));
		if (existing.coverageMode !== "observed-complete") {
			existing.coverageMode = template.coverageMode;
		}
		if (template.completenessBlockers !== undefined) {
			existing.completenessBlockers = template.completenessBlockers;
		}
		return;
	}
	templates.set(template.key, {
		...template,
		moduleIds: new Set([module.id]),
		moduleNames: new Set([moduleName(module)]),
	});
}

function addMapTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (!source.includes("resources/map/")) {
		return;
	}
	addTemplate(templates, module, {
		key: "map.base",
		domain: "map",
		outputPrefix: "kcs2/resources/map",
		pathTemplate: [
			literal("kcs2/resources/map/"),
			placeholder("areaId", "pad3"),
			literal("/"),
			placeholder("mapNo", "pad2"),
			literal(".png"),
		],
		requiredInputs: ["manifest.mapinfo"],
		coverageMode: "observed-complete",
	});
	addTemplate(templates, module, {
		key: "map.info",
		domain: "map",
		outputPrefix: "kcs2/resources/map",
		pathTemplate: [
			literal("kcs2/resources/map/"),
			placeholder("areaId", "pad3"),
			literal("/"),
			placeholder("mapNo", "pad2"),
			placeholder("suffix", "raw"),
		],
		requiredInputs: ["manifest.mapinfo"],
		coverageMode: source.includes("_info") || source.includes("_image") ? "partial" : "unresolved",
		completenessBlockers: [
			{
				kind: "partial-coverage",
				reason: "Decoded modules prove map sidecar path shape, but suffix membership and event/default variants are not fully proven from manifest.mapinfo alone.",
				requiredInputs: ["manifest.mapinfo"],
			},
		],
	});
}

function addGaugeTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	if (!module.source.includes("resources/gauge/") && !module.source.includes("_image")) {
		return;
	}
	addTemplate(templates, module, {
		key: "gauge.map",
		domain: "gauge",
		outputPrefix: "kcs2/resources/gauge",
		pathTemplate: [
			literal("kcs2/resources/gauge/"),
			placeholder("areaId", "pad3"),
			placeholder("mapNo", "pad2"),
			placeholder("variant", "raw"),
		],
		requiredInputs: ["manifest.mapinfo"],
		coverageMode: "partial",
		completenessBlockers: [
			{
				kind: "partial-coverage",
				reason: "Decoded modules prove gauge map path shape, but gauge image sidecars and event variants require cache-resident gauge JSON or a richer map variant input.",
				requiredInputs: ["manifest.mapinfo"],
			},
		],
	});
}

function addFurnitureTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (!source.includes("resources/furniture/") && !source.includes("getFurniture(")) {
		return;
	}
	for (const category of ["normal", "movable", "thumbnail", "scripts", "picture", "outside", "reward", "card"] as const) {
		if (!source.includes(category)) {
			continue;
		}
		addTemplate(templates, module, {
			key: `furniture.${category}`,
			domain: "furniture",
			outputPrefix: `kcs2/resources/furniture/${category}`,
			pathTemplate: [
				literal("kcs2/resources/furniture/"),
				literal(category),
				literal("/"),
				placeholder("furnitureId", "pad3"),
				placeholder("suffix", "raw"),
			],
			requiredInputs: ["manifest.furniture"],
			coverageMode: category === "picture" ? "partial" : "observed-complete",
		});
	}
}

function addUseItemTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (!source.includes("resources/useitem/")) {
		return;
	}
	addTemplate(templates, module, {
		key: "useitem.card",
		domain: "useitem",
		outputPrefix: "kcs2/resources/useitem/card",
		pathTemplate: [
			literal("kcs2/resources/useitem/card/"),
			placeholder("useitemId", "pad3"),
			literal(".png"),
		],
		requiredInputs: ["decoder.ui"],
		coverageMode: "partial",
	});
	addTemplate(templates, module, {
		key: "useitem.card_",
		domain: "useitem",
		outputPrefix: "kcs2/resources/useitem/card_",
		pathTemplate: [
			literal("kcs2/resources/useitem/card_/"),
			placeholder("useitemId", "pad3"),
			literal(".png"),
		],
		requiredInputs: ["decoder.ui"],
		coverageMode: "partial",
	});
}

function addAreaTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (!source.includes("resources/area/")) {
		return;
	}
	for (const family of ["sally", "airunit", "airunit_extend_confirm"] as const) {
		if (!source.includes(`resources/area/${family}/`)) {
			continue;
		}
		addTemplate(templates, module, {
			key: `area.${family}`,
			domain: "area",
			outputPrefix: `kcs2/resources/area/${family}`,
			pathTemplate: [
				literal("kcs2/resources/area/"),
				literal(family),
				literal("/"),
				placeholder("areaId", "raw"),
				literal(".png"),
			],
			requiredInputs: ["manifest.mapinfo"],
			coverageMode: "partial",
		});
	}
}

function addWorldSelectTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (!source.includes("worldselect/") && !source.includes("btn_chinjyufu")) {
		return;
	}
	addTemplate(templates, module, {
		key: "worldselect.chinjufu-buttons",
		domain: "worldselect",
		outputPrefix: "kcs2/resources/worldselect",
		pathTemplate: [
			literal("kcs2/resources/worldselect/btn_chinjyufu"),
			placeholder("worldId", "number"),
			placeholder("state", "raw"),
			literal(".png"),
		],
		requiredInputs: ["decoder.template-range"],
		coverageMode: "observed-complete",
		range: { start: 1, end: 20 },
	});
}

function addAudioTemplates(templates: Map<string, TemplateDraft>, module: TemplateSource): void {
	const source = module.source;
	if (source.includes("resources/bgm/")) {
		addTemplate(templates, module, {
			key: "bgm.category",
			domain: "bgm",
			outputPrefix: "kcs2/resources/bgm",
			pathTemplate: [
				literal("kcs2/resources/bgm/"),
				placeholder("category", "raw"),
				literal("/"),
				placeholder("bgmId", "pad3"),
				literal(".mp3"),
			],
			requiredInputs: ["manifest.bgm", "manifest.mapbgm"],
			coverageMode: "observed-complete",
		});
	}
	if (source.includes("titlecall_1")) {
		addTemplate(templates, module, {
			key: "voice.titlecall_1",
			domain: "voice",
			outputPrefix: "kcs2/resources/voice/titlecall_1",
			pathTemplate: [
				literal("kcs2/resources/voice/titlecall_1/"),
				placeholder("voiceId", "pad3"),
				literal(".mp3"),
			],
			requiredInputs: ["decoder.template-range"],
			coverageMode: "observed-complete",
			range: { start: 1, end: 103, pad: 3 },
		});
	}
	if (source.includes("titlecall_2")) {
		addTemplate(templates, module, {
			key: "voice.titlecall_2",
			domain: "voice",
			outputPrefix: "kcs2/resources/voice/titlecall_2",
			pathTemplate: [
				literal("kcs2/resources/voice/titlecall_2/"),
				placeholder("voiceId", "pad3"),
				literal(".mp3"),
			],
			requiredInputs: ["decoder.template-range"],
			coverageMode: "observed-complete",
			range: { start: 1, end: 64, pad: 3 },
		});
	}
	if (source.includes("9998")) {
		addTemplate(templates, module, {
			key: "sound.kc9998",
			domain: "sound",
			outputPrefix: "kcs/sound/kc9998",
			pathTemplate: [
				literal("kcs/sound/kc9998/"),
				placeholder("voiceId", "number"),
				literal(".mp3"),
			],
			requiredInputs: ["cache-source.sound-bucket"],
			coverageMode: "partial",
			completenessBlockers: [
				{
					kind: "unavailable-runtime-input",
					reason: "Decoded modules prove the kc9998 bucket path shape, but full membership depends on the cache-source sound bucket input.",
					requiredInputs: ["cache-source.sound-bucket"],
				},
			],
		});
	}
}

export function extractResourceTemplates(moduleGraph: ModuleGraph, supplementalSources: string[] = []): ExtractedResourceTemplates {
	const templates = new Map<string, TemplateDraft>();
	const sources: TemplateSource[] = [
		...moduleGraph.modules,
		...supplementalSources.map((source, index) => ({
			id: index === 0 ? "world.js" : `supplemental:${index}`,
			fileName: index === 0 ? "world.js" : `supplemental-${index}.js`,
			readableName: index === 0 ? "world.js" : `SupplementalSource${index}`,
			source,
		})),
	];
	for (const module of sources) {
		addMapTemplates(templates, module);
		addGaugeTemplates(templates, module);
		addFurnitureTemplates(templates, module);
		addUseItemTemplates(templates, module);
		addAreaTemplates(templates, module);
		addWorldSelectTemplates(templates, module);
		addAudioTemplates(templates, module);
	}

	return {
		families: [...templates.values()].sort((left, right) => left.key.localeCompare(right.key)),
	};
}

function countMode(families: TemplateDraft[], mode: ResourceCoverageMode): number {
	return families.filter(family => family.coverageMode === mode).length;
}

export function toResourceTemplatesAsset(
	scriptVersion: string,
	extracted: ExtractedResourceTemplates,
): ResourceTemplatesAsset {
	const families: ResourceTemplateFamily[] = extracted.families.map(family => ({
		key: family.key,
		domain: family.domain,
		outputPrefix: family.outputPrefix,
		pathTemplate: family.pathTemplate,
		requiredInputs: [...family.requiredInputs],
		coverageMode: family.coverageMode,
		provenance: {
			moduleIds: [...family.moduleIds].sort(),
			moduleNames: [...family.moduleNames].sort(),
		},
		...(family.completenessBlockers === undefined ? {} : { completenessBlockers: family.completenessBlockers }),
		...(family.range === undefined ? {} : { range: family.range }),
	}));

	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			familyCount: families.length,
			observedCompleteFamilyCount: countMode(extracted.families, "observed-complete"),
			partialFamilyCount: countMode(extracted.families, "partial"),
			unresolvedFamilyCount: countMode(extracted.families, "unresolved"),
		},
		families,
		unresolvedFamilies: families
			.filter(family => family.coverageMode === "unresolved")
			.map(family => family.key)
			.sort(),
	};
}
