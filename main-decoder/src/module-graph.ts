import generate from "@babel/generator";
import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import { annotateEnumLiterals } from "./enum-restore.ts";
import { formatJavaScript } from "./format.ts";
import type {
  CleanupTier,
  HotspotCleanupTotals,
  HotspotDeltaReportEntry,
  ModuleArtifact,
  ModuleDependencySummary,
  ModuleGraph,
  ModuleGraphSummary,
  ModuleHotspotCleanupMetrics,
  ModuleKind,
  ModuleShellMetrics,
  NamedGameHotspotSummary,
} from "./types.ts";

const OBFUSCATED_IDENTIFIER_RE = /^_0x[0-9a-fA-F]+$/;
const GAME_NAME_RE = /(API|Animation|Battle|Cell|Choice|Compass|Cutin|Deck|Dialog|Enemy|Event|Expedition|Fleet|Formation|Furniture|Gauge|Gear|Home|Hougeki|Incentive|Item|Kaizo|Layer|Marriage|Material|Mission|Model|Panel|Phase|Plane|Port|Practice|Preset|Quest|Raigeki|Rank|Repair|RequireInfo|Reward|Scene|Shutter|Ship|Slot|Sortie|Supply|Task|Tutorial|Unset|UseItem|User|View)/;
const HELPER_NAME_RE = /(Const|Settings|Util|Utils|Formatter|Parser|Helper)$/;
const GAME_SOURCE_MARKERS = [
  /\bPIXI\b/,
  /\bcreatejs\b/,
  /\bgetTexture\b/,
  /\bEventType\b/,
  /\bAPIBase\b/,
  /\bSceneBase\b/,
  /\bTaskBase\b/,
  /\bUIImageLoader\b/,
  /\b(?:api|mem|mst)_[a-z0-9_]+\b/i,
  /\b(?:ORGANIZE|COMMON|BATTLE|WEDDING|PORT|MAP|SLOT|GAUGE|EVENT)_[A-Z0-9_]+\b/,
] as const;
const VENDOR_SOURCE_MARKERS = [
  /\bSymbol\.(?:iterator|toStringTag|species)\b/,
  /\bReflect\./,
  /\bWeakMap\b/,
  /\bWeakSet\b/,
  /\bPromise\b/,
  /\bArrayBuffer\b/,
  /\bDataView\b/,
  /\bUint8Array\b/,
  /\bFloat32Array\b/,
  /\bURLSearchParams\b/,
  /\bglobalThis\b/,
  /\bDOMException\b/,
] as const;
const TS_HELPER_PARAMETER_NAMES: Partial<Record<string, string[][]>> = {
  __assign: [["target"]],
  __awaiter: [["thisArg", "_arguments", "promiseCtor", "generator"]],
  __createBinding: [
    ["target", "source", "key", "targetKey"],
    ["target", "source", "key", "targetKey"],
  ],
  __decorate: [["decorators", "target", "key", "descriptor"]],
  __extends: [
    ["derivedCtor", "baseCtor"],
    ["derivedCtor", "baseCtor"],
  ],
  __generator: [["thisArg", "body"]],
  __importDefault: [["mod"]],
  __importStar: [["mod"]],
  __metadata: [["metadataKey", "metadataValue"]],
  __read: [["iterable", "count"]],
  __rest: [["source", "excluded"]],
  __setModuleDefault: [
    ["target", "value"],
    ["target", "value"],
  ],
  __spreadArray: [["to", "from", "pack"]],
  __values: [["iterable"]],
};

type ModuleImportStyle = ModuleDependencySummary["importStyle"];

interface DependencyCapture {
  moduleId: string;
  localName?: string;
  importStyle: ModuleImportStyle;
}

interface FirstPassModuleRecord {
  id: string;
  displayId: string;
  rawSource: string;
  firstPassSource: string;
  exportNames: string[];
  readableName?: string;
  hasDefaultExport: boolean;
  canonicalParameterNames: string[];
  rawObfuscatedIdentifierCount: number;
  shellMetrics: ModuleShellMetrics;
  dependencies: DependencyCapture[];
}

interface ModuleClassificationInput {
  readableName?: string;
  exportNames: string[];
  dependencies: ModuleDependencySummary[];
  source: string;
}

interface ReadabilityTransformResult {
  source: string;
  cleanupTier: CleanupTier;
  localRenameCount: number;
  bodyNormalizationCount: number;
  beforeObfuscatedIdentifierCount: number;
  afterObfuscatedIdentifierCount: number;
  hotspotCleanup?: ModuleHotspotCleanupMetrics;
}

type RenameableFunctionPath = NodePath<t.FunctionDeclaration | t.FunctionExpression | t.ArrowFunctionExpression>;
type CleanupRuleName =
  | "enum-annotate"
  | "hex-literals"
  | "param-rename"
  | "local-rename"
  | "sequence-expression-split"
  | "sequence-return-split"
  | "legacy-sequence-if-split";

const PRIORITY_BODY_HOTSPOT_SCORE_THRESHOLD = 250;
const LEGACY_SEQUENCE_IF_SPLIT_TARGETS = new Set([
  "DutyModel_",
  "PhaseHougeki",
  "ShipChoiceView",
  "TaskRewardDialogModelChange",
  "SlotItemEffectUtil",
  "SlotItemFilterView",
  "SlotDisassemblyFilterView",
  "SupplyScene",
]);

function parseFile(source: string): t.File {
  return parse(source, {
    sourceType: "script",
    allowReturnOutsideFunction: true,
  });
}

function isModuleFactoryValue(node: t.Node): node is t.FunctionExpression | t.ArrowFunctionExpression {
  return t.isFunctionExpression(node) || t.isArrowFunctionExpression(node);
}

function isModuleTableObject(node: t.ObjectExpression): boolean {
  if (node.properties.length < 100) {
    return false;
  }

  return node.properties.every(property => {
    return t.isObjectProperty(property) && isModuleFactoryValue(property.value);
  });
}

function extractModuleTable(source: string): t.ObjectExpression {
  const ast = parseFile(source);
  let moduleTable: t.ObjectExpression | undefined;

  traverse(ast, {
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (!t.isObjectExpression(path.node.init)) {
        return;
      }

      if (!isModuleTableObject(path.node.init)) {
        return;
      }

      moduleTable = path.node.init;
      path.stop();
    },
  });

  if (moduleTable === undefined) {
    throw new Error("Failed to locate the webpack module table");
  }

  return moduleTable;
}

function countObfuscatedIdentifiers(source: string): number {
  return [...source.matchAll(/\b_0x[0-9a-fA-F]+\b/g)].length;
}

function countLines(source: string): number {
  if (source.length === 0) {
    return 0;
  }

  return source.split(/\r?\n/).length;
}

function countMarkerHits(source: string, markers: readonly RegExp[]): number {
  let count = 0;

  for (const marker of markers) {
    if (marker.test(source)) {
      count += 1;
    }
  }

  return count;
}

function unwrapTransparentExpression(node: t.Expression): t.Expression {
  if (t.isParenthesizedExpression(node)) {
    return unwrapTransparentExpression(node.expression);
  }

  return node;
}

function extractTsHelperName(node: t.Expression | null | undefined): string | undefined {
  if (node == null) {
    return undefined;
  }

  if (t.isMemberExpression(node) && t.isThisExpression(node.object) && !node.computed && t.isIdentifier(node.property)) {
    return node.property.name.startsWith("__") ? node.property.name : undefined;
  }

  if (t.isLogicalExpression(node)) {
    return extractTsHelperName(node.left) ?? extractTsHelperName(node.right);
  }

  if (t.isAssignmentExpression(node)) {
    return extractTsHelperName(node.right);
  }

  if (t.isSequenceExpression(node)) {
    for (const expression of node.expressions) {
      const helperName = extractTsHelperName(expression);
      if (helperName !== undefined) {
        return helperName;
      }
    }
  }

  if (t.isConditionalExpression(node)) {
    return extractTsHelperName(node.test) ?? extractTsHelperName(node.consequent) ?? extractTsHelperName(node.alternate);
  }

  if (t.isParenthesizedExpression(node)) {
    return extractTsHelperName(node.expression);
  }

  return undefined;
}

