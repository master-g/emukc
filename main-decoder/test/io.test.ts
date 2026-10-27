import { expect, test } from "bun:test";
import { mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { writeJsonAsset } from "../src/io.ts";

test("an asset whose only change is generatedAt is not rewritten", async () => {
  const path = join(mkdtempSync(join(tmpdir(), "emukc-decoder-")), "asset.json");
  const asset = (stamp: string, count: number) => ({ generatedAt: stamp, nested: { generatedAt: stamp, count } });

  await writeJsonAsset(path, asset("first", 1));
  await writeJsonAsset(path, asset("second", 1));
  expect(JSON.parse(readFileSync(path, "utf8"))).toEqual(asset("first", 1));

  await writeJsonAsset(path, asset("third", 2));
  expect(JSON.parse(readFileSync(path, "utf8"))).toEqual(asset("third", 2));
});
