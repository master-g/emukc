import type { AudioResourcesAsset, ModuleArtifact, ModuleGraph, ResourceCoverageMode } from "./types.ts";

interface ExtractedAudioResources {
	seIds: Set<number>;
	portBgmIds: Set<number>;
	battleBgmIds: Set<number>;
	fanfareBgmIds: Set<number>;
	tutorialVoiceStems: Set<string>;
	titlecallCategories: Set<string>;
	titlecallFileCounts: Map<string, number>;
	explicitPaths: Set<string>;
	explicitVoiceFiles: Set<string>;
}

function addNumericMatches(target: Set<number>, source: string, pattern: RegExp): void {
	for (const match of source.matchAll(pattern)) {
		const value = Number.parseInt(match[1] ?? "", 10);
		if (Number.isInteger(value) && value > 0) {
			target.add(value);
		}
	}
}

function addTitlecallRange(
	targetCounts: Map<string, number>,
	category: string,
	count: number,
): void {
	if (!Number.isInteger(count) || count <= 0) {
		return;
	}
	const existing = targetCounts.get(category) ?? 0;
	if (count > existing) {
		targetCounts.set(category, count);
	}
}

function coverageMode(values: Set<unknown>): ResourceCoverageMode {
	return values.size > 0 ? "partial" : "unresolved";
}

export function extractAudioResources(moduleGraph: ModuleGraph): ExtractedAudioResources {
	const extracted: ExtractedAudioResources = {
		seIds: new Set(),
		portBgmIds: new Set(),
		battleBgmIds: new Set(),
		fanfareBgmIds: new Set(),
		tutorialVoiceStems: new Set(),
		titlecallCategories: new Set(),
		titlecallFileCounts: new Map(),
		explicitPaths: new Set(),
		explicitVoiceFiles: new Set(),
	};

	for (const module of moduleGraph.modules) {
		const source = module.source;
		if (
			!source.includes("resources/se/")
			&& !source.includes("resources/bgm/")
			&& !source.includes("resources/voice/")
			&& !source.includes("tutorial-play-voice")
			&& !source.includes("titlecall_")
			&& !source.includes(".sound.bgm.play")
			&& !source.includes("sound.bgm.play(")
			&& !source.includes("sound.bgm.playBattleBGM(")
			&& !source.includes(".SE.play")
		) {
			continue;
		}

		addNumericMatches(extracted.seIds, source, /\bSE\.play\("(\d+)"\)/g);
		addNumericMatches(extracted.portBgmIds, source, /\bbgm\.play\((\d+),[^)]*"port"/g);
		addNumericMatches(extracted.battleBgmIds, source, /\bbgm\.play\((\d+),[^)]*"battle"/g);
		addNumericMatches(extracted.fanfareBgmIds, source, /\bbgm\.play\((\d+),[^)]*"fanfare"/g);
		addNumericMatches(extracted.battleBgmIds, source, /\bbgm\.playBattleBGM\((\d+)/g);
		addNumericMatches(extracted.fanfareBgmIds, source, /resources\/bgm\/fanfare\/(\d+)\.mp3/g);
		for (const match of source.matchAll(/\bbgm\.play\((\d+)(?:,\s*[^)]*)?\)/g)) {
			const value = Number.parseInt(match[1] ?? "", 10);
			if (!Number.isInteger(value) || value <= 0) {
				continue;
			}
			const fullCall = match[0] ?? "";
			if (!fullCall.includes('"battle"') && !fullCall.includes('"fanfare"') && !fullCall.includes('"port"')) {
				extracted.portBgmIds.add(value);
			}
		}

		for (const match of source.matchAll(/tutorial-play-voice",\s*"tutorial",\s*"([0-9A-Za-z_]+)"/g)) {
			const value = match[1];
			if (value !== undefined && value.length > 0) {
				extracted.tutorialVoiceStems.add(value);
			}
		}

		for (const match of source.matchAll(/"(titlecall_[12])"/g)) {
			const value = match[1];
			if (value !== undefined) {
				extracted.titlecallCategories.add(value);
			}
		}

		if (source.includes("titlecall_1")) {
			addTitlecallRange(
				extracted.titlecallFileCounts,
				"titlecall_1",
				source.includes("Math.floor(103 * Math.random()) + 1") ? 103 : 0,
			);
		}
		if (source.includes("titlecall_2")) {
			addTitlecallRange(
				extracted.titlecallFileCounts,
				"titlecall_2",
				source.includes("Math.floor(64 * Math.random()) + 1") ? 64 : 0,
			);
		}

		for (const match of source.matchAll(/resources\/(?:se|bgm|voice)\/[A-Za-z0-9_./-]+/g)) {
			extracted.explicitPaths.add(match[0]);
			if (match[0].startsWith("resources/voice/") && match[0].endsWith(".mp3")) {
				extracted.explicitVoiceFiles.add(match[0].slice("resources/voice/".length));
			}
		}
	}

	for (const [category, count] of extracted.titlecallFileCounts) {
		extracted.titlecallCategories.add(category);
		for (let index = 1; index <= count; index += 1) {
			extracted.explicitVoiceFiles.add(`${category}/${index.toString().padStart(3, "0")}.mp3`);
		}
	}

	return extracted;
}

export function toAudioResourcesAsset(
	scriptVersion: string,
	extracted: ExtractedAudioResources,
): AudioResourcesAsset {
	return {
		version: 1,
		generatedAt: new Date().toISOString(),
		scriptVersion,
		summary: {
			seIdCount: extracted.seIds.size,
			portBgmIdCount: extracted.portBgmIds.size,
			battleBgmIdCount: extracted.battleBgmIds.size,
			fanfareBgmIdCount: extracted.fanfareBgmIds.size,
			tutorialVoiceStemCount: extracted.tutorialVoiceStems.size,
			explicitPathCount: extracted.explicitPaths.size,
		},
		seIds: {
			coverageMode: coverageMode(extracted.seIds),
			ids: [...extracted.seIds].sort((left, right) => left - right),
		},
		bgm: {
			fanfareIds: {
				coverageMode: coverageMode(extracted.fanfareBgmIds),
				ids: [...extracted.fanfareBgmIds].sort((left, right) => left - right),
			},
			portIds: {
				coverageMode: coverageMode(extracted.portBgmIds),
				ids: [...extracted.portBgmIds].sort((left, right) => left - right),
			},
			battleIds: {
				coverageMode: coverageMode(extracted.battleBgmIds),
				ids: [...extracted.battleBgmIds].sort((left, right) => left - right),
			},
		},
		voice: {
			titlecallCategories: [...extracted.titlecallCategories].sort(),
			tutorialVoiceStems: [...extracted.tutorialVoiceStems].sort(),
			explicitFiles: [...extracted.explicitVoiceFiles].sort(),
		},
		explicitPaths: [...extracted.explicitPaths].sort(),
	};
}