function renameHelperParameters(
  helperPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>,
  preferredNames: readonly string[],
): void {
  helperPath.node.params.forEach((parameter, index) => {
    const preferredName = preferredNames[index];
    if (preferredName === undefined || !t.isIdentifier(parameter)) {
      return;
    }

    renameBinding(helperPath, parameter.name, preferredName);
  });
}

function renameKnownTsHelperInternals(
  initPath: NodePath<t.Expression | null | undefined>,
  helperName: string,
): void {
  const signatures = TS_HELPER_PARAMETER_NAMES[helperName];
  if (signatures === undefined) {
    return;
  }

  const remainingSignatures = [...signatures];
  const helperFunctionPaths: Array<NodePath<t.FunctionExpression | t.ArrowFunctionExpression>> = [];

  if (initPath.isFunctionExpression() || initPath.isArrowFunctionExpression()) {
    helperFunctionPaths.push(initPath);
  }

  initPath.traverse({
    FunctionExpression(path: NodePath<t.FunctionExpression>) {
      helperFunctionPaths.push(path);
    },
    ArrowFunctionExpression(path: NodePath<t.ArrowFunctionExpression>) {
      helperFunctionPaths.push(path);
    },
  });

  for (const helperPath of helperFunctionPaths) {
    const signatureIndex = remainingSignatures.findIndex(signature => signature.length === helperPath.node.params.length);
    if (signatureIndex === -1) {
      continue;
    }

    renameHelperParameters(helperPath, remainingSignatures[signatureIndex]!);
    remainingSignatures.splice(signatureIndex, 1);
    if (remainingSignatures.length === 0) {
      break;
    }
  }
}

function renameTsHelperAliases(factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>): void {
  factoryPath.traverse({
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (path.getFunctionParent() !== factoryPath) {
        return;
      }

      if (!t.isIdentifier(path.node.id)) {
        return;
      }

      const helperName = extractTsHelperName(path.node.init);
      if (helperName === undefined) {
        return;
      }

      renameBinding(factoryPath, path.node.id.name, helperName);
      renameKnownTsHelperInternals(path.get("init"), helperName);
    },
  });
}

function slugify(value: string): string {
  return value
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/[^A-Za-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .toLowerCase();
}

function toLowerCamelCase(value: string): string {
  const parts = slugify(value).split("-").filter(Boolean);
  if (parts.length === 0) {
    return value;
  }

  return parts
    .map((part, index) => (index === 0 ? part : `${part[0]!.toUpperCase()}${part.slice(1)}`))
    .join("");
}

function isReadableIdentifier(name: string): boolean {
  return t.isValidIdentifier(name) && !OBFUSCATED_IDENTIFIER_RE.test(name);
}

function isUsefulReadableName(name: string): boolean {
  return isReadableIdentifier(name) && /[A-Za-z]/.test(name);
}

function normalizePropertyName(node: t.Expression | t.PrivateName, computed: boolean): string | undefined {
  if (t.isIdentifier(node) && !computed) {
    return node.name;
  }
  if (t.isStringLiteral(node)) {
    return node.value;
  }
  if (t.isNumericLiteral(node)) {
    return String(node.value);
  }
  return undefined;
}

function normalizeModuleId(node: t.Expression | t.PrivateName): { id: string; displayId: string } {
  if (t.isNumericLiteral(node)) {
    return {
      id: String(node.value),
      displayId: typeof node.extra?.raw === "string" ? node.extra.raw : String(node.value),
    };
  }

  if (t.isStringLiteral(node)) {
    return {
      id: node.value,
      displayId: JSON.stringify(node.value),
    };
  }

  if (t.isIdentifier(node)) {
    return {
      id: node.name,
      displayId: node.name,
    };
  }

  throw new Error(`Unsupported module key type: ${node.type}`);
}

function parseFactoryFile(factorySource: string): t.File {
  return parseFile(`(${factorySource});`);
}

function findFactoryPath(ast: t.File): NodePath<t.FunctionExpression | t.ArrowFunctionExpression> {
  let factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression> | undefined;

  traverse(ast, {
    FunctionExpression(path: NodePath<t.FunctionExpression>) {
      if (factoryPath === undefined) {
        factoryPath = path;
        path.stop();
      }
    },
    ArrowFunctionExpression(path: NodePath<t.ArrowFunctionExpression>) {
      if (factoryPath === undefined) {
        factoryPath = path;
        path.stop();
      }
    },
  });

  if (factoryPath === undefined) {
    throw new Error("Failed to locate the module factory function");
  }

  return factoryPath;
}

function createAvailableName(factoryPath: RenameableFunctionPath, preferredName: string, oldName: string): string {
  if (preferredName === oldName) {
    return preferredName;
  }

  let candidate = preferredName;
  let suffix = 2;

  while (factoryPath.scope.hasOwnBinding(candidate) && candidate !== oldName) {
    candidate = `${preferredName}${suffix}`;
    suffix += 1;
  }

  return candidate;
}

function renameBinding(factoryPath: RenameableFunctionPath, oldName: string, preferredName: string): string {
  const targetName = createAvailableName(factoryPath, preferredName, oldName);
  if (targetName !== oldName) {
    factoryPath.scope.rename(oldName, targetName);
  }
  return targetName;
}

function renameFactoryParameters(factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>): string[] {
  const canonicalNames = ["module", "exports", "require"];

  return factoryPath.node.params.map((parameter: t.FunctionExpression["params"][number], index: number) => {
    if (!t.isIdentifier(parameter)) {
      return `<param-${index}>`;
    }

    const preferredName = canonicalNames[index];
    if (preferredName === undefined) {
      return parameter.name;
    }

    return renameBinding(factoryPath, parameter.name, preferredName);
  });
}

function isExportsMember(node: t.LVal | t.Expression, exportsName: string): node is t.MemberExpression {
  return t.isMemberExpression(node) && t.isIdentifier(node.object, {
    name: exportsName,
  });
}

function isModuleExportsMember(node: t.LVal | t.Expression, moduleName: string): node is t.MemberExpression {
  return t.isMemberExpression(node) && t.isIdentifier(node.object, {
    name: moduleName,
  }) && t.isIdentifier(node.property, {
    name: "exports",
  }) && !node.computed;
}

function extractAssignedIdentifier(node: t.Expression): string | undefined {
  if (t.isIdentifier(node)) {
    return node.name;
  }

  if (t.isAssignmentExpression(node)) {
    if (t.isIdentifier(node.left)) {
      return node.left.name;
    }
    return extractAssignedIdentifier(node.right);
  }

  if ((t.isFunctionExpression(node) || t.isClassExpression(node)) && t.isIdentifier(node.id)) {
    return node.id.name;
  }

  if (t.isSequenceExpression(node) && node.expressions.length > 0) {
    return extractAssignedIdentifier(node.expressions[node.expressions.length - 1]!);
  }

  if (t.isParenthesizedExpression(node)) {
    return extractAssignedIdentifier(node.expression);
  }

  return undefined;
}

function extractReturnedIdentifier(node: t.Expression | null | undefined): string | undefined {
  if (node == null) {
    return undefined;
  }

  const expression = unwrapTransparentExpression(node);
  if (t.isIdentifier(expression)) {
    return expression.name;
  }

  if (t.isSequenceExpression(expression) && expression.expressions.length > 0) {
    return extractReturnedIdentifier(expression.expressions[expression.expressions.length - 1]);
  }

  return extractAssignedIdentifier(expression);
}

function extractNamespaceWrapperInfo(
  node: t.Expression,
  exportsName: string,
): { exportName: string; namespaceBindingName: string } | undefined {
  const expression = unwrapTransparentExpression(node);
  if (!t.isLogicalExpression(expression) || expression.operator !== "||") {
    return undefined;
  }

  const left = unwrapTransparentExpression(expression.left);
  const right = unwrapTransparentExpression(expression.right);
  if (!t.isIdentifier(left) || !t.isAssignmentExpression(right) || right.operator !== "=") {
    return undefined;
  }

  if (!isExportsMember(right.left, exportsName)) {
    return undefined;
  }

  const exportName = normalizePropertyName(right.left.property, right.left.computed);
  if (exportName === undefined || !isReadableIdentifier(exportName)) {
    return undefined;
  }

  const namespaceBindingName = extractAssignedIdentifier(right.right);
  if (namespaceBindingName !== left.name) {
    return undefined;
  }

  return {
    exportName,
    namespaceBindingName,
  };
}

function getReturnedFunctionBindingPath(
  path: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>,
): { returnedIdentifier: string; functionPath: NodePath<t.FunctionDeclaration | t.FunctionExpression | t.ArrowFunctionExpression> } | undefined {
  if (!path.get("body").isBlockStatement()) {
    return undefined;
  }

  let returnedIdentifier: string | undefined;
  path.traverse({
    ReturnStatement(returnPath: NodePath<t.ReturnStatement>) {
      if (returnPath.getFunctionParent() !== path) {
        return;
      }

      const candidate = extractReturnedIdentifier(returnPath.node.argument);
      if (candidate !== undefined) {
        returnedIdentifier = candidate;
      }
    },
  });

  if (returnedIdentifier === undefined) {
    return undefined;
  }

  const binding = path.scope.getBinding(returnedIdentifier);
  if (binding === undefined) {
    return undefined;
  }

  if (binding.path.isFunctionDeclaration()) {
    return {
      returnedIdentifier,
      functionPath: binding.path,
    };
  }

  if (binding.path.isVariableDeclarator()) {
    const initPath = binding.path.get("init");
    if (initPath.isFunctionExpression() || initPath.isArrowFunctionExpression()) {
      return {
        returnedIdentifier,
        functionPath: initPath,
      };
    }
  }

  return undefined;
}

function isBaseCtorSelfAlias(node: t.Expression | null | undefined, baseCtorName: string): boolean {
  if (node == null) {
    return false;
  }

  const expression = unwrapTransparentExpression(node);
  if (!t.isLogicalExpression(expression) || expression.operator !== "||" || !t.isThisExpression(expression.right)) {
    return false;
  }

  const left = unwrapTransparentExpression(expression.left);
  if (!t.isCallExpression(left) || !t.isMemberExpression(left.callee) || left.callee.computed) {
    return false;
  }

  if (!t.isIdentifier(left.callee.object, { name: baseCtorName }) || !t.isIdentifier(left.callee.property, { name: "call" })) {
    return false;
  }

  const [firstArgument] = left.arguments;
  if (firstArgument === undefined || t.isSpreadElement(firstArgument) || !t.isThisExpression(firstArgument)) {
    return false;
  }

  return true;
}

function renameImmediateSelfAlias(
  functionPath: NodePath<t.FunctionDeclaration | t.FunctionExpression | t.ArrowFunctionExpression>,
  baseCtorName: string,
): boolean {
  let renamed = false;

  functionPath.traverse({
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (renamed || path.getFunctionParent() !== functionPath) {
        return;
      }

      if (!t.isIdentifier(path.node.id) || !OBFUSCATED_IDENTIFIER_RE.test(path.node.id.name)) {
        return;
      }

      if (!isBaseCtorSelfAlias(path.node.init, baseCtorName)) {
        return;
      }

      renameBinding(functionPath, path.node.id.name, "self");
      renamed = true;
      path.stop();
    },
  });

  return renamed;
}

