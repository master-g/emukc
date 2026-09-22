import generate from "@babel/generator";
import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type {
  BattleAttackTypeAcceptanceAsset,
  BattleAttackTypeFallback,
  BattleAttackTypeStage,
  BattleKnowledge,
  BattleKnowledgeSummary,
  BattleModuleIndexAsset,
  BattleModuleKnowledge,
  BattleProtocolFieldRule,
  BattleProtocolFieldsAsset,
  BattleResourceRule,
  BattleResourceRulesAsset,
  BattleSlotResourceTrigger,
  BattleSlotResourceTriggersAsset,
  ModuleArtifact,
  ModuleGraph,
} from "./types.ts";

const BATTLE_RELEVANT_NAME_RE =
  /^(Raw(?:Day|Night)BattleData|Battle(?:Common|Data|Record|Scene|Result|BGM|CommonModel|SceneModel)|PhaseHougeki(?:Base)?|Raigeki(?:Opening)?Data|Cutin[A-Za-z0-9]*|ShipBanner(?:Clone)?|Banner(?:[A-Za-z0-9]+)?|DamageNumber|Result(?:Dialog|View)|SlotItemEffectUtil)$/;
const BATTLE_FIELD_RE = /^api_[a-z0-9_]+$/;
const OBJ_UTIL_METHODS = new Map<string, BattleProtocolFieldRule["accessKind"]>([
  ["getNumber", "number"],
  ["getNumArray", "numArray"],
  ["getObject", "object"],
  ["getObjectArray", "objectArray"],
]);
const CONDITIONAL_ANCESTOR_TYPES = new Set<string>([
  "IfStatement",
  "ConditionalExpression",
  "LogicalExpression",
  "SwitchCase",
]);
const HOUGEKI_PROTOCOL_SOURCES = [
  "api_hougeki1.api_si_list[*][*]",
  "api_hougeki2.api_si_list[*][*]",
  "api_hougeki3.api_si_list[*][*]",
] as const;

function parseFactorySource(source: string): t.File {
  return parse(`(${source});`, {
    sourceType: "script",
    allowReturnOutsideFunction: true,
  });
}

function expressionToSource(node: t.Node | null | undefined): string | undefined {
  if (node == null) {
    return undefined;
  }

  if (t.isIdentifier(node)) {
    return node.name;
  }
  if (t.isThisExpression(node)) {
    return "this";
  }
  if (t.isStringLiteral(node)) {
    return node.value;
  }
  if (t.isNumericLiteral(node)) {
    return String(node.value);
  }
  if (t.isBooleanLiteral(node)) {
    return String(node.value);
  }
  if (t.isMemberExpression(node)) {
    return memberExpressionToString(node);
  }

  return generate(node, {
    compact: true,
    comments: false,
  }).code;
}

function memberExpressionToString(node: t.MemberExpression): string | undefined {
  const objectSource = t.isMemberExpression(node.object)
    ? memberExpressionToString(node.object)
    : expressionToSource(node.object);
  if (objectSource === undefined) {
    return undefined;
  }

  let propertySource: string | undefined;
  if (t.isIdentifier(node.property) && !node.computed) {
    propertySource = node.property.name;
  } else if (t.isStringLiteral(node.property)) {
    propertySource = node.property.value;
  } else if (t.isNumericLiteral(node.property)) {
    propertySource = String(node.property.value);
  } else {
    propertySource = expressionToSource(node.property);
  }

  if (propertySource === undefined) {
    return undefined;
  }

  return node.computed && !t.isIdentifier(node.property)
    ? `${objectSource}[${propertySource}]`
    : `${objectSource}.${propertySource}`;
}

function getCallExpressionChain(node: t.Expression | t.V8IntrinsicIdentifier): string | undefined {
  if (!t.isMemberExpression(node)) {
    return undefined;
  }

  return memberExpressionToString(node);
}

function isBattleRelevantModule(module: ModuleArtifact): boolean {
  if (module.readableName !== undefined && BATTLE_RELEVANT_NAME_RE.test(module.readableName)) {
    return true;
  }

  return module.source.includes("api_ship_ke")
    || module.source.includes("api_eSlot")
    || module.source.includes("TaskLoadShipResource")
    || module.source.includes("ShipLoader")
    || module.source.includes("SlotLoader")
    || module.source.includes("resources.getShip")
    || module.source.includes("resources.getSlotitem");
}

function inferTags(module: ModuleArtifact): string[] {
  const name = module.readableName ?? module.fileName;
  const tags = new Set<string>(["battle"]);

  if (/Raw(?:Day|Night)BattleData|BattleData|BattleRecord/.test(name)) {
    tags.add("protocol-core");
  }
  if (/RawDay|BattleData|Raigeki|Hougeki/.test(name)) {
    tags.add("day");
  }
  if (/RawNight|Night/.test(name)) {
    tags.add("night");
  }
  if (/Result/.test(name)) {
    tags.add("result");
  }
  if (/Cutin/.test(name)) {
    tags.add("cutin");
  }
  if (/Banner|ShipBanner/.test(name)) {
    tags.add("banner");
  }
  if (/Scene/.test(name)) {
    tags.add("scene");
  }
  if (module.source.includes("resources.getShip") || module.source.includes("ShipLoader")) {
    tags.add("ship-resource");
  }
  if (module.source.includes("resources.getSlotitem") || module.source.includes("SlotLoader")) {
    tags.add("slotitem-resource");
  }
  if (module.source.includes("getTexture(")) {
    tags.add("texture-provider");
  }

  return [...tags].sort();
}

