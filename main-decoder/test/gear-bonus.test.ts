import { describe, expect, test } from "bun:test";

import { convertGearBonus, GearBonusSyntaxError } from "../src/gear-bonus";
import { companionsOf, loadoutsFor } from "../src/gear-bonus-oracle";

const SOURCE = { repo: "KC3Kai/KC3Kai", commit: "abc", files: [] };
const META = `KC3Meta = { countryCtypeMap: { "Sweden": [89], "UnitedKingdom": [67, 88], "Japan": [1] } };`;

function table(entries: string, synergyGears = "surfaceRadar: 0, surfaceRadarIds: [28, 29], turbine: 0, turbineNonexist: 1, turbineIds: [33],"): string {
	return `(function(){ KC3GearBonus.explicitStatsBonusGears = function(){ return { "synergyGears": { ${synergyGears} }, ${entries} }; }; })();`;
}

describe("convertGearBonus", () => {
	test("flattens class, nation and ship rules in the order the source reads them", () => {
		const document = convertGearBonus(
			table(`"371": { count: 0, starsDist: [],
				byClass: {
					"89": [ { multiple: { "houg": 4, "saku": 6 } }, { remodel: 2, single: { "houg": 2 } } ],
					"70": { multiple: { "saku": 4 } },
					"79": "70",
				},
				byNation: { "UnitedKingdom": { minStars: 3, multiple: { "saku": 3 } }, "Sweden": "UnitedKingdom", "Japan": 70 },
				byShip: { ids: [1], single: { "houk": -1 } },
			}`),
			META,
			SOURCE,
		);

		expect(document.nations).toEqual({ Sweden: [89], UnitedKingdom: [67, 88], Japan: [1] });
		expect(document.synergyGears).toEqual({ surfaceRadar: [28, 29], turbine: [33] });
		expect(document.gears).toEqual([
			{
				key: "371",
				rules: [
					{ class: 70, multiple: { saku: 4 } },
					{ class: 79, multiple: { saku: 4 } },
					{ class: 89, multiple: { houg: 4, saku: 6 } },
					{ class: 89, remodel: 2, single: { houg: 2 } },
					{ nation: "UnitedKingdom", minStars: 3, multiple: { saku: 3 } },
					{ nation: "Sweden", minStars: 3, multiple: { saku: 3 } },
					{ nation: "Japan", multiple: { saku: 4 } },
					{ ids: [1], single: { houk: -1 } },
				],
			},
		]);
	});

	test("keeps entries in key order and splits the numeric rows of a synergy", () => {
		const document = convertGearBonus(
			table(`"t2_12": { count: 0, byShip: { single: { "tyku": 1 } } },
				"50": { count: 0, byShip: { synergy: {
					flags: ["surfaceRadar", "turbineNonexist"], countFlag: 0, multiple: { "houm": 1 },
					byCount: { gear: "this", "1": { "houg": 1 }, "2": { "houg": 3 } },
					byStars: { gearId: 28, isMultiple: true, "7": { "houk": 1 } },
				} } },
				"7": { count: 0 }`),
			META,
			SOURCE,
		);

		expect(document.gears.map((gear) => gear.key)).toEqual(["7", "50", "t2_12"]);
		expect(document.gears[1]?.rules[0]?.synergy).toEqual([
			{
				flags: ["surfaceRadar", "turbineNonexist"],
				countFlag: 0,
				multiple: { houm: 1 },
				byCount: { gear: "this", table: { "1": { houg: 1 }, "2": { houg: 3 } } },
				byStars: { gearId: "28", isMultiple: true, table: [{ minStars: 7, stats: { houk: 1 } }] },
			},
		]);
	});

	test("refuses what it does not know instead of dropping it", () => {
		const convert = (entries: string, synergyGears?: string) => () => convertGearBonus(table(entries, synergyGears), META, SOURCE);
		expect(convert(`"1": { count: 0, byShip: { onlyOnTuesdays: true } }`)).toThrow(GearBonusSyntaxError);
		expect(convert(`"1": { count: 0, byShip: { single: { "luck": 1 } } }`)).toThrow("unknown stat luck");
		expect(convert(`"1": { count: 0, byShip: { synergy: { flags: ["sonar"] } } }`)).toThrow("unknown flag sonar");
		expect(convert(`"1": { count: 0, byNation: { "Atlantis": {} } }`)).toThrow("unknown nation Atlantis");
		expect(convert(`"1": { count: 0, byShip: { single: { "houg": someVariable } } }`)).toThrow("not a plain literal");
		expect(convert(`"1": { count: 0 }`, "sonar: 0, sonarMinStars: 4, sonarIds: [1],")).toThrow("sonarMinStars");
	});
});

describe("corrections", () => {
	test("replace or follow an entry's rules, and add joined entries after the numbered ones", () => {
		const document = convertGearBonus(
			table(`"50": { count: 0, byShip: { single: { "houg": 1 } } }, "60": { count: 0, byShip: { single: { "houg": 2 } } }, "t2_12": { count: 0 }`),
			META,
			SOURCE,
			{
				"50": { why: "", append: [{ minStars: 9, multiple: { houk: 5 } }] },
				"60": { why: "", replace: [], append: [{ synergy: [{ requires: [{ gears: [50], minStars: 3 }], single: { tyku: 1 } } as never] }] },
				"50+60": { why: "", replace: [{ countCap: 2, multiple: { raig: 2 } }] },
				"55": { why: "", replace: [{ single: { saku: 1 } }] },
			},
		);

		expect(document.gears.map((gear) => gear.key)).toEqual(["50", "55", "60", "50+60", "t2_12"]);
		expect(document.gears[0]?.rules).toEqual([{ single: { houg: 1 } }, { minStars: 9, multiple: { houk: 5 } }]);
		expect(document.gears[2]?.rules).toEqual([{ synergy: [{ requires: [{ gears: [50], minStars: 3 }], single: { tyku: 1 }, flags: [] }] }]);
		expect(() => convertGearBonus(table(`"50": { count: 0 }`), META, SOURCE, { fifty: { why: "", replace: [] } })).toThrow("unknown entry fifty");
	});
});

describe("oracle loadouts", () => {
	test("pair an entry with the equipment its rules look at", () => {
		const document = convertGearBonus(
			table(`"50": { count: 0, byShip: { distinctGears: [50, 51], synergy: [ { flags: ["surfaceRadar", "turbineNonexist"] }, { flags: ["surfaceRadar"], byStars: { gearId: 60 } } ] } }`),
			META,
			SOURCE,
		);

		expect(companionsOf(document, "50")).toEqual([50, 51, 28, 33, 60]);
		const loadouts = loadoutsFor([50], [51, 28]);
		expect(loadouts).toContainEqual([[50, 10], [50, 10], [50, 10]]);
		expect(loadouts).toContainEqual([[50, 0], [28, 10]]);
		expect(loadouts).toContainEqual([[50, 10], [51, 10], [28, 10]]);
	});
});
