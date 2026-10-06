import { describe, expect, test } from "bun:test";

import { guarded, mulberry32, sameDistribution } from "../src/route-oracle.ts";

describe("route oracle helpers", () => {
	test("reading a field the probe never set names the field", () => {
		const fleet = guarded({ ships_length: 6 }, "adopt_fleet") as { ships_length: number; speed?: number };
		expect(fleet.ships_length).toBe(6);
		expect(() => fleet.speed).toThrow("adopt_fleet.speed is not provided by the probe");
	});

	test("the generator repeats itself for the same seed", () => {
		const first = mulberry32(7);
		const second = mulberry32(7);
		expect([first(), first(), first()]).toEqual([second(), second(), second()]);
	});

	test("distributions match within the tolerance and differ on a missing target", () => {
		expect(sameDistribution({ A: 0.45, B: 0.55 }, { B: 0.551, A: 0.449 })).toBe(true);
		expect(sameDistribution({ A: 1 }, { A: 0.5, B: 0.5 })).toBe(false);
		expect(sameDistribution({ A: 1 }, { B: 1 })).toBe(false);
	});
});