function normalizeUnaryIifeExpression(path: NodePath<t.CallExpression>): boolean {
  const parentPath = path.parentPath;
  if (!parentPath.isUnaryExpression({ operator: "!" })) {
    return false;
  }

  if (!parentPath.parentPath.isExpressionStatement()) {
    return false;
  }

  parentPath.replaceWith(path.node);
  return true;
}

function expandReturnedSequenceIntoStatements(
  functionPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>,
  returnedIdentifier: string,
): boolean {
  let expanded = false;

  functionPath.traverse({
    ReturnStatement(returnPath: NodePath<t.ReturnStatement>) {
      if (returnPath.getFunctionParent() !== functionPath) {
        return;
      }

      const argument = returnPath.node.argument;
      if (argument == null) {
        return;
      }

      const expression = unwrapTransparentExpression(argument);
      if (!t.isSequenceExpression(expression) || expression.expressions.length < 2) {
        return;
      }

      const finalExpression = expression.expressions[expression.expressions.length - 1]!;
      if (!t.isIdentifier(finalExpression, { name: returnedIdentifier })) {
        return;
      }

      returnPath.replaceWithMultiple([
        ...expression.expressions.slice(0, -1).map(currentExpression => t.expressionStatement(currentExpression)),
        t.returnStatement(finalExpression),
      ]);
      expanded = true;
      returnPath.stop();
    },
  });

  return expanded;
}

function normalizeNamespaceWrapperShells(
  factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>,
  exportsName: string,
): ModuleShellMetrics {
  const shellMetrics = createEmptyShellMetrics();

  factoryPath.traverse({
    CallExpression(path: NodePath<t.CallExpression>) {
      const calleePath = path.get("callee");
      if (!calleePath.isFunctionExpression() && !calleePath.isArrowFunctionExpression()) {
        return;
      }

      const [firstArgument] = path.node.arguments;
      if (firstArgument === undefined || t.isSpreadElement(firstArgument) || !t.isExpression(firstArgument)) {
        return;
      }

      const namespaceInfo = extractNamespaceWrapperInfo(firstArgument, exportsName);
      if (namespaceInfo === undefined) {
        return;
      }

      const [firstParameter] = calleePath.node.params;
      if (firstParameter === undefined || !t.isIdentifier(firstParameter)) {
        return;
      }

      shellMetrics.namespaceShellCount += 1;

      let normalized = false;
      const originalParameterName = firstParameter.name;
      if (renameBinding(calleePath, originalParameterName, namespaceInfo.exportName) !== originalParameterName) {
        shellMetrics.structuralTransformCount += 1;
        normalized = true;
      }

      if (normalizeUnaryIifeExpression(path)) {
        shellMetrics.structuralTransformCount += 1;
        normalized = true;
      }

      if (normalized) {
        shellMetrics.normalizedNamespaceShellCount += 1;
      }
    },
  });

  return shellMetrics;
}

function normalizeExportedClassShells(factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>): ModuleShellMetrics {
  const shellMetrics = createEmptyShellMetrics();

  factoryPath.traverse({
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (path.getFunctionParent() !== factoryPath) {
        return;
      }

      if (!t.isIdentifier(path.node.id) || !/^[A-Z]/.test(path.node.id.name)) {
        return;
      }

      const initPath = path.get("init");
      if (!initPath.isCallExpression()) {
        return;
      }

      const calleePath = initPath.get("callee");
      if (!calleePath.isFunctionExpression() && !calleePath.isArrowFunctionExpression()) {
        return;
      }

      const returnedFunctionBinding = getReturnedFunctionBindingPath(calleePath);
      if (returnedFunctionBinding === undefined) {
        return;
      }

      shellMetrics.classShellCount += 1;

      let normalized = false;
      let classBindingName = returnedFunctionBinding.returnedIdentifier;
      if (OBFUSCATED_IDENTIFIER_RE.test(returnedFunctionBinding.returnedIdentifier)) {
        classBindingName = renameBinding(calleePath, returnedFunctionBinding.returnedIdentifier, path.node.id.name);
        if (classBindingName !== returnedFunctionBinding.returnedIdentifier) {
          shellMetrics.structuralTransformCount += 1;
          normalized = true;
        }
      }

      let baseCtorName: string | undefined;
      const [firstParameter] = calleePath.node.params;
      if (calleePath.node.params.length === 1 && t.isIdentifier(firstParameter)) {
        const originalParameterName = firstParameter.name;
        if (OBFUSCATED_IDENTIFIER_RE.test(firstParameter.name) || !isReadableIdentifier(firstParameter.name)) {
          baseCtorName = renameBinding(calleePath, firstParameter.name, "baseCtor");
          if (baseCtorName !== originalParameterName) {
            shellMetrics.structuralTransformCount += 1;
            normalized = true;
          }
        } else {
          baseCtorName = firstParameter.name;
        }
      }

      if (baseCtorName !== undefined && renameImmediateSelfAlias(returnedFunctionBinding.functionPath, baseCtorName)) {
        shellMetrics.structuralTransformCount += 1;
        normalized = true;
      }

      if (expandReturnedSequenceIntoStatements(calleePath, classBindingName)) {
        shellMetrics.structuralTransformCount += 1;
        normalized = true;
      }

      if (normalized) {
        shellMetrics.normalizedClassShellCount += 1;
      }
    },
  });

  return shellMetrics;
}

