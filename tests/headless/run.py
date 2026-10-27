"""Drive the real client, headless, against a throwaway server.

    python run.py <scenario preset> ["<steps>"]

Without steps the scenario's own are played and its check is run. Steps given on the
command line are for finding the way through a new screen. Steps are space separated: `w:<seconds>` lets the game live
that long (its own time: see `advance`), `c:<x>,<y>` clicks the game canvas
(1200x720), `s:<name>` saves a screenshot, `api:<path>` waits until the client has called
that KCSAPI path since the last call a step waited for, and `u:<x>,<y>:<path>` does the
same while clicking that spot every few seconds (several spots, separated by `;`, are
clicked in turn). Every KCSAPI response is saved under api/ with what the client sent (`.request`), and the report lists the
resources the client asked for that the cache list misses or the origin does not have. The run fails on a page error, a failed request or a step that
times out; the report and screenshots land in .data/temp/headless/<scenario>/. A run that
has no result after `LIMIT` seconds is stopped, naming the step it stood at.

Needs the bootstrapped .data/codex, the resource cache, main-decoder/out/main.decoded.js
and Playwright for Python with Chrome installed. Never touches .data/emukc.db.
"""

import datetime
import json
import os
import re
import shutil
import signal
import socket
import sqlite3
import subprocess
import sys
import time
import tomllib
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
# The cache and mods the server is configured with, relative to its config as the server reads it.
CONFIG = tomllib.loads((ROOT / "emukc.config.toml").read_text())
CACHE = ROOT / CONFIG["cache_root"]
MODS = ROOT / CONFIG.get("mods_root", "z/mods")
PORT = 27777
# A scenario takes under a minute; this is for the run that never comes back.
LIMIT = 300
# Where the game canvas sits inside the page.
CANVAS_X, CANVAS_Y = 40, 0
# The same patch as main-decoder/src/client-runtime.ts, keeping the game running.
ENTRY = re.compile(r"var (_0x[0-9a-f]+) = (_0x[0-9a-f]+)\(32875\);\s*return \1 = \1\.default;")


# Places that are clicked while waiting: a neutral one, 進撃, and 単縦陣.
TAPS = "600,400;430,365;670,278"
BATTLE = f"u:{TAPS}:api_req_sortie/battleresult"
# From the entry page to the sortie screen's list of maps.
TO_MAPS = (
    "api:api_world/get_worldinfo u:310,85:api_start2/get_option_setting u:910,605:api_port/port w:5 "
    "c:295,400 w:3 u:345,450:api_get_member/mapinfo w:4"
)
# 決定, then 出撃開始.
START = "c:1015,668 w:4 u:830,668:api_req_map/start"
# 煙幕 on a single fleet's formation screen (`SmokeButton._getPosition`, 51 by 50).
SMOKE_BUTTON = "648,522"
# 6-3's cell 3 (C), where `kcs2/resources/map/006/03_info.json` puts it.
SPOT_C = "436,444"
# 5-6's O and Q (cells 28 and 30), from `map/005/06_info20.json`.
SPOT_5_6_O, SPOT_5_6_Q = "672,259", "782,244"


def responses(work: Path, path: str) -> list[dict]:
    """What the server answered to every call of `path`, in order."""
    return [json.loads(dump.read_text())["api_data"] for dump in sorted((work / "api").glob(f"*_{path.replace('/', '.')}.json"))]


def check_smoke_screen(work: Path) -> list[str]:
    problems = []
    request = next((work / "api").glob("*_api_req_sortie.battle.request")).read_text()
    if "api_smoke_flag=1" not in request:
        problems.append(f"the client did not ask for smoke: {request}")
    smoke = responses(work, "api_req_sortie/battle")[0]["api_smoke_type"]
    if smoke not in (1, 2, 3):
        problems.append(f"the battle laid no screen: api_smoke_type {smoke}")
    return problems


def check_transport_5_6(work: Path) -> list[str]:
    problems = []
    visited = [cell["api_no"] for cell in responses(work, "api_req_map/next")]
    if visited != [6, 8, 9, 11]:
        problems.append(f"the fleet went {visited}, not C2, D, the landing point E and the boss G")
    landing = responses(work, "api_req_sortie/battleresult")[-1].get("api_landing_hp")
    if landing != {"api_max_hp": 280, "api_now_hp": 280, "api_sub_value": 40}:
        problems.append(f"the boss result landed {landing}, not 40 of 280")
    return problems


def check_air_corps_6_4(work: Path) -> list[str]:
    problems = []
    deployed = responses(work, "api_req_air_corps/set_plane")[-1]
    slot = deployed["api_plane_info"][0]
    if (slot["api_state"], slot["api_count"], deployed["api_distance"]["api_base"]) != (1, 18, 9):
        problems.append(f"the deployment answered {deployed}, not eighteen bombers reaching 9")
    if deployed.get("api_after_bauxite") != 11000 - 216:
        problems.append(f"the deployment left {deployed.get('api_after_bauxite')} bauxite, not 216 less")
    added = responses(work, "api_req_air_corps/expand_base")[-1][0]
    if added["api_rid"] != 2:
        problems.append(f"the expansion added air corps {added['api_rid']}, not the second")
    if not list((work / "api").glob("*_api_req_air_corps.set_action.json")):
        problems.append("closing the panel sent no orders")
    if not list((work / "api").glob("*_api_req_map.start_air_base.json")):
        problems.append("the air corps was given no cells")
    battles = [battle for path in ("api_req_sortie/battle", "api_req_sortie/ld_airbattle") for battle in responses(work, path)]
    attacks = [battle["api_air_base_attack"] for battle in battles if battle.get("api_air_base_attack")]
    if len(attacks) != 1 or len(attacks[0]) != 2 or attacks[0][0]["api_squadron_plane"][0] != {"api_mst_id": 169, "api_count": 18}:
        problems.append(f"the air corps should attack D twice, starting with eighteen bombers; the battles carried {attacks}")
    home = responses(work, "api_get_member/mapinfo")[-1]["api_air_base"][0]["api_plane_info"][0]
    left = home["api_count"]
    if home["api_cond"] != 2:
        problems.append(f"a squadron at 22 should come home orange, not api_cond {home['api_cond']}")
    timed = responses(work, "api_port/airCorpsCondRecoveryWithTimer")
    if not timed or timed[-1]["api_plane_info"][0]["api_cond"] != 2:
        problems.append(f"the timed recovery should answer the squadron as it is; it answered {timed}")
    supplied = responses(work, "api_req_air_corps/supply")[-1]
    stock = responses(work, "api_port/port")[-1]["api_material"]
    paid = (stock[0]["api_value"] - supplied["api_after_fuel"], stock[3]["api_value"] - supplied["api_after_bauxite"])
    if supplied["api_plane_info"][0]["api_count"] != 18 or paid != ((18 - left) * 3, (18 - left) * 5):
        problems.append(f"filling {18 - left} aircraft answered {supplied} for {paid} fuel and bauxite")
    return problems


def check_quest_equipment(work: Path) -> list[str]:
    problems = []
    calls = [dump.name.split("_", 1)[1].removesuffix(".json") for dump in sorted((work / "api").glob("*.json"))]
    claims = [claim["api_bounus"] for claim in responses(work, "api_req_quest/clearitemget")]
    if len(claims) != 3:
        return [f"three quests should be claimed, not {len(claims)}"]
    turned = claims[0][0]
    if (turned["api_type"], turned["api_item"].get("api_id_to"), turned["api_item"].get("api_slotitem_level")) != (15, 94, 10):
        problems.append(f"614 should turn the piece into 94 at its ten stars; it answered {turned}")
    taken = claims[1][-1]
    if (taken["api_type"], taken["api_item"].get("api_id_from")) != (16, 9999):
        problems.append(f"637 should answer a consumption entry; it answered {taken}")
    # The client reads its equipment again after every one of the three.
    after = [calls[at + 1 :][: calls[at + 1 :].index("api_get_member.questlist")] for at, call in enumerate(calls) if call == "api_req_quest.clearitemget"]
    if not all("api_get_member.slot_item" in reads and "api_get_member.unsetslot" in reads for reads in after):
        problems.append(f"the client should read its equipment again after each claim; it read {after}")
    held = sorted((item["api_slotitem_id"], item["api_level"]) for item in responses(work, "api_get_member/slot_item")[-1])
    kept = [item for item in held if item[0] in (93, 94, 19, 37, 168)]
    if kept != [(94, 10), (168, 0)]:
        problems.append(f"only the converted piece and the reward should be left of the quests' equipment, not {kept}")
    flagship = responses(work, "api_port/port")[-1]["api_ship"][0]
    carried = [slot for slot in flagship["api_slot"] if slot > 0]
    if len(carried) != 1:
        problems.append(f"the flagship should carry the converted piece alone, not {carried}")
    return problems


