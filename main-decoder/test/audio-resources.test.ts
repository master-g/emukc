import { describe, expect, test } from "bun:test";

import { extractAudioResources, toAudioResourcesAsset } from "../src/audio-resources.ts";
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

describe("extractAudioResources", () => {
	test("extracts SE ids, BGM ids, and tutorial voice stems", () => {
		const graph = makeGraph([
			makeModule("a1", `
				seModule.SE.play("103");
				sound.bgm.play(44, true, 0, "battle");
				sound.bgm.play(7, false, 0, "port");
				this._ev.emit("tutorial-play-voice", "tutorial", "023_a");
			`),
		]);

		const extracted = extractAudioResources(graph);
		expect(extracted.seIds.has(103)).toBe(true);
		expect(extracted.battleBgmIds.has(44)).toBe(true);
		expect(extracted.portBgmIds.has(7)).toBe(true);
		expect(extracted.tutorialVoiceStems.has("023_a")).toBe(true);
	});

	test("captures titlecall ranges as explicit voice files", () => {
		const graph = makeGraph([
			makeModule("a-titlecall", `
				var titleCall1 = Math.floor(103 * Math.random()) + 1;
				sound.voice.play("titlecall_1", titleCall1, onEnd);
				var titleCall2 = Math.floor(64 * Math.random()) + 1;
				sound.voice.play("titlecall_2", titleCall2, onEnd);
			`),
		]);

		const asset = toAudioResourcesAsset("6.2.8.0", extractAudioResources(graph));
		expect(asset.voice.explicitFiles).toContain("titlecall_1/001.mp3");
		expect(asset.voice.explicitFiles).toContain("titlecall_1/103.mp3");
		expect(asset.voice.explicitFiles).toContain("titlecall_2/001.mp3");
		expect(asset.voice.explicitFiles).toContain("titlecall_2/064.mp3");
	});

	test("captures fanfare and battle BGM ids from sound manager calls", () => {
		const graph = makeGraph([
			makeModule("a-bgm", `
				sound.bgm.play(3, false, 0, "fanfare");
				sound.bgm.playBattleBGM(132);
			`),
		]);

		const extracted = extractAudioResources(graph);
		expect(extracted.fanfareBgmIds.has(3)).toBe(true);
		expect(extracted.battleBgmIds.has(132)).toBe(true);
	});

	test("captures uncategorized numeric bgm.play calls as port coverage", () => {
		const graph = makeGraph([
			makeModule("a-port-bgm", `
				sound.bgm.play(102);
				sound.bgm.play(125, true, 0);
			`),
		]);

		const extracted = extractAudioResources(graph);
		expect(extracted.portBgmIds.has(102)).toBe(true);
		expect(extracted.portBgmIds.has(125)).toBe(true);
	});

	test("captures titlecall categories and explicit voice paths", () => {
		const graph = makeGraph([
			makeModule("a2", `
				sound.voice.play("titlecall_1", randomId);
				var path = "resources/voice/tutorial/021.mp3";
			`),
		]);

		const asset = toAudioResourcesAsset("6.2.8.0", extractAudioResources(graph));
		expect(asset.voice.titlecallCategories).toContain("titlecall_1");
		expect(asset.voice.explicitFiles).toContain("tutorial/021.mp3");
		expect(asset.summary.explicitPathCount).toBeGreaterThan(0);
	});
});
