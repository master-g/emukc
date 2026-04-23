import { describe, expect, test } from "bun:test";

import { buildPathRules } from "../src/path-rules.ts";

describe("buildPathRules", () => {
	test("builds non-empty path rules from current Rust baselines", () => {
		const rules = buildPathRules();

		expect(rules.shipStandardCategories).toContain("banner");
		expect(rules.shipFullCategories).toEqual(["full", "full_dmg"]);
		expect(rules.slotStandardCategories).toContain("btxt_flat");
		expect(rules.shipDamageVariants.banner).toEqual(["banner_dmg", "banner_g_dmg", "banner_g"]);
		expect(rules.enemyPlaneIds).toEqual(Array.from({ length: 25 }, (_, index) => index + 1));
		expect(rules.btxtFlatSlotIds.length).toBeGreaterThan(300);
		expect(rules.characterHoleIds).toEqual([42]);
		expect(rules.specialShips).toContain(639);
		expect(rules.spRemodelShips).toContain(501);
		expect(rules.spRemodelMes).toContain(73);
		expect(rules.cardRounds).toEqual([524, 525]);
		expect(rules.rewardShips).toContain(900);
		expect(rules.eventShipHoles.full.length).toBeGreaterThan(0);
		expect(rules.enemyShipHoles.fullDmg.length).toBeGreaterThan(0);
	});
});
