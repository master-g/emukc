import type { ResourceManifest } from "./resource-manifest.ts";

export interface SourcePaths {
  kcConstPath: string;
  mainJsPath: string;
  worldJsPath: string;
  outputDir: string;
}

export interface LoadedSources {
  paths: SourcePaths;
  kcConstSource: string;
  mainSource: string;
  worldSource?: string;
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
  worldJsPath?: string;
  outputDir?: string;
  maxPasses?: number;
  writeOutputs?: boolean;
  emitResourceManifest?: boolean;
  syncAssets?: boolean;
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
  resourcesDir: string;
  resourceCategoriesFile: string;
  resourceIdSetsFile: string;
  audioResourcesFile: string;
  cacheRulesFile: string;
  uiResourcesFile: string;
  resourceTemplatesFile: string;
  resourceManifestFile?: string;
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
  decoderCoverageAssets: {
    shipIdSetResolvedCount: number;
    shipIdSetUnresolvedCount: number;
    slotIdSetResolvedCount: number;
    slotIdSetUnresolvedCount: number;
    seIdCount: number;
    portBgmIdCount: number;
    battleBgmIdCount: number;
    tutorialVoiceStemCount: number;
    mapDefaultFileCount: number;
    mapEventFileCount: number;
    useItemCardIdCount: number;
    useItemUnderlineIdCount: number;
    templateFamilyCount: number;
  };
  inputPaths: {
    kcConstPath: string;
    mainJsPath: string;
    worldJsPath: string;
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

export type ResourceCategorySource =
  | "resources.getShip"
  | "ShipLoader.add"
  | "resources.getSlotitem"
  | "SlotLoader.add"
  | "explicit-path";

export interface ResourceCategoryEntry {
  source: ResourceCategorySource;
  targetType: string;
  moduleIds: string[];
  moduleNames: string[];
}

export interface ResourceCategoriesAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  summary: {
    shipTargetTypeCount: number;
    slotTargetTypeCount: number;
    spRemodelSubcategoryCount: number;
    shipGenerationGroupCount: number;
    slotGenerationGroupCount: number;
  };
  shipTargetTypes: ResourceCategoryEntry[];
  slotTargetTypes: ResourceCategoryEntry[];
  shipGenerationGroups: {
    defaultFriendly: string[];
    defaultAbyssal: string[];
    friendGraph: string[];
    enemyGraph: string[];
  };
  slotGenerationGroups: {
    default: string[];
    baga: string[];
    airunit: string[];
  };
  spRemodelSubcategories: string[];
}

export type ResourceCoverageMode = "observed-complete" | "partial" | "unresolved";

export type ResourceTemplateBlockerKind =
  | "missing-descriptor-evidence"
  | "partial-coverage"
  | "unavailable-runtime-input"
  | "uncovered-residual-membership";

export type ResourceTemplateDomain =
  | "map"
  | "gauge"
  | "furniture"
  | "bgm"
  | "sound"
  | "voice"
  | "useitem"
  | "area"
  | "worldselect"
  | "se";

export type ResourceTemplateInput =
  | "manifest.mapinfo"
  | "manifest.mapbgm"
  | "manifest.bgm"
  | "manifest.furniture"
  | "manifest.useitem"
  | "cache-source.sound-bucket"
  | "decoder.audio"
  | "decoder.ui"
  | "decoder.template-range";

export interface ResourceTemplateLiteralSegment {
  kind: "literal";
  value: string;
}

export interface ResourceTemplatePlaceholderSegment {
  kind: "placeholder";
  name: string;
  format?: "number" | "pad2" | "pad3" | "raw";
}

export type ResourceTemplateSegment =
  | ResourceTemplateLiteralSegment
  | ResourceTemplatePlaceholderSegment;

export interface ResourceTemplateRange {
  start: number;
  end: number;
  pad?: number;
}

export interface ResourceTemplateProvenance {
  moduleIds: string[];
  moduleNames: string[];
}

export interface ResourceTemplateCompletenessBlocker {
  kind: ResourceTemplateBlockerKind;
  reason: string;
  requiredInputs?: ResourceTemplateInput[];
}

export interface ResourceTemplateFamily {
  key: string;
  domain: ResourceTemplateDomain;
  outputPrefix: string;
  pathTemplate: ResourceTemplateSegment[];
  requiredInputs: ResourceTemplateInput[];
  coverageMode: ResourceCoverageMode;
  provenance: ResourceTemplateProvenance;
  completenessBlockers?: ResourceTemplateCompletenessBlocker[];
  range?: ResourceTemplateRange;
}

export interface ResourceTemplatesAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  summary: {
    familyCount: number;
    observedCompleteFamilyCount: number;
    partialFamilyCount: number;
    unresolvedFamilyCount: number;
  };
  families: ResourceTemplateFamily[];
  unresolvedFamilies: string[];
}

