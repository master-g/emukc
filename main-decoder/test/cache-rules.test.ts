import { describe, expect, test } from "bun:test";

import { extractCacheRules, toCacheRulesAsset } from "../src/cache-rules.ts";
import type { ModuleArtifact, ModuleGraph, ModuleGraphSummary } from "../src/types.ts";

function wrapModule(body: string): string {
	return `function(module, exports, require) { ${body} }`;
}

function makeModule(overrides: Partial<ModuleArtifact> & { id: string; source: string }): ModuleArtifact {
	return {
		displayId: overrides.id,
		fileName: `${overrides.id}.js`,
		moduleKind: "game",
		cleanupTier: "none",
		exportNames: [],
		hasDefaultExport: false,
		canonicalParameterNames: [],
		rawObfuscatedIdentifierCount: 0,
		transformedObfuscatedIdentifierCount: 0,
		obfuscatedIdentifierDelta: 0,
		shellMetrics: {
			namespaceShellCount: 0,
			normalizedNamespaceShellCount: 0,
			classShellCount: 0,
			normalizedClassShellCount: 0,
			structuralTransformCount: 0,
		},
		lineCount: 1,
		dependencies: [],
		readableName: overrides.id,
		...overrides,
	};
}

function makeGraph(modules: ModuleArtifact[]): ModuleGraph {
	return {
		modules,
		summary: {} as ModuleGraphSummary,
	};
}

