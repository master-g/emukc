export interface SourcePaths {
  kcConstPath: string;
  mainJsPath: string;
  outputDir: string;
}

export interface LoadedSources {
  paths: SourcePaths;
  kcConstSource: string;
  mainSource: string;
  scriptVersion: string;
}

export interface BundleSections {
  wrapperBangIndex: number;
  helperStartIndex: number;
  decoderFunctionName: string;
  helperFunctionNames: string[];
  prefixSource: string;
  helperSource: string;
  wrapperSource: string;
  decoderRuntimeSource: string;
}

export interface MarkerSummary {
  moduleExportsCount: number;
  esModuleCount: number;
  suffixUtilCount: number;
  definePropertyCount: number;
}

export interface BattleKnowledgeModuleDependency {
  moduleId: string;
  readableName?: string;
}

export interface BattleProtocolFieldRule {
  id: string;
  moduleId: string;
  readableName: string;
  field: string;
  accessKind: "number" | "numArray" | "object" | "objectArray" | "member";
  sourceObject?: string;
  conditional: boolean;
  phases: string[];
}

export interface BattleResourceRule {
  id: string;
  moduleId: string;
  readableName: string;
  resourceKind: "ship" | "slotitem" | "texture-provider" | "explicit-path";
  action: "getShip" | "getSlotitem" | "ship-loader" | "slot-loader" | "getTexture" | "explicit-path";
  targetType?: string;
  provider?: string;
  textureIds: number[];
  shipMstIdSource?: string;
  damagedSource?: string;
  slotMstIdSources: string[];
  explicitPaths: string[];
  triggerHints: string[];
}

export interface BattleSlotResourceTrigger {
  id: string;
  consumerModuleId: string;
  consumerReadableName: string;
  protocolSources: string[];
  resourceTarget: "slot/item_up" | "slot/item_on" | "slot/btxt_flat";
  confidence: "high" | "candidate";
  notes: string;
}

export interface BattleModuleKnowledge {
  id: string;
  readableName: string;
  fileName: string;
  moduleKind: ModuleKind;
  cleanupTier: CleanupTier;
  tags: string[];
  dependencies: BattleKnowledgeModuleDependency[];
  protocolFields: string[];
  resourceRuleIds: string[];
  explicitResourcePaths: string[];
}

export interface BattleKnowledgeSummary {
  moduleCount: number;
  protocolFieldCount: number;
  resourceRuleCount: number;
  slotResourceTriggerCount: number;
  explicitResourcePathCount: number;
  shipResourceRuleCount: number;
  slotitemResourceRuleCount: number;
  textureProviderRuleCount: number;
}

export interface BattleProtocolFieldsAsset {
  scriptVersion: string;
  summary: Pick<BattleKnowledgeSummary, "moduleCount" | "protocolFieldCount">;
  fields: BattleProtocolFieldRule[];
}

export interface BattleResourceRulesAsset {
  scriptVersion: string;
  summary: Pick<
    BattleKnowledgeSummary,
    "moduleCount" | "resourceRuleCount" | "explicitResourcePathCount" | "shipResourceRuleCount" | "slotitemResourceRuleCount" | "textureProviderRuleCount"
  >;
  rules: BattleResourceRule[];
}

export interface BattleModuleIndexAsset {
  scriptVersion: string;
  summary: Pick<BattleKnowledgeSummary, "moduleCount" | "protocolFieldCount" | "resourceRuleCount">;
  modules: BattleModuleKnowledge[];
}

export interface BattleSlotResourceTriggersAsset {
  scriptVersion: string;
  summary: Pick<BattleKnowledgeSummary, "moduleCount" | "slotResourceTriggerCount">;
  triggers: BattleSlotResourceTrigger[];
}

export interface BattleKnowledge {
  summary: BattleKnowledgeSummary;
  protocolFields: BattleProtocolFieldRule[];
  resourceRules: BattleResourceRule[];
  slotResourceTriggers: BattleSlotResourceTrigger[];
  modules: BattleModuleKnowledge[];
}

export interface ModuleDependencySummary {
  moduleId: string;
  readableName?: string;
  localName?: string;
  importStyle: "require" | "wrapped-require";
}

export type ModuleKind = "game" | "helper" | "vendor";
export type CleanupTier = "none" | "named-game" | "priority-body";

export interface ModuleShellMetrics {
  namespaceShellCount: number;
  normalizedNamespaceShellCount: number;
  classShellCount: number;
  normalizedClassShellCount: number;
  structuralTransformCount: number;
}

export interface NamedGameHotspotSummary {
  id: string;
  readableName: string;
  fileName: string;
  hotspotScore: number;
  obfuscatedIdentifierCount: number;
  obfuscatedIdentifierDelta: number;
  structuralTransformCount: number;
}

export interface ModuleHotspotCleanupMetrics {
  beforeObfuscatedIdentifierCount: number;
  afterObfuscatedIdentifierCount: number;
  obfuscatedIdentifierDelta: number;
  localRenameCount: number;
  bodyNormalizationCount: number;
  appliedRules: string[];
}