function normalizeExportedShells(
  factoryPath: NodePath<t.FunctionExpression | t.ArrowFunctionExpression>,
  exportsName: string,
): ModuleShellMetrics {
  const shellMetrics = createEmptyShellMetrics();
  mergeShellMetrics(shellMetrics, normalizeNamespaceWrapperShells(factoryPath, exportsName));
  mergeShellMetrics(shellMetrics, normalizeExportedClassShells(factoryPath));
  return shellMetrics;
}

function extractRequireTarget(node: t.Node, requireName: string): { moduleId: string; importStyle: ModuleImportStyle } | undefined {
  if (!t.isCallExpression(node)) {
    return undefined;
  }

  if (t.isIdentifier(node.callee, { name: requireName })) {
    const [firstArgument] = node.arguments;
    if (firstArgument !== undefined && !t.isSpreadElement(firstArgument)) {
      if (t.isNumericLiteral(firstArgument)) {
        return {
          moduleId: String(firstArgument.value),
          importStyle: "require",
        };
      }

      if (t.isStringLiteral(firstArgument)) {
        return {
          moduleId: firstArgument.value,
          importStyle: "require",
        };
      }
    }

    return undefined;
  }

  const [firstArgument] = node.arguments;
  if (firstArgument !== undefined && !t.isSpreadElement(firstArgument)) {
    const nestedTarget = extractRequireTarget(firstArgument, requireName);
    if (nestedTarget !== undefined) {
      return {
        moduleId: nestedTarget.moduleId,
        importStyle: "wrapped-require",
      };
    }
  }

  return undefined;
}

function inferReadableName(exportNames: string[], hasDefaultExport: boolean, defaultExportName?: string): string | undefined {
  const readableExports = exportNames.filter(isUsefulReadableName);

  if (readableExports.length === 1) {
    return readableExports[0];
  }

  if (readableExports.length > 1) {
    const pascalCaseExports = readableExports.filter(name => /^[A-Z]/.test(name));
    if (pascalCaseExports.length === 1) {
      return pascalCaseExports[0];
    }
  }

  if (hasDefaultExport && defaultExportName !== undefined && isUsefulReadableName(defaultExportName)) {
    return defaultExportName;
  }

  return undefined;
}

function buildModuleKindCounts(): Record<ModuleKind, number> {
  return {
    game: 0,
    helper: 0,
    vendor: 0,
  };
}

function createEmptyShellMetrics(): ModuleShellMetrics {
  return {
    namespaceShellCount: 0,
    normalizedNamespaceShellCount: 0,
    classShellCount: 0,
    normalizedClassShellCount: 0,
    structuralTransformCount: 0,
  };
}

function mergeShellMetrics(target: ModuleShellMetrics, source: ModuleShellMetrics): void {
  target.namespaceShellCount += source.namespaceShellCount;
  target.normalizedNamespaceShellCount += source.normalizedNamespaceShellCount;
  target.classShellCount += source.classShellCount;
  target.normalizedClassShellCount += source.normalizedClassShellCount;
  target.structuralTransformCount += source.structuralTransformCount;
}

function createEmptyHotspotCleanupTotals(): HotspotCleanupTotals {
  return {
    moduleCount: 0,
    localRenameCount: 0,
    bodyNormalizationCount: 0,
    obfuscatedIdentifierDelta: 0,
  };
}

function computeNamedGameHotspotScoreFromCounts(
  moduleKind: ModuleKind,
  readableName: string | undefined,
  rawObfuscatedIdentifierCount: number,
  transformedObfuscatedIdentifierCount: number,
  shellMetrics: ModuleShellMetrics,
): number | undefined {
  if (moduleKind !== "game" || readableName === undefined) {
    return undefined;
  }

  const obfuscatedIdentifierDelta = rawObfuscatedIdentifierCount - transformedObfuscatedIdentifierCount;
  return transformedObfuscatedIdentifierCount
    + Math.floor(obfuscatedIdentifierDelta / 2)
    + shellMetrics.structuralTransformCount * 25;
}

function computeNamedGameHotspotScore(module: ModuleArtifact): number | undefined {
  return computeNamedGameHotspotScoreFromCounts(
    module.moduleKind,
    module.readableName,
    module.rawObfuscatedIdentifierCount,
    module.transformedObfuscatedIdentifierCount,
    module.shellMetrics,
  );
}

function selectCleanupTier(
  moduleKind: ModuleKind,
  readableName: string | undefined,
  preliminaryHotspotScore: number | undefined,
): CleanupTier {
  if (moduleKind !== "game" || readableName === undefined) {
    return "none";
  }

  if (preliminaryHotspotScore !== undefined && preliminaryHotspotScore >= PRIORITY_BODY_HOTSPOT_SCORE_THRESHOLD) {
    return "priority-body";
  }

  return "named-game";
}

function getBeforeHotspotObfuscatedIdentifierCount(module: ModuleArtifact): number {
  return module.hotspotCleanup?.beforeObfuscatedIdentifierCount ?? module.transformedObfuscatedIdentifierCount;
}

function getBeforeHotspotScore(module: ModuleArtifact): number | undefined {
  return computeNamedGameHotspotScoreFromCounts(
    module.moduleKind,
    module.readableName,
    module.rawObfuscatedIdentifierCount,
    getBeforeHotspotObfuscatedIdentifierCount(module),
    module.shellMetrics,
  );
}

function toNamedGameHotspotSummary(
  module: ModuleArtifact,
  options: {
    hotspotScore?: number;
    obfuscatedIdentifierCount?: number;
  } = {},
): NamedGameHotspotSummary | undefined {
  const hotspotScore = options.hotspotScore ?? module.hotspotScore;
  const obfuscatedIdentifierCount = options.obfuscatedIdentifierCount ?? module.transformedObfuscatedIdentifierCount;
  if (module.readableName === undefined || hotspotScore === undefined) {
    return undefined;
  }

  return {
    id: module.id,
    readableName: module.readableName,
    fileName: module.fileName,
    hotspotScore,
    obfuscatedIdentifierCount,
    obfuscatedIdentifierDelta: module.rawObfuscatedIdentifierCount - obfuscatedIdentifierCount,
    structuralTransformCount: module.shellMetrics.structuralTransformCount,
  };
}

function toHotspotDeltaReportEntry(module: ModuleArtifact): HotspotDeltaReportEntry | undefined {
  if (module.readableName === undefined || module.hotspotCleanup === undefined) {
    return undefined;
  }

  const beforeHotspotScore = getBeforeHotspotScore(module);
  const afterHotspotScore = module.hotspotScore;
  if (beforeHotspotScore === undefined || afterHotspotScore === undefined) {
    return undefined;
  }

  return {
    id: module.id,
    readableName: module.readableName,
    fileName: module.fileName,
    beforeHotspotScore,
    afterHotspotScore,
    beforeObfuscatedIdentifierCount: module.hotspotCleanup.beforeObfuscatedIdentifierCount,
    afterObfuscatedIdentifierCount: module.hotspotCleanup.afterObfuscatedIdentifierCount,
    obfuscatedIdentifierDelta: module.hotspotCleanup.obfuscatedIdentifierDelta,
    localRenameCount: module.hotspotCleanup.localRenameCount,
    bodyNormalizationCount: module.hotspotCleanup.bodyNormalizationCount,
  };
}