def check_air_raid_6_5(work: Path) -> list[str]:
    raids = [step["api_destruction_battle"] for step in responses(work, "api_req_map/next") if "api_destruction_battle" in step]
    if len(raids) != 1:
        return [f"one raid should come on the way to the boss, not {len(raids)}"]
    raid = raids[0]
    problems = []
    attack = raid["api_air_base_attack"]
    if attack["api_plane_from"][0] != [1] or len((attack["api_map_squadron_plane"] or {}).get("1", [])) != 1:
        problems.append(f"the first air corps should send its one squadron up; the raid carried {attack}")
    if len(raid["api_ship_ke"]) != 6 or raid["api_f_nowhps"] != [200] * len(raid["api_f_nowhps"]):
        problems.append(f"six raiders should meet bases at 200: {raid['api_ship_ke']}, {raid['api_f_nowhps']}")
    hit = sum(attack["api_stage3"]["api_fdam"])
    if (raid["api_lost_kind"] == 4) != (hit == 0):
        problems.append(f"a raid that did {hit} damage answered api_lost_kind {raid['api_lost_kind']}")
    return problems


def after_battle(battle: dict) -> list[int]:
    """The friendly fleet's hit points once everything in a battle packet has been played."""
    left = list(battle["api_f_nowhps"])
    for name in ("api_opening_taisen", "api_hougeki1", "api_hougeki2", "api_hougeki3", "api_hougeki"):
        shelling = battle.get(name) or {}
        for by_enemy, targets, damages in zip(shelling.get("api_at_eflag") or [], shelling.get("api_df_list") or [], shelling.get("api_damage") or []):
            for target, damage in zip(targets, damages):
                if by_enemy and target >= 0:
                    left[target] -= int(damage)
    taken = [(battle.get(name) or {}).get("api_fdam") for name in ("api_opening_atack", "api_raigeki")]
    taken.append(((battle.get("api_kouku") or {}).get("api_stage3") or {}).get("api_fdam"))
    for damages in filter(None, taken):
        left = [hp - int(damage) for hp, damage in zip(left, damages)]
    return [max(hp, 0) for hp in left]


def check_fair_battle(work: Path) -> list[str]:
    problems = []
    day = responses(work, "api_req_sortie/battle")[0]
    left = after_battle(day)
    nights = responses(work, "api_req_battle_midnight/battle")
    if bool(day["api_midnight_flag"]) != bool(nights):
        problems.append(f"the day battle said api_midnight_flag {day['api_midnight_flag']} and {len(nights)} night battles were fought")
    if nights:
        if nights[0]["api_f_nowhps"] != left:
            problems.append(f"the day battle left the fleet at {left} and the night began at {nights[0]['api_f_nowhps']}")
        left = after_battle(nights[0])
    home = {ship["api_id"]: ship["api_nowhp"] for ship in responses(work, "api_port/port")[-1]["api_ship"]}
    if [home[ship] for ship in range(1, len(left) + 1)] != left:
        problems.append(f"the battles left the fleet at {left} and it came home at {home}")
    return problems


FAIR_2_1 = (
    f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
    "u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home"
)