function inferPhases(tags: readonly string[]): string[] {
  const phases = new Set<string>();
  if (tags.includes("day")) {
    phases.add("day");
  }
  if (tags.includes("night")) {
    phases.add("night");
  }
  if (tags.includes("result")) {
    phases.add("result");
  }
  if (tags.includes("cutin")) {
    phases.add("cutin");
  }
  if (phases.size === 0) {
    phases.add("day");
  }
  return [...phases];
}

function hasConditionalAncestor(path: NodePath<t.Node>): boolean {
  let current = path.parentPath;
  while (current != null) {
    if (CONDITIONAL_ANCESTOR_TYPES.has(current.node.type)) {
      return true;
    }
    current = current.parentPath;
  }
  return false;
}

function buildProtocolFieldId(module: ModuleArtifact, field: string, accessKind: string, sourceObject: string | undefined): string {
  return [
    module.id,
    field,
    accessKind,
    sourceObject ?? "unknown-source",
  ].join(":");
}

function collectProtocolFields(module: ModuleArtifact, phases: readonly string[]): BattleProtocolFieldRule[] {
  const ast = parseFactorySource(module.source);
  const fields = new Map<string, BattleProtocolFieldRule>();

  traverse(ast, {
    CallExpression(path: NodePath<t.CallExpression>) {
      const calleeChain = getCallExpressionChain(path.node.callee);
      if (calleeChain === undefined) {
        return;
      }

      const calleeName = calleeChain.split(".").at(-1);
      const accessKind = calleeName === undefined ? undefined : OBJ_UTIL_METHODS.get(calleeName);
      if (accessKind === undefined) {
        return;
      }

      const stringArgument = path.node.arguments.find(argument => {
        return !t.isSpreadElement(argument) && t.isStringLiteral(argument) && BATTLE_FIELD_RE.test(argument.value);
      });
      if (stringArgument === undefined || t.isSpreadElement(stringArgument) || !t.isStringLiteral(stringArgument)) {
        return;
      }

      const sourceObjectArgument = path.node.arguments[0];
      const sourceObject = sourceObjectArgument !== undefined && !t.isSpreadElement(sourceObjectArgument)
        ? expressionToSource(sourceObjectArgument)
        : undefined;
      const id = buildProtocolFieldId(module, stringArgument.value, accessKind, sourceObject);

      fields.set(id, {
        id,
        moduleId: module.id,
        readableName: module.readableName ?? module.fileName,
        field: stringArgument.value,
        accessKind,
        sourceObject,
        conditional: hasConditionalAncestor(path),
        phases: [...phases],
      });
    },
  });

  return [...fields.values()].sort((left, right) => left.id.localeCompare(right.id));
}

function buildResourceRuleId(module: ModuleArtifact, parts: Array<string | number | undefined>): string {
  return [module.id, ...parts.map(part => String(part ?? "none"))].join(":");
}

function collectExplicitPathRules(module: ModuleArtifact, tags: readonly string[]): BattleResourceRule[] {
  const explicitPaths = [...new Set([...module.source.matchAll(/kcs2\/resources\/[A-Za-z0-9_./-]+/g)].map(match => match[0]))];
  if (explicitPaths.length === 0) {
    return [];
  }

  return [{
    id: buildResourceRuleId(module, ["explicit-paths"]),
    moduleId: module.id,
    readableName: module.readableName ?? module.fileName,
    resourceKind: "explicit-path",
    action: "explicit-path",
    textureIds: [],
    slotMstIdSources: [],
    explicitPaths,
    triggerHints: [...tags],
  }];
}

