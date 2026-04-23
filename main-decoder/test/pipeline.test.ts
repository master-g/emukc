import { expect, test } from "bun:test";
import { mkdtempSync, existsSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { runDecodePipeline } from "../src/pipeline.ts";

test("decodes the current cached main.js", async () => {
  const result = await runDecodePipeline({ writeOutputs: false });

  expect(result.summary.scriptVersion).toMatch(/^\d+\.\d+\.\d+\.\d+$/);
  expect(result.summary.markers.moduleExportsCount).toBeGreaterThan(0);
  expect(result.summary.markers.esModuleCount).toBeGreaterThan(0);
  expect(result.summary.markers.suffixUtilCount).toBeGreaterThan(0);
  expect(result.summary.markers.definePropertyCount).toBeGreaterThan(0);
  expect(result.summary.assessment.remainingDecoderCalls).toBe(0);
  expect(result.summary.assessment.stringDecodeCoveragePercent).toBe(100);
  expect(result.summary.moduleGraph.moduleCount).toBeGreaterThan(1000);
  expect(result.summary.battleKnowledge.moduleCount).toBeGreaterThan(20);
  expect(result.summary.battleKnowledge.protocolFieldCount).toBeGreaterThan(10);
  expect(result.summary.battleKnowledge.resourceRuleCount).toBeGreaterThan(10);
  expect(result.summary.moduleGraph.modulesWithReadableNames).toBeGreaterThanOrEqual(1917);
  expect(result.summary.moduleGraph.moduleKindCounts.game).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.moduleKindCounts.helper).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.shellMetrics.namespaceShellCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.shellMetrics.normalizedNamespaceShellCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.shellMetrics.classShellCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.shellMetrics.normalizedClassShellCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.shellMetrics.structuralTransformCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.totalObfuscatedIdentifierDelta).toBeGreaterThan(116397);
  expect(
    result.summary.moduleGraph.moduleKindCounts.game
    + result.summary.moduleGraph.moduleKindCounts.helper
    + result.summary.moduleGraph.moduleKindCounts.vendor,
  ).toBe(result.summary.moduleGraph.moduleCount);
  expect(result.summary.moduleGraph.topObfuscatedGameModules.length).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.topStructuralTransformModules.length).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.topNamedGameHotspotsBeforeCleanup.length).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.topNamedGameHotspots.length).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.hotspotCleanupTotals.moduleCount).toBeGreaterThan(9);
  expect(result.summary.moduleGraph.hotspotCleanupTotals.localRenameCount).toBeGreaterThan(382);
  expect(result.summary.moduleGraph.hotspotCleanupTotals.bodyNormalizationCount).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.hotspotCleanupTotals.obfuscatedIdentifierDelta).toBeGreaterThan(1275);
  expect(result.summary.moduleGraph.hotspotDeltaReport.length).toBeGreaterThan(0);
  expect(result.summary.moduleGraph.topNamedGameHotspotsBeforeCleanup.some(module => module.readableName === "DutyModel_")).toBe(true);
  expect(result.summary.moduleGraph.topNamedGameHotspots.some(module => module.readableName === "DutyModel_")).toBe(true);
  expect(result.summary.moduleGraph.topNamedGameHotspots.some(module => module.readableName === "ShipChoiceView")).toBe(true);
  expect(result.summary.moduleGraph.hotspotDeltaReport.some(module => module.readableName === "ShipChoiceView" && module.bodyNormalizationCount > 0)).toBe(true);
  expect(result.summary.moduleGraph.hotspotDeltaReport.some(module => module.readableName === "PhaseHougeki" && module.localRenameCount > 0)).toBe(true);

  const suffixUtil = result.moduleGraph.modules.find(module => module.readableName === "SuffixUtil");
  const portApi = result.moduleGraph.modules.find(module => module.readableName === "PortAPI");
  const shipChoiceView = result.moduleGraph.modules.find(module => module.readableName === "ShipChoiceView");
  const shipBanner = result.moduleGraph.modules.find(module => module.readableName === "ShipBanner");
  const cutinPreload = result.moduleGraph.modules.find(module => module.readableName === "CutinResourcesPreloadTask");
  const dutyModel = result.moduleGraph.modules.find(module => module.id === "56360");
  const phaseHougeki = result.moduleGraph.modules.find(module => module.id === "65622");
  expect(result.battleKnowledge.protocolFields.some(field => field.field === "api_stage_flag")).toBe(true);
  expect(result.battleKnowledge.protocolFields.some(field => field.field === "api_kouku")).toBe(true);
  expect(result.battleKnowledge.resourceRules.some(rule => rule.action === "getShip" && rule.targetType === "banner")).toBe(true);
  expect(result.battleKnowledge.resourceRules.some(rule => rule.action === "getShip" && rule.targetType === "full")).toBe(true);
  expect(result.battleKnowledge.resourceRules.some(rule => rule.action === "getSlotitem" && rule.targetType === "item_up")).toBe(true);
  expect(suffixUtil?.moduleKind).toBe("helper");
  expect(suffixUtil?.shellMetrics.namespaceShellCount).toBeGreaterThan(0);
  expect(suffixUtil?.shellMetrics.normalizedNamespaceShellCount).toBeGreaterThan(0);
  expect(suffixUtil?.source).toContain("var __createBinding = this && this.__createBinding");
  expect(suffixUtil?.source).toContain("__importStar = this && this.__importStar");
  expect(suffixUtil?.source).toContain("(function(SuffixUtil) {");
  expect(dutyModel?.moduleKind).toBe("game");
  expect(dutyModel?.cleanupTier).toBe("priority-body");
  expect(dutyModel?.moduleKind).toBe("game");
  expect(dutyModel?.cleanupTier).toBe("priority-body");
  expect(dutyModel?.hotspotCleanup?.obfuscatedIdentifierDelta).toBeGreaterThan(0);
  expect(dutyModel?.hotspotCleanup?.appliedRules).toContain("param-rename");
  expect(dutyModel?.source).toContain("var rawProgress = this._getRawProgress()");
  expect(dutyModel?.source).toContain("unsetSlotCount = this._getUnsetSlotCount");
  expect(dutyModel?.source).toContain("clist = this._clist");
  expect(phaseHougeki?.moduleKind).toBe("game");
  expect(phaseHougeki?.cleanupTier).toBe("priority-body");
  expect(phaseHougeki?.hotspotCleanup?.localRenameCount).toBeGreaterThan(0);
  expect(phaseHougeki?.hotspotCleanup?.appliedRules).toContain("legacy-sequence-if-split");
  expect(phaseHougeki?.source).toContain("var self = this,");
  expect(phaseHougeki?.source).toContain("aShip = this._getAShip");
  expect(phaseHougeki?.source).toContain("daihatsuEffectType = battleCommonModule.BattleCommon.getDaihatsuEffectType");
  expect(phaseHougeki?.source).toContain("scene = this._scene");
  expect(phaseHougeki?.source).toMatch(/\.start\(function\(\)/);
  expect(shipBanner?.source).toContain("resources.getShip");
  expect(cutinPreload?.source).toContain("ShipLoader");
  expect(shipChoiceView?.cleanupTier).toBe("priority-body");
  expect(shipChoiceView?.shellMetrics.classShellCount).toBeGreaterThan(0);
  expect(shipChoiceView?.shellMetrics.normalizedClassShellCount).toBeGreaterThan(0);
  expect(shipChoiceView?.shellMetrics.structuralTransformCount).toBeGreaterThan(0);
  expect(shipChoiceView?.hotspotScore).toBeGreaterThan(0);
  expect(shipChoiceView?.hotspotCleanup?.bodyNormalizationCount).toBeGreaterThan(0);
  expect(shipChoiceView?.hotspotCleanup?.appliedRules).toContain("sequence-expression-split");
  expect(shipChoiceView?.source).toContain("exports.ShipChoiceView = ShipChoiceView;");
  expect(shipChoiceView?.source).toContain("ShipChoiceView = function(baseCtor) {");
  expect(shipChoiceView?.source).toMatch(/function ShipChoiceView\(_0x[0-9a-fA-F]+, japanese, _0x[0-9a-fA-F]+, onFilter\)/);
  expect(shipChoiceView?.source).toContain("var self = baseCtor.call(this) || this;");
  expect(shipChoiceView?.source).toContain("self.FILTER_TAB_NUM = 8;");
  expect(shipChoiceView?.source).toContain("self.ITEM_NUM = 10;");
  expect(shipChoiceView?.source).toContain("self.onFilter = onFilter");
  expect(shipChoiceView?.source).toMatch(/null == _0x[0-9a-fA-F]+ \|\| _0x[0-9a-fA-F]+\.length != self\.FILTER_TAB_NUM/);
  expect(shipChoiceView?.source).toContain("self._japanese = japanese");
  expect(shipChoiceView?.source).toMatch(/self\.onFilter\(_0x[0-9a-fA-F]+, self\._eventFilter\.filter_status\)/);
  expect(shipChoiceView?.source).toMatch(/self\.updateSelectAll\(_0x[0-9a-fA-F]+\)/);
  expect(shipChoiceView?.source).toContain("var listItem = this.listItems[");
  expect(shipChoiceView?.source).toContain("var texture = organizeFilterModule.ORGANIZE_FILTER.getTexture");
  expect(shipChoiceView?.source).toContain("__extends(ShipChoiceView, baseCtor);");
  expect(shipChoiceView?.source).toContain("return ShipChoiceView;");
  expect(portApi?.moduleKind).toBe("game");
});