def check_anti_air_cut_in(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    fire = responses(work, "api_req_sortie/battle")[0]["api_kouku"]["api_stage2"].get("api_air_fire")
    # 秋月 is the sixth ship; her three kinds show two or three of 122, 122 and 106.
    if not fire:
        problems.append("no 対空カットイン this time: all three kinds fail about 7 times in a hundred, so run it again")
    elif fire["api_idx"] != 5 or fire["api_kind"] not in (1, 2, 3) or not set(fire["api_use_items"]) <= {122, 106}:
        problems.append(f"秋月 should fire kind 1, 2 or 3 from the sixth place; the battle carried {fire}")
    return problems


def check_night_air_attack(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the fleet strikes with one carrier by day, so the battle should have gone into the night"]
    shelling = nights[0]["api_hougeki"]
    flown = [flag for by_enemy, flag in zip(shelling["api_at_eflag"], shelling["api_n_mother_list"]) if not by_enemy]
    if not flown or set(flown) != {1}:
        problems.append(f"every carrier should fly her night attack; api_n_mother_list said {flown}")
    # A 空母カットイン is sp 6 whichever it is; what tells them apart is the aircraft shown,
    # one night fighter, then dive bombers, then torpedo bombers, by her place.
    shows = [[254, 320], [254, 257], [254, 242, 242], [254], [254], [254]]
    ours = [(at, kind, [int(item) for item in shown]) for by_enemy, at, kind, shown in zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_si_list"]) if not by_enemy]
    for at, kind, shown in ours:
        if kind not in (0, 6):
            problems.append(f"carrier {at} attacked with kind {kind}")
        if kind == 6 and shown != shows[at]:
            problems.append(f"carrier {at}'s cut-in shows {shows[at]}; it carried {shown}")
    if not any(kind == 6 and at < 3 for at, kind, _ in ours):
        problems.append("none of the 62, 63 and 64 carriers cut in this time: run it again")
    print("night attacks (carrier, kind, shown):", ours)
    return problems


def check_night_asw(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so no submarine was attacked at night: run it again"]
    night = nights[0]
    shelling = night["api_hougeki"]
    # The whole fleet is submarines. A night attack on one is one plain hit, not flown; a miss
    # does nothing, and after a day battle a hit sinks nobody (at 1 HP it does nothing either).
    rows = list(zip(shelling["api_at_eflag"], shelling["api_df_list"], shelling["api_sp_list"], shelling["api_n_mother_list"], shelling["api_cl_list"], shelling["api_damage"], shelling["api_at_list"], shelling["api_si_list"]))
    attacks = [row[1:6] for row in rows if row[0]]
    # They attack too, with more than a quarter of the hull: plainly, or the flagship with the
    # torpedo cut-in her late-model torpedo and radar make, which shows both.
    ours = [row for row in rows if not row[0]]
    if not ours:
        problems.append("no submarine attacked at night: run it again")
    for _, targets, kind, flown, _, _, at, shown in ours:
        before = night["api_f_nowhps"][at]
        if before * 4 <= night["api_f_maxhps"][at]:
            problems.append(f"submarine {at} attacked at night with {before} of {night['api_f_maxhps'][at]}")
        if flown or (kind, len(targets)) not in ({(0, 1), (3, 2)} if at == 0 else {(0, 1)}):
            problems.append(f"submarine {at} attacked with kind {kind}, {len(targets)} hits, flown {flown}")
        if kind == 3 and [int(item) for item in shown] != [213, 210]:
            problems.append(f"her torpedo cut-in shows the torpedo, then the radar; it carried {shown}")
    print("submarines' attacks (ship, kind, shown):", [(row[6], row[2], row[7]) for row in ours])
    if not attacks:
        problems.append("no enemy attacked at night: run it again")
    for targets, kind, flown, hits, damages in attacks:
        if (len(targets), kind, flown) != (1, 0, 0) or (hits[0] == 0 and int(damages[0]) != 0):
            problems.append(f"a night attack on a submarine carried {targets}, kind {kind}, flown {flown}, {hits}, {damages}")
    sunk = [at for at, (before, after) in enumerate(zip(night["api_f_nowhps"], after_battle(night))) if before > 0 >= after]
    if sunk:
        problems.append(f"submarines {sunk} were sunk in the night after a day battle")
    print("night began at", night["api_f_nowhps"], "attacks on submarines (target, cl, damage):", [(row[0][0], row[3][0], int(row[4][0])) for row in attacks])
    return problems


def check_enemy_night_carrier(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the battle ended by day this time, so no enemy carrier attacked at night: run it again"]
    night = nights[0]
    shelling = night["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_n_mother_list"])
    # 軽母ヌ級flagship carries no night aircraft: she attacks, and the packet does not say flown.
    flags = [flown for by_enemy, at, flown in rows if by_enemy and night["api_ship_ke"][at] == 1560]
    if not flags:
        problems.append("軽母ヌ級flagship did not attack at night (sunk or heavily damaged by day?): run it again")
    elif set(flags) != {0}:
        problems.append(f"軽母ヌ級flagship has no night air crew; api_n_mother_list said {flags}")
    return problems


def check_enemy_night_crew(work: Path) -> list[str]:
    cells = [step["api_no"] for step in responses(work, "api_req_map/start") + responses(work, "api_req_map/next")]
    nights = responses(work, "api_req_battle_midnight/battle")
    print("cells", cells, "night battles", len(nights))
    if not nights or cells[-1] != 13:
        return [f"the fleet was to reach M (cell 13) and fight there at night; it went by {cells}"]
    if nights[-1]["api_ship_ke"][0] != 2060:
        return [f"M was led by {nights[-1]['api_ship_ke'][0]} this time (4 in 100), not ヒ船団棲姫: run it again"]
    shelling = nights[-1]["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_n_mother_list"], shelling["api_si_list"])
    # She has a night crew of her own and night aircraft: her attack is flown.
    hers = [(flown, shown) for by_enemy, at, flown, shown in rows if by_enemy and at == 0]
    kinds = [kind for by_enemy, at, kind in zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"]) if by_enemy and at == 0]
    # Her night fighter and night attacker make the 空母カットイン 62, which shows both.
    print("her attacks (flown, shown):", hers, "kinds", kinds)
    if not hers:
        return ["ヒ船団棲姫 did not attack at night"]
    return [f"ヒ船団棲姫 flies her night attacks; api_n_mother_list said {flown}" for flown, _ in hers if flown != 1]


def check_enemy_night_crew_5_6(work: Path) -> list[str]:
    cells = [step["api_no"] for step in responses(work, "api_req_map/start") + responses(work, "api_req_map/next")]
    nights = responses(work, "api_req_battle_midnight/battle")
    print("cells", cells, "night battles", len(nights))
    if not nights or cells != [20, 28, 30, 38, 41]:
        return [f"the fleet was to go by I, O, Q and U to X (cell 41) and fight there at night; it went by {cells}"]
    if nights[-1]["api_ship_ke"][0] not in (1971, 1974):
        return [f"X was led by {nights[-1]['api_ship_ke'][0]}, not 南太平洋空母棲姫"]
    shelling = nights[-1]["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_n_mother_list"], shelling["api_sp_list"], shelling["api_si_list"])
    # She has a night crew of her own and night aircraft: her attack is flown.
    hers = [(flown, kind, shown) for by_enemy, at, flown, kind, shown in rows if by_enemy and at == 0]
    print("her attacks (flown, kind, shown):", hers)
    if not hers:
        return ["南太平洋空母棲姫 did not attack at night"]
    return [f"南太平洋空母棲姫 flies her night attacks; api_n_mother_list said {flown}" for flown, _, _ in hers if flown != 1]


def check_night_cut_in(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so nothing was rolled for at night: run it again"]
    night = nights[0]
    shelling = night["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_df_list"], shelling["api_si_list"])
    attacks = [row[1:] for row in rows if not row[0]]
    # What each ship may come up with, by her place: her cut-ins, a double attack, or a plain attack.
    may = [{0, 2}, {0, 3}, {0, 5}, {0, 2, 7}, {0, 3, 9}, {0, 1}]
    for at, kind, targets, shown in attacks:
        if kind not in may[at]:
            problems.append(f"ship {at} attacked with kind {kind}; her equipment makes {sorted(may[at])}")
        if len(targets) != (2 if kind in (1, 2, 3) else 1):
            problems.append(f"an attack of kind {kind} carried {len(targets)} hits")
        if kind == 2 and [int(item) for item in shown] != [2, 15]:
            problems.append(f"a main gun and torpedo cut-in shows the gun, then the torpedo; it carried {shown}")
    if not any(kind >= 2 for _, kind, _, _ in attacks):
        problems.append("no cut-in came up this time: run it again")
    # The star shell is the flagship's, fired 7 times in 10.
    if night["api_flare_pos"][0] not in (-1, 0) or night["api_flare_pos"][1] != -1:
        problems.append(f"only the flagship carries a star shell; api_flare_pos said {night['api_flare_pos']}")
    print("night attacks (ship, kind, shown):", [(at, kind, shown) for at, kind, _, shown in attacks], "api_flare_pos", night["api_flare_pos"])
    return problems


def check_night_destroyer_cut_in(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so nothing was rolled for at night: run it again"]
    shelling = nights[0]["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_df_list"], shelling["api_si_list"])
    attacks = [row[1:] for row in rows if not row[0]]
    # By her place, what she may come up with and what each cut-in shows: 古鷹's guns and
    # secondary (4), the radar lookout cut-in (8, twice 12), the drum one (10, twice 14), the gun,
    # torpedo and radar one (7, twice 11) or her gun and torpedo (2), and on the last two torpedoes
    # and the torpedo squadron's lookout (9, twice 13) or her two torpedoes (3).
    may = [{0, 4}, {0, 8, 12}, {0, 10, 14}, {0, 2, 7, 11}, {0, 10, 14}, {0, 3, 9, 13}]
    shows = {4: [50, 50, 12], 8: [15, 129, 28], 12: [15, 129, 28], 10: [15, 412, 75], 14: [15, 412, 75], 7: [2, 15, 28], 11: [2, 15, 28], 2: [2, 15], 9: [15, 15, 412], 13: [15, 15, 412], 3: [15, 15]}
    for at, kind, targets, shown in attacks:
        if kind not in may[at]:
            problems.append(f"ship {at} attacked with kind {kind}; her equipment makes {sorted(may[at])}")
        if len(targets) != (2 if kind in (2, 3, 11, 12, 13, 14) else 1):
            problems.append(f"an attack of kind {kind} carried {len(targets)} hits")
        if kind in shows and [int(item) for item in shown] != shows[kind]:
            problems.append(f"a cut-in of kind {kind} shows {shows[kind]}; it carried {shown}")
    if not any(kind >= 4 for _, kind, _, _ in attacks):
        problems.append("no cut-in came up this time: run it again")
    print("night attacks (ship, kind, shown):", [(at, kind, shown) for at, kind, _, shown in attacks])
    return problems


def check_night_zuiun(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so nothing was rolled for at night: run it again"]
    shelling = nights[0]["api_hougeki"]
    rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_df_list"], shelling["api_si_list"], shelling["api_n_mother_list"])
    attacks = [row[1:] for row in rows if not row[0]]
    # The two cruisers may cut in with their 夜間瑞雲 (200) or attack twice with their guns; the
    # destroyers carry nothing. The cut-in hits twice, not flown, and shows three pieces (the
    # client reads three unchecked): the 瑞雲 first, the radar of the cut-in that has one, guns.
    shows = [([490, 490, 50], [490, 50, 50]), ([490, 28, 50], [490, 50, 50])]
    for at, kind, targets, shown, flown in attacks:
        if kind not in ({0, 1, 200} if at < 2 else {0}):
            problems.append(f"ship {at} attacked with kind {kind}")
        # One hit when the first sinks the last enemy.
        if kind == 200 and (len(targets) not in (1, 2) or flown or [int(item) for item in shown] not in shows[at]):
            problems.append(f"ship {at}'s 瑞雲 cut-in carried {len(targets)} hits, flown {flown}, shown {shown}")
    if 200 not in [kind for _, kind, _, _, _ in attacks]:
        problems.append("no 瑞雲 cut-in came up this time: run it again")
    print("night attacks (ship, kind, shown):", [(at, kind, shown) for at, kind, _, shown, _ in attacks])
    return problems


def check_night_contact(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    seiku = responses(work, "api_req_sortie/battle")[0]["api_kouku"]["api_stage1"]["api_disp_seiku"]
    if seiku not in (1, 2):
        problems.append(f"the fighter takes the air from an enemy without one; api_disp_seiku said {seiku}")
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so no scout was flown: run it again"]
    # The flagship's scout comes into contact every time at her level; the enemy carries none.
    if nights[0]["api_touch_plane"] != [540, -1]:
        problems.append(f"the flagship's scout is the fleet's and the enemy has none; api_touch_plane said {nights[0]['api_touch_plane']}")
    print("api_disp_seiku", seiku, "api_touch_plane", nights[0]["api_touch_plane"])
    return problems


def check_season(version: str):
    """吹雪's graph in getData is the season's, and her art was not fetched from the origin, which
    holds only today's: an older version can only have come from the cache history."""

    def check(work: Path) -> list[str]:
        graph = next(g for g in responses(work, "api_start2/getData")[0]["api_mst_shipgraph"] if g["api_id"] == 9)
        problems = []
        if graph["api_version"][0] != version:
            problems.append(f"吹雪's art should be version {version}; getData said {graph['api_version']}")
        log = re.sub(r"\x1b\[[0-9;]*m", "", (work / "server.log").read_text())
        if re.search(r"🛬 \S*/kcs2/resources/ship/full/0009_", log):
            problems.append("吹雪's art was fetched from the origin, not the cache history")
        return problems

    return check


def check_practice_night_contact(work: Path) -> list[str]:
    seiku = responses(work, "api_req_practice/battle")[0]["api_kouku"]["api_stage1"]["api_disp_seiku"]
    nights = responses(work, "api_req_practice/midnight_battle")
    print("api_disp_seiku", seiku, "api_touch_plane", nights[0]["api_touch_plane"] if nights else None)
    # The fleet has no aircraft, so the rival's carriers take the air (ours lost: 3 or 4).
    if seiku not in (3, 4):
        return [f"the rival's carriers take the air from a fleet without aircraft; api_disp_seiku said {seiku}"]
    if not nights:
        return ["one side was beaten by day this time, so there was no night: run it again"]
    # 川内改二's 九八式水上偵察機(夜偵) comes into contact 17 times in 25 at level 99.
    if nights[0]["api_touch_plane"] != [-1, 102]:
        return [f"the rival's night scout (102) should be in contact; api_touch_plane said {nights[0]['api_touch_plane']} (it misses 8 in 25): run it again"]
    return []


def check_kongou_night_assault(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    nights = responses(work, "api_req_battle_midnight/battle")
    if not nights:
        return problems + ["the enemy was sunk by day this time, so the flagship had no turn at night: run it again"]
    shelling = nights[0]["api_hougeki"]
    rows = list(zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_df_list"], shelling["api_damage"]))
    ours = [row[1:] for row in rows if not row[0]]
    assaults = [row for row in ours if row[1] == 104]
    if not assaults:
        problems.append("the night assault did not come up this time (78 in 100): run it again")
    for at, _, targets, damages in assaults:
        # One row in the flagship's turn: her hit and her partner's, or hers alone if it sank
        # the last of the enemy.
        if at != 0 or len(targets) not in (1, 2) or len(damages) != len(targets):
            problems.append(f"the night assault is the flagship's row with her hit and her partner's; it carried ship {at}, {targets}, {damages}")
    if len(assaults) > 1 or sum(1 for row in ours if row[0] == 0) > 1:
        problems.append(f"the flagship has one turn at night; the rows were {[(row[0], row[1]) for row in ours]}")
    print("our night rows (ship, kind, targets, damage):", ours)
    return problems


def check_nagato_broadside(work: Path) -> list[str]:
    problems = check_fair_battle(work)
    day = responses(work, "api_req_sortie/battle")[0]
    if day["api_formation"][0] != 4:
        problems.append(f"the fleet was to fight in 梯形陣; api_formation said {day['api_formation']}")
    ours = []
    for name in ("api_hougeki1", "api_hougeki2", "api_hougeki3"):
        shelling = day.get(name) or {}
        rows = zip(shelling.get("api_at_eflag", []), shelling.get("api_at_list", []), shelling.get("api_at_type", []), shelling.get("api_df_list", []), shelling.get("api_damage", []))
        ours += [row[1:] for row in rows if not row[0]]
    # It is rolled for again in the flagship's turn at night if the day did not make it.
    nights = responses(work, "api_req_battle_midnight/battle")
    if nights:
        shelling = nights[0]["api_hougeki"]
        rows = zip(shelling["api_at_eflag"], shelling["api_at_list"], shelling["api_sp_list"], shelling["api_df_list"], shelling["api_damage"])
        night = [row[1:] for row in rows if not row[0]]
        print("our night rows (ship, kind, targets, damage):", night)
        ours += [row for row in night if row[1] >= 100]
    specials = [row for row in ours if row[1] >= 100]
    if not specials:
        problems.append("the special attack did not come up this time (62 in 100 a turn of the flagship's): run it again")
    for at, kind, targets, damages in specials:
        # One row in the flagship's turn: her two hits and her partner's, fewer if the enemy ran out.
        if at != 0 or kind != 101 or not 1 <= len(targets) <= 3 or len(damages) != len(targets):
            problems.append(f"the special attack is one row of 長門改二's, kind 101, of three hits; it carried ship {at}, kind {kind}, {targets}, {damages}")
    if len(specials) > 1:
        problems.append(f"a sortie makes one special attack; the rows were {[(row[0], row[1]) for row in specials]}")
    print("our day rows (ship, kind, targets, damage):", ours)
    return problems


def damaged_partner_check(partner: int, wedc: list[int], wedd: list[int], kind: int, formation: int):
    """The check of a two-ship special attack whose second ship starts moderately damaged: the
    client centres her damaged art on `api_wedc`/`api_wedd`, which getData has to carry."""
    def check(work: Path) -> list[str]:
        problems = []
        graph = next(g for g in responses(work, "api_start2/getData")[0]["api_mst_shipgraph"] if g["api_id"] == partner)
        if graph.get("api_wedc") != wedc or graph.get("api_wedd") != wedd:
            problems.append(f"getData should carry {partner}'s damaged face (api_wedc {wedc}, api_wedd {wedd}); it said {graph.get('api_wedc')}, {graph.get('api_wedd')}")
        day = responses(work, "api_req_sortie/battle")[0]
        if day["api_formation"][0] != formation:
            problems.append(f"the fleet was to fight in formation {formation}; api_formation said {day['api_formation']}")
        hp, top = day["api_f_nowhps"][1], day["api_f_maxhps"][1]
        if not top / 4 < hp <= top / 2:
            problems.append(f"the second ship was to start moderately damaged; she had {hp} of {top}")
        ours = []
        for name in ("api_hougeki1", "api_hougeki2", "api_hougeki3"):
            shelling = day.get(name) or {}
            ours += [(at, made) for flag, at, made in zip(shelling.get("api_at_eflag", []), shelling.get("api_at_list", []), shelling.get("api_at_type", [])) if not flag]
        print("our day rows (ship, kind):", ours)
        if (0, kind) not in ours:
            problems.append(f"the special attack ({kind}) did not come up by day this time: run it again")
        return problems
    return check


# 中部海域, 6-3: the choice of route at A, where C is tapped, a battle at C and at E, the
# reconnaissance cells G and H, the boss J, home. Photographed through G and H.
RECON_6_3 = (
    f"{TO_MAPS} c:800,680 w:4 c:280,525 w:3 {START} w:8 s:choice u:{TAPS};{SPOT_C}:api_req_map/next w:1 s:leaving {BATTLE} "
    # The call for G is made by 進撃; the compass then waits for a tap of its own.
    f"u:{TAPS}:api_req_map/next {BATTLE} u:{TAPS}:api_req_map/next c:600,400 "
    + " ".join(f"w:1.5 s:g{n}" for n in range(1, 9))
    + f" u:{TAPS}:api_req_map/next "
    + " ".join(f"w:1.5 s:h{n}" for n in range(1, 9))
    + f" u:{TAPS}:api_req_map/next {BATTLE} w:14 s:result u:{TAPS}:api_port/port w:5 s:home"
)


def check_recon(searched: list[dict]):
    """6-3's route through G and H, with what each reconnaissance should answer."""

    def check(work: Path) -> list[str]:
        problems = []
        steps = responses(work, "api_req_map/next")
        visited = [cell["api_no"] for cell in steps]
        if visited != [3, 5, 7, 12, 10]:
            return [f"the fleet went {visited}, not C, E, the reconnaissance cells G and H and the boss J"]
        # A is a choice of route: the start offers B and C, and the client sends the one tapped.
        offered = responses(work, "api_req_map/start")[0].get("api_select_route")
        asked = sorted((work / "api").glob("*_api_req_map.next.request"))[0].read_text()
        print("A offered:", offered, "and the client asked:", asked)
        if offered != {"api_select_cells": [2, 3]} or "api_cell_id=3" not in asked.split("&"):
            problems.append(f"A offered {offered} and the client asked {asked!r}, not for C of B and C")
        found = {}
        for label, cell, wanted in zip("GH", steps[2:4], searched):
            items = cell.get("api_itemget") or []
            print(f"{label} answered:", cell.get("api_airsearch"), items)
            # Something is found by a success (1) or a great one (2), nothing otherwise.
            if cell.get("api_airsearch") != wanted or len(items) != (1 if wanted["api_result"] else 0):
                problems.append(f"{label} should answer {wanted} with {'one thing' if wanted['api_result'] else 'nothing'} found")
            for item in items:
                found[item["api_id"]] = found.get(item["api_id"], 0) + item["api_getcount"]
        before, after = (
            {m["api_id"]: m["api_value"] for m in port["api_material"]} for port in responses(work, "api_port/port")[:2]
        )
        gained = {kind: after[kind] - before[kind] for kind in after if after[kind] != before[kind]}
        print("found:", found, "gained in port:", gained)
        if gained != found:
            problems.append(f"the fleet found {found} and headquarters gained {gained}")
        return problems

    return check


# 鎮守府海域's extra operations, 1-6: A, a battle at E, the resource cell G, the air battle F, a
# battle at B, and the step onto the goal N.
ESCORT_1_6 = (
    f"{TO_MAPS} c:1105,415 w:4 c:660,420 w:3 {START} u:{TAPS}:api_req_map/next {BATTLE} "
    f"u:{TAPS}:api_req_map/next w:3 s:resource u:{TAPS}:api_req_map/next {BATTLE} "
    f"u:{TAPS}:api_req_map/next {BATTLE} u:{TAPS}:api_req_map/next "
)
# The goal photographed, and home: for an arrival that brings no bonus.
ESCORT_GOAL = " ".join(f"w:2 s:goal{n}" for n in range(1, 11)) + f" u:{TAPS}:api_port/port w:5 s:home"


def check_escort_arrival(unit: int):
    """An arrival at N that does not empty the gauge: one resource, `unit` to ten times it."""

    def check(work: Path) -> list[str]:
        steps = responses(work, "api_req_map/next")
        visited = [cell["api_no"] for cell in steps]
        if visited != [3, 4, 12, 13, 17]:
            return [f"the fleet went {visited}, not E, G, F, B and the goal N"]
        goal = steps[-1]
        print("the goal answered:", {key: goal.get(key) for key in ("api_event_id", "api_itemget_eo_comment", "api_itemget_eo_result", "api_get_eo_rate")})
        gain = goal.get("api_itemget_eo_comment") or {}
        problems = []
        if gain.get("api_usemst") != 4 or gain.get("api_id") not in (1, 2, 3, 4) or gain.get("api_getcount") not in range(unit, unit * 10 + 1, unit):
            problems.append(f"the goal gave {gain}, not one resource in {unit} to {unit * 10}")
        if "api_itemget_eo_result" in goal or "api_get_eo_rate" in goal:
            problems.append("only the arrival that empties the gauge brings the bonus")
        return problems

    return check


def check_escort_1_6(work: Path) -> list[str]:
    problems = []
    steps = responses(work, "api_req_map/next")
    visited = [cell["api_no"] for cell in steps]
    if visited != [3, 4, 12, 13, 17]:
        problems.append(f"the fleet went {visited}, not E, G, F, B and the goal N")
        return problems
    goal = steps[-1]
    print("the goal answered:", {key: goal.get(key) for key in ("api_event_id", "api_itemget_eo_comment", "api_itemget_eo_result", "api_get_eo_rate")})
    # The seventh arrival: 燃料 1000, a プレゼント箱 and 戦果 75.
    if goal.get("api_itemget_eo_comment") != {"api_usemst": 4, "api_id": 1, "api_getcount": 1000}:
        problems.append(f"the goal gave {goal.get('api_itemget_eo_comment')}, not 1000 fuel")
    if goal.get("api_itemget_eo_result") != {"api_usemst": 5, "api_id": 60, "api_getcount": 1} or goal.get("api_get_eo_rate") != 75:
        problems.append(f"the goal's bonus was {goal.get('api_itemget_eo_result')} and {goal.get('api_get_eo_rate')}")
    ports = responses(work, "api_port/port")
    fuel = [next(m["api_value"] for m in port["api_material"] if m["api_id"] == 1) for port in ports]
    print("fuel in port, before and after:", fuel)
    # The 75 is kept: the ranking shows it, decoded as the client does (`__drawRanking`),
    # with the experience of five battles too little to add one.
    ranking = responses(work, "api_req_ranking/mxltvkpyuklh")
    member = ports[-1]["api_basic"]["api_member_id"]
    keys = [8931, 1201, 1156, 5061, 4569, 4732, 3779, 4568, 5695, 4619, 4912, 5669, 6586]
    seeds = [3187, 3596, 6413, 9628, 7279, 7678, 6023, 2564, 9558, 9272]
    seed = (seeds[member % 10] - seeds[member % 10] % 100) / 100
    rows = [(row["api_mxltvkpyuklh"], row["api_mtjmdcwtvhdr"], row["api_wuhnhojjxmke"] / keys[row["api_mxltvkpyuklh"] % 13] / seed - 91) for row in (ranking[-1]["api_list"] if ranking else [])]
    print("the ranking (place, name, 戦果):", rows)
    # From 22:00 JST on the month's last day the 75 is not kept and the ranking is the next
    # month's (`war-result-ranking.md`): midnight at UTC+11 is in another month than at UTC+9.
    now = datetime.datetime.now(datetime.timezone.utc)
    kept = 75 if (now + datetime.timedelta(hours=11)).month == (now + datetime.timedelta(hours=9)).month else 0
    if rows != [(1, ports[-1]["api_basic"]["api_nickname"], kept)]:
        problems.append(f"the ranking should list the admiral alone, first, with {kept}; it listed {rows}")
    return problems


# The port with 吹雪 as secretary, dressed as on a date: the cache history serves her art of then.
SEASON_PORT = "secretary:9 api:api_world/get_worldinfo u:310,85:api_start2/get_option_setting u:910,605:api_port/port w:15 s:port"

SCENARIOS = {
    # 6-3 with two 二式大艇 and two 零観, who make a great success of both reconnaissances.
    "recon_6_3": (RECON_6_3, check_recon([{"api_plane_type": 1, "api_result": 2}] * 2)),
    # The same with two 零観 alone, a score of 12: a success at G and a failure at H, shown with
    # the seaplane's picture.
    "recon_6_3_seaplane": (
        RECON_6_3,
        check_recon([{"api_plane_type": 2, "api_result": 1}, {"api_plane_type": 2, "api_result": 0}]),
    ),
    # The same with nothing that flies: both reconnaissances are called off.
    "recon_6_3_called_off": (RECON_6_3, check_recon([{"api_plane_type": 0, "api_result": 0}] * 2)),
    # 鎮守府海域's extra operations, 1-6, with its gauge one arrival short of empty: A, a battle at
    # E, the resource cell G, the air battle F, a battle at B and the goal N, which empties the
    # gauge and hands over the bonus; then home, and the ranking that keeps its 戦果.
    # Photographed through the goal and the ranking.
    "escort_1_6": (
        f"sunk:16:6 {ESCORT_1_6}"
        + " ".join(f"w:2 s:goal{n}" for n in range(1, 11))
        # 戦果, then the プレゼント箱, each closed by a tap anywhere.
        + " c:600,400 " + " ".join(f"w:2 s:bonus{n}" for n in range(1, 7))
        + f" c:600,400 w:4 s:box u:{TAPS}:api_port/port w:5 s:home"
        # 戦績表示, then its ランキング tab.
        + " u:240,75:api_get_member/record w:4 u:80,282:api_req_ranking/mxltvkpyuklh w:4 s:ranking",
        check_escort_1_6,
    ),
    # 1-6 again with its gauge untouched: the goal gives one resource, and the gauge loses a notch.
    "escort_1_6_standing": (
        ESCORT_1_6 + ESCORT_GOAL,
        check_escort_arrival(100),
    ),
    # 1-6 again after its gauge was emptied: the goal gives a tenth as much.
    "escort_1_6_cleared": (
        f"sunk:16:7 cleared:16 {ESCORT_1_6}{ESCORT_GOAL}",
        check_escort_arrival(10),
    ),
    # 演習 against a rival whose ships are written over with 川内改二 and three 赤城 at level 99:
    # their fighters take the air from six destroyers, and her 九八式夜偵 comes into contact in the
    # night battle. Photographed through the night.
    "practice_night_contact": (
        "api:api_world/get_worldinfo u:310,85:api_start2/get_option_setting u:910,605:api_port/port w:5 "
        # 出撃, then 演習; the rivals are drawn by the first list, and written over after it.
        "u:295,400:api_get_member/chart_additional_info w:3 u:670,450:api_get_member/practice rival:158,83,83,83 w:4 "
        # The first rival, 演習を挑む, 演習開始!!
        "u:340,300:api_req_member/get_practice_enemyinfo w:4 c:357,610 w:4 c:683,643 w:4 "
        "u:600,400;670,278:api_req_practice/battle u:600,400;770,365:api_req_practice/midnight_battle w:4 "
        + " ".join(f"w:2 s:night{n}" for n in range(1, 7))
        + " u:600,400;770,365:api_req_practice/battle_result w:14 s:result",
        check_practice_night_contact,
    ),
    # Four destroyers into 1-1 with three smoke generators: 煙幕 is pressed on the formation
    # screen, and the battle opens in a screen of smoke. Photographed through its opening.
    "smoke_screen": (
        f"{TO_MAPS} c:280,280 w:3 {START} "
        + " ".join(f"c:600,400 w:2" for _ in range(6))
        + f" s:formation c:{SMOKE_BUTTON} w:1 s:smoke_on u:670,278:api_req_sortie/battle "
        + " ".join(f"w:2 s:smoke{n}" for n in range(1, 11))
        + f" {BATTLE}",
        check_smoke_screen,
    ),
    # One battle of 1-1, up to the choice between going on and going home.
    "fresh_1_1": (f"{TO_MAPS} c:280,280 w:3 {START} {BATTLE}", lambda work: []),
    # 南西諸島海域, 2-1, without the cheats: one battle, into the night if it is offered (夜戦突入
    # sits where 撤退 does on the next choice), and home. What the packets say the fleet was
    # left with has to be what it comes home with.
    "gunnery_cutin": (FAIR_2_1, check_fair_battle),
    # Six night carriers, three of them short of ammunition and three striking not at all: the night
    # battle is theirs, flown as aircraft, as a 空母カットイン when it forms. Photographed through the night.
    "night_air_attack": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_night_air_attack,
    ),
    # The same with six destroyers, who seldom finish by day: this is the one that reaches the night.
    "leveled_for_mid_boss": (FAIR_2_1, check_fair_battle),
    # The same with six submarines, for the enemy to attack at night and to attack it.
    "night_asw": (FAIR_2_1, check_night_asw),
    # 南西諸島海域's extra operation, 2-5, whose first battle meets carriers: 秋月 in the sixth
    # place fires her 対空カットイン at what they send. Photographed through the air phase.
    "anti_air_cut_in": (
        f"{TO_MAPS} c:320,680 w:3 c:1105,415 w:4 c:700,275 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "w:16 " + " ".join(f"w:1 s:air{n}" for n in range(1, 9))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_anti_air_cut_in,
    ),
    # 2-1 with six destroyers who have a cut-in or a double attack to roll for each, and a star
    # shell. Photographed through the night.
    "night_cut_in": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_night_cut_in,
    ),
    # 2-1 with 古鷹 and five destroyers with the lookout cut-ins: types 4 and 8 to 14.
    # Photographed through the night.
    "night_destroyer_cut_in": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_night_destroyer_cut_in,
    ),
    # 2-1 with 最上改二特 and 三隈改二特 leading: the night 瑞雲 cut-in. Photographed through the night.
    "night_zuiun": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_night_zuiun,
    ),
    # 2-1 with 金剛改二丙 and 榛名改二乙 leading: the Kongou class's night assault in the
    # flagship's turn. Photographed through the night.
    "kongou_night_assault": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_kongou_night_assault,
    ),
    # 2-1 with 由良改二 leading, a night scout and a fighter aboard: the scout over the enemy
    # at night. Photographed through the night.
    "night_contact": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:4 " + " ".join(f"w:2 s:night{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_night_contact,
    ),
    # 2-1 in 梯形陣 with 長門改二 and 陸奥改二 leading: the flagship's special gunnery attack.
    # Photographed through the day.
    "nagato_broadside": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;772,516:api_req_sortie/battle "
        + "w:6 " + " ".join(f"w:2 s:day{n}" for n in range(1, 17))
        + " u:600,400;770,365:api_req_sortie/battleresult " + " ".join(f"w:2 s:after{n}" for n in range(1, 8))
        + " u:600,400;770,365:api_port/port w:5 s:home",
        check_nagato_broadside,
    ),
    # 2-1 in 梯形陣 with 陸奥改二 and a damaged 長門改二 leading: the second ship's damaged art
    # in the Mutsu broadside. Photographed through the day every half second.
    "mutsu_broadside": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;772,516:api_req_sortie/battle "
        + "w:6 " + " ".join(f"w:0.5 s:day{n}" for n in range(1, 65))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        damaged_partner_check(541, [329, 12], [98, 175], 102, 4),
    ),
    # 2-1 in 複縦陣 with Richelieu Deux and a damaged Jean Bart改 leading: the second ship's
    # damaged art in the Richelieu class's attack. Photographed through the day every half second.
    "richelieu_attack": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;870,278:api_req_sortie/battle "
        + "w:6 " + " ".join(f"w:0.5 s:day{n}" for n in range(1, 65))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        damaged_partner_check(724, [176, -32], [109, 205], 105, 2),
    ),
    # 2-5 again, sent to C by a battleship: its flagship 軽母ヌ級flagship attacks at night, which
    # the client plays as aircraft. Photographed through the night.
    "enemy_night_carrier": (
        f"{TO_MAPS} c:320,680 w:3 c:1105,415 w:4 c:700,275 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        + "u:600,400;770,365:api_req_battle_midnight/battle w:8 " + " ".join(f"w:2 s:night{n}" for n in range(1, 13))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_enemy_night_carrier,
    ),
    # 南西海域, 7-4, with two heavy cruisers and four destroyers nothing can hurt: by C, E, J and
    # K to M, and into the night against ヒ船団棲姫, who flies her attack there. Photographed
    # through the night.
    "enemy_night_crew": (
        f"{TO_MAPS} c:510,680 w:4 c:930,520 w:3 {START} "
        + f"{BATTLE} u:{TAPS}:api_req_map/next " * 4
        + "u:600,400;670,278:api_req_sortie/battle u:600,400;770,365:api_req_battle_midnight/battle w:8 "
        + " ".join(f"w:2 s:night{n}" for n in range(1, 13))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_enemy_night_crew,
    ),
    # 5-6 in its fourth phase: from its second start to I and O, where O and then Q are tapped,
    # battles at Q and U, and X, whose flagship 南太平洋空母棲姫 has a night crew of her own and
    # flies her attack. Photographed through the night.
    "enemy_night_crew_5_6": (
        f"stage:56:phase4:4 {TO_MAPS} c:700,680 w:3 c:1105,415 w:4 c:660,420 w:3 {START} w:8 s:choice "
        f"u:600,400;{SPOT_5_6_O}:api_req_map/next w:8 s:choice2 u:600,400;{SPOT_5_6_Q}:api_req_map/next "
        + f"{BATTLE} u:{TAPS}:api_req_map/next " * 2
        + "u:600,400;670,278:api_req_sortie/battle u:600,400;770,365:api_req_battle_midnight/battle w:8 "
        + " ".join(f"w:2 s:night{n}" for n in range(1, 13))
        + " u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_enemy_night_crew_5_6,
    ),
    # 南方海域, its extra operations, 5-6; three battles, the landing point, the boss, home.
    "transport_5_6": (
        f"{TO_MAPS} c:700,680 w:3 c:1105,415 w:4 c:660,420 w:3 {START} {BATTLE} {BATTLE} {BATTLE} "
        f"u:{TAPS}:api_req_map/next w:3 s:landing {BATTLE} w:14 s:result u:{TAPS}:api_port/port",
        check_transport_5_6,
    ),
    # 中部海域, the air corps: deploy bombers to the first squadron, order a sortie, add a second air
    # corps and close the panel (which is when the client sends the orders); sortie 6-4, point the air
    # corps at D twice, fight B and then D with its two attacks; go home to a squadron tired orange
    # and resupply it.
    "air_corps_6_4": (
        f"{TO_MAPS} c:800,680 w:4 c:720,182 w:5 c:1032,383 w:4 c:760,267 w:4 u:1017,655:api_req_air_corps/set_plane "
        "w:4 s:deployed c:1162,237 w:4 c:1030,180 w:4 u:477,477:api_req_air_corps/expand_base w:8 "
        "u:250,150:api_req_air_corps/set_action w:4 "
        "c:925,525 w:4 c:1015,668 w:4 u:830,668:api_req_map/start w:8 c:437,304 w:2 c:437,304 w:3 s:targets "
        f"u:157,90:api_req_map/start_air_base w:4 {BATTLE} u:{TAPS}:api_req_map/next {BATTLE} "
        "u:600,400;770,365:api_port/port w:6 tire:22 c:295,400 w:3 u:345,450:api_get_member/mapinfo w:4 c:800,680 w:4 "
        "c:720,182 w:5 s:home c:1030,382 w:4 u:1055,608:api_req_air_corps/supply w:5 s:supplied",
        check_air_corps_6_4,
    ),
    # 中部海域 with 6-5's boss sunk twice: deploy bombers, order the air corps to defend (in the
    # database: the raid only needs the server to know), sortie 6-5 through A, C, D, G to the boss
    # and home. The raid comes with one of the steps, at the latest the one onto the boss; each is
    # photographed twice, because nothing tells the steps which one it was.
    "air_raid_6_5": (
        f"sunk:65:2 {TO_MAPS} c:800,680 w:4 c:720,182 w:5 c:1032,383 w:4 c:760,267 w:4 u:1017,655:api_req_air_corps/set_plane "
        f"w:4 c:250,150 w:4 order:2 c:1105,415 w:4 c:700,275 w:3 {START} {BATTLE} "
        + " ".join(f"u:{TAPS}:api_req_map/next w:7 s:step{n} w:9 s:step{n}_later {BATTLE}" for n in (1, 2, 3))
        # The boss is a combined fleet, whose result has its own address.
        + f" u:{TAPS}:api_req_map/next w:7 s:step4 w:9 s:step4_later u:{TAPS}:api_req_combined_battle/battleresult"
        + f" w:14 u:{TAPS}:api_port/port w:5 s:home",
        check_air_raid_6_5,
    ),
    # The quest list, its 工廠 filter, and three quests claimed from the second row: 614 turns the
    # flagship's piece into another, 637 takes one from her and gives none back, 641 takes loose
    # ones. The first click on the list after 大淀 leaves does nothing, so one is spent on the header.
    "quest_equipment": (
        "api:api_world/get_worldinfo u:310,85:api_start2/get_option_setting u:910,605:api_port/port w:6 "
        "u:825,75:api_get_member/questlist w:15 c:500,125 w:3 c:1040,130 w:4 s:list "
        + " ".join(
            f"u:500,307:api_req_quest/clearitemget w:4 s:reward{n} c:600,605 w:5 s:after{n} u:600,605:api_get_member/questlist w:4"
            for n in (1, 2, 3)
        )
        + " s:claimed u:125,690:api_port/port w:5 s:port",
        check_quest_equipment,
    ),
    "season_2026_09_20": (SEASON_PORT, check_season("59")),
    "season_2026_10_14": (SEASON_PORT, check_season("60")),
}

