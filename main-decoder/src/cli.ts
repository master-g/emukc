import { DEFAULT_KCS_CONST_PATH, DEFAULT_MAIN_JS_PATH, DEFAULT_MAX_PASSES, DEFAULT_OUTPUT_DIR } from "./defaults.ts";
import { runDecodePipeline } from "./pipeline.ts";
import type { PipelineOptions } from "./types.ts";

function printHelp(): void {
  console.log(`Usage: bun run decode -- [options]

Options:
  --const <path>       Path to kcs_const.js
  --main <path>        Path to main.js
  --out <path>         Output directory
  --max-passes <n>     Maximum decode passes (default: ${DEFAULT_MAX_PASSES})
  --sync-assets        Sync decoder coverage assets and battle assets into ../crates/emukc_bootstrap/assets
  --sync-battle-assets Sync battle JSON assets into ../crates/emukc_bootstrap/assets
  --sync-resource-manifest Extract resource manifest from all modules and sync
  --no-write           Do not write output artifacts
  --help               Show this help message

Defaults:
  --const ${DEFAULT_KCS_CONST_PATH}
  --main  ${DEFAULT_MAIN_JS_PATH}
  --out   ${DEFAULT_OUTPUT_DIR}`);
}

function parsePositiveInteger(rawValue: string, flagName: string): number {
  const parsed = Number.parseInt(rawValue, 10);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`${flagName} expects a positive integer, received: ${rawValue}`);
  }
  return parsed;
}

export function parseArgs(args: string[]): PipelineOptions & { help?: boolean } {
  const options: PipelineOptions & { help?: boolean } = {};

  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];

    switch (argument) {
      case "--const": {
        const value = args[index + 1];
        if (value === undefined) {
          throw new Error("--const requires a path");
        }
        options.kcConstPath = value;
        index += 1;
        break;
      }
      case "--main": {
        const value = args[index + 1];
        if (value === undefined) {
          throw new Error("--main requires a path");
        }
        options.mainJsPath = value;
        index += 1;
        break;
      }
      case "--out": {
        const value = args[index + 1];
        if (value === undefined) {
          throw new Error("--out requires a path");
        }
        options.outputDir = value;
        index += 1;
        break;
      }
      case "--max-passes": {
        const value = args[index + 1];
        if (value === undefined) {
          throw new Error("--max-passes requires a number");
        }
        options.maxPasses = parsePositiveInteger(value, "--max-passes");
        index += 1;
        break;
      }
      case "--sync-battle-assets":
        options.syncBattleAssets = true;
        break;
      case "--sync-assets":
        options.syncAssets = true;
        options.syncBattleAssets = true;
        options.syncResourceManifest = true;
        break;
      case "--sync-resource-manifest":
        options.syncResourceManifest = true;
        break;
      case "--no-write":
        options.writeOutputs = false;
        break;
      case "--help":
      case "-h":
        options.help = true;
        break;
      default:
        throw new Error(`Unknown argument: ${argument}`);
    }
  }

  return options;
}

function formatMarkerStatus(label: string, count: number): string {
  return `${label}: ${count}`;
}

