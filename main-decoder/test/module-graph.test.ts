import { expect, test } from "bun:test";

import { extractModuleGraph } from "../src/module-graph.ts";

function buildModuleTableSource(modules: Record<string, string>): string {
  const fillerEntries = Array.from({ length: 100 }, (_, index) => {
    return `${900000 + index}: function(module, exports, require) {}`;
  });
  const moduleEntries = Object.entries(modules).map(([moduleId, body]) => {
    return `${moduleId}: function(module, exports, require) {\n${body}\n}`;
  });

  return `var __moduleTable = {\n${[...fillerEntries, ...moduleEntries].join(",\n")}\n};`;
}

function createObfuscatedDeclarations(start: number, count: number): string {
  return Array.from({ length: count }, (_, index) => {
    const bindingId = `_0x${(start + index).toString(16)}`;
    return `var ${bindingId} = ${index};`;
  }).join("\n");
}

function getModuleByReadableName(
  source: string,
  readableName: string,
  occurrence = 0,
) {
  const matches = extractModuleGraph(source).modules.filter(module => module.readableName === readableName);
  const target = matches[occurrence];
  if (target === undefined) {
    throw new Error(`Expected module ${readableName} at occurrence ${occurrence}`);
  }
  return target;
}

function getModuleById(source: string, moduleId: string) {
  const target = extractModuleGraph(source).modules.find(module => module.id === moduleId);
  if (target === undefined) {
    throw new Error(`Expected module ${moduleId}`);
  }
  return target;
}

test("renames named-game params and locals while skipping conflicting inferred names", () => {
  const source = buildModuleTableSource({
    101: `
function ConflictView(_0x101, _0x102) {
  this.foo = _0x101;
  this.bar = _0x102;
  var _0x103 = this._baz;
  var _0x104 = this._qux;
  var _0x105 = this._items;
  var _0x106 = this._items;
  return _0x103 + _0x104 + _0x105.length + _0x106.length;
}
exports.ConflictView = ConflictView;
`,
  });

  const module = getModuleByReadableName(source, "ConflictView");

  expect(module.cleanupTier).toBe("named-game");
  expect(module.hotspotCleanup?.appliedRules).toContain("param-rename");
  expect(module.hotspotCleanup?.appliedRules).toContain("local-rename");
  expect(module.source).toContain("function ConflictView(foo, bar)");
  expect(module.source).toContain("var baz = this._baz;");
  expect(module.source).toContain("var qux = this._qux;");
  expect(module.source).toContain("var _0x105 = this._items;");
  expect(module.source).toContain("var _0x106 = this._items;");
  expect(module.source).not.toContain("var items = this._items;");
  expect(module.source).not.toContain("var items2 = this._items;");
});

test("skips renames when params and locals infer the same candidate name", () => {
  const source = buildModuleTableSource({
    151: `
function SharedNameView(_0x151) {
  this.foo = _0x151;
  var _0x152 = this._foo;
  return _0x152;
}
exports.SharedNameView = SharedNameView;
`,
  });

  const module = getModuleByReadableName(source, "SharedNameView");

  expect(module.cleanupTier).toBe("named-game");
  expect(module.source).toContain("function SharedNameView(_0x151)");
  expect(module.source).toContain("var _0x152 = this._foo;");
  expect(module.source).not.toContain("function SharedNameView(foo)");
  expect(module.source).not.toContain("var foo = this._foo;");
});

test("splits priority-body sequence expressions and returns", () => {
  const source = buildModuleTableSource({
    201: `
function SequenceView() {
  ${createObfuscatedDeclarations(0x600, 260)}
  _0x600 += 1, _0x601 += 2, this._state = _0x600 + _0x601;
  return _0x602 += 3, _0x603 += 4, _0x600 + _0x601 + _0x602 + _0x603;
}
exports.SequenceView = SequenceView;
`,
  });

  const module = getModuleByReadableName(source, "SequenceView");

  expect(module.cleanupTier).toBe("priority-body");
  expect(module.hotspotCleanup?.appliedRules).toContain("sequence-expression-split");
  expect(module.hotspotCleanup?.appliedRules).toContain("sequence-return-split");
  expect(module.source).toContain("_0x600 += 1;");
  expect(module.source).toContain("_0x601 += 2;");
  expect(module.source).toContain("this._state = _0x600 + _0x601;");
  expect(module.source).toContain("return _0x600 + _0x601 + _0x602 + _0x603;");
  expect(module.source).not.toContain("_0x600 += 1, _0x601 += 2, this._state");
  expect(module.source).not.toContain("return _0x602 += 3, _0x603 += 4,");
});

test("preserves non-legacy if-test sequences even in the priority-body cohort", () => {
  const source = buildModuleTableSource({
    301: `
function GuardedView() {
  ${createObfuscatedDeclarations(0x700, 260)}
  if (_0x700 += 1, _0x701 += 2, _0x700 < _0x701) {
    return _0x700;
  }
  return _0x701;
}
exports.GuardedView = GuardedView;
`,
  });

  const module = getModuleByReadableName(source, "GuardedView");

  expect(module.cleanupTier).toBe("priority-body");
  expect(module.hotspotCleanup?.appliedRules).not.toContain("legacy-sequence-if-split");
  expect(module.source).toContain("if (_0x700 += 1, _0x701 += 2, _0x700 < _0x701)");
});

