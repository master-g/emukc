import generate from "@babel/generator";
import { parse } from "@babel/parser";
import traverse, { type NodePath } from "@babel/traverse";
import * as t from "@babel/types";

import type {
  BattleAttackTypeFallback,
  BattleAttackTypeStage,
  ModuleArtifact,
  ModuleGraph,
} from "./types.ts";

function parseFactorySource(source: string): t.File {
  return parse(`(${source});`, {
    sourceType: "script",
    allowReturnOutsideFunction: true,
  });
}

// ---------------------------------------------------------------------------
// Attack-type acceptance (R2)
// ---------------------------------------------------------------------------

/**
 * The stages whose attack-type field the server fills. `side` disambiguates
 * the two same-named `PhaseHougeki` modules: the day copy accepts 2 for 連撃
 * and 7 for 空母カットイン, the night copy accepts 1 and 6 for the same two —
 * picking the wrong one makes the acceptance set silently wrong. Matching on
 * the consumer (which dispatcher requires it), never on a webpack module id.
 */
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

/**
 * Which battle side a consumer module serves, decided by the dispatchers that
 * require it. Ambiguity is drift, not something to resolve by guessing.
 */
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

/**
 * The identifier every branch of a dispatch chain compares against. Taken from
 * the chain itself so the extractor does not depend on the local variable name
 * surviving minification.
 */
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

/**
 * Numeric literals the expression compares against `subject`. Numbers that
 * appear anywhere else in the test are ignored, so an unrelated index or
 * bitmask never enters the acceptance set.
 */
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

/**
 * Numeric literals a function body compares against the local it binds from
 * `<record>.type`, plus whether it throws when nothing matched.
 */
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

/**
 * A fallback phase in another module whose constructor names its accepted
 * values outright and throws on everything else.
 */
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

  // `module.dependencies` already carries every `require(N)` binding the module
  // graph resolved, so the namespace a `new <ns>.<Name>()` names can be mapped
  // to a module id without walking the AST again.
  const requireBindings = new Map(
    module.dependencies
      .filter(dependency => dependency.importStyle === "require" && dependency.localName !== undefined)
      .map(dependency => [dependency.localName as string, dependency.moduleId]),
  );
  return {
    acceptedValues,
    fallback: extractDelegatedAcceptance(fallbackMethod, ast, requireBindings, moduleById) ?? null,
  };
}

/**
 * Night consumers still have no protocol-source mapping in
 * `battle_slot_resource_triggers.json`, whose `protocolSources` only lists the
 * day `api_hougeki1/2/3`. This asset keys acceptance off the consumer module
 * instead, so that gap does not block it -- but the gap is still open.
 */
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
  const unresolved: string[] = [];

  for (const spec of ATTACK_TYPE_STAGE_SPECS) {
    const candidates = moduleGraph.modules
      .filter(module => module.readableName === spec.consumerReadableName)
      .filter(module => classifyConsumerSide(module, dependentIndex.get(module.id) ?? []) === spec.side);
    if (candidates.length === 0) {
      unresolved.push(spec.id);
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

  // Resolving some stages but not all means the graph is a battle graph whose
  // shape moved: a consumer was renamed, or a dispatcher stopped requiring it.
  // Dropping the stage would leave the Rust-side assertion permanently true for
  // that phase, so it is drift and it stops the run. A graph that resolved none
  // is a partial graph (a unit-test fixture), not drift.
  if (stages.length > 0 && unresolved.length > 0) {
    throw new Error(
      `attack-type acceptance resolved ${stages.length} stage(s) but not [${unresolved.join(", ")}]; `
      + "the consuming modules moved, and shipping the asset without them would silently disable those checks",
    );
  }

  return stages;
}
