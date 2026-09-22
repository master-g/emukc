import { expect, test } from "bun:test";

import { extractBattleAttackTypeStages, extractBattleKnowledge } from "../src/battle-knowledge.ts";
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

// ---------------------------------------------------------------------------
// Attack-type acceptance (R2)
// ---------------------------------------------------------------------------

/**
 * The shape the real bundle uses: a nested ternary on `record.type`, ending in
 * a fallback method. `d_indexes[0]` and `getSlotitem(1)` put numbers in the
 * body that must never reach the acceptance set.
 */
function phaseHougekiSource(name: string, chain: string, fallbackBody: string): string {
  return `function(module, exports, require) {
    var danchakuModule = require(90992);
    var ${name} = function() {
      function ${name}() {}
      ${name}.prototype._hougeki = function(record) {
        var type = record.type;
        ${chain}
      };
      ${name}.prototype._normal = function(record) {
        var dShip = this._getDShip(record.d_indexes[0], record.flag);
        var slot = record.getSlotitem(1);
      };
      ${name}.prototype._double = function(record) {};
      ${name}.prototype._kuboCI = function(record) {};
      ${name}.prototype._special = function(record) {
        ${fallbackBody}
      };
      return ${name};
    }();
    exports.${name} = ${name};
  }`;
}

const DELEGATING_FALLBACK = `
  var aShip = this._getAShip(record.a_index, record.flag);
  new danchakuModule.PhaseAttackDanchaku(this._scene, record.type, aShip, record.getSlotitem(0)).start();
`;

const DANCHAKU_SOURCE = `function(module, exports, require) {
  var PhaseAttackDanchaku = function() {
    function PhaseAttackDanchaku(scene, type, attacker, slot) {
      var self = this;
      if (self._slot = slot, 3 == type) self._cutin = new CutinDanchaku1();
      else if (4 == type) self._cutin = new CutinDanchaku2();
      else if (200 == type) self._cutin = new CutinDanchaku1(1);
      else {
        if (201 != type) throw new Error();
        self._cutin = new CutinDanchaku1(2);
      }
      return self;
    }
    return PhaseAttackDanchaku;
  }();
  exports.PhaseAttackDanchaku = PhaseAttackDanchaku;
}`;

function dispatcherSource(name: string, hougekiModuleId: number): string {
  return `function(module, exports, require) {
    var hougekiModule = require(${hougekiModuleId});
    exports.${name} = function() {};
  }`;
}

function attackTypeGraph(overrides: { nightDispatcherName?: string; dayAlsoRequiresNightHougeki?: boolean } = {}): ModuleGraph {
  return createGraph([
    createModule({
      id: "90992",
      fileName: "module-90992-phase-attack-danchaku.js",
      moduleKind: "game",
      cleanupTier: "priority-body",
      readableName: "PhaseAttackDanchaku",
      source: DANCHAKU_SOURCE,
    }),
    createModule({
      id: "1830",
      fileName: "module-1830-phase-hougeki.js",
      moduleKind: "game",
      cleanupTier: "priority-body",
      readableName: "PhaseHougeki",
      source: phaseHougekiSource(
        "PhaseHougeki",
        "0 == type ? this._normal(record) : 2 == type ? this._double(record) : 7 == type ? this._kuboCI(record) : this._special(record);",
        DELEGATING_FALLBACK,
      ),
      dependencies: [{
        moduleId: "90992",
        readableName: "PhaseAttackDanchaku",
        localName: "danchakuModule",
        importStyle: "require",
      }],
    }),
    createModule({
      id: "74885",
      fileName: "module-74885-phase-hougeki.js",
      moduleKind: "game",
      cleanupTier: "priority-body",
      readableName: "PhaseHougeki",
      source: phaseHougekiSource(
        "PhaseHougeki",
        "0 == type ? this._normal(record) : 1 == type ? this._double(record) : 6 == type ? this._kuboCI(record) : this._special(record);",
        `
          var type = record.type;
          var phase;
          if (2 == type) phase = new SpSR();
          else if (3 == type) phase = new SpRR();
          if (null == phase) throw new Error();
        `,
      ),
    }),
    createModule({
      id: "16599",
      fileName: "module-16599-phase-pre-anti-submarine.js",
      moduleKind: "game",
      cleanupTier: "priority-body",
      readableName: "PhasePreAntiSubmarine",
      source: phaseHougekiSource(
        "PhasePreAntiSubmarine",
        "0 == type ? this._normal(record) : 2 == type ? this._double(record) : this._special(record);",
        DELEGATING_FALLBACK,
      ),
      dependencies: [{
        moduleId: "90992",
        readableName: "PhaseAttackDanchaku",
        localName: "danchakuModule",
        importStyle: "require",
      }],
    }),
    createModule({
      id: "16718",
      fileName: "module-16718-phase-day.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: "PhaseDay",
      source: dispatcherSource("PhaseDay", 1830),
      dependencies: [
        { moduleId: "1830", readableName: "PhaseHougeki", importStyle: "require" },
        { moduleId: "16599", readableName: "PhasePreAntiSubmarine", importStyle: "require" },
        ...(overrides.dayAlsoRequiresNightHougeki === true
          ? [{ moduleId: "74885", readableName: "PhaseHougeki", importStyle: "require" as const }]
          : []),
      ],
    }),
    createModule({
      id: "27665",
      fileName: "module-27665-phase-night.js",
      moduleKind: "game",
      cleanupTier: "named-game",
      readableName: overrides.nightDispatcherName ?? "PhaseNight",
      source: dispatcherSource("PhaseNight", 74885),
      dependencies: [{ moduleId: "74885", readableName: "PhaseHougeki", importStyle: "require" }],
    }),
  ]);
}