function classifyModuleKind(module: ModuleClassificationInput): ModuleKind {
  const ownNames = [module.readableName, ...module.exportNames].filter((name): name is string => name !== undefined);
  const dependencyNames = module.dependencies
    .map(dependency => dependency.readableName)
    .filter((name): name is string => name !== undefined);
  const uniqueDependencyNames = [...new Set(dependencyNames)];

  let gameScore = 0;
  let helperScore = 0;
  let vendorScore = 0;

  for (const name of ownNames) {
    if (GAME_NAME_RE.test(name)) {
      gameScore += 3;
    }
    if (HELPER_NAME_RE.test(name)) {
      helperScore += 3;
    }
  }

  for (const name of uniqueDependencyNames) {
    if (GAME_NAME_RE.test(name)) {
      gameScore += 1;
    }
    if (HELPER_NAME_RE.test(name)) {
      helperScore += 1;
    }
  }

  gameScore += countMarkerHits(module.source, GAME_SOURCE_MARKERS) * 2;
  vendorScore += countMarkerHits(module.source, VENDOR_SOURCE_MARKERS) * 2;

  if (ownNames.length === 0 && module.exportNames.length === 0 && vendorScore > 0 && gameScore === 0) {
    vendorScore += 2;
  }

  if (module.readableName === undefined && uniqueDependencyNames.length === 0 && vendorScore === 0 && gameScore === 0) {
    vendorScore += 1;
  }

  if (gameScore >= helperScore && gameScore >= vendorScore && gameScore > 0) {
    return "game";
  }

  if (helperScore >= vendorScore && helperScore > 0) {
    return "helper";
  }

  if (vendorScore > 0) {
    return "vendor";
  }

  if (ownNames.some(name => GAME_NAME_RE.test(name))) {
    return "game";
  }

  if (ownNames.some(name => HELPER_NAME_RE.test(name))) {
    return "helper";
  }

  if (uniqueDependencyNames.some(name => GAME_NAME_RE.test(name))) {
    return "game";
  }

  return "vendor";
}

function buildModuleFileName(moduleId: string, readableName?: string): string {
  const readableSlugValue = readableName === undefined ? "" : slugify(readableName);
  const readableSlug = readableSlugValue.length === 0 ? "" : `-${readableSlugValue}`;
  return `module-${moduleId}${readableSlug}.js`;
}

function generateFactorySource(ast: t.File): string {
  const expressionStatement = ast.program.body[0];
  if (!t.isExpressionStatement(expressionStatement)) {
    throw new Error("Expected the factory AST to contain a single expression statement");
  }

  return formatJavaScript(generate(expressionStatement.expression, {
    compact: false,
    comments: true,
  }).code);
}

function analyzeModuleFirstPass(moduleId: string, displayId: string, rawSource: string): FirstPassModuleRecord {
  const rawObfuscatedIdentifierCount = countObfuscatedIdentifiers(rawSource);
  const ast = parseFactoryFile(rawSource);
  const factoryPath = findFactoryPath(ast);
  const canonicalParameterNames = renameFactoryParameters(factoryPath);
  renameTsHelperAliases(factoryPath);

  const moduleName = canonicalParameterNames[0] ?? "module";
  const exportsName = canonicalParameterNames[1] ?? "exports";
  const requireName = canonicalParameterNames[2] ?? "require";

  const exportNames = new Set<string>();
  const namespaceRenameCandidates: Array<{ bindingName: string; exportName: string }> = [];
  const dependencies: DependencyCapture[] = [];
  let hasDefaultExport = false;
  let defaultExportName: string | undefined;

  traverse(ast, {
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      const init = path.node.init;
      if (!t.isIdentifier(path.node.id) || init == null) {
        return;
      }

      const dependency = extractRequireTarget(init, requireName);
      if (dependency !== undefined) {
        dependencies.push({
          moduleId: dependency.moduleId,
          localName: path.node.id.name,
          importStyle: dependency.importStyle,
        });
      }
    },
    AssignmentExpression(path: NodePath<t.AssignmentExpression>) {
      if (isExportsMember(path.node.left, exportsName)) {
        const exportName = normalizePropertyName(path.node.left.property, path.node.left.computed);
        if (exportName === undefined || exportName === "__esModule") {
          return;
        }

        if (exportName === "default") {
          hasDefaultExport = true;
        } else {
          exportNames.add(exportName);
        }

        const assignedIdentifier = extractAssignedIdentifier(path.node.right);
        if (assignedIdentifier !== undefined && OBFUSCATED_IDENTIFIER_RE.test(assignedIdentifier) && isReadableIdentifier(exportName)) {
          namespaceRenameCandidates.push({
            bindingName: assignedIdentifier,
            exportName,
          });
        }

        return;
      }

      if (!isModuleExportsMember(path.node.left, moduleName)) {
        return;
      }

      hasDefaultExport = true;
      if ((t.isFunctionExpression(path.node.right) || t.isClassExpression(path.node.right)) && t.isIdentifier(path.node.right.id)) {
        defaultExportName = path.node.right.id.name;
      }
    },
    CallExpression(path: NodePath<t.CallExpression>) {
      if (!t.isMemberExpression(path.node.callee)) {
        return;
      }
      if (!t.isIdentifier(path.node.callee.object, { name: "Object" })) {
        return;
      }
      if (!t.isIdentifier(path.node.callee.property, { name: "defineProperty" })) {
        return;
      }

      const [target, property] = path.node.arguments;
      if (target === undefined || property === undefined || t.isSpreadElement(target) || t.isSpreadElement(property)) {
        return;
      }

      if (!t.isIdentifier(target, { name: exportsName })) {
        return;
      }

      if (t.isStringLiteral(property) && property.value !== "__esModule" && property.value !== "default") {
        exportNames.add(property.value);
      }
    },
  });

  for (const candidate of namespaceRenameCandidates) {
    if (factoryPath.scope.hasBinding(candidate.bindingName)) {
      renameBinding(factoryPath, candidate.bindingName, candidate.exportName);
    }
  }

  const shellMetrics = normalizeExportedShells(factoryPath, exportsName);

  const firstPassSource = generateFactorySource(ast);

  return {
    id: moduleId,
    displayId,
    rawSource,
    firstPassSource,
    exportNames: [...exportNames].sort((left, right) => left.localeCompare(right)),
    readableName: inferReadableName([...exportNames], hasDefaultExport, defaultExportName),
    hasDefaultExport,
    canonicalParameterNames,
    rawObfuscatedIdentifierCount,
    shellMetrics,
    dependencies,
  };
}

function buildDependencyAlias(moduleReadableName: string): string {
  return `${toLowerCamelCase(moduleReadableName)}Module`;
}

function isLegacySequenceIfSplitTarget(readableName: string | undefined): readableName is string {
  return readableName !== undefined && LEGACY_SEQUENCE_IF_SPLIT_TARGETS.has(readableName);
}

function isThisOrSelfExpression(node: t.Expression | t.PrivateName): boolean {
  return t.isThisExpression(node) || t.isIdentifier(node, { name: "self" });
}

function flattenSequenceExpressions(expression: t.Expression): t.Expression[] {
  if (t.isSequenceExpression(expression)) {
    return expression.expressions.flatMap(flattenSequenceExpressions);
  }

  return [expression];
}

function replaceStatementWithMany(path: NodePath<t.Statement>, statements: t.Statement[]): void {
  if (statements.length === 0) {
    return;
  }

  if (statements.length === 1) {
    path.replaceWith(statements[0]!);
  } else if (path.inList) {
    path.replaceWithMultiple(statements);
  } else {
    path.replaceWith(t.blockStatement(statements));
  }

  path.skip();
}

function toPropertyBindingName(propertyName: string): string | undefined {
  const normalized = propertyName.replace(/^_+/, "");
  if (normalized.length <= 1 || !isReadableIdentifier(normalized) || !/[A-Za-z]/.test(normalized)) {
    return undefined;
  }

  return normalized;
}

function toCollectionItemBindingName(propertyName: string): string | undefined {
  const bindingName = toPropertyBindingName(propertyName);
  if (bindingName === undefined) {
    return undefined;
  }

  if (bindingName.endsWith("ies") && bindingName.length > 3) {
    return `${bindingName.slice(0, -3)}y`;
  }

  if (bindingName.endsWith("s") && !bindingName.endsWith("ss") && bindingName.length > 1) {
    return bindingName.slice(0, -1);
  }

  return `${bindingName}Item`;
}

