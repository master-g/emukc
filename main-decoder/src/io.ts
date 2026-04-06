import { mkdir } from "node:fs/promises";
import { dirname, resolve } from "node:path";

import { DEFAULT_KCS_CONST_PATH, DEFAULT_MAIN_JS_PATH, DEFAULT_OUTPUT_DIR } from "./defaults.ts";
import type { LoadedSources, PipelineOptions, SourcePaths } from "./types.ts";

const SCRIPT_VERSION_RE = /scriptVesion\s*=\s*["']([^"'\\\r\n]+)["']|scriptVersion\s*=\s*["']([^"'\\\r\n]+)["']/;

async function readRequiredText(path: string): Promise<string> {
  const file = Bun.file(path);
  if (!(await file.exists())) {
    throw new Error(`Required input file does not exist: ${path}`);
  }

  return await file.text();
}

export function resolveSourcePaths(options: PipelineOptions = {}): SourcePaths {
  return {
    kcConstPath: resolve(options.kcConstPath ?? DEFAULT_KCS_CONST_PATH),
    mainJsPath: resolve(options.mainJsPath ?? DEFAULT_MAIN_JS_PATH),
    outputDir: resolve(options.outputDir ?? DEFAULT_OUTPUT_DIR),
  };
}

export function extractScriptVersion(kcConstSource: string): string {
  const match = SCRIPT_VERSION_RE.exec(kcConstSource);
  const scriptVersion = match?.[1] ?? match?.[2];

  if (scriptVersion === undefined) {
    throw new Error("Failed to extract scriptVesion/scriptVersion from kcs_const.js");
  }

  return scriptVersion;
}

export async function loadLocalSources(options: PipelineOptions = {}): Promise<LoadedSources> {
  const paths = resolveSourcePaths(options);
  const [kcConstSource, mainSource] = await Promise.all([
    readRequiredText(paths.kcConstPath),
    readRequiredText(paths.mainJsPath),
  ]);

  return {
    paths,
    kcConstSource,
    mainSource,
    scriptVersion: extractScriptVersion(kcConstSource),
  };
}

export async function writeTextFile(path: string, content: string): Promise<void> {
  await mkdir(dirname(path), { recursive: true });
  await Bun.write(path, content);
}
