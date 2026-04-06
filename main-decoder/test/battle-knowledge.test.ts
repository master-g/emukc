import { expect, test } from "bun:test";

import { extractBattleKnowledge } from "../src/battle-knowledge.ts";
import type { ModuleArtifact, ModuleGraph } from "../src/types.ts";

function createModule(overrides: Partial<ModuleArtifact> & Pick<ModuleArtifact, "id" | "fileName" | "moduleKind" | "cleanupTier" | "source">): ModuleArtifact {
  return {
    id: overrides.id,
    displayId: overrides.id,
    fileName: overrides.fileName,
    moduleKind: overrides.moduleKind,
    cleanupTier: overrides.cleanupTier,
    readableName: overrides.readableName,
    exportNames: overrides.exportNames ?? [],
    hasDefaultExport: overrides.hasDefaultExport ?? false,
    canonicalParameterNames: overrides.canonicalParameterNames ?? [],
    rawObfuscatedIdentifierCount: 0,
    transformedObfuscatedIdentifierCount: 0,
    obfuscatedIdentifierDelta: 0,
    shellMetrics: overrides.shellMetrics ?? {
      namespaceShellCount: 0,
      normalizedNamespaceShellCount: 0,
      classShellCount: 0,
      normalizedClassShellCount: 0,
      structuralTransformCount: 0,
    },
    lineCount: 1,
    dependencies: overrides.dependencies ?? [],
    source: overrides.source,
    hotspotScore: overrides.hotspotScore,
    hotspotCleanup: overrides.hotspotCleanup,
  };
}

function createGraph(modules: ModuleArtifact[]): ModuleGraph {
  return {
    modules,
    summary: {
      moduleCount: modules.length,
      modulesWithNamedExports: 0,
      modulesWithReadableNames: modules.filter(module => module.readableName !== undefined).length,
      moduleKindCounts: {
        game: modules.filter(module => module.moduleKind === "game").length,
        helper: modules.filter(module => module.moduleKind === "helper").length,
        vendor: modules.filter(module => module.moduleKind === "vendor").length,
      },
      totalDependencies: 0,
      totalRawObfuscatedIdentifiers: 0,
      totalTransformedObfuscatedIdentifiers: 0,
      totalObfuscatedIdentifierDelta: 0,
      shellMetrics: {
        namespaceShellCount: 0,
        normalizedNamespaceShellCount: 0,
        classShellCount: 0,
        normalizedClassShellCount: 0,
        structuralTransformCount: 0,
      },
      namedModulesPreview: [],
      topObfuscatedModules: [],
      topObfuscatedGameModules: [],
      topStructuralTransformModules: [],
      topNamedGameHotspotsBeforeCleanup: [],
      topNamedGameHotspots: [],
      hotspotCleanupTotals: {
        moduleCount: 0,
        localRenameCount: 0,
        bodyNormalizationCount: 0,
        obfuscatedIdentifierDelta: 0,
      },
      hotspotDeltaReport: [],
    },
  };
}

test("extracts battle protocol fields from raw day battle modules", () => {
  const graph = createGraph([
    createModule({
      id: "83034",
      fileName: "module-83034-raw-day-battle-data.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: "RawDayBattleData",
      source: `function(module, exports, require) {
        var objUtilModule = require(1);
        function RawDayBattleData(o) { this._o = o; }
        Object.defineProperty(RawDayBattleData.prototype, "stage_flag", { get: function() { return objUtilModule.ObjUtil.getNumArray(this._o, "api_stage_flag"); }});
        Object.defineProperty(RawDayBattleData.prototype, "air_war", { get: function() { var object = objUtilModule.ObjUtil.getObject(this._o, "api_kouku"); return object; }});
        Object.defineProperty(RawDayBattleData.prototype, "hougeki1", { get: function() { var objectArray = objUtilModule.ObjUtil.getObjectArray(this._o, "api_hougeki1"); return objectArray; }});
      }`,
    }),
  ]);

  const knowledge = extractBattleKnowledge(graph);

  expect(knowledge.summary.protocolFieldCount).toBe(3);
  expect(knowledge.protocolFields.map(field => field.field)).toEqual([
    "api_hougeki1",
    "api_kouku",
    "api_stage_flag",
  ]);
  expect(knowledge.protocolFields.find(field => field.field === "api_kouku")?.accessKind).toBe("object");
});

