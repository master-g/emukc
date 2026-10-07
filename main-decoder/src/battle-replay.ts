// Replays dumped battle responses through the client's own data models.
//
// `battle validate` checks a packet against rules extracted from the client, so a
// misreading of the protocol can sit in the packet and in the rule at once. This loads the
// decoded client's record classes instead — `BattleRecordDay`, `BattleRecordNight` and what
// they build — reads every attack through them in the order the client's phase runner plays
// them, and checks that the HP the client would end up showing is the HP the server settled.
//
// It covers how the client *reads* a packet. It does not run the phase classes themselves,
// so animation, resource loading and scene sequencing stay untested.
//
// Input is the directory `EMUKC_DUMP_DIR` receives from the gameplay test
// `enemy_combined_boss_runs_day_night_and_result`. A manually run diagnostic, not a test.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

/** Webpack ids, checked against 6.3.5.0; a new client build renumbers them. */
const MODULES = { entry: 32875, recordDay: 66019, recordNight: 53311 };

/** `PhaseDay_06vs12`: the order the client plays a single fleet against an enemy combined one. */
const DAY_ORDER = ["air_war", "taisen_opening", "raigeki_opening", "hougeki1", "raigeki", "hougeki2", "hougeki3"] as const;

type Json = Record<string, unknown>;
type ClientRequire = (id: number) => any;

/** Load the decoded bundle without starting the game, and hand back its module loader. */
function loadClient(bundlePath: string): ClientRequire {
	// The record classes pull in utility modules that touch the renderer at load time. Nothing
	// here calls into it, so anything that swallows property reads and calls will do.
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
	const bootstrap = new RegExp(`var (_0x[0-9a-f]+) = (_0x[0-9a-f]+)\\(${MODULES.entry}\\);\\s*return \\1 = \\1\\.default;`);
	const found = source.match(bootstrap);
	if (!found) {
		throw new Error(`bundle bootstrap not found in ${bundlePath}; the entry module id probably changed`);
	}
	const patched = source.replace(bootstrap, `globalThis.__clientRequire = ${found[2]}; return {};`);
	const module = { exports: {} };
	new Function("self", "require", "module", "exports", patched)(globalThis, () => ({}), module, module.exports);
	return globals.__clientRequire;
}

/** One side's ships in the client's index space: the escort deck sits at 6..=11. */
class Side {
	readonly hp = new Map<number, number>();

	constructor(
		readonly name: string,
		now: (index: number) => number,
		max: (index: number) => number,
	) {
		for (let index = 0; index < 12; index++) {
			if (max(index) > 0) {
				this.hp.set(index, now(index));
			}
		}
	}

	indices(): number[] {
		return [...this.hp.keys()];
	}
}

class Replay {
	readonly problems: string[] = [];
	private attacks = 0;

	constructor(
		readonly friend: Side,
		readonly enemy: Side,
	) {}

	get attackCount(): number {
		return this.attacks;
	}

	problem(message: string) {
		this.problems.push(message);
	}

	/** Apply one hit the way the client does: the target must be a ship it has on screen. */
	hit(phase: string, attackerIsEnemy: boolean, attacker: number | null, target: number, damage: number) {
		const attackers = attackerIsEnemy ? this.enemy : this.friend;
		const targets = attackerIsEnemy ? this.friend : this.enemy;
		this.attacks++;
		if (attacker !== null) {
			const hp = attackers.hp.get(attacker);
			if (hp === undefined) {
				this.problem(`${phase}: ${attackers.name} attacker ${attacker} is not a ship in the packet`);
			} else if (hp <= 0) {
				this.problem(`${phase}: ${attackers.name} ${attacker} attacks after it was sunk`);
			}
		}
		const hp = targets.hp.get(target);
		if (hp === undefined) {
			this.problem(`${phase}: ${targets.name} target ${target} is not a ship in the packet`);
			return;
		}
		targets.hp.set(target, hp - damage);
	}

	/** `HougekiListData` / `HougekiListNightData`. */
	shelling(phase: string, data: any, enemyDeck?: (index: number) => boolean) {
		for (const attack of data?.list ?? []) {
			const attackerIsEnemy = attack.flag === 1;
			if (enemyDeck && attackerIsEnemy && !enemyDeck(attack.a_index)) {
				this.problem(`${phase}: enemy ${attack.a_index} attacks from the deck that is not fighting`);
			}
			attack.d_indexes.forEach((target: number, slot: number) => {
				if (target < 0) {
					return;
				}
				if (enemyDeck && !attackerIsEnemy && !enemyDeck(target)) {
					this.problem(`${phase}: enemy ${target} is hit on the deck that is not fighting`);
				}
				this.hit(phase, attackerIsEnemy, attack.a_index, target, attack.getDamage(slot));
			});
		}
	}

	/** `RaigekiData`: one target per ship. */
	torpedo(phase: string, data: any) {
		if (!data) {
			return;
		}
		for (const index of this.friend.indices()) {
			const target = data.getAttackTo_f(index);
			if (target >= 0) {
				this.hit(phase, false, index, target, data.getDamage_f(index));
			}
		}
		for (const index of this.enemy.indices()) {
			const target = data.getAttackTo_e(index);
			if (target >= 0) {
				this.hit(phase, true, index, target, data.getDamage_e(index));
			}
		}
	}