function toMethodResultBindingName(methodName: string): string | undefined {
  const normalized = methodName.replace(/^_+/, "");
  const prefixMatch = normalized.match(/^(get|is|has)([A-Z].*)$/);
  if (prefixMatch === null) {
    return undefined;
  }

  const [, , suffix] = prefixMatch;
  if (suffix === undefined || suffix.length === 0) {
    return undefined;
  }

  const bindingName = `${suffix[0]!.toLowerCase()}${suffix.slice(1)}`;
  if (!isUsefulReadableName(bindingName)) {
    return undefined;
  }

  return bindingName;
}

function deriveBindingNameFromMemberExpression(expression: t.MemberExpression): string | undefined {
  if (isThisOrSelfExpression(expression.object)) {
    const propertyName = normalizePropertyName(expression.property, expression.computed);
    if (propertyName !== undefined) {
      return toPropertyBindingName(propertyName);
    }
  }

  if (expression.computed && t.isMemberExpression(expression.object) && isThisOrSelfExpression(expression.object.object)) {
    const propertyName = normalizePropertyName(expression.object.property, expression.object.computed);
    if (propertyName !== undefined) {
      return toCollectionItemBindingName(propertyName);
    }
  }

  if (t.isIdentifier(expression.object) && !expression.computed) {
    const propertyName = normalizePropertyName(expression.property, expression.computed);
    if (propertyName !== undefined) {
      return toPropertyBindingName(propertyName);
    }
  }

  return undefined;
}

function deriveBindingNameFromInitializer(initializer: t.Expression | null | undefined): string | undefined {
  if (initializer === undefined || initializer === null) {
    return undefined;
  }

  if (t.isThisExpression(initializer)) {
    return "self";
  }

  if (t.isMemberExpression(initializer)) {
    return deriveBindingNameFromMemberExpression(initializer);
  }

  if (t.isCallExpression(initializer) && t.isMemberExpression(initializer.callee)) {
    const propertyName = normalizePropertyName(initializer.callee.property, initializer.callee.computed);
    if (propertyName !== undefined) {
      return toMethodResultBindingName(propertyName);
    }
  }

  return undefined;
}

function collectHighConfidenceParamCandidates(functionPath: RenameableFunctionPath): Map<string, string> {
  const paramNames = functionPath.node.params
    .filter((parameter): parameter is t.Identifier => t.isIdentifier(parameter))
    .map(parameter => parameter.name)
    .filter(name => OBFUSCATED_IDENTIFIER_RE.test(name));

  if (paramNames.length === 0) {
    return new Map<string, string>();
  }

  const paramNameSet = new Set(paramNames);
  const candidateNames = new Map<string, Set<string>>();

  functionPath.traverse({
    AssignmentExpression(path: NodePath<t.AssignmentExpression>) {
      if (path.getFunctionParent() !== functionPath) {
        return;
      }

      if (!t.isIdentifier(path.node.right) || !paramNameSet.has(path.node.right.name)) {
        return;
      }

      if (!t.isMemberExpression(path.node.left) || path.node.left.computed) {
        return;
      }

      if (!t.isThisExpression(path.node.left.object) && !t.isIdentifier(path.node.left.object, { name: "self" })) {
        return;
      }

      const propertyName = normalizePropertyName(path.node.left.property, false);
      if (propertyName === undefined) {
        return;
      }

      const bindingName = toPropertyBindingName(propertyName);
      if (bindingName === undefined) {
        return;
      }

      const existingNames = candidateNames.get(path.node.right.name) ?? new Set<string>();
      existingNames.add(bindingName);
      candidateNames.set(path.node.right.name, existingNames);
    },
  });

  const renameCandidates = new Map<string, string>();
  for (const paramName of paramNames) {
    const matches = candidateNames.get(paramName);
    if (matches === undefined || matches.size !== 1) {
      continue;
    }

    const [preferredName] = [...matches];
    if (preferredName !== undefined) {
      renameCandidates.set(paramName, preferredName);
    }
  }

  return renameCandidates;
}

function collectHighConfidenceLocalCandidates(functionPath: RenameableFunctionPath): Map<string, string> {
  const renameTargets = new Map<string, string>();

  functionPath.traverse({
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (path.getFunctionParent() !== functionPath) {
        return;
      }

      if (!t.isIdentifier(path.node.id) || !OBFUSCATED_IDENTIFIER_RE.test(path.node.id.name)) {
        return;
      }

      const preferredName = deriveBindingNameFromInitializer(path.node.init);
      if (preferredName === undefined) {
        return;
      }

      renameTargets.set(path.node.id.name, preferredName);
    },
  });

  return renameTargets;
}

function countPreferredNameUsage(candidateMaps: Array<Map<string, string>>): Map<string, number> {
  const preferredNameUsage = new Map<string, number>();

  for (const candidateMap of candidateMaps) {
    for (const preferredName of candidateMap.values()) {
      preferredNameUsage.set(preferredName, (preferredNameUsage.get(preferredName) ?? 0) + 1);
    }
  }

  return preferredNameUsage;
}

function renameCollectedBindings(
  functionPath: RenameableFunctionPath,
  renameTargets: Map<string, string>,
  preferredNameUsage: Map<string, number>,
): number {
  let renameCount = 0;
  for (const [oldName, preferredName] of renameTargets) {
    if (preferredNameUsage.get(preferredName) === 1 && renameBinding(functionPath, oldName, preferredName) !== oldName) {
      renameCount += 1;
    }
  }

  return renameCount;
}

function renameHighConfidenceBindings(functionPath: RenameableFunctionPath): {
  paramRenameCount: number;
  localRenameCount: number;
} {
  const paramRenameTargets = collectHighConfidenceParamCandidates(functionPath);
  const localRenameTargets = collectHighConfidenceLocalCandidates(functionPath);
  const preferredNameUsage = countPreferredNameUsage([paramRenameTargets, localRenameTargets]);

  return {
    paramRenameCount: renameCollectedBindings(functionPath, paramRenameTargets, preferredNameUsage),
    localRenameCount: renameCollectedBindings(functionPath, localRenameTargets, preferredNameUsage),
  };
}

function normalizeSequenceExpressionStatements(ast: t.File): number {
  let transformCount = 0;
  traverse(ast, {
    ExpressionStatement(path: NodePath<t.ExpressionStatement>) {
      if (!t.isSequenceExpression(path.node.expression)) {
        return;
      }

      const expressions = flattenSequenceExpressions(path.node.expression);
      if (expressions.length <= 1) {
        return;
      }

      replaceStatementWithMany(
        path,
        expressions.map(expression => t.expressionStatement(t.cloneNode(expression, true))),
      );
      transformCount += 1;
    },
  });

  return transformCount;
}

function normalizeSequenceReturnStatements(ast: t.File): number {
  let transformCount = 0;
  traverse(ast, {
    ReturnStatement(path: NodePath<t.ReturnStatement>) {
      if (!t.isSequenceExpression(path.node.argument)) {
        return;
      }

      const expressions = flattenSequenceExpressions(path.node.argument);
      if (expressions.length <= 1) {
        return;
      }

      const returnValue = expressions.pop();
      if (returnValue === undefined) {
        return;
      }

      replaceStatementWithMany(path, [
        ...expressions.map(expression => t.expressionStatement(t.cloneNode(expression, true))),
        t.returnStatement(t.cloneNode(returnValue, true)),
      ]);
      transformCount += 1;
    },
  });

  return transformCount;
}

function normalizeLegacySequenceIfTests(ast: t.File, readableName?: string): number {
  if (!isLegacySequenceIfSplitTarget(readableName)) {
    return 0;
  }

  let transformCount = 0;
  traverse(ast, {
    IfStatement(path: NodePath<t.IfStatement>) {
      if (!t.isSequenceExpression(path.node.test)) {
        return;
      }

      const expressions = flattenSequenceExpressions(path.node.test);
      if (expressions.length <= 1) {
        return;
      }

      const testExpression = expressions.pop();
      if (testExpression === undefined) {
        return;
      }

      const alternate = path.node.alternate;
      replaceStatementWithMany(path, [
        ...expressions.map(expression => t.expressionStatement(t.cloneNode(expression, true))),
        t.ifStatement(
          t.cloneNode(testExpression, true),
          t.cloneNode(path.node.consequent, true) as t.Statement,
          alternate == null ? null : t.cloneNode(alternate, true) as t.Statement,
        ),
      ]);
      transformCount += 1;
    },
  });

  return transformCount;
}

