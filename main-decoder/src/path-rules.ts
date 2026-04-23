import { readFileSync } from "node:fs";
import { resolve } from "node:path";

export interface ResourceManifestShipHoles {
	full: number[];
	fullDmg: number[];
	up: number[];
	upDmg: number[];
}

export interface ResourceManifestPathRules {
	shipDamageVariants: Record<string, string[]>;
	shipStandardCategories: string[];
	shipFullCategories: string[];
	slotStandardCategories: string[];
	enemyPlaneIds: number[];
	btxtFlatSlotIds: number[];
	characterHoleIds: number[];
	eventShipHoles: ResourceManifestShipHoles;
	enemyShipHoles: ResourceManifestShipHoles;
	specialShips: number[];
	spRemodelShips: number[];
	spRemodelMes: number[];
	cardRounds: number[];
	rewardShips: number[];
}

let cachedPathRules: ResourceManifestPathRules | undefined;

function repoPath(...segments: string[]): string {
	return resolve(import.meta.dir, "../..", ...segments);
}

function readRepoFile(...segments: string[]): string {
	return readFileSync(repoPath(...segments), "utf8");
}

function requireMatch(source: string, pattern: RegExp, label: string): RegExpExecArray {
	const match = pattern.exec(source);
	if (match === null) {
		throw new Error(`Failed to parse ${label}`);
	}
	return match;
}

function parseStringArrayBlock(source: string, label: string): string[] {
	return [...source.matchAll(/"([^"]+)"/g)].map(match => match[1] ?? "").filter(value => value.length > 0);
}

function parseNumberArrayBlock(source: string, label: string): number[] {
	const values = [...source.matchAll(/-?\d+/g)].map(match => Number.parseInt(match[0], 10));
	if (values.some(Number.isNaN)) {
		throw new Error(`Failed to parse numeric values for ${label}`);
	}
	return values;
}

function parseRustConstStringArray(source: string, name: string): string[] {
	const match = requireMatch(
		source,
		new RegExp(`const ${name}: &\\[&str\\] = &\\[(.*?)\\];`, "s"),
		name,
	);
	return parseStringArrayBlock(match[1] ?? "", name);
}

function parseRustLazyNumberArray(source: string, name: string): number[] {
	const match = requireMatch(
		source,
		new RegExp(`static ${name}: LazyLock<Vec<i64>> = LazyLock::new\\(\\|\\|\\s*(?:\\{\\s*)?vec!\\[(.*?)\\]\\s*(?:\\})?\\);`, "s"),
		name,
	);
	return parseNumberArrayBlock(match[1] ?? "", name);
}

function parseRustEnemyPlaneIds(source: string): number[] {
	const match = requireMatch(source, /const ENEMY_PLANE_MAX_ID: usize = (\d+);/, "ENEMY_PLANE_MAX_ID");
	const maxId = Number.parseInt(match[1] ?? "0", 10);
	return Array.from({ length: maxId }, (_, index) => index + 1);
}

function parseRustDamageVariants(source: string): Record<string, string[]> {
	const block = requireMatch(
		source,
		/const SHIP_DAMAGE_VARIANTS: &\[\(&str, &\[&str\]\)\] = &\[(.*?)\];/s,
		"SHIP_DAMAGE_VARIANTS",
	)[1] ?? "";
	const entries = [...block.matchAll(/\("([^"]+)",\s*&\[(.*?)\]\)/gs)];
	return Object.fromEntries(entries.map(match => [match[1] ?? "", parseStringArrayBlock(match[2] ?? "", "SHIP_DAMAGE_VARIANTS entry")]));
}

function parseRustShipHoles(source: string, name: string): ResourceManifestShipHoles {
	const block = requireMatch(
		source,
		new RegExp(`static ${name}: LazyLock<[^>]+> = LazyLock::new\\(\\|\\| [^{]+\\{([\\s\\S]*?)\\n\\}\\);`, "m"),
		name,
	)[1] ?? "";

	const parseField = (field: string): number[] => {
		const fieldMatch = new RegExp(`${field}: vec!\\[(.*?)\\]`, "s").exec(block);
		return fieldMatch === null ? [] : parseNumberArrayBlock(fieldMatch[1] ?? "", `${name}.${field}`);
	};

	return {
		full: parseField("full"),
		fullDmg: parseField("full_dmg"),
		up: parseField("up"),
		upDmg: parseField("up_dmg"),
	};
}

export function buildPathRules(): ResourceManifestPathRules {
	if (cachedPathRules !== undefined) {
		return cachedPathRules;
	}

	const generateRs = readRepoFile("crates", "emukc_bootstrap", "src", "make_list", "manifest", "generate.rs");
	const slotRs = readRepoFile("crates", "emukc_bootstrap", "src", "make_list", "source", "kcs2", "resources", "slot.rs");
	const shipRs = readRepoFile("crates", "emukc_bootstrap", "src", "make_list", "source", "kcs2", "resources", "ship.rs");

	cachedPathRules = {
		shipDamageVariants: parseRustDamageVariants(generateRs),
		shipStandardCategories: parseRustConstStringArray(generateRs, "SHIP_STANDARD_CATEGORIES"),
		shipFullCategories: parseRustConstStringArray(generateRs, "SHIP_FULL_CATEGORIES"),
		slotStandardCategories: parseRustConstStringArray(generateRs, "SLOT_STANDARD_CATEGORIES"),
		enemyPlaneIds: parseRustEnemyPlaneIds(slotRs),
		btxtFlatSlotIds: parseRustLazyNumberArray(slotRs, "BTXT_FLAT_IDS"),
		characterHoleIds: parseRustLazyNumberArray(slotRs, "CHARACTER_HOLES"),
		eventShipHoles: parseRustShipHoles(shipRs, "EVENT_SHIP_HOLES"),
		enemyShipHoles: parseRustShipHoles(shipRs, "ENEMY_SHIP_HOLES"),
		specialShips: parseRustLazyNumberArray(shipRs, "SPECIAL_SHIPS"),
		spRemodelShips: parseRustLazyNumberArray(shipRs, "SP_REMODEL_SHIPS"),
		spRemodelMes: parseRustLazyNumberArray(shipRs, "SP_REMODEL_MES"),
		cardRounds: parseRustLazyNumberArray(shipRs, "CARD_ROUNDS"),
		rewardShips: parseRustLazyNumberArray(shipRs, "REWARDS"),
	};

	return cachedPathRules;
}