function collectResourceRules(module: ModuleArtifact, tags: readonly string[]): BattleResourceRule[] {
  const ast = parseFactorySource(module.source);
  const shipRules = new Map<string, BattleResourceRule>();
  const slotRules = new Map<string, BattleResourceRule>();
  const textureRules = new Map<string, BattleResourceRule>();
  const shipLoaderBindings = new Set<string>();
  const slotLoaderBindings = new Set<string>();

  traverse(ast, {
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (!t.isIdentifier(path.node.id) || path.node.init == null || !t.isNewExpression(path.node.init)) {
        return;
      }

      const calleeChain = getCallExpressionChain(path.node.init.callee);
      if (calleeChain === undefined) {
        return;
      }

      if (calleeChain.endsWith("ShipLoader")) {
        shipLoaderBindings.add(path.node.id.name);
      }
      if (calleeChain.endsWith("SlotLoader")) {
        slotLoaderBindings.add(path.node.id.name);
      }
    },
  });

  traverse(ast, {
    CallExpression(path: NodePath<t.CallExpression>) {
      const calleeChain = getCallExpressionChain(path.node.callee);
      const memberCallee = t.isMemberExpression(path.node.callee) ? path.node.callee : undefined;
      const isAliasedLoaderAdd = memberCallee !== undefined
        && t.isIdentifier(memberCallee.object)
        && t.isIdentifier(memberCallee.property, { name: "add" });
      const loaderAliasName = isAliasedLoaderAdd && memberCallee !== undefined && t.isIdentifier(memberCallee.object)
        ? memberCallee.object.name
        : undefined;
      const normalizedCalleeChain = calleeChain ?? (
        shipLoaderBindings.has(loaderAliasName ?? "")
          ? `${loaderAliasName}.ShipLoader.add`
          : slotLoaderBindings.has(loaderAliasName ?? "")
            ? `${loaderAliasName}.SlotLoader.add`
            : undefined
      );
      if (normalizedCalleeChain === undefined) {
        return;
      }

      if (normalizedCalleeChain.endsWith("resources.getShip")) {
        const [shipIdArg, damagedArg, typeArg] = path.node.arguments;
        if (shipIdArg === undefined || damagedArg === undefined || typeArg === undefined) {
          return;
        }
        if (t.isSpreadElement(shipIdArg) || t.isSpreadElement(damagedArg) || t.isSpreadElement(typeArg) || !t.isStringLiteral(typeArg)) {
          return;
        }

        const shipMstIdSource = expressionToSource(shipIdArg);
        const damagedSource = expressionToSource(damagedArg);
        const id = buildResourceRuleId(module, ["getShip", typeArg.value, shipMstIdSource, damagedSource]);
        shipRules.set(id, {
          id,
          moduleId: module.id,
          readableName: module.readableName ?? module.fileName,
          resourceKind: "ship",
          action: "getShip",
          targetType: typeArg.value,
          textureIds: [],
          shipMstIdSource,
          damagedSource,
          slotMstIdSources: [],
          explicitPaths: [],
          triggerHints: [...tags],
        });
        return;
      }

      if (normalizedCalleeChain.endsWith("ShipLoader.add")) {
        const [shipIdArg, damagedArg, typeArg] = path.node.arguments;
        if (shipIdArg === undefined || damagedArg === undefined || typeArg === undefined) {
          return;
        }
        if (t.isSpreadElement(shipIdArg) || t.isSpreadElement(damagedArg) || t.isSpreadElement(typeArg) || !t.isStringLiteral(typeArg)) {
          return;
        }

        const shipMstIdSource = expressionToSource(shipIdArg);
        const damagedSource = expressionToSource(damagedArg);
        const id = buildResourceRuleId(module, ["ship-loader", typeArg.value, shipMstIdSource, damagedSource]);
        shipRules.set(id, {
          id,
          moduleId: module.id,
          readableName: module.readableName ?? module.fileName,
          resourceKind: "ship",
          action: "ship-loader",
          targetType: typeArg.value,
          textureIds: [],
          shipMstIdSource,
          damagedSource,
          slotMstIdSources: [],
          explicitPaths: [],
          triggerHints: [...tags],
        });
        return;
      }

      if (normalizedCalleeChain.endsWith("resources.getSlotitem") || normalizedCalleeChain.endsWith("SlotLoader.add")) {
        const [slotIdArg, typeArg] = path.node.arguments;
        if (slotIdArg === undefined || typeArg === undefined) {
          return;
        }
        if (t.isSpreadElement(slotIdArg) || t.isSpreadElement(typeArg) || !t.isStringLiteral(typeArg)) {
          return;
        }

        const slotMstIdSource = expressionToSource(slotIdArg);
        const action = normalizedCalleeChain.endsWith("resources.getSlotitem") ? "getSlotitem" : "slot-loader";
        const id = buildResourceRuleId(module, [action, typeArg.value, slotMstIdSource]);
        slotRules.set(id, {
          id,
          moduleId: module.id,
          readableName: module.readableName ?? module.fileName,
          resourceKind: "slotitem",
          action,
          targetType: typeArg.value,
          textureIds: [],
          slotMstIdSources: slotMstIdSource === undefined ? [] : [slotMstIdSource],
          explicitPaths: [],
          triggerHints: [...tags],
        });
        return;
      }

      if (normalizedCalleeChain.endsWith("getTexture")) {
        const provider = normalizedCalleeChain.split(".").at(-2);
        if (provider === undefined) {
          return;
        }

        const numericIds = path.node.arguments
          .filter((argument): argument is t.NumericLiteral => !t.isSpreadElement(argument) && t.isNumericLiteral(argument))
          .map(argument => argument.value);
        const id = buildResourceRuleId(module, ["getTexture", provider]);
        const existing = textureRules.get(id) ?? {
          id,
          moduleId: module.id,
          readableName: module.readableName ?? module.fileName,
          resourceKind: "texture-provider" as const,
          action: "getTexture" as const,
          provider,
          textureIds: [],
          slotMstIdSources: [],
          explicitPaths: [],
          triggerHints: [...tags],
        };

        existing.textureIds = [...new Set([...existing.textureIds, ...numericIds])].sort((left, right) => left - right);
        textureRules.set(id, existing);
      }
    },
  });

  return [
    ...shipRules.values(),
    ...slotRules.values(),
    ...textureRules.values(),
    ...collectExplicitPathRules(module, tags),
  ].sort((left, right) => left.id.localeCompare(right.id));
}