function normalizeHexLiterals(ast: t.File): number {
  let transformCount = 0;
  traverse(ast, {
    NumericLiteral(path: NodePath<t.NumericLiteral>) {
      const raw = path.node.extra?.raw;
      if (typeof raw !== "string" || !raw.startsWith("0x") && !raw.startsWith("0X")) {
        return;
      }
      path.node.extra = { ...path.node.extra, raw: String(path.node.value) };
      transformCount += 1;
    },
  });
  return transformCount;
}

function pushAppliedRule(appliedRules: CleanupRuleName[], rule: CleanupRuleName, count: number): void {
  if (count > 0) {
    appliedRules.push(rule);
  }
}

function applyReadabilityTransforms(
  factorySource: string,
  cleanupTier: CleanupTier,
  readableName?: string,
): ReadabilityTransformResult {
  if (cleanupTier === "none") {
    const ast = parseFactoryFile(factorySource);
    const hexLiteralCount = normalizeHexLiterals(ast);
    if (hexLiteralCount > 0) {
      const source = generateFactorySource(ast);
      const obfuscatedIdentifierCount = countObfuscatedIdentifiers(source);
      return {
        source,
        cleanupTier,
        localRenameCount: 0,
        bodyNormalizationCount: 0,
        beforeObfuscatedIdentifierCount: obfuscatedIdentifierCount,
        afterObfuscatedIdentifierCount: obfuscatedIdentifierCount,
        hotspotCleanup: {
          beforeObfuscatedIdentifierCount: obfuscatedIdentifierCount,
          afterObfuscatedIdentifierCount: obfuscatedIdentifierCount,
          obfuscatedIdentifierDelta: 0,
          localRenameCount: 0,
          bodyNormalizationCount: 0,
          appliedRules: ["hex-literals" as CleanupRuleName],
        },
      };
    }
    const obfuscatedIdentifierCount = countObfuscatedIdentifiers(factorySource);
    return {
      source: factorySource,
      cleanupTier,
      localRenameCount: 0,
      bodyNormalizationCount: 0,
      beforeObfuscatedIdentifierCount: obfuscatedIdentifierCount,
      afterObfuscatedIdentifierCount: obfuscatedIdentifierCount,
    };
  }

  const ast = parseFactoryFile(factorySource);
  const factoryPath = findFactoryPath(ast);
  const beforeObfuscatedIdentifierCount = countObfuscatedIdentifiers(factorySource);
  let localRenameCount = 0;
  const appliedRules: CleanupRuleName[] = [];
  let paramRenameCount = 0;
  let directLocalRenameCount = 0;

  traverse(ast, {
    FunctionDeclaration(path: NodePath<t.FunctionDeclaration>) {
      const renameCounts = renameHighConfidenceBindings(path);
      paramRenameCount += renameCounts.paramRenameCount;
      directLocalRenameCount += renameCounts.localRenameCount;
    },
    FunctionExpression(path: NodePath<t.FunctionExpression>) {
      if (path === factoryPath) {
        return;
      }

      const renameCounts = renameHighConfidenceBindings(path);
      paramRenameCount += renameCounts.paramRenameCount;
      directLocalRenameCount += renameCounts.localRenameCount;
    },
    ArrowFunctionExpression(path: NodePath<t.ArrowFunctionExpression>) {
      if (path === factoryPath) {
        return;
      }

      const renameCounts = renameHighConfidenceBindings(path);
      paramRenameCount += renameCounts.paramRenameCount;
      directLocalRenameCount += renameCounts.localRenameCount;
    },
  });

  localRenameCount = paramRenameCount + directLocalRenameCount;
  pushAppliedRule(appliedRules, "param-rename", paramRenameCount);
  pushAppliedRule(appliedRules, "local-rename", directLocalRenameCount);

  const hexLiteralCount = normalizeHexLiterals(ast);
  pushAppliedRule(appliedRules, "hex-literals", hexLiteralCount);

  const enumAnnotationCount = annotateEnumLiterals(ast);
  pushAppliedRule(appliedRules, "enum-annotate", enumAnnotationCount);

  let bodyNormalizationCount = 0;
  const expressionSplitCount = normalizeSequenceExpressionStatements(ast);
  const returnSplitCount = normalizeSequenceReturnStatements(ast);
  pushAppliedRule(appliedRules, "sequence-expression-split", expressionSplitCount);
  pushAppliedRule(appliedRules, "sequence-return-split", returnSplitCount);
  bodyNormalizationCount += expressionSplitCount + returnSplitCount;

  if (cleanupTier === "priority-body") {
    const legacyIfSplitCount = normalizeLegacySequenceIfTests(ast, readableName);
    bodyNormalizationCount += legacyIfSplitCount;
    pushAppliedRule(appliedRules, "legacy-sequence-if-split", legacyIfSplitCount);
  }

  const source = localRenameCount > 0 || bodyNormalizationCount > 0 || hexLiteralCount > 0 || enumAnnotationCount > 0 ? generateFactorySource(ast) : factorySource;
  const afterObfuscatedIdentifierCount = countObfuscatedIdentifiers(source);

  return {
    source,
    cleanupTier,
    localRenameCount,
    bodyNormalizationCount,
    beforeObfuscatedIdentifierCount,
    afterObfuscatedIdentifierCount,
    hotspotCleanup: {
      beforeObfuscatedIdentifierCount,
      afterObfuscatedIdentifierCount,
      obfuscatedIdentifierDelta: beforeObfuscatedIdentifierCount - afterObfuscatedIdentifierCount,
      localRenameCount,
      bodyNormalizationCount,
      appliedRules,
    },
  };
}

function applyDependencyRenames(
  factorySource: string,
  dependencyCaptures: DependencyCapture[],
  moduleNameById: Map<string, string>,
): { source: string; dependencies: ModuleDependencySummary[] } {
  const dependencies: ModuleDependencySummary[] = dependencyCaptures.map(dependency => ({
    moduleId: dependency.moduleId,
    readableName: moduleNameById.get(dependency.moduleId),
    localName: dependency.localName,
    importStyle: dependency.importStyle,
  }));

  const renameTargets = dependencies.filter(dependency => {
    return dependency.localName !== undefined && OBFUSCATED_IDENTIFIER_RE.test(dependency.localName) && dependency.readableName !== undefined;
  });

  if (renameTargets.length === 0) {
    return {
      source: factorySource,
      dependencies,
    };
  }

  const ast = parseFactoryFile(factorySource);
  const factoryPath = findFactoryPath(ast);
  const renameMap = new Map<string, string>();

  for (const target of renameTargets) {
    const originalLocalName = target.localName;
    if (originalLocalName === undefined) {
      continue;
    }

    const preferredName = buildDependencyAlias(target.readableName!);
    const renamedBinding = renameBinding(factoryPath, originalLocalName, preferredName);
    renameMap.set(originalLocalName, renamedBinding);
  }

  for (const dependency of dependencies) {
    if (dependency.localName !== undefined && renameMap.has(dependency.localName)) {
      dependency.localName = renameMap.get(dependency.localName);
    }
  }

  return {
    source: generateFactorySource(ast),
    dependencies,
  };
}

