import { resolve } from "node:path";

import {
  extractBattleKnowledge,
  toBattleModuleIndexAsset,
  toBattleProtocolFieldsAsset,
  toBattleResourceRulesAsset,
  toBattleSlotResourceTriggersAsset,
} from "./battle-knowledge.ts";
import { decodeBundle } from "./decode.ts";
import { formatJavaScript } from "./format.ts";
import { loadLocalSources, writeTextFile } from "./io.ts";
import { extractModuleGraph } from "./module-graph.ts";
import { extractResourceCategories, toResourceCategoriesAsset } from "./resource-categories.ts";
import { extractResourceManifest } from "./resource-manifest.ts";
import { splitBundle } from "./split.ts";
import type { PipelineArtifacts, PipelineOptions, PipelineResult } from "./types.ts";

async function writeArtifacts(result: PipelineResult, options: PipelineOptions): Promise<PipelineArtifacts> {
  const { outputDir } = result.loaded.paths;
  const modulesDir = resolve(outputDir, "modules");
  const battleDir = resolve(outputDir, "battle");
  const resourcesDir = resolve(outputDir, "resources");
  const bootstrapAssetsDir = resolve(import.meta.dir, "../../crates/emukc_bootstrap/assets");
  const artifacts: PipelineArtifacts = {
    versionFile: resolve(outputDir, "version.txt"),
    decoderRuntimeFile: resolve(outputDir, "decoder-runtime.js"),
    wrapperFile: resolve(outputDir, "main.bundle.js"),
    decodedMainFile: resolve(outputDir, "main.decoded.js"),
    summaryFile: resolve(outputDir, "summary.json"),
    moduleGraphFile: resolve(modulesDir, "module-graph.json"),
    hotspotDeltaReportFile: resolve(modulesDir, "hotspot-delta-report.json"),
    battleProtocolFieldsFile: resolve(battleDir, "battle_protocol_fields.json"),
    battleResourceRulesFile: resolve(battleDir, "battle_resource_rules.json"),
    battleModuleIndexFile: resolve(battleDir, "battle_module_index.json"),
    battleSlotResourceTriggersFile: resolve(battleDir, "battle_slot_resource_triggers.json"),
    resourcesDir,
    resourceCategoriesFile: resolve(resourcesDir, "resource_categories.json"),
    modulesDir,
  };

  const battleProtocolFieldsAsset = toBattleProtocolFieldsAsset(result.loaded.scriptVersion, result.battleKnowledge);
  const battleResourceRulesAsset = toBattleResourceRulesAsset(result.loaded.scriptVersion, result.battleKnowledge);
  const battleModuleIndexAsset = toBattleModuleIndexAsset(result.loaded.scriptVersion, result.battleKnowledge);
  const battleSlotResourceTriggersAsset = toBattleSlotResourceTriggersAsset(result.loaded.scriptVersion, result.battleKnowledge);

  await writeTextFile(artifacts.versionFile, `${result.loaded.scriptVersion}\n`);
  await writeTextFile(artifacts.decoderRuntimeFile, formatJavaScript(result.sections.decoderRuntimeSource));
  await writeTextFile(artifacts.wrapperFile, formatJavaScript(result.sections.wrapperSource));
  await writeTextFile(artifacts.decodedMainFile, formatJavaScript(result.decoded.decodedSource));
  await writeTextFile(artifacts.summaryFile, `${JSON.stringify(result.summary, null, 2)}\n`);
  await writeTextFile(
    artifacts.hotspotDeltaReportFile,
    `${JSON.stringify({
      beforeCleanup: result.summary.moduleGraph.topNamedGameHotspotsBeforeCleanup,
      afterCleanup: result.summary.moduleGraph.topNamedGameHotspots,
      totals: result.summary.moduleGraph.hotspotCleanupTotals,
      deltaReport: result.summary.moduleGraph.hotspotDeltaReport,
    }, null, 2)}\n`,
  );
  await writeTextFile(
    artifacts.moduleGraphFile,
    `${JSON.stringify({
      summary: result.moduleGraph.summary,
      modules: result.moduleGraph.modules.map(module => ({
        id: module.id,
        displayId: module.displayId,
        fileName: module.fileName,
        moduleKind: module.moduleKind,
        cleanupTier: module.cleanupTier,
        readableName: module.readableName,
        exportNames: module.exportNames,
        hasDefaultExport: module.hasDefaultExport,
        canonicalParameterNames: module.canonicalParameterNames,
        rawObfuscatedIdentifierCount: module.rawObfuscatedIdentifierCount,
        transformedObfuscatedIdentifierCount: module.transformedObfuscatedIdentifierCount,
        obfuscatedIdentifierDelta: module.obfuscatedIdentifierDelta,
        hotspotScore: module.hotspotScore,
        hotspotCleanup: module.hotspotCleanup,
        shellMetrics: module.shellMetrics,
        lineCount: module.lineCount,
        dependencies: module.dependencies,
      })),
    }, null, 2)}\n`,
  );
  await writeTextFile(artifacts.battleProtocolFieldsFile, `${JSON.stringify(battleProtocolFieldsAsset, null, 2)}\n`);
  await writeTextFile(artifacts.battleResourceRulesFile, `${JSON.stringify(battleResourceRulesAsset, null, 2)}\n`);
  await writeTextFile(artifacts.battleModuleIndexFile, `${JSON.stringify(battleModuleIndexAsset, null, 2)}\n`);
  await writeTextFile(artifacts.battleSlotResourceTriggersFile, `${JSON.stringify(battleSlotResourceTriggersAsset, null, 2)}\n`);
  await writeTextFile(artifacts.resourceCategoriesFile, `${JSON.stringify(result.resourceCategories, null, 2)}\n`);
  if (options.syncBattleAssets === true) {
    await writeTextFile(resolve(bootstrapAssetsDir, "battle_protocol_fields.json"), `${JSON.stringify(battleProtocolFieldsAsset, null, 2)}\n`);
    await writeTextFile(resolve(bootstrapAssetsDir, "battle_resource_rules.json"), `${JSON.stringify(battleResourceRulesAsset, null, 2)}\n`);
    await writeTextFile(resolve(bootstrapAssetsDir, "battle_module_index.json"), `${JSON.stringify(battleModuleIndexAsset, null, 2)}\n`);
    await writeTextFile(resolve(bootstrapAssetsDir, "battle_slot_resource_triggers.json"), `${JSON.stringify(battleSlotResourceTriggersAsset, null, 2)}\n`);
  }
  if (options.syncAssets === true) {
    await writeTextFile(resolve(bootstrapAssetsDir, "resource_categories.json"), `${JSON.stringify(result.resourceCategories, null, 2)}\n`);
  }
  if (options.syncResourceManifest === true) {
    const manifest = extractResourceManifest(result.moduleGraph);
    await writeTextFile(resolve(bootstrapAssetsDir, "resource_manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
  }
  await Promise.all(
    result.moduleGraph.modules.map(module => writeTextFile(resolve(modulesDir, module.fileName), module.source)),
  );

  return artifacts;
}

export async function runDecodePipeline(options: PipelineOptions = {}): Promise<PipelineResult> {
  const loaded = await loadLocalSources(options);
  const sections = splitBundle(loaded.mainSource);
  const decoded = decodeBundle(sections, { maxPasses: options.maxPasses });
  const moduleGraph = extractModuleGraph(decoded.decodedSource);
  const battleKnowledge = extractBattleKnowledge(moduleGraph);
  const resourceCategories = toResourceCategoriesAsset(loaded.scriptVersion, extractResourceCategories(moduleGraph));

  const resourceManifest = options.syncResourceManifest === true
    ? extractResourceManifest(moduleGraph)
    : undefined;

  const result: PipelineResult = {
    loaded,
    sections,
    decoded,
    moduleGraph,
    battleKnowledge,
    resourceCategories,
    resourceManifestSummary: resourceManifest?.summary,
    summary: {
      scriptVersion: loaded.scriptVersion,
      decoderFunctionName: sections.decoderFunctionName,
      helperFunctionNames: sections.helperFunctionNames,
      aliasCount: decoded.aliasCount,
      passCount: decoded.passCount,
      markers: decoded.markers,
      assessment: decoded.assessment,
      moduleGraph: moduleGraph.summary,
      battleKnowledge: battleKnowledge.summary,
      inputPaths: {
        kcConstPath: loaded.paths.kcConstPath,
        mainJsPath: loaded.paths.mainJsPath,
      },
    },
  };

  if (options.writeOutputs !== false) {
    result.artifacts = await writeArtifacts(result, options);
  }

  return result;
}