	/** `RaigekiOpeningData`: a ship may fire at several targets. */
	openingTorpedo(phase: string, data: any) {
		if (!data) {
			return;
		}
		for (const index of this.friend.indices()) {
			const damages = data.getMultiDamage_f(index);
			data.getMultiAttackTo_f(index).forEach((target: number, slot: number) => {
				if (target >= 0) {
					this.hit(phase, false, index, target, damages[slot] ?? 0);
				}
			});
		}
		for (const index of this.enemy.indices()) {
			const damages = data.getMultiDamage_e(index);
			data.getMultiAttackTo_e(index).forEach((target: number, slot: number) => {
				if (target >= 0) {
					this.hit(phase, true, index, target, damages[slot] ?? 0);
				}
			});
		}
	}

	/** `AirWarData`: stage 3 carries damage per ship, with no attacker. */
	airWar(phase: string, data: any) {
		if (!data?.hasStage3Data()) {
			return;
		}
		for (const index of this.friend.indices()) {
			const damage = data.stage3_f.getDamage(index);
			if (damage > 0) {
				this.hit(phase, true, null, index, damage);
			}
		}
		for (const index of this.enemy.indices()) {
			const damage = data.stage3_e.getDamage(index);
			if (damage > 0) {
				this.hit(phase, false, null, index, damage);
			}
		}
	}

	/** Compare what the replay arrived at with what the server reports next. */
	expect(label: string, side: Side, expected: (index: number) => number) {
		for (const [index, hp] of side.hp) {
			const shown = Math.max(0, hp);
			const want = expected(index);
			if (shown !== want) {
				this.problem(`${label}: ${side.name} ${index} ends at ${shown} in the client, the server has ${want}`);
			}
		}
	}
}

function readJson(path: string): Json {
	return JSON.parse(readFileSync(path, "utf8"));
}

function main() {
	const dumpDir = resolve(process.argv[2] ?? ".");
	const client = loadClient(resolve(import.meta.dir, "../out/main.decoded.js"));
	const { BattleRecordDay } = client(MODULES.recordDay);
	const { BattleRecordNight } = client(MODULES.recordNight);

	const day = new BattleRecordDay(readJson(join(dumpDir, "ec_battle.json")));
	const night = new BattleRecordNight(readJson(join(dumpDir, "ec_midnight_battle.json")));
	const settled = readJson(join(dumpDir, "final_hp.json")) as { friendly: number[]; enemy: number[] };

	const dayCommon = day._common;
	const replay = new Replay(
		new Side(
			"friend",
			(i) => dayCommon.getHPNowFriend(i),
			(i) => dayCommon.getHPMaxFriend(i),
		),
		new Side(
			"enemy",
			(i) => dayCommon.getHPNowEnemy(i),
			(i) => dayCommon.getHPMaxEnemy(i),
		),
	);

	if (!dayCommon.isCombinedEnemy()) {
		replay.problem("day: the client does not see an enemy combined fleet");
	}
	if (dayCommon.isCombinedFriend()) {
		replay.problem("day: the client sees a friendly combined fleet");
	}
	for (const index of replay.enemy.indices()) {
		if (!(dayCommon.getMstIDEnemy(index) > 0)) {
			replay.problem(`day: enemy ${index} has HP but no ship id`);
		}
	}

	for (const phase of DAY_ORDER) {
		const data = day.raw[phase];
		if (phase === "air_war") {
			replay.airWar(phase, data);
		} else if (phase === "raigeki_opening") {
			replay.openingTorpedo(phase, data);
		} else if (phase === "raigeki") {
			replay.torpedo(phase, data);
		} else {
			replay.shelling(phase, data);
		}
	}
	const dayAttacks = replay.attackCount;

	const nightCommon = night._common;
	replay.expect("night entry", replay.friend, (i) => nightCommon.getHPNowFriend(i));
	replay.expect("night entry", replay.enemy, (i) => nightCommon.getHPNowEnemy(i));

	// 1 is the main fleet, 2 the escort fleet.
	const activeEnemy = nightCommon.getActiveDeckEnemy();
	const onActiveDeck = (index: number) => (activeEnemy === 2 ? index >= 6 : index < 6);
	if (nightCommon.getActiveDeckFriend() !== 1) {
		replay.problem(`night: the client picks friendly deck ${nightCommon.getActiveDeckFriend()}`);
	}
	replay.shelling("night", night.raw.hougeki, onActiveDeck);

	replay.expect("settled", replay.friend, (i) => settled.friendly[i] ?? Number.NaN);
	replay.expect("settled", replay.enemy, (i) => settled.enemy[i] ?? Number.NaN);

	console.log(
		`client replay: ${replay.friend.hp.size} friendly, ${replay.enemy.hp.size} enemy ships; ` +
			`${dayAttacks} hits by day, ${replay.attackCount - dayAttacks} by night against enemy deck ${activeEnemy}`,
	);
	console.log(`enemy HP after the night: ${replay.enemy.indices().map((i) => Math.max(0, replay.enemy.hp.get(i)!)).join(" ")}`);
	if (replay.problems.length > 0) {
		console.log(`${replay.problems.length} problem(s):`);
		for (const problem of replay.problems) {
			console.log(`- ${problem}`);
		}
		process.exit(1);
	}
	console.log("the client reads the same battle the server settled");
}

main();