# The season a scenario's server is configured with.
SEASON_OF = {"season_2026_09_20": "2026-09-20", "season_2026_10_14": "2026-10-14"}

# Scenarios played with another scenario's preset.
PRESET_OF = {
    "season_2026_09_20": "fresh_1_1",
    "season_2026_10_14": "fresh_1_1",
    "escort_1_6_standing": "escort_1_6",
    "escort_1_6_cleared": "escort_1_6",
    "enemy_night_crew_5_6": "transport_5_6",
    "practice_night_contact": "leveled_for_mid_boss",
}
# Fought as the server would fight them for a player.
FAIR = {"gunnery_cutin", "leveled_for_mid_boss", "anti_air_cut_in", "night_air_attack", "night_asw", "night_cut_in", "night_destroyer_cut_in", "night_zuiun", "enemy_night_carrier", "kongou_night_assault", "nagato_broadside", "night_contact", "practice_night_contact"}
# Never hurt, but sinking nothing at a blow: for an enemy several battles in who has to live.
UNHURT = {"enemy_night_crew", "enemy_night_crew_5_6", "mutsu_broadside", "richelieu_attack"}


def resource_report(work: Path, requested: set[str]) -> dict:
    """What the client asked other sites for, what it asked this one for that the cache list
    does not name, and what the server had
    to go to the origin for: a file fetched there is missing from the cache, one refused
    there is an address the origin does not have."""
    cache_list = CACHE / "cache_resources.nedb"
    listed = {json.loads(line)["path"] for line in cache_list.read_text().splitlines() if line}
    log = re.sub(r"\x1b\[[0-9;]*m", "", (work / "server.log").read_text())
    return {
        "off_site": sorted(path for path in requested if "://" in path),
        "not_in_cache_list": sorted(path for path in requested - listed if "://" not in path),
        "fetched_from_origin": sorted(set(re.findall(r"🛬 (\S+)", log))),
        "missing_on_origin": sorted(set(re.findall(r"🚫 404 on (\S+?),", log))),
    }