function toBattleModuleKnowledge(
  module: ModuleArtifact,
  tags: readonly string[],
  protocolFields: readonly BattleProtocolFieldRule[],
  resourceRules: readonly BattleResourceRule[],
): BattleModuleKnowledge {
  const explicitResourcePaths = resourceRules.flatMap(rule => rule.explicitPaths);
  return {
    id: module.id,
    readableName: module.readableName ?? module.fileName,
    fileName: module.fileName,
    moduleKind: module.moduleKind,
    cleanupTier: module.cleanupTier,
    tags: [...tags],
    dependencies: module.dependencies.map(dependency => ({
      moduleId: dependency.moduleId,
      readableName: dependency.readableName,
    })),
    protocolFields: protocolFields.map(rule => rule.id),
    resourceRuleIds: resourceRules.map(rule => rule.id),
    explicitResourcePaths: [...new Set(explicitResourcePaths)].sort(),
  };
}

function inferSlotTriggerConfidence(module: BattleModuleKnowledge, resourceRule: BattleResourceRule): BattleSlotResourceTrigger["confidence"] | undefined {
  if (resourceRule.resourceKind !== "slotitem") {
    return undefined;
  }

  if (!["item_up", "item_on", "btxt_flat"].includes(resourceRule.targetType ?? "")) {
    return undefined;
  }

  if (module.tags.includes("cutin")) {
    return resourceRule.targetType === "btxt_flat" ? "high" : "candidate";
  }

  if (module.readableName === "TaskLoadResourcesBattle") {
    return "candidate";
  }

  return undefined;
}

function toBattleSlotResourceTarget(targetType: string): BattleSlotResourceTrigger["resourceTarget"] {
  if (targetType === "item_on") {
    return "slot/item_on";
  }
  if (targetType === "btxt_flat") {
    return "slot/btxt_flat";
  }
  return "slot/item_up";
}

function buildSlotResourceTriggerId(module: BattleModuleKnowledge, resourceTarget: BattleSlotResourceTrigger["resourceTarget"]): string {
  return `${module.id}:${resourceTarget}`;
}

function buildSlotResourceTriggerNotes(module: BattleModuleKnowledge, resourceRule: BattleResourceRule): string {
  if (module.readableName === "CutinResourcesPreloadTask") {
    return "Cutin preload logic loads slot images and labels before battle cutin rendering.";
  }
  if (module.tags.includes("cutin")) {
    return "Cutin rendering module requests slot resources derived from attack equipment ids.";
  }
  if (module.readableName === "TaskLoadResourcesBattle") {
    return "Battle scene preload task may request slot resources before attack animations.";
  }
  return `Battle-related module ${module.readableName} requests slot resources derived from battle data.`;
}

function collectSlotResourceTriggers(
  modules: readonly BattleModuleKnowledge[],
  resourceRules: readonly BattleResourceRule[],
): BattleSlotResourceTrigger[] {
  const moduleById = new Map(modules.map(module => [module.id, module]));
  const triggers = new Map<string, BattleSlotResourceTrigger>();

  for (const resourceRule of resourceRules) {
    const module = moduleById.get(resourceRule.moduleId);
    if (module === undefined) {
      continue;
    }

    const confidence = inferSlotTriggerConfidence(module, resourceRule);
    if (confidence === undefined || resourceRule.targetType === undefined) {
      continue;
    }

    const resourceTarget = toBattleSlotResourceTarget(resourceRule.targetType);
    const id = buildSlotResourceTriggerId(module, resourceTarget);
    const existing = triggers.get(id);
    if (existing !== undefined) {
      if (existing.confidence === "candidate" && confidence === "high") {
        existing.confidence = "high";
      }
      continue;
    }

    triggers.set(id, {
      id,
      consumerModuleId: module.id,
      consumerReadableName: module.readableName,
      protocolSources: [...HOUGEKI_PROTOCOL_SOURCES],
      resourceTarget,
      confidence,
      notes: buildSlotResourceTriggerNotes(module, resourceRule),
    });
  }

  return [...triggers.values()].sort((left, right) => left.id.localeCompare(right.id));
}

// ---------------------------------------------------------------------------
// Attack-type acceptance (R2)
// ---------------------------------------------------------------------------

/// The stages whose attack-type field the server fills. `side` disambiguates
/// the two same-named `PhaseHougeki` modules: the day copy accepts 2 for 連撃
/// and 7 for 空母カットイン, the night copy accepts 1 and 6 for the same two —
/// picking the wrong one makes the acceptance set silently wrong. Matching on
/// the consumer (which dispatcher requires it), never on a webpack module id.
const ATTACK_TYPE_STAGE_SPECS = [
  {
    id: "day-shelling",
    consumerReadableName: "PhaseHougeki",
    side: "day",
    protocolField: "api_at_type",
    protocolSources: [
      "api_hougeki1.api_at_type[*]",
      "api_hougeki2.api_at_type[*]",
      "api_hougeki3.api_at_type[*]",
    ],
  },
  {
    id: "night-shelling",
    consumerReadableName: "PhaseHougeki",
    side: "night",
    protocolField: "api_sp_list",
    protocolSources: [
      "api_hougeki.api_sp_list[*]",
      "api_n_hougeki1.api_sp_list[*]",
      "api_n_hougeki2.api_sp_list[*]",
    ],
  },
  {
    id: "opening-anti-submarine",
    consumerReadableName: "PhasePreAntiSubmarine",
    side: "day",
    protocolField: "api_at_type",
    protocolSources: ["api_opening_taisen.api_at_type[*]"],
  },
] as const satisfies ReadonlyArray<{
  id: BattleAttackTypeStage["id"];
  consumerReadableName: string;
  side: "day" | "night";
  protocolField: string;
  protocolSources: readonly string[];
}>;

