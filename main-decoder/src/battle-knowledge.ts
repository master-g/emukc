import generate from "@babel/generator";
import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type {
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

function buildBattleKnowledgeSummary(
  modules: readonly BattleModuleKnowledge[],
  protocolFields: readonly BattleProtocolFieldRule[],
  resourceRules: readonly BattleResourceRule[],
  slotResourceTriggers: readonly BattleSlotResourceTrigger[],
): BattleKnowledgeSummary {
  return {
    moduleCount: modules.length,
    protocolFieldCount: protocolFields.length,
    resourceRuleCount: resourceRules.length,
    slotResourceTriggerCount: slotResourceTriggers.length,
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

  return {
    summary: buildBattleKnowledgeSummary(sortedModules, uniqueProtocolFields, uniqueResourceRules, slotResourceTriggers),
    protocolFields: uniqueProtocolFields,
    resourceRules: uniqueResourceRules,
    slotResourceTriggers,
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