function createModuleSummary(modules: ModuleArtifact[]): ModuleGraphSummary {
  const modulesWithNamedExports = modules.filter(module => module.exportNames.length > 0).length;
  const modulesWithReadableNames = modules.filter(module => module.readableName !== undefined).length;
  const moduleKindCounts = buildModuleKindCounts();
  const shellMetrics = createEmptyShellMetrics();
  const hotspotCleanupTotals = createEmptyHotspotCleanupTotals();
  const totalDependencies = modules.reduce((count, module) => count + module.dependencies.length, 0);
  const totalRawObfuscatedIdentifiers = modules.reduce((count, module) => count + module.rawObfuscatedIdentifierCount, 0);
  const totalTransformedObfuscatedIdentifiers = modules.reduce(
    (count, module) => count + module.transformedObfuscatedIdentifierCount,
    0,
  );
  const totalObfuscatedIdentifierDelta = modules.reduce((count, module) => count + module.obfuscatedIdentifierDelta, 0);

  for (const module of modules) {
    moduleKindCounts[module.moduleKind] += 1;
    mergeShellMetrics(shellMetrics, module.shellMetrics);
    if (module.hotspotCleanup !== undefined) {
      hotspotCleanupTotals.moduleCount += 1;
      hotspotCleanupTotals.localRenameCount += module.hotspotCleanup.localRenameCount;
      hotspotCleanupTotals.bodyNormalizationCount += module.hotspotCleanup.bodyNormalizationCount;
      hotspotCleanupTotals.obfuscatedIdentifierDelta += module.hotspotCleanup.obfuscatedIdentifierDelta;
    }
  }

  const hotspotDeltaReport = modules
    .map(module => toHotspotDeltaReportEntry(module))
    .filter((module): module is HotspotDeltaReportEntry => module !== undefined)
    .sort((left, right) => {
      if (right.obfuscatedIdentifierDelta !== left.obfuscatedIdentifierDelta) {
        return right.obfuscatedIdentifierDelta - left.obfuscatedIdentifierDelta;
      }

      if (right.bodyNormalizationCount !== left.bodyNormalizationCount) {
        return right.bodyNormalizationCount - left.bodyNormalizationCount;
      }

      return right.localRenameCount - left.localRenameCount;
    });

  return {
    moduleCount: modules.length,
    modulesWithNamedExports,
    modulesWithReadableNames,
    moduleKindCounts,
    totalDependencies,
    totalRawObfuscatedIdentifiers,
    totalTransformedObfuscatedIdentifiers,
    totalObfuscatedIdentifierDelta,
    shellMetrics,
    namedModulesPreview: modules
      .filter((module): module is ModuleArtifact & { readableName: string } => module.readableName !== undefined)
      .slice(0, 20)
      .map(module => ({
        id: module.id,
        readableName: module.readableName,
        fileName: module.fileName,
      })),
    topObfuscatedModules: [...modules]
      .sort((left, right) => right.transformedObfuscatedIdentifierCount - left.transformedObfuscatedIdentifierCount)
      .slice(0, 20)
      .map(module => ({
        id: module.id,
        moduleKind: module.moduleKind,
        readableName: module.readableName,
        fileName: module.fileName,
        obfuscatedIdentifierCount: module.transformedObfuscatedIdentifierCount,
      })),
    topStructuralTransformModules: modules
      .filter(module => module.shellMetrics.structuralTransformCount > 0)
      .sort((left, right) => {
        if (right.shellMetrics.structuralTransformCount !== left.shellMetrics.structuralTransformCount) {
          return right.shellMetrics.structuralTransformCount - left.shellMetrics.structuralTransformCount;
        }

        return right.obfuscatedIdentifierDelta - left.obfuscatedIdentifierDelta;
      })
      .slice(0, 20)
      .map(module => ({
        id: module.id,
        readableName: module.readableName,
        fileName: module.fileName,
        structuralTransformCount: module.shellMetrics.structuralTransformCount,
        obfuscatedIdentifierDelta: module.obfuscatedIdentifierDelta,
      })),
    topObfuscatedGameModules: modules
      .filter(module => module.moduleKind === "game")
      .sort((left, right) => right.transformedObfuscatedIdentifierCount - left.transformedObfuscatedIdentifierCount)
      .slice(0, 20)
      .map(module => ({
        id: module.id,
        readableName: module.readableName,
        fileName: module.fileName,
        obfuscatedIdentifierCount: module.transformedObfuscatedIdentifierCount,
      })),
    topNamedGameHotspotsBeforeCleanup: modules
      .filter((module): module is ModuleArtifact & { readableName: string } => {
        return module.moduleKind === "game" && module.readableName !== undefined;
      })
      .sort((left, right) => {
        const leftScore = getBeforeHotspotScore(left) ?? 0;
        const rightScore = getBeforeHotspotScore(right) ?? 0;
        if (rightScore !== leftScore) {
          return rightScore - leftScore;
        }

        return getBeforeHotspotObfuscatedIdentifierCount(right) - getBeforeHotspotObfuscatedIdentifierCount(left);
      })
      .slice(0, 20)
      .map(module => toNamedGameHotspotSummary(module, {
        hotspotScore: getBeforeHotspotScore(module),
        obfuscatedIdentifierCount: getBeforeHotspotObfuscatedIdentifierCount(module),
      })!)
      .filter((module): module is NamedGameHotspotSummary => module !== undefined),
    topNamedGameHotspots: modules
      .filter((module): module is ModuleArtifact & { readableName: string; hotspotScore: number } => {
        return module.moduleKind === "game" && module.readableName !== undefined && module.hotspotScore !== undefined;
      })
      .sort((left, right) => {
        if (right.hotspotScore !== left.hotspotScore) {
          return right.hotspotScore - left.hotspotScore;
        }

        return right.transformedObfuscatedIdentifierCount - left.transformedObfuscatedIdentifierCount;
      })
      .slice(0, 20)
      .map(module => toNamedGameHotspotSummary(module)!)
      .filter((module): module is NamedGameHotspotSummary => module !== undefined),
    hotspotCleanupTotals,
    hotspotDeltaReport,
  };
}

export function extractModuleGraph(decodedSource: string): ModuleGraph {
  const moduleTable = extractModuleTable(decodedSource);
  const firstPassModules: FirstPassModuleRecord[] = [];

  for (const property of moduleTable.properties) {
    if (!t.isObjectProperty(property) || !isModuleFactoryValue(property.value)) {
      continue;
    }

    const { id, displayId } = normalizeModuleId(property.key);
    const rawSource = formatJavaScript(generate(property.value, {
      compact: false,
      comments: true,
    }).code);

    firstPassModules.push(analyzeModuleFirstPass(id, displayId, rawSource));
  }

  const moduleNameById = new Map<string, string>();
  for (const module of firstPassModules) {
    if (module.readableName !== undefined) {
      moduleNameById.set(module.id, module.readableName);
    }
  }

  const modules: ModuleArtifact[] = firstPassModules.map(module => {
    const renamedDependencies = applyDependencyRenames(module.firstPassSource, module.dependencies, moduleNameById);
    const dependencyRenamedSource = renamedDependencies.source;
    const moduleKind = classifyModuleKind({
      readableName: module.readableName,
      exportNames: module.exportNames,
      dependencies: renamedDependencies.dependencies,
      source: dependencyRenamedSource,
    });
    const preCleanupObfuscatedIdentifierCount = countObfuscatedIdentifiers(dependencyRenamedSource);
    const preliminaryHotspotScore = computeNamedGameHotspotScoreFromCounts(
      moduleKind,
      module.readableName,
      module.rawObfuscatedIdentifierCount,
      preCleanupObfuscatedIdentifierCount,
      module.shellMetrics,
    );
    const cleanupTier = selectCleanupTier(moduleKind, module.readableName, preliminaryHotspotScore);
    const readabilityTransforms = applyReadabilityTransforms(dependencyRenamedSource, cleanupTier, module.readableName);
    const source = readabilityTransforms.source;
    const transformedObfuscatedIdentifierCount = readabilityTransforms.afterObfuscatedIdentifierCount;
    const obfuscatedIdentifierDelta = module.rawObfuscatedIdentifierCount - transformedObfuscatedIdentifierCount;

    const moduleArtifact: ModuleArtifact = {
      id: module.id,
      displayId: module.displayId,
      fileName: buildModuleFileName(module.id, module.readableName),
      moduleKind,
      cleanupTier: readabilityTransforms.cleanupTier,
      readableName: module.readableName,
      exportNames: module.exportNames,
      hasDefaultExport: module.hasDefaultExport,
      canonicalParameterNames: module.canonicalParameterNames,
      rawObfuscatedIdentifierCount: module.rawObfuscatedIdentifierCount,
      transformedObfuscatedIdentifierCount,
      obfuscatedIdentifierDelta,
      shellMetrics: module.shellMetrics,
      hotspotCleanup: readabilityTransforms.hotspotCleanup,
      lineCount: countLines(source),
      dependencies: renamedDependencies.dependencies,
      source,
    };

    moduleArtifact.hotspotScore = computeNamedGameHotspotScore(moduleArtifact);
    return moduleArtifact;
  });

  return {
    modules,
    summary: createModuleSummary(modules),
  };
}