const DISPATCH_METHOD_NAME = "_hougeki";
const DAY_DISPATCHER_RE = /^PhaseDay/;
const NIGHT_DISPATCHER_RE = /^Phase(?:Night|AllyAttack)/;

interface DispatchExtraction {
  acceptedValues: number[];
  fallback: BattleAttackTypeFallback | null;
}

function buildDependentIndex(moduleGraph: ModuleGraph): Map<string, ModuleArtifact[]> {
  const index = new Map<string, ModuleArtifact[]>();
  for (const module of moduleGraph.modules) {
    for (const dependency of module.dependencies) {
      const dependents = index.get(dependency.moduleId);
      if (dependents === undefined) {
        index.set(dependency.moduleId, [module]);
      } else {
        dependents.push(module);
      }
    }
  }
  return index;
}

/// Which battle side a consumer module serves, decided by the dispatchers that
/// require it. Ambiguity is drift, not something to resolve by guessing.
function classifyConsumerSide(module: ModuleArtifact, dependents: readonly ModuleArtifact[]): "day" | "night" {
  const names = dependents.map(dependent => dependent.readableName).filter((name): name is string => name !== undefined);
  const dayNames = names.filter(name => DAY_DISPATCHER_RE.test(name));
  const nightNames = names.filter(name => NIGHT_DISPATCHER_RE.test(name));

  if (dayNames.length > 0 && nightNames.length === 0) {
    return "day";
  }
  if (nightNames.length > 0 && dayNames.length === 0) {
    return "night";
  }
  throw new Error(
    `cannot disambiguate battle side for module ${module.id} (${module.readableName ?? module.fileName}): `
    + `day dispatchers [${dayNames.join(", ")}], night dispatchers [${nightNames.join(", ")}]`,
  );
}

function collectRequireBindings(ast: t.File): Map<string, string> {
  const bindings = new Map<string, string>();
  traverse(ast, {
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (!t.isIdentifier(path.node.id) || path.node.init == null) {
        return;
      }
      const call = t.isCallExpression(path.node.init) ? path.node.init : undefined;
      if (call === undefined || !t.isIdentifier(call.callee, { name: "require" })) {
        return;
      }
      const [idArgument] = call.arguments;
      if (idArgument === undefined || !t.isNumericLiteral(idArgument)) {
        return;
      }
      bindings.set(path.node.id.name, String(idArgument.value));
    },
  });
  return bindings;
}

function findPrototypeMethod(ast: t.File, methodName: string): t.Function | undefined {
  let found: t.Function | undefined;
  traverse(ast, {
    AssignmentExpression(path: NodePath<t.AssignmentExpression>) {
      if (found !== undefined) {
        return;
      }
      const { left, right } = path.node;
      if (!t.isMemberExpression(left) || !t.isIdentifier(left.property, { name: methodName })) {
        return;
      }
      if (!t.isMemberExpression(left.object) || !t.isIdentifier(left.object.property, { name: "prototype" })) {
        return;
      }
      if (t.isFunctionExpression(right) || t.isArrowFunctionExpression(right)) {
        found = right;
      }
    },
  });
  return found;
}

function findConditionalChain(fn: t.Function): t.ConditionalExpression | undefined {
  if (!t.isBlockStatement(fn.body)) {
    return undefined;
  }
  for (const statement of fn.body.body) {
    if (t.isExpressionStatement(statement) && t.isConditionalExpression(statement.expression)) {
      return statement.expression;
    }
  }
  return undefined;
}

/// The identifier every branch of a dispatch chain compares against. Taken from
/// the chain itself so the extractor does not depend on the local variable name
/// surviving minification.
function dispatchSubjectName(test: t.Expression): string | undefined {
  if (t.isLogicalExpression(test)) {
    return dispatchSubjectName(test.left as t.Expression) ?? dispatchSubjectName(test.right);
  }
  if (!t.isBinaryExpression(test)) {
    return undefined;
  }
  if (t.isNumericLiteral(test.left) && t.isIdentifier(test.right)) {
    return test.right.name;
  }
  if (t.isIdentifier(test.left) && t.isNumericLiteral(test.right)) {
    return test.left.name;
  }
  return undefined;
}

/// Numeric literals the expression compares against `subject`. Numbers that
/// appear anywhere else in the test are ignored, so an unrelated index or
/// bitmask never enters the acceptance set.
function collectComparedValues(test: t.Node, subject: string, operators: readonly string[], out: Set<number>): void {
  if (t.isLogicalExpression(test)) {
    collectComparedValues(test.left, subject, operators, out);
    collectComparedValues(test.right, subject, operators, out);
    return;
  }
  if (t.isSequenceExpression(test)) {
    for (const expression of test.expressions) {
      collectComparedValues(expression, subject, operators, out);
    }
    return;
  }
  if (!t.isBinaryExpression(test) || !operators.includes(test.operator)) {
    return;
  }
  if (t.isNumericLiteral(test.left) && t.isIdentifier(test.right, { name: subject })) {
    out.add(test.left.value);
    return;
  }
  if (t.isIdentifier(test.left, { name: subject }) && t.isNumericLiteral(test.right)) {
    out.add(test.right.value);
  }
}