export interface ResourceIdSetEntry {
  coverageMode: ResourceCoverageMode;
  ids: number[];
  moduleIds: string[];
  moduleNames: string[];
}

export interface ResourceIdSetsAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  coverageMode: "mainjs-observed";
  summary: {
    shipCategoryCount: number;
    slotitemCategoryCount: number;
    resolvedCategoryCount: number;
    unresolvedCategoryCount: number;
  };
  shipIdSets: {
    specialShips: ResourceIdSetEntry;
    spRemodelShips: ResourceIdSetEntry;
    spRemodelMessageShips: ResourceIdSetEntry;
    cardRoundShips: ResourceIdSetEntry;
    rewardShips: ResourceIdSetEntry;
  };
  slotitemIdSets: {
    btxtFlatIds: ResourceIdSetEntry;
    itemUpIds: ResourceIdSetEntry;
  };
  unresolvedKeys: string[];
}

export interface CacheRuleProvenance {
  moduleIds: string[];
  moduleNames: string[];
}

export interface CacheRuleSpecialCase {
  damaged: boolean;
  shipIds: number[];
}

export interface CacheRuleSpecialShipRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "special_cases";
  cases: CacheRuleSpecialCase[];
}

export type CacheRuleShipSelectorScope = "default-friendly" | "default-abyssal";
export type CacheRuleDamagedState = "false" | "true" | "variable";

export interface CacheRuleShipTargetSemanticCase {
  rawTargetType: string;
  selectorScope: CacheRuleShipSelectorScope;
  damagedState: CacheRuleDamagedState;
  targetTypes: string[];
}

export interface CacheRuleShipTargetSemanticsRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "ship_target_semantics";
  cases: CacheRuleShipTargetSemanticCase[];
}

export interface CacheRuleItemUpRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "item_up_normalization";
  replaceMap: Record<string, number>;
  enemySlotBorder?: number;
  exclude: Array<{ type: string; mstId: number }>;
}

export interface CacheRuleBtxtFlatRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "btxt_flat_non_enemy_runtime_slots";
  excludeEnemyItems: boolean;
}

export interface CacheRuleObservedSlotSubsetRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "observed_slot_subset";
  ids: number[];
}

export interface CacheRuleShipVoiceFormula {
  base: number;
  multiplier: number;
  shipIdOffset: number;
  modulo: number;
  maxFormulaVoiceId: number;
  voiceDiffs: number[];
}

export interface CacheRuleShipVoiceRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "ship_voice_formula";
  formula?: CacheRuleShipVoiceFormula;
  requiredShipGraphFields: string[];
  baseVoiceIds: number[];
  beLeftVoiceIds: number[];
  beLeftTiredVoiceIds: number[];
  timeSignalStartVoiceId?: number;
  timeSignalVoiceCount?: number;
  specialArtShipIds: number[];
  specialVoiceIds: number[];
}