test("splits legacy if-test sequences only for the proven-safe cohort", () => {
  const source = buildModuleTableSource({
    401: `
function SupplyScene() {
  ${createObfuscatedDeclarations(0x800, 260)}
  if (_0x800 += 1, _0x801 += 2, _0x800 < _0x801) {
    return _0x800;
  }
  return _0x801;
}
exports.SupplyScene = SupplyScene;
`,
  });

  const module = getModuleByReadableName(source, "SupplyScene");

  expect(module.cleanupTier).toBe("priority-body");
  expect(module.hotspotCleanup?.appliedRules).toContain("legacy-sequence-if-split");
  expect(module.source).toContain("_0x800 += 1;");
  expect(module.source).toContain("_0x801 += 2;");
  expect(module.source).toContain("if (_0x800 < _0x801)");
  expect(module.source).not.toContain("if (_0x800 += 1, _0x801 += 2, _0x800 < _0x801)");
});

test("disambiguates dependency aliases when readable names collide", () => {
  const source = buildModuleTableSource({
    501: `
function ConfirmView() {}
exports.ConfirmView = ConfirmView;
`,
    502: `
function ConfirmView() {}
exports.ConfirmView = ConfirmView;
`,
    503: `
var _0x901 = require(501);
var _0x902 = require(502);
function ConsumerView() {
  return _0x901.ConfirmView || _0x902.ConfirmView;
}
exports.ConsumerView = ConsumerView;
`,
  });

  const consumer = getModuleById(source, "503");

  expect(consumer.cleanupTier).toBe("named-game");
  expect(consumer.dependencies.map(dependency => dependency.localName)).toEqual([
    "confirmViewModule",
    "confirmViewModule2",
  ]);
  expect(consumer.source).toContain("var confirmViewModule = require(501);");
  expect(consumer.source).toContain("var confirmViewModule2 = require(502);");
});

test("normalizes hex literals to decimal in module output", () => {
  const source = buildModuleTableSource({
    601: `
function HexView(_0xabc) {
  var x = 0x0;
  var y = 0xFF;
  if (0x1 == _0xabc) return 0x64;
  if (0x191 == _0xabc || 0x190 == _0xabc) return 0x12c;
  return x + y;
}
exports.HexView = HexView;
`,
  });

  const module = getModuleByReadableName(source, "HexView");

  expect(module.source).toContain("var x = 0;");
  expect(module.source).toContain("var y = 255;");
  expect(module.source).toContain("if (1 == _0xabc)");
  expect(module.source).toContain("return 100;");
  expect(module.source).toContain("YAMATO_B");
  expect(module.source).toContain("YAMATO_A");
  expect(module.source).toContain("return 300;");
});

test("splits sequence expressions and returns in named-game (non-priority) modules", () => {
  const source = buildModuleTableSource({
    701: `
function SimpleView() {
  var a = 1, b = 2;
  a += 1, b += 2, this._result = a + b;
  return a += 10, b += 20, a + b;
}
exports.SimpleView = SimpleView;
`,
  });

  const module = getModuleByReadableName(source, "SimpleView");

  expect(module.cleanupTier).toBe("named-game");
  expect(module.hotspotCleanup?.appliedRules).toContain("sequence-expression-split");
  expect(module.hotspotCleanup?.appliedRules).toContain("sequence-return-split");
  expect(module.source).toContain("a += 1;");
  expect(module.source).toContain("b += 2;");
  expect(module.source).toContain("this._result = a + b;");
  expect(module.source).toContain("a += 10;");
  expect(module.source).toContain("b += 20;");
  expect(module.source).toContain("return a + b;");
  expect(module.source).not.toContain("a += 1, b += 2, this._result");
  expect(module.source).not.toContain("return a += 10, b += 20,");
});

test("annotates known enum values in comparison contexts", () => {
  const source = buildModuleTableSource({
    801: `
function BattleView(data) {
  if (100 == data.type) return "nelson";
  if (101 === data.type) return "nagato";
  if (102 != data.type) return "not_mutsu";
  if (103 !== data.type) return "not_colorado";
  if (400 == data.type || 401 == data.type) return "yamato";
  if (200 == data.type) return "zuiun";
  if (1000 == data.type) return "sp_type4";
  if (6 == data.type) return "kubo";
  return "unknown";
}
exports.BattleView = BattleView;
`,
  });

  const module = getModuleByReadableName(source, "BattleView");

  expect(module.source).toMatch(/100\s*\/\*\s*NELSON_TOUCH\s*\*\//);
  expect(module.source).toMatch(/101\s*\/\*\s*NAGATO\s*\*\//);
  expect(module.source).toMatch(/102\s*\/\*\s*MUTSU\s*\*\//);
  expect(module.source).toMatch(/103\s*\/\*\s*COLORADO\s*\*\//);
  expect(module.source).toMatch(/400\s*\/\*\s*YAMATO_A\s*\*\//);
  expect(module.source).toMatch(/401\s*\/\*\s*YAMATO_B\s*\*\//);
  expect(module.source).toMatch(/200\s*\/\*\s*ZUIUN_CUTIN\s*\*\//);
  expect(module.source).toMatch(/1000\s*\/\*\s*SP_TYPE4\s*\*\//);
  expect(module.source).toMatch(/6\s*\/\*\s*KUBO_CI\s*\*\//);
  expect(module.hotspotCleanup?.appliedRules).toContain("enum-annotate");
});

test("renames locals assigned from readable and obfuscated object property access", () => {
  const source = buildModuleTableSource({
    901: `
function ObjAccessView(scene, data) {
  var _0xaa1 = scene.data;
  var _0xaa2 = data.type;
  var _0xaa3 = data.list;
  var _0xaa4 = _0xaa2.items;
  return _0xaa1 + _0xaa2 + _0xaa3 + _0xaa4;
}
exports.ObjAccessView = ObjAccessView;
`,
  });

  const module = getModuleByReadableName(source, "ObjAccessView");

  expect(module.source).toContain("var data2 = scene.data;");
  expect(module.source).toContain("var type = data.type;");
  expect(module.source).toContain("var list = data.list;");
  expect(module.source).toContain("var items = type.items;");
});
