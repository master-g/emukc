// Loads the decoded client bundle without starting the game, for tools that run the client's
// own code against data the server produced.

import { readFileSync } from "node:fs";

export type ClientRequire = (id: number) => any;

export interface Client {
	require: ClientRequire;
	/** The id of the module that declares the export `name`; ids change with every build. */
	moduleExporting(name: string): number;
	/** The decoded source of one module. */
	moduleSource(id: number): string;
}

/** The webpack id of the bundle's entry module, checked against 6.3.5.0. */
const ENTRY_MODULE = 32875;

export function loadClient(bundlePath: string, entry = ENTRY_MODULE): Client {
	// The modules pull in utility code that touches the renderer at load time. Nothing here
	// calls into it, so anything that swallows property reads and calls will do.
	const stub: any = new Proxy(function () {}, {
		get: (target, key) => (key === "prototype" ? (target as any).prototype : key === Symbol.toPrimitive ? () => 0 : stub),
		apply: () => stub,
		construct: () => ({}),
	});
	const globals = globalThis as any;
	globals.window = globalThis;
	for (const name of ["PIXI", "document", "createjs", "Howl", "Howler", "WebFont"]) {
		globals[name] = stub;
	}

	const source = readFileSync(bundlePath, "utf8");
	const bootstrap = new RegExp(`var (_0x[0-9a-f]+) = (_0x[0-9a-f]+)\\(${entry}\\);\\s*return \\1 = \\1\\.default;`);
	const found = source.match(bootstrap);
	if (!found) {
		throw new Error(`bundle bootstrap not found in ${bundlePath}; the entry module id probably changed`);
	}
	const patched = source.replace(bootstrap, `globalThis.__clientRequire = ${found[2]}; return {};`);
	const module = { exports: {} };
	new Function("self", "require", "module", "exports", patched)(globalThis, () => ({}), module, module.exports);

	// Modules are the entries of one object literal, each starting `<hex id>(` at the same depth.
	const starts = [...source.matchAll(/^ {6}(0x[0-9a-f]+)\(/gm)].map((match) => ({ id: Number(match[1]), at: match.index }));
	const moduleAt = (index: number) => starts.findLast((start) => start.at <= index);
	return {
		require: globals.__clientRequire,
		moduleExporting(name) {
			const declared = source.indexOf(`.${name} = undefined`);
			const owner = declared < 0 ? undefined : moduleAt(declared);
			if (!owner) throw new Error(`no module of ${bundlePath} declares the export ${name}`);
			return owner.id;
		},
		moduleSource(id) {
			const index = starts.findIndex((start) => start.id === id);
			if (index < 0) throw new Error(`no module ${id} in ${bundlePath}`);
			return source.slice(starts[index]!.at, starts[index + 1]?.at);
		},
	};
}