export interface CacheRuleSoundBucketRule extends CacheRuleProvenance {
  coverageMode: ResourceCoverageMode;
  kind: "sound_bucket";
  bucket: "9997" | "9998" | "9999";
  voiceIds: number[];
  hasDynamicVoiceIds: boolean;
}

export interface CacheRulesAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  summary: {
    shipRuleCount: number;
    slotRuleCount: number;
    soundRuleCount: number;
    observedCompleteRuleCount: number;
    partialRuleCount: number;
    unresolvedRuleCount: number;
  };
  resourceManifest: ResourceManifest;
  resourceCategories: ResourceCategoriesAsset;
  shipRules: {
    special: CacheRuleSpecialShipRule;
    targetSemantics: CacheRuleShipTargetSemanticsRule;
  };
  slotRules: {
    itemUp: CacheRuleItemUpRule;
    btxtFlat: CacheRuleBtxtFlatRule;
    itemUp2: CacheRuleObservedSlotSubsetRule;
    itemOn2: CacheRuleObservedSlotSubsetRule;
  };
  soundRules: {
    shipVoices: CacheRuleShipVoiceRule;
    kc9997: CacheRuleSoundBucketRule;
    kc9998: CacheRuleSoundBucketRule;
    kc9999: CacheRuleSoundBucketRule;
  };
  unresolvedRules: string[];
}

export interface AudioResourceIdGroup {
  coverageMode: ResourceCoverageMode;
  ids: number[];
}

export interface AudioResourcesAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  summary: {
    seIdCount: number;
    portBgmIdCount: number;
    battleBgmIdCount: number;
    fanfareBgmIdCount: number;
    tutorialVoiceStemCount: number;
    explicitPathCount: number;
  };
  seIds: AudioResourceIdGroup;
  bgm: {
    fanfareIds: AudioResourceIdGroup;
    portIds: AudioResourceIdGroup;
    battleIds: AudioResourceIdGroup;
  };
  voice: {
    titlecallCategories: string[];
    tutorialVoiceStems: string[];
    explicitFiles: string[];
  };
  explicitPaths: string[];
}

export interface UiResourcePathGroup {
  coverageMode: ResourceCoverageMode;
  files: string[];
}

export interface UiResourceIdGroup {
  coverageMode: ResourceCoverageMode;
  ids: string[];
}

export interface UiResourcesAsset {
  version: 1;
  generatedAt: string;
  scriptVersion: string;
  summary: {
    mapDefaultFileCount: number;
    mapEventFileCount: number;
    furnitureCategoryCount: number;
    useItemCardIdCount: number;
    useItemUnderlineIdCount: number;
    areaSallyIdCount: number;
    areaAirunitIdCount: number;
    worldSelectFileCount: number;
  };
  map: {
    defaultFiles: UiResourcePathGroup;
    eventFiles: UiResourcePathGroup;
  };
  furniture: {
    categories: string[];
    explicitPaths: string[];
  };
  useItem: {
    cardIds: UiResourceIdGroup;
    underlineIds: UiResourceIdGroup;
  };
  area: {
    sallyIds: UiResourceIdGroup;
    airunitIds: UiResourceIdGroup;
    airunitExtendConfirmIds: UiResourceIdGroup;
  };
  worldSelect: {
    files: string[];
  };
}

export interface PipelineResult {
  loaded: LoadedSources;
  sections: BundleSections;
  decoded: DecodedBundle;
  moduleGraph: ModuleGraph;
  battleKnowledge: BattleKnowledge;
  resourceCategories: ResourceCategoriesAsset;
  resourceIdSets: ResourceIdSetsAsset;
  audioResources: AudioResourcesAsset;
  cacheRules: CacheRulesAsset;
  uiResources: UiResourcesAsset;
  resourceTemplates: ResourceTemplatesAsset;
  resourceManifest?: unknown;
  resourceManifestSummary?: ResourceManifestSummary;
  summary: DecodeSummary;
  artifacts?: PipelineArtifacts;
}
