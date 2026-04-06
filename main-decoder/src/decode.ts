import { DEFAULT_MAX_PASSES } from "./defaults.ts";
import type { BundleSections, DecodeAssessment, DecodedBundle, DecodeOptions, MarkerSummary } from "./types.ts";

const ALIAS_ASSIGNMENT_RE = /(?:var|,)\s*(_0x[0-9a-fA-F]+)\s*=\s*(_0x[0-9a-fA-F]+)(?=[,;])/g;
const DECODER_CALL_RE = /\b(_0x[0-9a-fA-F]+)\(\s*(0x[0-9a-fA-F]+|\d+)\s*\)/g;

type DecoderFunction = (value: number | string) => string;

interface CleanupTransform {
  name: string;
  apply(source: string): { source: string; changes: number };
}

function replaceAndCount(
  source: string,
  pattern: RegExp,
  replacement: string | ((match: string, ...args: string[]) => string),
): { source: string; changes: number } {
  let changes = 0;
  const nextSource = source.replace(pattern, (match, ...args) => {
    changes += 1;
    if (typeof replacement === "string") {
      return replacement;
    }
    return replacement(match, ...(args.slice(0, -2) as string[]));
  });

  return { source: nextSource, changes };
}

const CLEANUP_TRANSFORMS: CleanupTransform[] = [
  {
    name: "indexer-parens",
    apply(source) {
      return replaceAndCount(source, /\[\((['"])([^"'\\]*(?:\\.[^"'\\]*)*)\1\)\]/g, (_match, quote, value) => {
        return `[${quote}${value}${quote}]`;
      });
    },
  },
  {
    name: "identifier-dot-access",
    apply(source) {
      return replaceAndCount(source, /(\b[$A-Z_a-z][$\w]*)\[['"]([$A-Z_a-z][$\w]*)['"]\](?![$\w])/g, (_match, target, property) => {
        return `${target}.${property}`;
      });
    },
  },
  {
    name: "closed-expression-dot-access",
    apply(source) {
      return replaceAndCount(source, /([\])'}])\s*\[['"]([$A-Z_a-z][$\w]*)['"]\](?![$\w])/g, (_match, target, property) => {
        return `${target}.${property}`;
      });
    },
  },
  {
    name: "regex-dot-access",
    apply(source) {
      return replaceAndCount(
        source,
        /((?:\/[^/\\\n]*(?:\\.[^/\\\n]*)*\/[dgimsuvy]*))\s*\[['"]([$A-Z_a-z][$\w]*)['"]\](?![$\w])/g,
        (_match, target, property) => {
          return `${target}.${property}`;
        },
      );
    },
  },
  {
    name: "hex-literals",
    apply(source) {
      return replaceAndCount(source, /([ ([\-!])0x([0-9a-fA-F]+)/g, (_match, prefix, hex) => {
        return `${prefix}${Number.parseInt(hex, 16)}`;
      });
    },
  },
  {
    name: "true-literal",
    apply(source) {
      return replaceAndCount(source, /(?<![$\w])!0(?=\W)/g, "true");
    },
  },
  {
    name: "false-literal",
    apply(source) {
      return replaceAndCount(source, /(?<![$\w])!1(?=\W)/g, "false");
    },
  },
  {
    name: "undefined-literal",
    apply(source) {
      return replaceAndCount(source, /(?<![$\w])void 0(?=\W)/g, "undefined");
    },
  },
  {
    name: "toString-index",
    apply(source) {
      return replaceAndCount(source, /([([])(\d+)\.toString\(\)/g, (_match, prefix, value) => {
        return `${prefix}["${value}"]`;
      });
    },
  },
];

function createDecoder(sections: BundleSections): DecoderFunction {
  const factory = new Function(`${sections.decoderRuntimeSource}; return ${sections.decoderFunctionName};`);
  const decoderCandidate = factory();
  if (typeof decoderCandidate !== "function") {
    throw new Error(`Decoder helper ${sections.decoderFunctionName} did not evaluate to a function`);
  }

  return (value: number | string) => {
    const decodedValue = decoderCandidate(value);
    if (typeof decodedValue !== "string") {
      throw new Error(`Decoder helper ${sections.decoderFunctionName} returned a non-string value`);
    }
    return decodedValue;
  };
}

function collectDecoderAliases(source: string, decoderFunctionName: string): Set<string> {
  const aliases = new Set<string>([decoderFunctionName]);
  let aliasesExpanded = true;

  while (aliasesExpanded) {
    aliasesExpanded = false;
    ALIAS_ASSIGNMENT_RE.lastIndex = 0;

    for (const match of source.matchAll(ALIAS_ASSIGNMENT_RE)) {
      const assignedAlias = match[1];
      const assignedValue = match[2];
      if (assignedAlias === undefined || assignedValue === undefined) {
        continue;
      }

      if (aliases.has(assignedValue) && !aliases.has(assignedAlias)) {
        aliases.add(assignedAlias);
        aliasesExpanded = true;
      }
    }
  }

  return aliases;
}

function stripAliasDeclarations(source: string, aliases: Set<string>): string {
  const withoutStandaloneAliases = source.replace(
    /var\s+(_0x[0-9a-fA-F]+)\s*=\s*(_0x[0-9a-fA-F]+)\s*;\s*/g,
    (match, assignedAlias: string, assignedValue: string) => {
      if (aliases.has(assignedAlias) && aliases.has(assignedValue)) {
        return "";
      }
      return match;
    },
  );

  return withoutStandaloneAliases.replace(
    /var\s+(_0x[0-9a-fA-F]+)\s*=\s*(_0x[0-9a-fA-F]+)\s*,\s*/g,
    (match, assignedAlias: string, assignedValue: string) => {
      if (aliases.has(assignedAlias) && aliases.has(assignedValue)) {
        return "var ";
      }
      return match;
    },
  );
}

function countOccurrences(source: string, needle: string): number {
  let count = 0;
  let fromIndex = 0;

  while (true) {
    const nextIndex = source.indexOf(needle, fromIndex);
    if (nextIndex === -1) {
      return count;
    }

    count += 1;
    fromIndex = nextIndex + needle.length;
  }
}

function countRemainingDecoderCalls(source: string, aliases: Set<string>): number {
  let remainingCalls = 0;
  DECODER_CALL_RE.lastIndex = 0;

  for (const match of source.matchAll(DECODER_CALL_RE)) {
    const callee = match[1];
    if (callee !== undefined && aliases.has(callee)) {
      remainingCalls += 1;
    }
  }

  return remainingCalls;
}

function countObfuscatedIdentifiers(source: string): number {
  return [...source.matchAll(/\b_0x[0-9a-fA-F]+\b/g)].length;
}

function summarizeMarkers(source: string): MarkerSummary {
  return {
    moduleExportsCount: countOccurrences(source, "module.exports"),
    esModuleCount: countOccurrences(source, "__esModule"),
    suffixUtilCount: countOccurrences(source, "SuffixUtil"),
    definePropertyCount: countOccurrences(source, "Object.defineProperty"),
  };
}

function buildAssessment(
  replacedDecoderCalls: number,
  remainingDecoderCalls: number,
  remainingObfuscatedIdentifiers: number,
  markers: MarkerSummary,
): DecodeAssessment {
  const totalDecoderCalls = replacedDecoderCalls + remainingDecoderCalls;
  const stringDecodeCoveragePercent =
    totalDecoderCalls === 0 ? 100 : Number(((replacedDecoderCalls / totalDecoderCalls) * 100).toFixed(2));
  const notes: string[] = [];

  if (remainingDecoderCalls === 0) {
    notes.push("String-table decoder calls were fully eliminated from the emitted bundle.");
  } else {
    notes.push(`Some string-table decoder calls remain (${remainingDecoderCalls}), so the output is only partially decoded.`);
  }

  if (markers.moduleExportsCount > 0 && markers.esModuleCount > 0) {
    notes.push("Webpack/UMD module structure is visible again in the decoded output.");
  }

  if (markers.suffixUtilCount > 0) {
    notes.push("Game-specific symbols such as SuffixUtil are readable again.");
  }

  if (remainingObfuscatedIdentifiers > 10000) {
    notes.push("Many local identifiers still use _0x... names, so this is string-level deobfuscation rather than full symbolic recovery.");
  }

  return {
    replacedDecoderCalls,
    remainingDecoderCalls,
    remainingObfuscatedIdentifiers,
    stringDecodeCoveragePercent,
    notes,
  };
}

export function decodeBundle(sections: BundleSections, options: DecodeOptions = {}): DecodedBundle {
  const maxPasses = options.maxPasses ?? DEFAULT_MAX_PASSES;
  const decoder = createDecoder(sections);
  const aliases = collectDecoderAliases(sections.wrapperSource, sections.decoderFunctionName);
  const cleanupCounts: Record<string, number> = {};
  let replacedDecoderCalls = 0;

  let decodedSource = sections.wrapperSource;
  let completedPasses = 0;

  for (let pass = 0; pass < maxPasses; pass += 1) {
    let passChanged = false;

    decodedSource = decodedSource.replace(DECODER_CALL_RE, (match, callee: string, argument: string) => {
      if (!aliases.has(callee)) {
        return match;
      }

      passChanged = true;
      replacedDecoderCalls += 1;
      return JSON.stringify(decoder(argument));
    });

    for (const transform of CLEANUP_TRANSFORMS) {
      const result = transform.apply(decodedSource);
      decodedSource = result.source;

      if (result.changes > 0) {
        passChanged = true;
        cleanupCounts[transform.name] = (cleanupCounts[transform.name] ?? 0) + result.changes;
      }
    }

    completedPasses = pass + 1;
    if (!passChanged) {
      break;
    }
  }

  decodedSource = stripAliasDeclarations(decodedSource, aliases);
  const markers = summarizeMarkers(decodedSource);
  const assessment = buildAssessment(
    replacedDecoderCalls,
    countRemainingDecoderCalls(decodedSource, aliases),
    countObfuscatedIdentifiers(decodedSource),
    markers,
  );

  return {
    decodedSource,
    aliasCount: aliases.size,
    passCount: completedPasses,
    cleanupCounts,
    markers,
    assessment,
  };
}