/// Numeric literals a function body compares against the local it binds from
/// `<record>.type`, plus whether it throws when nothing matched.
function extractGuardedAcceptance(fn: t.Function, ast: t.File): { values: number[]; closed: boolean } | undefined {
  let subject: string | undefined;
  let closed = false;
  const values = new Set<number>();

  traverse(ast, {
    VariableDeclarator(path: NodePath<t.VariableDeclarator>) {
      if (subject !== undefined || path.getFunctionParent()?.node !== fn) {
        return;
      }
      if (!t.isIdentifier(path.node.id) || !t.isMemberExpression(path.node.init) || !t.isIdentifier(path.node.init.property, { name: "type" })) {
        return;
      }
      subject = path.node.id.name;
    },
  });
  if (subject === undefined) {
    return undefined;
  }

  const boundSubject = subject;
  traverse(ast, {
    enter(path: NodePath<t.Node>) {
      if (path.getFunctionParent()?.node !== fn) {
        return;
      }
      if (t.isIfStatement(path.node) || t.isConditionalExpression(path.node) || t.isLogicalExpression(path.node)) {
        const test = t.isLogicalExpression(path.node) ? path.node : path.node.test;
        collectComparedValues(test, boundSubject, ["==", "==="], values);
      }
      if (t.isThrowStatement(path.node)) {
        closed = true;
      }
    },
  });

  return values.size === 0 ? undefined : { values: [...values].sort((left, right) => left - right), closed };
}

/// A fallback phase in another module whose constructor names its accepted
/// values outright and throws on everything else.
function extractDelegatedAcceptance(
  fallbackMethod: t.Function,
  ast: t.File,
  requireBindings: ReadonlyMap<string, string>,
  moduleById: ReadonlyMap<string, ModuleArtifact>,
): BattleAttackTypeFallback | undefined {
  let target: { module: ModuleArtifact; exportName: string; subjectIndex: number } | undefined;

  traverse(ast, {
    NewExpression(path: NodePath<t.NewExpression>) {
      if (target !== undefined || path.getFunctionParent()?.node !== fallbackMethod) {
        return;
      }
      const { callee } = path.node;
      if (!t.isMemberExpression(callee) || !t.isIdentifier(callee.object) || !t.isIdentifier(callee.property)) {
        return;
      }
      const moduleId = requireBindings.get(callee.object.name);
      const module = moduleId === undefined ? undefined : moduleById.get(moduleId);
      if (module === undefined) {
        return;
      }
      // Which constructor argument carries the attack type: the one the caller
      // passes `record.type` into.
      const subjectIndex = path.node.arguments.findIndex(argument => {
        return t.isMemberExpression(argument) && t.isIdentifier(argument.property, { name: "type" });
      });
      if (subjectIndex < 0) {
        return;
      }
      target = { module, exportName: callee.property.name, subjectIndex };
    },
  });
  if (target === undefined) {
    return undefined;
  }

  const { module, exportName, subjectIndex } = target;
  const targetAst = parseFactorySource(module.source);
  let ctor: t.FunctionDeclaration | undefined;
  traverse(targetAst, {
    FunctionDeclaration(path: NodePath<t.FunctionDeclaration>) {
      if (ctor === undefined && t.isIdentifier(path.node.id, { name: exportName })) {
        ctor = path.node;
      }
    },
  });
  if (ctor === undefined) {
    throw new Error(`module ${module.id} has no ${exportName} constructor`);
  }

  const subjectParam = ctor.params[subjectIndex];
  if (subjectParam === undefined || !t.isIdentifier(subjectParam)) {
    throw new Error(`${exportName} in module ${module.id} has no identifier parameter at index ${subjectIndex}`);
  }
  const subject = subjectParam.name;

  const values = new Set<number>();
  let closed = false;
  const ctorNode = ctor;
  traverse(targetAst, {
    IfStatement(path: NodePath<t.IfStatement>) {
      if (path.getFunctionParent()?.node !== ctorNode) {
        return;
      }
      // `N == type` accepts N; `N != type` guarding a throw accepts N too --
      // that is the last branch before the error.
      collectComparedValues(path.node.test, subject, ["==", "==="], values);
      const consequent = path.node.consequent;
      if (t.isThrowStatement(consequent) || (t.isBlockStatement(consequent) && consequent.body.some(statement => t.isThrowStatement(statement)))) {
        closed = true;
        collectComparedValues(path.node.test, subject, ["!=", "!=="], values);
      }
    },
  });

  return {
    readableName: module.readableName ?? module.fileName,
    moduleIds: [module.id],
    acceptedValues: [...values].sort((left, right) => left - right),
    closed,
  };
}