def workspace(scenario: str) -> tuple[Path, Path]:
    """A fresh workspace holding a copy of the codex, and the config that points at it."""
    work = ROOT / ".data/temp/headless" / scenario
    shutil.rmtree(work, ignore_errors=True)
    shutil.copytree(ROOT / ".data/codex", work / "codex")
    game = work / "codex/game_config.json"
    # Short battles that are always won, except where the battle itself is what is looked at.
    cheats = scenario not in FAIR
    game.write_text(
        json.dumps(json.loads(game.read_text()) | {"god_mode": cheats, "one_hit_kill": cheats and scenario not in UNHURT})
    )

    replaced = ("workspace_root", "cache_root", "mods_root", "bind", "tls_cert", "tls_key", "season")
    kept = [
        line
        for line in (ROOT / "emukc.config.toml").read_text().splitlines()
        if not line.startswith(replaced)
    ]
    config = work / "emukc.config.toml"
    config.write_text(
        "\n".join(
            [
                f'workspace_root = "{work}"',
                f'cache_root = "{CACHE}"',
                f'mods_root = "{MODS}"',
                f'bind = "127.0.0.1:{PORT}"',
                *([f'season = "{SEASON_OF[scenario]}"'] if scenario in SEASON_OF else []),
                *kept,
            ]
        )
    )
    if scenario in SEASON_OF:
        shutil.copytree(ROOT / ".data/seasons", work / "seasons")
    (work / "api").mkdir()
    return work, config