describe("extractCacheRules", () => {
	test("captures ship voice formula, voicef gates, and sound buckets", () => {
		const graph = makeGraph([
			makeModule({
				id: "voice-constants",
				readableName: "VoiceConstants",
				source: `voice\\":[2475,6547,1471,8691,7847,3595,1767,3311,2507,9651,5321,4473,7117,5947,9489,2669,8741,6149,1301,7297,2975,6413,8391,9705,2243,2091,4231,3107,9499,4205,6013,3393,6401,6985,3683,9447,3287,5181,7587,9353,2135,4947,5405,5223,9457,5767,9265,8191,3927,3061,2805,3273,7331]`,
			}),
			makeModule({
				id: "voice-manager",
				readableName: "VoiceManager",
				source: wrapModule(`
					function makeUrl(mstId, voiceId) {
						return (parsed = parseInt(voiceId)) <= 53 ? (17 * (mstId + 7) * constants.voice[parsed - 1] % 99173 + 100000).toString() : voiceId;
					}
				`),
			}),
			makeModule({
				id: "ship-mst-model",
				readableName: "ShipMstModel",
				source: wrapModule(`
					Object.defineProperty(ShipMstModel.prototype, "availableBeLeftVoice", { get: function() { return (1 & this._voiceFlag) > 0; } });
					Object.defineProperty(ShipMstModel.prototype, "availableBeLeftVoices", { get: function() { return (4 & this._voiceFlag) > 0; } });
					Object.defineProperty(ShipMstModel.prototype, "availableTimeSignalVoice", { get: function() { return (2 & this._voiceFlag) > 0; } });
				`),
			}),
			makeModule({
				id: "be-left-voice",
				readableName: "BeLeftVoiceTimer",
				source: wrapModule(`
					1 == option.voice_be_left && (1 == this._enabled_129 && this._tired >= 50 ? sound.voice.play(this._mst_id.toString(), 129) : 1 == this._enabled_029 && sound.voice.play(this._mst_id.toString(), 29));
				`),
			}),
			makeModule({
				id: "time-signal",
				readableName: "TimeSignal",
				source: wrapModule(`
					this._enabled_timeSignal && sound.voice.preload(this._mst_id.toString(), this._voicehour + 30);
					this._enabled_timeSignal && sound.voice.play(this._mst_id.toString(), this._voicehour + 30, this._onEnd);
				`),
			}),
			makeModule({
				id: "ship-voice-consumer",
				readableName: "ShipVoiceConsumer",
				source: wrapModule(`
					sound.voice.play(shipId.toString(), 25);
					sound.voice.play(shipId.toString(), 7);
					sound.voice.playAtRandom(shipId.toString(), [16, 18], [50, 50]);
					sound.voice.play(shipId.toString(), 900);
				`),
			}),
			makeModule({
				id: "bucket-9999",
				readableName: "MamiyaOption",
				source: wrapModule(`
					sound.voice.playAtRandom("9999", [11, 12], [50, 50]);
					sound.voice.play("9999", 308);
					sound.voice.play("9999", model.voiceID, null, "duty");
				`),
			}),
			makeModule({
				id: "bucket-9998",
				readableName: "EnemyVoiceConst",
				source: wrapModule(`
					if ("Boss" == name) return { "a": [3505871, 3505872], "d": [3505873] };
					return 611231750;
					sound.voice.play("9998", model.voice_id);
				`),
			}),
			makeModule({
				id: "bucket-9997",
				readableName: "ShipDetailContent",
				source: wrapModule(`
					(extraVoiceId = data.api_voice_id, playVoice(9997, extraVoiceId));
					this._playVoice(9997, 701);
				`),
			}),
			makeModule({
				id: "special-art",
				readableName: "SpecialArtCutin",
				source: wrapModule(`
					(571 != shipMstId && 576 != shipMstId || 0 != damaged) && (541 != shipMstId && 573 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);

		expect(extracted.soundRules.shipVoices.kind).toBe("ship_voice_formula");
		expect(extracted.soundRules.shipVoices.formula).toEqual({
			base: 100000,
			maxFormulaVoiceId: 53,
			modulo: 99173,
			multiplier: 17,
			shipIdOffset: 7,
			voiceDiffs: [2475, 6547, 1471, 8691, 7847, 3595, 1767, 3311, 2507, 9651, 5321, 4473, 7117, 5947, 9489, 2669, 8741, 6149, 1301, 7297, 2975, 6413, 8391, 9705, 2243, 2091, 4231, 3107, 9499, 4205, 6013, 3393, 6401, 6985, 3683, 9447, 3287, 5181, 7587, 9353, 2135, 4947, 5405, 5223, 9457, 5767, 9265, 8191, 3927, 3061, 2805, 3273, 7331],
		});
		expect(extracted.soundRules.shipVoices.baseVoiceIds).toEqual(expect.arrayContaining([7, 16, 18, 25]));
		expect(extracted.soundRules.shipVoices.beLeftVoiceIds).toEqual([29]);
		expect(extracted.soundRules.shipVoices.beLeftTiredVoiceIds).toEqual([129]);
		expect(extracted.soundRules.shipVoices.timeSignalStartVoiceId).toBe(30);
		expect(extracted.soundRules.shipVoices.timeSignalVoiceCount).toBe(24);
		expect(extracted.soundRules.shipVoices.specialArtShipIds).toEqual([541, 571, 573, 576]);
		expect(extracted.soundRules.shipVoices.specialVoiceIds).toEqual([900]);
		expect(extracted.soundRules.kc9999.voiceIds).toEqual(expect.arrayContaining([11, 12, 308]));
		expect(extracted.soundRules.kc9999.hasDynamicVoiceIds).toBe(true);
		expect(extracted.soundRules.kc9998.voiceIds).toEqual(expect.arrayContaining([3505871, 3505872, 3505873, 611231750]));
		expect(extracted.soundRules.kc9998.hasDynamicVoiceIds).toBe(true);
		expect(extracted.soundRules.kc9997.voiceIds).toEqual([701]);
		expect(extracted.soundRules.kc9997.coverageMode).toBe("partial");
		expect(extracted.soundRules.kc9997.hasDynamicVoiceIds).toBe(true);
	});

	test("captures EnemyVoiceConst tables as kc9998 coverage", () => {
		const graph = makeGraph([
			makeModule({
				id: "enemy-voice-const",
				readableName: "EnemyVoiceConst",
				source: wrapModule(`
					EnemyVoiceConst._getVoiceIDs = function(scene, battle, ship) {
						if (3 == areaId && 5 == mapNo) {
							if (1587 == mst_id || 1589 == mst_id) return { "a": [3505871, 3505872], "d": [3505873] };
						}
						return "軽巡ム級" == name ? 611231750 : -1;
					};
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.soundRules.kc9998.voiceIds).toEqual(expect.arrayContaining([3505871, 3505872, 3505873, 611231750]));
		expect(extracted.soundRules.kc9998.coverageMode).toBe("observed-complete");
	});

	test("captures banner-family ship target semantics and slot detail subsets", () => {
		const graph = makeGraph([
			makeModule({
				id: "banner-image",
				readableName: "BannerImage",
				source: wrapModule(`
					function BannerImage() {}
					BannerImage.prototype._getTexture = function() {
						if (2 == this._damaged || 1 == this._taihi) return resources.getShip(this._mst_id, true, "banner_g");
						return resources.getShip(this._mst_id, 0 != this._damaged, "banner");
					};
					BannerImage.prototype._getTextureCombinedFriend = function() {
						if (2 == this._damaged || 1 == this._taihi) return resources.getShip(this._mst_id, true, "banner2_g");
						return resources.getShip(this._mst_id, 0 != this._damaged, "banner2");
					};
					BannerImage.prototype._getTextureCombinedEnemy = function() {
						if (2 == this._damaged || 1 == this._taihi) return resources.getShip(this._mst_id, true, "banner3_g", enemyBreak);
						return resources.getShip(this._mst_id, 0 != this._damaged, "banner3", enemyBreak);
					};
				`),
			}),
			makeModule({
				id: "album-const",
				readableName: "AlbumConst",
				source: wrapModule(`
					AlbumConst.ADD_IMAGE_SLOTS = [525, 526];
				`),
			}),
			makeModule({
				id: "slot-detail-task",
				readableName: "TaskShowSlotDetail",
				source: wrapModule(`
					AlbumConst.ADD_IMAGE_SLOTS.includes(slotId) && (slotLoader.add(slotId, "item_up2"), slotLoader.add(slotId, "item_on2"));
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.shipRules.targetSemantics.coverageMode).toBe("observed-complete");
		expect(extracted.shipRules.targetSemantics.cases).toEqual(
			expect.arrayContaining([
				{ rawTargetType: "banner_g", selectorScope: "default-friendly", damagedState: "true", targetTypes: ["banner_g_dmg"] },
				{ rawTargetType: "banner_g", selectorScope: "default-abyssal", damagedState: "true", targetTypes: ["banner_g_dmg"] },
				{ rawTargetType: "banner", selectorScope: "default-abyssal", damagedState: "variable", targetTypes: ["banner"] },
				{ rawTargetType: "banner3_g", selectorScope: "default-abyssal", damagedState: "true", targetTypes: ["banner3_g_dmg"] },
			]),
		);
		expect(extracted.slotRules.itemUp2.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.itemUp2.ids).toEqual([525, 526]);
		expect(extracted.slotRules.itemOn2.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.itemOn2.ids).toEqual([525, 526]);
	});

	test("keeps partial banner-family target semantics fallback-safe", () => {
		const graph = makeGraph([
			makeModule({
				id: "partial-banner-image",
				readableName: "PartialBannerImage",
				source: wrapModule(`
					function PartialBannerImage() {}
					PartialBannerImage.prototype._getTexture = function() {
						return resources.getShip(this._mst_id, true, "banner_g");
					};
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);

		expect(extracted.shipRules.targetSemantics.coverageMode).toBe("partial");
		expect(extracted.shipRules.targetSemantics.cases).toEqual([]);
		expect(extracted.shipRules.targetSemantics.moduleNames).toEqual(["PartialBannerImage"]);
	});

	test("captures special ship cases from decoded conditional branches", () => {
		const graph = makeGraph([
			makeModule({
				id: "special-module",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					var damaged = attacker.isDamaged();
					(571 != shipMstId && 576 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.shipRules.special.coverageMode).toBe("observed-complete");
		expect(extracted.shipRules.special.cases).toEqual([
			{ damaged: false, shipIds: [571, 576] },
		]);
	});

	test("captures item_up normalization and btxt_flat non-enemy rule", () => {
		const graph = makeGraph([
			makeModule({
				id: "slot-loader",
				readableName: "SlotLoader",
				source: wrapModule(`
					exports.ITEMUP_REPLACE = { 1519: 1516, 1520: 1517 };
					function SlotLoader() {
						this.EXCLUDE_RES = [{ type: "item_character", mst_id: 42 }, { type: "item_up", mst_id: 496 }];
					}
					SlotLoader.prototype.add = function(mstId, type) {
						"item_up" == type && (1 == exports.ITEMUP_REPLACE.hasOwnProperty(mstId.toString())
							? mstId = exports.ITEMUP_REPLACE[mstId]
							: mstId > slotConstModule.SlotConst.ENEMY_SLOT_BORDER && (mstId -= slotConstModule.SlotConst.ENEMY_SLOT_BORDER));
						return mstId;
					};
				`),
			}),
			makeModule({
				id: "slot-const",
				readableName: "SlotConst",
				source: wrapModule(`
					SlotConst.ENEMY_SLOT_BORDER = 1500;
				`),
			}),
			makeModule({
				id: "btxt-module",
				source: wrapModule(`
					if (1 == this._night && 0 == slotUtilModule.SlotUtil.isEnemyItem(this._slot1.mstID)) {
						slotLoader.add(this._slot1.mstID, "btxt_flat");
					}
				`),
			}),
		]);

		const extracted = extractCacheRules(graph);
		expect(extracted.slotRules.itemUp.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.itemUp.enemySlotBorder).toBe(1500);
		expect(extracted.slotRules.itemUp.replaceMap).toEqual({
			"1519": 1516,
			"1520": 1517,
		});
		expect(extracted.slotRules.itemUp.exclude).toEqual([
			{ mstId: 496, type: "item_up" },
		]);
		expect(extracted.slotRules.btxtFlat.coverageMode).toBe("observed-complete");
		expect(extracted.slotRules.btxtFlat.excludeEnemyItems).toBe(true);
	});

	test("converts extracted rules into a unified cache-rules asset", () => {
		const graph = makeGraph([
			makeModule({
				id: "special-module",
				source: wrapModule(`
					var shipMstId = attacker.mst_id;
					var damaged = attacker.isDamaged();
					(541 != shipMstId || 0 != damaged)
						? shipLoader.add(shipMstId, damaged, "full")
						: shipLoader.add(shipMstId, false, "special");
				`),
			}),
			makeModule({
				id: "slot-const",
				readableName: "SlotConst",
				source: wrapModule(`SlotConst.ENEMY_SLOT_BORDER = 1500;`),
			}),
			makeModule({
				id: "slot-loader",
				readableName: "SlotLoader",
				source: wrapModule(`
					exports.ITEMUP_REPLACE = { 1519: 1516 };
					function SlotLoader() {
						this.EXCLUDE_RES = [{ type: "item_up", mst_id: 496 }];
					}
				`),
			}),
			makeModule({
				id: "banner-image",
				source: wrapModule(`resources.getShip(shipId, true, "banner_g");`),
			}),
		]);

		const asset = toCacheRulesAsset("6.2.8.0", extractCacheRules(graph));
		expect(asset.version).toBe(1);
		expect(asset.summary.shipRuleCount).toBe(2);
		expect(asset.summary.slotRuleCount).toBe(4);
		expect(asset.summary.soundRuleCount).toBe(4);
		expect(asset.shipRules.special.cases).toEqual([{ damaged: false, shipIds: [541] }]);
		expect(asset.shipRules.targetSemantics.coverageMode).toBe("partial");
		expect(asset.shipRules.targetSemantics.cases).toEqual([]);
		expect(asset.slotRules.itemUp.enemySlotBorder).toBe(1500);
		expect(asset.soundRules.shipVoices.kind).toBe("ship_voice_formula");
		expect(asset.unresolvedRules).toContain("slotRules.itemUp2");
		expect(asset.unresolvedRules).toContain("slotRules.itemOn2");
		expect(asset.unresolvedRules).toContain("slotRules.btxtFlat");
	});
});
