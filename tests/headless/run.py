"""Drive the real client, headless, against a throwaway server.

    python run.py <scenario preset> ["<steps>"]

Without steps the scenario's own are played and its check is run. Steps given on the
command line are for finding the way through a new screen. Steps are space separated: `w:<seconds>` lets the game live
that long (its own time: see `advance`), `c:<x>,<y>` clicks the game canvas
(1200x720), `s:<name>` saves a screenshot, `api:<path>` waits until the client has called
that KCSAPI path since the last call a step waited for, and `u:<x>,<y>:<path>` does the
same while clicking that spot every few seconds (several spots, separated by `;`, are
clicked in turn). Every KCSAPI response is saved under api/, and the report lists the
resources the client asked for that the cache list misses or the origin does not have. The run fails on a page error, a failed request or a step that
times out; the report and screenshots land in .data/temp/headless/<scenario>/.

Needs the bootstrapped .data/codex, the resource cache, main-decoder/out/main.decoded.js
and Playwright for Python with Chrome installed. Never touches .data/emukc.db.
"""

import json
import re
import shutil
import socket
import sqlite3
import subprocess
import sys
import time
from pathlib import Path

from playwright.sync_api import sync_playwright

ROOT = Path(__file__).resolve().parents[2]
PORT = 27777
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


def responses(work: Path, path: str) -> list[dict]:
    """What the server answered to every call of `path`, in order."""
    return [json.loads(dump.read_text())["api_data"] for dump in sorted((work / "api").glob(f"*_{path.replace('/', '.')}.json"))]


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


def check_gunnery_cutin(work: Path) -> list[str]:
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
    if [home[ship] for ship in range(1, 7)] != left:
        problems.append(f"the battles left the fleet at {left} and it came home at {home}")
    return problems


SCENARIOS = {
    # One battle of 1-1, up to the choice between going on and going home.
    "fresh_1_1": (f"{TO_MAPS} c:280,280 w:3 {START} {BATTLE}", lambda work: []),
    # 南西諸島海域, 2-1, without the cheats: one battle, into the night if it is offered (夜戦突入
    # sits where 撤退 does on the next choice), and home. What the packets say the fleet was
    # left with has to be what it comes home with.
    "gunnery_cutin": (
        f"{TO_MAPS} c:320,680 w:3 c:280,280 w:3 {START} u:600,400;670,278:api_req_sortie/battle "
        "u:600,400;770,365:api_req_sortie/battleresult w:14 s:result u:600,400;770,365:api_port/port w:5 s:home",
        check_gunnery_cutin,
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
}

# Fought as the server would fight them for a player.
FAIR = {"gunnery_cutin"}


def resource_report(work: Path, requested: set[str]) -> dict:
    """What the client asked other sites for, what it asked this one for that the cache list
    does not name, and what the server had
    to go to the origin for: a file fetched there is missing from the cache, one refused
    there is an address the origin does not have."""
    cache_list = ROOT / "z/cache/cache_resources.nedb"
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
    game.write_text(json.dumps(json.loads(game.read_text()) | {"god_mode": cheats, "one_hit_kill": cheats}))

    replaced = ("workspace_root", "cache_root", "mods_root", "bind", "tls_cert", "tls_key")
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
                f'cache_root = "{ROOT / "z/cache"}"',
                f'mods_root = "{ROOT / "z/mods"}"',
                f'bind = "127.0.0.1:{PORT}"',
                *kept,
            ]
        )
    )
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
        [*emukcd, "new-session", "--name", "headless", "--pass", "1234567", "--scenario", scenario, "--no-open", "--no-start"],
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