def wait_for_port() -> None:
    for _ in range(100):
        with socket.socket() as probe:
            if probe.connect_ex(("127.0.0.1", PORT)) == 0:
                return
        time.sleep(0.2)
    raise SystemExit(f"the server did not start listening on {PORT}")


def main() -> int:
    scenario = sys.argv[1]
    steps, check = SCENARIOS.get(scenario, ("", lambda work: []))
    if len(sys.argv) > 2:
        steps, check = sys.argv[2], lambda work: []
    steps = steps.split()
    work, config = workspace(scenario)
    emukcd = [str(ROOT / "target/debug/emukcd"), "-c", str(config)]
    session = subprocess.run(
        [*emukcd, "new-session", "--name", "headless", "--pass", "1234567", "--scenario", PRESET_OF.get(scenario, scenario), "--no-open", "--no-start"],
        capture_output=True,
        text=True,
        check=True,
    )
    url = next(line for line in session.stdout.splitlines() if "api_token=" in line)

    client = (ROOT / "main-decoder/out/main.decoded.js").read_text()
    entry = ENTRY.search(client)
    if not entry:
        raise SystemExit("bundle bootstrap not found; the entry module id probably changed")
    client = f"{client[: entry.start()]}globalThis.__clientRequire = {entry.group(2)}; {client[entry.start() :]}"

    report = {"scenario": scenario, "page_errors": [], "failed_requests": [], "api": [], "failed_step": None, "problems": []}
    server = subprocess.Popen([*emukcd, "serve", "--no-banner"], stdout=(work / "server.log").open("w"), stderr=subprocess.STDOUT)

    at = "starting the browser"

    def out_of_time(*_) -> None:
        # The steps have deadlines of their own, but a browser call can block under them.
        server.terminate()
        last = report["api"][-1] if report["api"] else "none"
        print(f"FAILED: no result after {LIMIT} s, at {at!r}; last KCSAPI call: {last}", flush=True)
        os._exit(1)

    signal.signal(signal.SIGALRM, out_of_time)
    signal.alarm(LIMIT)
    try:
        wait_for_port()
        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(channel="chrome", headless=True, args=["--mute-audio", "--enable-unsafe-swiftshader"])
            page = browser.new_page(viewport={"width": 1280, "height": 800})
            page.on("pageerror", lambda error: report["page_errors"].append(str(error)[:500]))

            requested = set()

            def on_response(response):
                path = response.url.split("?")[0].split(f":{PORT}/", 1)[-1]
                # What the server makes up itself is not a cached resource.
                made_here = ("kcsapi/", "emukc", "gadgets/", "social/", "kcs2/index.php", "kcs2/world.html", "kcs2/version.json", "kcs2/resources/world/")
                if not path.startswith(made_here):
                    requested.add(path)
                if response.status >= 400:
                    report["failed_requests"].append([response.status, response.url])
                if "/kcsapi/" in response.url:
                    path = response.url.split("/kcsapi/")[1]
                    report["api"].append(path)
                    dump = work / "api" / f"{len(report['api']):03}_{path.replace('/', '.')}.json"
                    dump.write_text(response.text().removeprefix("svdata="))
                    dump.with_suffix(".request").write_text(response.request.post_data or "")

            page.on("response", on_response)
            page.route(re.compile(r"/kcs2/js/main\.js"), lambda route: route.fulfill(body=client, content_type="application/javascript"))
            # The page runs on Chrome's virtual time: its timers fire one after another without
            # waiting, and the clock stands still while a request is out. So an animation costs
            # what its ticks cost to compute, and a wait is never eaten by loading.
            cdp = page.context.new_cdp_session(page)
            expired = []
            cdp.on("Emulation.virtualTimeBudgetExpired", lambda _: expired.append(True))

            def advance(ms: float) -> None:
                """Let the page live `ms` of its own time; it is frozen again when this returns."""
                expired.clear()
                cdp.send(
                    "Emulation.setVirtualTimePolicy",
                    {"policy": "pauseIfNetworkFetchesPending", "budget": ms, "maxVirtualTimeTaskStarvationCount": 100},
                )
                deadline = time.time() + 120
                while not expired:
                    if time.time() > deadline:
                        raise SystemExit(f"the page did not get through {ms} ms of its own time in two minutes")
                    page.wait_for_timeout(5)

            at = "loading the page"
            page.goto(url, wait_until="domcontentloaded")
            cursor = 0

            def click(x: int, y: int) -> None:
                # Some buttons only take a press after they have seen the pointer arrive.
                page.mouse.move(CANVAS_X + x, CANVAS_Y + y)
                advance(200)
                page.mouse.down()
                advance(80)
                page.mouse.up()

            def called(path: str, taps: list[tuple[int, int]]) -> bool:
                """Wait for a call made after the last one a step waited for, tapping meanwhile."""
                nonlocal cursor
                deadline = time.time() + 180
                while path not in report["api"][cursor:] and time.time() < deadline:
                    for tap in taps:
                        click(*tap)
                    advance(2000 if taps else 250)
                if path not in report["api"][cursor:]:
                    return False
                cursor += report["api"][cursor:].index(path) + 1
                return True

            for step in steps:
                at = step
                kind, _, value = step.partition(":")
                if kind == "w":
                    advance(float(value) * 1000)
                elif kind == "c":
                    click(*map(int, value.split(",")))
                elif kind == "s":
                    page.screenshot(path=work / f"{value}.png")
                elif kind == "tire":
                    # Nothing short of several sorties tires a squadron that far.
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute("UPDATE plane_info SET condition = ?", (int(value),))
                elif kind == "sunk":
                    # A gauge half down takes sorties to reach: `sunk:<map id>:<times>`.
                    map_id, times = value.split(":")
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute("UPDATE map_record SET defeat_count = ? WHERE map_id = ?", (int(times), int(map_id)))
                elif kind == "stage":
                    # A later phase of a map with several: `stage:<map id>:<variant>:<gauge index>`.
                    map_id, stage_id, index = value.split(":")
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute(
                            "UPDATE map_record SET stage_id = ?, gauge_index = ?, defeat_count = 0 WHERE map_id = ?",
                            (stage_id, int(index), int(map_id)),
                        )
                elif kind == "rival":
                    # Every practice rival fields these ships at level 99: `rival:<mst id>,<mst id>`.
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute("DELETE FROM rival_ship")
                        for rival, profile in db.execute("SELECT id, profile_id FROM rival").fetchall():
                            for mst_id in value.split(","):
                                db.execute(
                                    "INSERT INTO rival_ship (profile_id, rival_id, mst_id, level, star) VALUES (?, ?, ?, 99, 0)",
                                    (profile, rival, int(mst_id)),
                                )
                elif kind == "secretary":
                    # The first fleet's flagship becomes this ship: `secretary:<mst id>`. Only her
                    # looks change; her stats stay the preset ship's, which the port does not show.
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute(
                            "UPDATE ship SET mst_id = ? WHERE id = (SELECT ship_1 FROM fleet WHERE \"index\" = 1)",
                            (int(value),),
                        )
                elif kind == "cleared":
                    # A gauge already emptied this month: `cleared:<map id>`.
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute("UPDATE map_record SET cleared = 1 WHERE map_id = ?", (int(value),))
                elif kind == "order":
                    # Every air corps gets this order (2 is 防空); the client still shows its own.
                    with sqlite3.connect(work / "emukc.db", timeout=30) as db:
                        db.execute("UPDATE airbase SET action = ?", (int(value),))
                elif kind in ("api", "u"):
                    spot, _, path = value.rpartition(":")
                    taps = [tuple(map(int, at.split(","))) for at in spot.split(";")] if spot else []
                    if not called(path, taps):
                        report["failed_step"] = step
                        page.screenshot(path=work / "failed.png")
                        break
            browser.close()
    finally:
        server.terminate()
        server.wait()

    report["resources"] = {"requested": len(requested)} | resource_report(work, requested)
    if not report["failed_step"]:
        report["problems"] = check(work)
    (work / "report.json").write_text(json.dumps(report, indent=1, ensure_ascii=False))
    failed = bool(
        report["page_errors"]
        or report["failed_requests"]
        or report["failed_step"]
        or report["problems"]
        or report["resources"]["missing_on_origin"]
    )
    print(json.dumps(report | {"api": len(report["api"])}, indent=1, ensure_ascii=False))
    print(f"{'FAILED' if failed else 'ok'}: {work / 'report.json'}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
