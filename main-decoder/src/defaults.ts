import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

const PROJECT_ROOT = resolve(import.meta.dir, "..");
const REPOSITORY_ROOT = resolve(PROJECT_ROOT, "..");
const CONFIG_PATH = resolve(REPOSITORY_ROOT, "emukc.config.toml");

/** The server's `cache_root`, relative to its config as the server reads it; `z/cache` with no config. */
export const CACHE_ROOT = resolve(
  REPOSITORY_ROOT,
  existsSync(CONFIG_PATH) ? (Bun.TOML.parse(readFileSync(CONFIG_PATH, "utf8")) as { cache_root: string }).cache_root : "z/cache",
);

export const DEFAULT_KCS_CONST_PATH = resolve(CACHE_ROOT, "gadget_html5/js/kcs_const.js");
export const DEFAULT_MAIN_JS_PATH = resolve(CACHE_ROOT, "kcs2/js/main.js");
export const DEFAULT_WORLD_JS_PATH = resolve(REPOSITORY_ROOT, "assets/www/emukc/js/world.js");
export const DEFAULT_OUTPUT_DIR = resolve(PROJECT_ROOT, "out");
export const DEFAULT_MAX_PASSES = 8;