export async function runCli(args: string[] = Bun.argv.slice(2)): Promise<void> {
  const options = parseArgs(args);
  if (options.help) {
    printHelp();
    return;
  }

  const result = await runDecodePipeline(options);

  console.log(`Decoded main.js version ${result.summary.scriptVersion}`);
  console.log(`Decoder helper: ${result.summary.decoderFunctionName}`);
  console.log(`Helper functions: ${result.summary.helperFunctionNames.join(", ")}`);
  console.log(`Decoder aliases: ${result.summary.aliasCount}`);
  console.log(`Decode passes: ${result.summary.passCount}`);
  console.log(`Replaced decoder calls: ${result.summary.assessment.replacedDecoderCalls}`);
  console.log(`Remaining decoder calls: ${result.summary.assessment.remainingDecoderCalls}`);
  console.log(`Remaining _0x identifiers: ${result.summary.assessment.remainingObfuscatedIdentifiers}`);
  console.log(`String decode coverage: ${result.summary.assessment.stringDecodeCoveragePercent}%`);
  console.log(`Extracted modules: ${result.summary.moduleGraph.moduleCount}`);
  console.log(`Modules with readable names: ${result.summary.moduleGraph.modulesWithReadableNames}`);
  console.log(`Modules with named exports: ${result.summary.moduleGraph.modulesWithNamedExports}`);
  console.log(`Game modules: ${result.summary.moduleGraph.moduleKindCounts.game}`);
  console.log(`Helper modules: ${result.summary.moduleGraph.moduleKindCounts.helper}`);
  console.log(`Vendor modules: ${result.summary.moduleGraph.moduleKindCounts.vendor}`);
  console.log(
    `Namespace shells normalized: ${result.summary.moduleGraph.shellMetrics.normalizedNamespaceShellCount}/${result.summary.moduleGraph.shellMetrics.namespaceShellCount}`,
  );
  console.log(
    `Class shells normalized: ${result.summary.moduleGraph.shellMetrics.normalizedClassShellCount}/${result.summary.moduleGraph.shellMetrics.classShellCount}`,
  );
  console.log(`Structural shell transforms: ${result.summary.moduleGraph.shellMetrics.structuralTransformCount}`);
  console.log(`Module _0x delta: ${result.summary.moduleGraph.totalObfuscatedIdentifierDelta}`);
  console.log(`Battle knowledge modules: ${result.summary.battleKnowledge.moduleCount}`);
  console.log(`Battle protocol fields: ${result.summary.battleKnowledge.protocolFieldCount}`);
  console.log(`Battle resource rules: ${result.summary.battleKnowledge.resourceRuleCount}`);
  console.log(`Decoder ship id-sets resolved: ${result.summary.decoderCoverageAssets.shipIdSetResolvedCount}`);
  console.log(`Decoder ship id-sets unresolved: ${result.summary.decoderCoverageAssets.shipIdSetUnresolvedCount}`);
  console.log(`Decoder slot id-sets resolved: ${result.summary.decoderCoverageAssets.slotIdSetResolvedCount}`);
  console.log(`Decoder slot id-sets unresolved: ${result.summary.decoderCoverageAssets.slotIdSetUnresolvedCount}`);
  console.log(`Decoder SE ids: ${result.summary.decoderCoverageAssets.seIdCount}`);
  console.log(`Decoder port BGM ids: ${result.summary.decoderCoverageAssets.portBgmIdCount}`);
  console.log(`Decoder battle BGM ids: ${result.summary.decoderCoverageAssets.battleBgmIdCount}`);
  console.log(`Decoder tutorial voice stems: ${result.summary.decoderCoverageAssets.tutorialVoiceStemCount}`);
  console.log(`Decoder map default files: ${result.summary.decoderCoverageAssets.mapDefaultFileCount}`);
  console.log(`Decoder map event files: ${result.summary.decoderCoverageAssets.mapEventFileCount}`);
  console.log(`Decoder useitem card ids: ${result.summary.decoderCoverageAssets.useItemCardIdCount}`);
  console.log(`Decoder useitem underline ids: ${result.summary.decoderCoverageAssets.useItemUnderlineIdCount}`);
  if (result.summary.moduleGraph.hotspotCleanupTotals.moduleCount > 0) {
    console.log(`Hotspot local renames: ${result.summary.moduleGraph.hotspotCleanupTotals.localRenameCount}`);
    console.log(`Hotspot body normalizations: ${result.summary.moduleGraph.hotspotCleanupTotals.bodyNormalizationCount}`);
    console.log(`Hotspot _0x delta: ${result.summary.moduleGraph.hotspotCleanupTotals.obfuscatedIdentifierDelta}`);
  }
  if (result.summary.moduleGraph.topNamedGameHotspots.length > 0) {
    const hotspotPreview = result.summary.moduleGraph.topNamedGameHotspots
      .slice(0, 3)
      .map(hotspot => `${hotspot.readableName}(${hotspot.hotspotScore})`)
      .join(", ");
    console.log(`Top named game hotspots: ${hotspotPreview}`);
  }
  if (result.summary.moduleGraph.hotspotDeltaReport.length > 0) {
    const hotspotDeltaPreview = result.summary.moduleGraph.hotspotDeltaReport
      .slice(0, 3)
      .map(hotspot => `${hotspot.readableName}(-${hotspot.obfuscatedIdentifierDelta}, r${hotspot.localRenameCount}, b${hotspot.bodyNormalizationCount})`)
      .join(", ");
    console.log(`Hotspot delta preview: ${hotspotDeltaPreview}`);
  }
  console.log(formatMarkerStatus("module.exports", result.summary.markers.moduleExportsCount));
  console.log(formatMarkerStatus("__esModule", result.summary.markers.esModuleCount));
  console.log(formatMarkerStatus("SuffixUtil", result.summary.markers.suffixUtilCount));
  console.log(formatMarkerStatus("Object.defineProperty", result.summary.markers.definePropertyCount));
  for (const note of result.summary.assessment.notes) {
    console.log(`- ${note}`);
  }

  if (result.artifacts !== undefined) {
    console.log(`Artifacts written to: ${result.loaded.paths.outputDir}`);
    if (options.syncAssets === true) {
      console.log("Decoder coverage assets synced to: ../crates/emukc_bootstrap/assets");
    }
    if (options.syncBattleAssets === true) {
      console.log("Battle assets synced to: ../crates/emukc_bootstrap/assets");
    }
    if (options.syncResourceManifest === true && result.resourceManifestSummary !== undefined) {
      const s = result.resourceManifestSummary;
      console.log(`Resource manifest synced: ${s.totalEntries} entries (${s.shipEntryCount} ship, ${s.slotitemEntryCount} slotitem, ${s.textureProviderEntryCount} texture, ${s.totalExplicitPaths} explicit paths) from ${s.modulesCovered} modules`);
    }
  }
}

if (import.meta.main) {
  await runCli();
}
