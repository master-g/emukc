import { parse } from "@babel/parser";
import { expect, test } from "bun:test";

import { decodeBundle } from "../src/decode.ts";

const STRINGS = ["Sequence", "bg", "execute", "bind", "onTick"];

function decode(wrapperSource: string): string {
  return decodeBundle({
    wrapperBangIndex: 0,
    helperStartIndex: 0,
    decoderFunctionName: "_0xdec",
    helperFunctionNames: ["_0xdec"],
    prefixSource: "",
    helperSource: "",
    wrapperSource,
    decoderRuntimeSource: `function _0xdec(n){return ${JSON.stringify(STRINGS)}[n-0x10];}`,
  }).decodedSource;
}

// main.js 6.3.6.0 is ES2015: decoder aliases are `const`, and class members are keyed by
// decoder calls.
test("decodes an ES2015 bundle into source that still parses", () => {
  const decoded = decode([
    "(function(){const _0xa=_0xdec;",
    "class _0xc{static[_0xa(0x10)](){}get[_0xa(0x11)](){return 1;}[_0xa(0x12)](_0xb){}",
    "constructor(){this[_0xa(0x14)]=1;}[_0xa(0x12)](_0xb,_0xd=(0x1)){}}",
    "var _0xe=function(){}[_0xa(0x13)](this);})",
  ].join(""));

  expect(() => parse(decoded, { sourceType: "script" })).not.toThrow();
  expect(decoded).toContain("static Sequence(){}");
  expect(decoded).toContain("get bg(){");
  expect(decoded).toContain("}execute(_0xb,_0xd=(1)){}");
  expect(decoded).toContain("this.onTick=1");
  expect(decoded).toContain("function(){}.bind(this)");
  expect(decoded).not.toContain("_0xdec(");
});
