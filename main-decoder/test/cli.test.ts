import { expect, test } from "bun:test";

import { parseArgs } from "../src/cli.ts";

test("parseArgs keeps battle asset sync disabled by default", () => {
  const options = parseArgs(["--no-write"]);

  expect(options.writeOutputs).toBe(false);
  expect(options.syncBattleAssets).toBeUndefined();
});

test("parseArgs enables explicit battle asset syncing", () => {
  const options = parseArgs(["--sync-battle-assets"]);

  expect(options.syncBattleAssets).toBe(true);
});

test("parseArgs enables consolidated asset syncing", () => {
  const options = parseArgs(["--sync-assets"]);

  expect(options.syncAssets).toBe(true);
  expect(options.syncBattleAssets).toBe(true);
  expect(options.syncResourceManifest).toBe(true);
});