test("extracts ship and slotitem resource rules from preload and banner modules", () => {
  const graph = createGraph([
    createModule({
      id: "37638",
      fileName: "module-37638-ship-banner.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: "ShipBanner",
      source: `function(module, exports, require) {
        var commonMiscModule = require(1);
        function ShipBanner() {}
        ShipBanner.prototype.updateImage = function(shipMstId, damaged) {
          this._image.texture = gameData.resources.getShip(shipMstId, damaged, "banner");
          this._fallback.texture = commonMiscModule.COMMON_MISC.getTexture(6);
        };
      }`,
    }),
    createModule({
      id: "58441",
      fileName: "module-58441-cutin-resources-preload-task.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: "CutinResourcesPreloadTask",
      source: `function(module, exports, require) {
        function CutinResourcesPreloadTask() {}
        CutinResourcesPreloadTask.prototype.getShipTexture = function() {
          return gameData.resources.getShip(this._ship_mst_id, this._ship_damaged, "full");
        };
        CutinResourcesPreloadTask.prototype._getSlotTexture = function(slotMstId) {
          return gameData.resources.getSlotitem(slotMstId, "item_up");
        };
        CutinResourcesPreloadTask.prototype._loadShipImage = function() {
          var loader = new shipLoaderModule.ShipLoader();
          loader.add(this._ship_mst_id, this._ship_damaged, "full");
        };
        CutinResourcesPreloadTask.prototype._addLoadTask = function(loader, slotMstId) {
          loader.add(slotMstId, "item_on");
          loader.add(slotMstId, "btxt_flat");
        };
      }`,
    }),
  ]);

  const knowledge = extractBattleKnowledge(graph);
  const resourceRuleIds = knowledge.resourceRules.map(rule => rule.id);

  expect(knowledge.summary.shipResourceRuleCount).toBeGreaterThanOrEqual(2);
  expect(knowledge.summary.slotitemResourceRuleCount).toBeGreaterThanOrEqual(1);
  expect(resourceRuleIds.some(id => id.includes("getShip") && id.includes("banner"))).toBe(true);
  expect(resourceRuleIds.some(id => id.includes("getShip") && id.includes("full"))).toBe(true);
  expect(resourceRuleIds.some(id => id.includes("getSlotitem") && id.includes("item_up"))).toBe(true);
  expect(knowledge.resourceRules.find(rule => rule.provider === "COMMON_MISC")?.textureIds).toEqual([6]);
});

test("collects slot resource triggers for cutin slot text consumers", () => {
  const graph = createGraph([
    createModule({
      id: "69595",
      fileName: "module-69595-cutin-canvas-sp-rdj.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: "CutinCanvasSpRDJ",
      source: `function(module, exports, require) {
        function CutinCanvasSpRDJ() {}
        CutinCanvasSpRDJ.prototype.update = function(slotMstId) {
          this._name1.texture = gameData.resources.getSlotitem(slotMstId, "btxt_flat");
          this._item1.texture = gameData.resources.getSlotitem(slotMstId, "item_up");
        };
      }`,
    }),
  ]);

  const knowledge = extractBattleKnowledge(graph);

  expect(knowledge.summary.slotResourceTriggerCount).toBeGreaterThanOrEqual(2);
  expect(knowledge.slotResourceTriggers.some(trigger => {
    return trigger.consumerReadableName === "CutinCanvasSpRDJ"
      && trigger.resourceTarget === "slot/btxt_flat"
      && trigger.protocolSources.includes("api_hougeki1.api_si_list[*][*]");
  })).toBe(true);
});
