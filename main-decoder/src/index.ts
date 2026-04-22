export { DEFAULT_KCS_CONST_PATH, DEFAULT_MAIN_JS_PATH, DEFAULT_MAX_PASSES, DEFAULT_OUTPUT_DIR } from "./defaults.ts";
export {
  extractBattleKnowledge,
  toBattleModuleIndexAsset,
  toBattleProtocolFieldsAsset,
  toBattleResourceRulesAsset,
  toBattleSlotResourceTriggersAsset,
} from "./battle-knowledge.ts";
export { decodeBundle } from "./decode.ts";
export { formatJavaScript } from "./format.ts";
export { extractScriptVersion, loadLocalSources, resolveSourcePaths } from "./io.ts";
export { extractModuleGraph } from "./module-graph.ts";
export { extractResourceCategories, toResourceCategoriesAsset } from "./resource-categories.ts";
export { extractResourceManifest } from "./resource-manifest.ts";
export type { ResourceManifest, ResourceManifestEntry, ResourceManifestShipEntry, ResourceManifestSlotitemEntry, ResourceManifestTextureProviderEntry, ResourceManifestExplicitPathEntry } from "./resource-manifest.ts";
export { runDecodePipeline } from "./pipeline.ts";
export { splitBundle } from "./split.ts";
export type {
  BattleKnowledge,
  BattleKnowledgeModuleDependency,
  BattleKnowledgeSummary,
  BattleModuleIndexAsset,
  BattleModuleKnowledge,
  BattleProtocolFieldRule,
  BattleProtocolFieldsAsset,
  BattleResourceRule,
  BattleResourceRulesAsset,
  BattleSlotResourceTrigger,
  BattleSlotResourceTriggersAsset,
  BundleSections,
  CleanupTier,
  DecodeAssessment,
  DecodeOptions,
  DecodedBundle,
  DecodeSummary,
  HotspotCleanupTotals,
  HotspotDeltaReportEntry,
  LoadedSources,
  MarkerSummary,
  ModuleArtifact,
  ModuleDependencySummary,
  ModuleGraph,
  ModuleGraphSummary,
  ModuleHotspotCleanupMetrics,
  ModuleKind,
  ModuleShellMetrics,
  NamedGameHotspotSummary,
  PipelineArtifacts,
  PipelineOptions,
  PipelineResult,
  ResourceCategoriesAsset,
  ResourceCategoryEntry,
  SourcePaths,
} from "./types.ts";