export interface HotspotCleanupTotals {
  moduleCount: number;
  localRenameCount: number;
  bodyNormalizationCount: number;
  obfuscatedIdentifierDelta: number;
}

export interface HotspotDeltaReportEntry {
  id: string;
  readableName: string;
  fileName: string;
  beforeHotspotScore: number;
  afterHotspotScore: number;
  beforeObfuscatedIdentifierCount: number;
  afterObfuscatedIdentifierCount: number;
  obfuscatedIdentifierDelta: number;
  localRenameCount: number;
  bodyNormalizationCount: number;
}

export interface ModuleArtifact {
  id: string;
  displayId: string;
  fileName: string;
  moduleKind: ModuleKind;
  cleanupTier: CleanupTier;
  readableName?: string;
  exportNames: string[];
  hasDefaultExport: boolean;
  canonicalParameterNames: string[];
  rawObfuscatedIdentifierCount: number;
  transformedObfuscatedIdentifierCount: number;
  obfuscatedIdentifierDelta: number;
  hotspotScore?: number;
  hotspotCleanup?: ModuleHotspotCleanupMetrics;
  shellMetrics: ModuleShellMetrics;
  lineCount: number;
  dependencies: ModuleDependencySummary[];
  source: string;
}

export interface ModuleGraphSummary {
  moduleCount: number;
  modulesWithNamedExports: number;
  modulesWithReadableNames: number;
  moduleKindCounts: Record<ModuleKind, number>;
  totalDependencies: number;
  totalRawObfuscatedIdentifiers: number;
  totalTransformedObfuscatedIdentifiers: number;
  totalObfuscatedIdentifierDelta: number;
  shellMetrics: ModuleShellMetrics;
  namedModulesPreview: Array<{
    id: string;
    readableName: string;
    fileName: string;
  }>;
  topObfuscatedModules: Array<{
    id: string;
    moduleKind: ModuleKind;
    readableName?: string;
    fileName: string;
    obfuscatedIdentifierCount: number;
  }>;
  topObfuscatedGameModules: Array<{
    id: string;
    readableName?: string;
    fileName: string;
    obfuscatedIdentifierCount: number;
  }>;
  topStructuralTransformModules: Array<{
    id: string;
    readableName?: string;
    fileName: string;
    structuralTransformCount: number;
    obfuscatedIdentifierDelta: number;
  }>;
  topNamedGameHotspotsBeforeCleanup: NamedGameHotspotSummary[];
  topNamedGameHotspots: NamedGameHotspotSummary[];
  hotspotCleanupTotals: HotspotCleanupTotals;
  hotspotDeltaReport: HotspotDeltaReportEntry[];
}

export interface ModuleGraph {
  modules: ModuleArtifact[];
  summary: ModuleGraphSummary;
}

export interface DecodeAssessment {
  replacedDecoderCalls: number;
  remainingDecoderCalls: number;
  remainingObfuscatedIdentifiers: number;
  stringDecodeCoveragePercent: number;
  notes: string[];
}

export interface DecodedBundle {
  decodedSource: string;
  aliasCount: number;
  passCount: number;
  cleanupCounts: Record<string, number>;
  markers: MarkerSummary;
  assessment: DecodeAssessment;
}

export interface DecodeOptions {
  maxPasses?: number;
}

export interface PipelineOptions {
  kcConstPath?: string;
  mainJsPath?: string;
  outputDir?: string;
  maxPasses?: number;
  writeOutputs?: boolean;
  syncBattleAssets?: boolean;
  syncResourceManifest?: boolean;
}

export interface PipelineArtifacts {
  versionFile: string;
  decoderRuntimeFile: string;
  wrapperFile: string;
  decodedMainFile: string;
  summaryFile: string;
  moduleGraphFile: string;
  hotspotDeltaReportFile: string;
  battleProtocolFieldsFile: string;
  battleResourceRulesFile: string;
  battleModuleIndexFile: string;
  battleSlotResourceTriggersFile: string;
  modulesDir: string;
}

export interface DecodeSummary {
  scriptVersion: string;
  decoderFunctionName: string;
  helperFunctionNames: string[];
  aliasCount: number;
  passCount: number;
  markers: MarkerSummary;
  assessment: DecodeAssessment;
  moduleGraph: ModuleGraphSummary;
  battleKnowledge: BattleKnowledgeSummary;
  inputPaths: {
    kcConstPath: string;
    mainJsPath: string;
  };
}

export interface ResourceManifestSummary {
  totalEntries: number;
  shipEntryCount: number;
  slotitemEntryCount: number;
  textureProviderEntryCount: number;
  explicitPathEntryCount: number;
  totalExplicitPaths: number;
  modulesCovered: number;
}

export interface PipelineResult {
  loaded: LoadedSources;
  sections: BundleSections;
  decoded: DecodedBundle;
  moduleGraph: ModuleGraph;
  battleKnowledge: BattleKnowledge;
  resourceManifestSummary?: ResourceManifestSummary;
  summary: DecodeSummary;
  artifacts?: PipelineArtifacts;
}