function extractDispatch(module: ModuleArtifact, moduleById: ReadonlyMap<string, ModuleArtifact>): DispatchExtraction {
  const ast = parseFactorySource(module.source);
  const method = findPrototypeMethod(ast, DISPATCH_METHOD_NAME);
  if (method === undefined) {
    throw new Error(`module ${module.id} (${module.readableName ?? module.fileName}) has no ${DISPATCH_METHOD_NAME} dispatch`);
  }
  const chain = findConditionalChain(method);
  if (chain === undefined) {
    throw new Error(`module ${module.id} (${module.readableName ?? module.fileName}) ${DISPATCH_METHOD_NAME} is not a conditional chain`);
  }

  const subject = dispatchSubjectName(chain.test);
  if (subject === undefined) {
    throw new Error(`module ${module.id} (${module.readableName ?? module.fileName}) ${DISPATCH_METHOD_NAME} compares no numeric literal`);
  }

  const values = new Set<number>();
  let current: t.ConditionalExpression = chain;
  for (;;) {
    collectComparedValues(current.test, subject, ["==", "==="], values);
    if (!t.isConditionalExpression(current.alternate)) {
      break;
    }
    current = current.alternate;
  }
  const acceptedValues = [...values].sort((left, right) => left - right);

  // The final alternate is the fallback: a `this._xxx(record)` call. That
  // method either guards the remaining values itself, or hands them to another
  // phase's constructor. Either way its acceptance joins this stage's.
  const fallbackCall = current.alternate;
  if (!t.isCallExpression(fallbackCall) || !t.isMemberExpression(fallbackCall.callee) || !t.isIdentifier(fallbackCall.callee.property)) {
    return { acceptedValues, fallback: null };
  }
  const methodName = fallbackCall.callee.property.name;
  const fallbackMethod = findPrototypeMethod(ast, methodName);
  if (fallbackMethod === undefined) {
    return { acceptedValues, fallback: null };
  }

  const guarded = extractGuardedAcceptance(fallbackMethod, ast);
  if (guarded !== undefined) {
    return {
      acceptedValues,
      fallback: {
        readableName: `${module.readableName ?? module.fileName}.${methodName}`,
        moduleIds: [module.id],
        acceptedValues: guarded.values,
        closed: guarded.closed,
      },
    };
  }

  const requireBindings = collectRequireBindings(ast);
  return {
    acceptedValues,
    fallback: extractDelegatedAcceptance(fallbackMethod, ast, requireBindings, moduleById) ?? null,
  };
}

/// Night consumers still have no protocol-source mapping in
/// `battle_slot_resource_triggers.json`, whose `protocolSources` only lists the
/// day `api_hougeki1/2/3`. This asset keys acceptance off the consumer module
/// instead, so that gap does not block it -- but the gap is still open.
const NIGHT_TRIGGER_GAP_NOTE =
  "battle_slot_resource_triggers.json still maps no protocol source to night consumers; the sources here are this asset's own.";

function buildStageNotes(spec: (typeof ATTACK_TYPE_STAGE_SPECS)[number], fallback: BattleAttackTypeFallback | null): string {
  const base = `${spec.consumerReadableName} (${spec.side}) dispatches ${spec.protocolField} directly for the values it names.`;
  const body = fallback === null
    ? `${base} It has no fallback, so its acceptance set is closed.`
    : `${base} Everything else goes to ${fallback.readableName}. ${
      fallback.closed
        ? `${fallback.readableName} throws on anything outside its own set, so the effective set is closed.`
        : `${fallback.readableName} accepts the rest without a visible guard.`
    }`;
  return spec.side === "night" ? `${body} ${NIGHT_TRIGGER_GAP_NOTE}` : body;
}

export function extractBattleAttackTypeStages(moduleGraph: ModuleGraph): BattleAttackTypeStage[] {
  const dependentIndex = buildDependentIndex(moduleGraph);
  const moduleById = new Map(moduleGraph.modules.map(module => [module.id, module]));
  const stages: BattleAttackTypeStage[] = [];

  for (const spec of ATTACK_TYPE_STAGE_SPECS) {
    const candidates = moduleGraph.modules
      .filter(module => module.readableName === spec.consumerReadableName)
      .filter(module => classifyConsumerSide(module, dependentIndex.get(module.id) ?? []) === spec.side);
    if (candidates.length === 0) {
      continue;
    }

    let acceptedValues: number[] | undefined;
    let fallback: BattleAttackTypeFallback | null = null;
    const consumerModuleIds: string[] = [];

    for (const module of candidates) {
      const extraction = extractDispatch(module, moduleById);
      const serialized = extraction.acceptedValues.join(",");
      if (acceptedValues === undefined) {
        acceptedValues = extraction.acceptedValues;
      } else if (acceptedValues.join(",") !== serialized) {
        throw new Error(
          `modules named ${spec.consumerReadableName} on the ${spec.side} side disagree on ${spec.protocolField}: `
          + `[${acceptedValues.join(", ")}] vs [${serialized}]`,
        );
      }
      consumerModuleIds.push(module.id);

      const extracted = extraction.fallback;
      if (extracted === null) {
        continue;
      }
      if (fallback === null) {
        fallback = extracted;
      } else if (fallback.acceptedValues.join(",") !== extracted.acceptedValues.join(",")) {
        throw new Error(
          `fallback ${fallback.readableName} copies disagree on ${spec.protocolField}: `
          + `[${fallback.acceptedValues.join(", ")}] vs [${extracted.acceptedValues.join(", ")}]`,
        );
      } else {
        fallback.moduleIds = [...new Set([...fallback.moduleIds, ...extracted.moduleIds])].sort();
      }
    }

    const direct = acceptedValues ?? [];
    const effective = [...new Set([...direct, ...(fallback?.acceptedValues ?? [])])].sort((left, right) => left - right);
    stages.push({
      id: spec.id,
      protocolField: spec.protocolField,
      protocolSources: [...spec.protocolSources],
      consumerReadableName: spec.consumerReadableName,
      consumerModuleIds: consumerModuleIds.sort(),
      acceptedValues: direct,
      fallback,
      effectiveAcceptedValues: effective,
      notes: buildStageNotes(spec, fallback),
    });
  }

  return stages;
}

