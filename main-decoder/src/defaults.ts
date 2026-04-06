import { resolve } from "node:path";

const PROJECT_ROOT = resolve(import.meta.dir, "..");
const REPOSITORY_ROOT = resolve(PROJECT_ROOT, "..");

export const DEFAULT_KCS_CONST_PATH = resolve(REPOSITORY_ROOT, "z/cache/gadget_html5/js/kcs_const.js");
export const DEFAULT_MAIN_JS_PATH = resolve(REPOSITORY_ROOT, "z/cache/kcs2/js/main.js");
export const DEFAULT_OUTPUT_DIR = resolve(PROJECT_ROOT, "out");
export const DEFAULT_MAX_PASSES = 8;