test("writes resource manifest as a normal output artifact when requested", async () => {
  const outputDir = mkdtempSync(join(tmpdir(), "emukc-decoder-"));
  const result = await runDecodePipeline({
    outputDir,
    writeOutputs: true,
    emitResourceManifest: true,
  } as any);

  expect(result.artifacts?.resourceManifestFile).toBeDefined();
  expect(existsSync(result.artifacts!.resourceManifestFile)).toBe(true);

  const manifest = JSON.parse(readFileSync(result.artifacts!.resourceManifestFile, "utf8"));
  expect(manifest.version).toBeGreaterThanOrEqual(2);
  expect(Array.isArray(manifest.entries)).toBe(true);
  expect(existsSync(result.artifacts!.resourceCategoriesFile)).toBe(true);
  expect(existsSync(result.artifacts!.resourceIdSetsFile)).toBe(true);
  expect(existsSync(result.artifacts!.audioResourcesFile)).toBe(true);
  expect(existsSync(result.artifacts!.cacheRulesFile)).toBe(true);
  expect(existsSync(result.artifacts!.uiResourcesFile)).toBe(true);

  const resourceIdSets = JSON.parse(readFileSync(result.artifacts!.resourceIdSetsFile, "utf8"));
  const audioResources = JSON.parse(readFileSync(result.artifacts!.audioResourcesFile, "utf8"));
  const cacheRules = JSON.parse(readFileSync(result.artifacts!.cacheRulesFile, "utf8"));
  const uiResources = JSON.parse(readFileSync(result.artifacts!.uiResourcesFile, "utf8"));
  expect(resourceIdSets.version).toBe(1);
  expect(audioResources.version).toBe(1);
  expect(cacheRules.version).toBe(1);
  expect(uiResources.version).toBe(1);
}, 120000);