function buildBattleKnowledgeSummary(
  modules: readonly BattleModuleKnowledge[],
  protocolFields: readonly BattleProtocolFieldRule[],
  resourceRules: readonly BattleResourceRule[],
  slotResourceTriggers: readonly BattleSlotResourceTrigger[],
  attackTypeStages: readonly BattleAttackTypeStage[],
): BattleKnowledgeSummary {
  return {
    moduleCount: modules.length,
    protocolFieldCount: protocolFields.length,
    resourceRuleCount: resourceRules.length,
    slotResourceTriggerCount: slotResourceTriggers.length,
    attackTypeStageCount: attackTypeStages.length,
    explicitResourcePathCount: resourceRules.reduce((count, rule) => count + rule.explicitPaths.length, 0),
    shipResourceRuleCount: resourceRules.filter(rule => rule.resourceKind === "ship").length,
    slotitemResourceRuleCount: resourceRules.filter(rule => rule.resourceKind === "slotitem").length,
    textureProviderRuleCount: resourceRules.filter(rule => rule.resourceKind === "texture-provider").length,
  };
}

export function extractBattleKnowledge(moduleGraph: ModuleGraph): BattleKnowledge {
  const relevantModules = moduleGraph.modules
    .filter(isBattleRelevantModule)
    .filter((module): module is ModuleArtifact & { readableName: string } => module.readableName !== undefined);

  const protocolFields: BattleProtocolFieldRule[] = [];
  const resourceRules: BattleResourceRule[] = [];
  const modules: BattleModuleKnowledge[] = [];

  for (const module of relevantModules) {
    const tags = inferTags(module);
    const phases = inferPhases(tags);
    const moduleProtocolFields = collectProtocolFields(module, phases);
    const moduleResourceRules = collectResourceRules(module, tags);

    protocolFields.push(...moduleProtocolFields);
    resourceRules.push(...moduleResourceRules);
    modules.push(toBattleModuleKnowledge(module, tags, moduleProtocolFields, moduleResourceRules));
  }

  const uniqueProtocolFields = [...new Map(protocolFields.map(field => [field.id, field])).values()]
    .sort((left, right) => left.id.localeCompare(right.id));
  const uniqueResourceRules = [...new Map(resourceRules.map(rule => [rule.id, rule])).values()]
    .sort((left, right) => left.id.localeCompare(right.id));
  const sortedModules = modules.sort((left, right) => left.id.localeCompare(right.id));
  const slotResourceTriggers = collectSlotResourceTriggers(sortedModules, uniqueResourceRules);
  const attackTypeStages = extractBattleAttackTypeStages(moduleGraph);

  return {
    summary: buildBattleKnowledgeSummary(
      sortedModules,
      uniqueProtocolFields,
      uniqueResourceRules,
      slotResourceTriggers,
      attackTypeStages,
    ),
    protocolFields: uniqueProtocolFields,
    resourceRules: uniqueResourceRules,
    slotResourceTriggers,
    attackTypeStages,
    modules: sortedModules,
  };
}

export function toBattleProtocolFieldsAsset(scriptVersion: string, knowledge: BattleKnowledge): BattleProtocolFieldsAsset {
  return {
    scriptVersion,
    summary: {
      moduleCount: knowledge.summary.moduleCount,
      protocolFieldCount: knowledge.summary.protocolFieldCount,
    },
    fields: knowledge.protocolFields,
  };
}

export function toBattleResourceRulesAsset(scriptVersion: string, knowledge: BattleKnowledge): BattleResourceRulesAsset {
  return {
    scriptVersion,
    summary: {
      moduleCount: knowledge.summary.moduleCount,
      resourceRuleCount: knowledge.summary.resourceRuleCount,
      explicitResourcePathCount: knowledge.summary.explicitResourcePathCount,
      shipResourceRuleCount: knowledge.summary.shipResourceRuleCount,
      slotitemResourceRuleCount: knowledge.summary.slotitemResourceRuleCount,
      textureProviderRuleCount: knowledge.summary.textureProviderRuleCount,
    },
    rules: knowledge.resourceRules,
  };
}

export function toBattleModuleIndexAsset(scriptVersion: string, knowledge: BattleKnowledge): BattleModuleIndexAsset {
  return {
    scriptVersion,
    summary: {
      moduleCount: knowledge.summary.moduleCount,
      protocolFieldCount: knowledge.summary.protocolFieldCount,
      resourceRuleCount: knowledge.summary.resourceRuleCount,
    },
    modules: knowledge.modules,
  };
}

export function toBattleSlotResourceTriggersAsset(scriptVersion: string, knowledge: BattleKnowledge): BattleSlotResourceTriggersAsset {
  return {
    scriptVersion,
    summary: {
      moduleCount: knowledge.summary.moduleCount,
      slotResourceTriggerCount: knowledge.summary.slotResourceTriggerCount,
    },
    triggers: knowledge.slotResourceTriggers,
  };
}

export function toBattleAttackTypeAcceptanceAsset(scriptVersion: string, knowledge: BattleKnowledge): BattleAttackTypeAcceptanceAsset {
  return {
    scriptVersion,
    summary: {
      attackTypeStageCount: knowledge.summary.attackTypeStageCount,
    },
    stages: knowledge.attackTypeStages,
  };
}
