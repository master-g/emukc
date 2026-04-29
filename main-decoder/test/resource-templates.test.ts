import { describe, expect, test } from "bun:test";

import { extractResourceTemplates, toResourceTemplatesAsset } from "../src/resource-templates.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function makeModule(id: string, source: string): ModuleArtifact {
	return {
		id,
		displayId: id,
		fileName: `${id}.js`,
		moduleKind: "game",
		cleanupTier: "none",
		exportNames: [],
		hasDefaultExport: false,
		canonicalParameterNames: [],
		rawObfuscatedIdentifierCount: 0,
		transformedObfuscatedIdentifierCount: 0,
		obfuscatedIdentifierDelta: 0,
		shellMetrics: { namespaceShellCount: 0, normalizedNamespaceShellCount: 0, classShellCount: 0, normalizedClassShellCount: 0, structuralTransformCount: 0 },
		lineCount: 1,
		dependencies: [],
		readableName: id,
		source: `function(module, exports) { ${source} }`,
	};
}

function makeGraph(modules: ModuleArtifact[]): ModuleGraph {
	return { modules, summary: {} as ModuleGraphSummary };
}

describe("extractResourceTemplates", () => {
	test("emits template-backed family descriptors with inputs and provenance", () => {
		const graph = makeGraph([
			makeModule("map-loader", `
				var p = settings.path_root + "resources/map/" ["concat"](MathUtil.zeroPadding(area, 3), '/').concat(MathUtil.zeroPadding(no, 2), '_').concat(suffix);
				var info = settings.path_root + "resources/map/" ["concat"](MathUtil.zeroPadding(area, 3), '/').concat(MathUtil.zeroPadding(no, 2), "_info.json");
			`),
			makeModule("gauge-loader", `
				var g = settings.path_root + "resources/gauge/" ["concat"](MathUtil.zeroPadding(area, 3)).concat(MathUtil.zeroPadding(no, 2), ".json");
			`),
			makeModule("furniture-loader", `
				var pattern = new RegExp("^resources/furniture/(movable|normal|thumbnail|picture|outside|card)/.+");
				var p = settings.path_root + "resources/furniture/" ["concat"]("movable", "/").concat(id, "_").concat(version, ".json");
			`),
			makeModule("useitem-loader", `
				return type == 1 ? settings.path_root + "resources/useitem/card/" + id + ".png" : settings.path_root + "resources/useitem/card_/" + id + ".png";
			`),
			makeModule("area-loader", `
				var s = settings.path_root + "resources/area/sally/" ["concat"](area, ".png");
				var a = settings.path_root + "resources/area/airunit/" ["concat"](area, ".png");
			`),
			makeModule("audio-loader", `
				audio.src = settings.path_root + "resources/bgm/" ["concat"](category, "/").concat(id, ".mp3");
				sound.voice.play("9998", voiceId);
				sound.voice.play("titlecall_1", Math.floor(103 * Math.random()) + 1);
				sound.voice.play("titlecall_2", Math.floor(64 * Math.random()) + 1);
			`),
		]);

		const asset = toResourceTemplatesAsset("6.2.8.0", extractResourceTemplates(graph, [
			`
				this.IMG_BG = "resources/worldselect/bg.jpg";
				for (var o = 0; o < 20; o++) {
					r.add("world" + (o + 1), "resources/worldselect/btn_chinjyufu" + (o + 1) + "_off.png");
				}
			`,
		]));
		const keys = asset.families.map(family => family.key);
		expect(keys).toEqual(expect.arrayContaining([
			"map.base",
			"map.info",
			"gauge.map",
			"furniture.movable",
			"furniture.normal",
			"useitem.card",
			"useitem.card_",
			"area.sally",
			"area.airunit",
			"bgm.category",
			"sound.kc9998",
			"voice.titlecall_1",
			"voice.titlecall_2",
			"worldselect.chinjufu-buttons",
		]));

		const map = asset.families.find(family => family.key === "map.base");
		expect(map?.coverageMode).toBe("observed-complete");
		expect(map?.requiredInputs).toContain("manifest.mapinfo");
		expect(map?.provenance.moduleIds).toContain("map-loader");
		expect(map?.pathTemplate.some(segment => segment.kind === "placeholder" && segment.name === "areaId")).toBe(true);
		expect(map?.completenessBlockers).toBeUndefined();

		const mapInfo = asset.families.find(family => family.key === "map.info");
		expect(mapInfo?.coverageMode).toBe("partial");
		expect(mapInfo?.requiredInputs).toContain("manifest.mapinfo");
		expect(mapInfo?.completenessBlockers?.[0]?.kind).toBe("partial-coverage");

		const sound = asset.families.find(family => family.key === "sound.kc9998");
		expect(sound?.coverageMode).toBe("partial");
		expect(sound?.requiredInputs).toContain("cache-source.sound-bucket");
		expect(sound?.completenessBlockers?.[0]?.kind).toBe("unavailable-runtime-input");

		const gauge = asset.families.find(family => family.key === "gauge.map");
		expect(gauge?.coverageMode).toBe("partial");
		expect(gauge?.requiredInputs).toContain("manifest.mapinfo");
		expect(gauge?.completenessBlockers?.[0]?.kind).toBe("partial-coverage");

		const bgm = asset.families.find(family => family.key === "bgm.category");
		expect(bgm?.coverageMode).toBe("observed-complete");
		expect(bgm?.requiredInputs).toEqual(expect.arrayContaining(["manifest.bgm", "manifest.mapbgm"]));
		expect(bgm?.completenessBlockers).toBeUndefined();

		const worldSelect = asset.families.find(family => family.key === "worldselect.chinjufu-buttons");
		expect(worldSelect?.coverageMode).toBe("observed-complete");
		expect(worldSelect?.requiredInputs).toContain("decoder.template-range");
		expect(worldSelect?.provenance.moduleIds).toContain("world.js");

		expect(asset.summary.familyCount).toBe(asset.families.length);
		expect(asset.summary.observedCompleteFamilyCount).toBeGreaterThan(0);
	});
});