test("extracts the attack types each battle stage dispatches", () => {
  const stages = extractBattleAttackTypeStages(attackTypeGraph());
  const day = stages.find(stage => stage.id === "day-shelling");

  expect(day?.acceptedValues).toEqual([0, 2, 7]);
  // `d_indexes[0]` and `getSlotitem(1)` sit in the module body; neither index
  // is an attack type.
  expect(day?.acceptedValues).not.toContain(1);
  expect(day?.consumerModuleIds).toEqual(["1830"]);
  expect(day?.protocolField).toBe("api_at_type");
});

test("merges a delegating fallback's closed set into the stage's effective set", () => {
  const stages = extractBattleAttackTypeStages(attackTypeGraph());
  const opening = stages.find(stage => stage.id === "opening-anti-submarine");

  expect(opening?.acceptedValues).toEqual([0, 2]);
  expect(opening?.fallback?.readableName).toBe("PhaseAttackDanchaku");
  // The `201 != type` guard in front of the throw accepts 201, and the throw
  // itself is what makes the set closed.
  expect(opening?.fallback?.acceptedValues).toEqual([3, 4, 200, 201]);
  expect(opening?.fallback?.closed).toBe(true);
  expect(opening?.effectiveAcceptedValues).toEqual([0, 2, 3, 4, 200, 201]);
  expect(opening?.effectiveAcceptedValues).not.toContain(7);
});

test("merges a fallback the consumer guards itself", () => {
  const stages = extractBattleAttackTypeStages(attackTypeGraph());
  const night = stages.find(stage => stage.id === "night-shelling");

  expect(night?.fallback?.readableName).toBe("PhaseHougeki._special");
  expect(night?.fallback?.acceptedValues).toEqual([2, 3]);
  expect(night?.fallback?.closed).toBe(true);
  expect(night?.notes).toContain("battle_slot_resource_triggers.json");
});

test("tells the day and night PhaseHougeki apart by their consumers", () => {
  const stages = extractBattleAttackTypeStages(attackTypeGraph());
  const day = stages.find(stage => stage.id === "day-shelling");
  const night = stages.find(stage => stage.id === "night-shelling");

  // Same readableName, mutually exclusive meanings: 2 is 連撃 by day and
  // 主砲魚雷 by night, 7 is 空母カットイン by day and 潜水艦系 by night.
  expect(day?.acceptedValues).toContain(2);
  expect(night?.acceptedValues).not.toContain(2);
  expect(night?.acceptedValues).toContain(1);
  expect(night?.protocolField).toBe("api_sp_list");
  expect(day?.consumerModuleIds).not.toEqual(night?.consumerModuleIds);
});

test("refuses to guess when the consumers do not disambiguate a stage", () => {
  // Day and night dispatchers both requiring the same PhaseHougeki: the
  // consumer rule no longer decides, and picking either copy would silently
  // produce the wrong acceptance set.
  const graph = attackTypeGraph({ dayAlsoRequiresNightHougeki: true });

  expect(() => extractBattleAttackTypeStages(graph)).toThrow(/cannot disambiguate battle side/);
});

test("refuses to merge same-named consumers that disagree on a stage", () => {
  const graph = attackTypeGraph({ nightDispatcherName: "PhaseDayFromNight" });

  expect(() => extractBattleAttackTypeStages(graph)).toThrow(/disagree on api_at_type/);
});
