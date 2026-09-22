#!/usr/bin/env python3
"""Pull read-only endpoints from the LIVE KanColle server with your own session.

Companion to fetch_map_start.py, which talks to the local emulator instead.
Read docs/solutions/best-practices/live-api-investigation.md before using it:
the session dies the moment the game page closes, and some endpoints write.

    export EMUKC_LIVE_TOKEN=...      # api_token from the game page's request
    export EMUKC_LIVE_HOST=https://w14h.kancolle-server.com
    python3 scripts/fetch_live_api.py --member-id 12345678

Writes one JSON per endpoint (svdata= prefix stripped) under z/snapshot/<date>/,
which is gitignored. Never commit a token or a snapshot.
"""

import argparse
import json
import os
import random
import sys
import time
import urllib.parse
import urllib.request
from datetime import date
from pathlib import Path

# Everything the game page sends, minus the cookies it does not need.
HEADERS = {
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:154.0) Gecko/20100101 Firefox/154.0",
    "Accept": "application/json, text/plain, */*",
    "Accept-Language": "en-US,en;q=0.9",
    "Content-Type": "application/x-www-form-urlencoded",
    "Sec-Fetch-Dest": "empty",
    "Sec-Fetch-Mode": "cors",
    "Sec-Fetch-Site": "same-origin",
    "DNT": "1",
}

# Verified read-only on 2026-09-22. api_port/port is handled separately because
# it needs the signed api_port key; api_get_member/base_air_corps is gone (404).
READ_ONLY = [
    ("api_get_member/require_info", ""),
    ("api_get_member/mapinfo", ""),
    ("api_get_member/slot_item", ""),
    ("api_get_member/deck", ""),
    ("api_get_member/material", ""),
    ("api_get_member/ndock", ""),
    ("api_get_member/kdock", ""),
    ("api_get_member/mission", ""),
    ("api_get_member/useitem", ""),
    ("api_get_member/preset_deck", ""),
    ("api_get_member/practice", ""),
    ("api_req_kousyou/remodel_slotlist", ""),
    ("api_start2/getData", ""),
    # The server filters by tab, so every tab is its own sample.
    *[(f"api_get_member/questlist", f"&api_tab_id={t}&api_page_no=1") for t in (0, 9, 1, 2, 3, 4, 5)],
]

# PORT_API_SEED, mirrored from emukc_crypto::PortApiKey (the authority).
PORT_API_SEED = [3187, 3596, 6413, 9628, 7279, 7678, 6023, 2564, 9558, 9272]


def port_api_key(member_id, now_sec=None, draws=None):
    """The api_port signature. Mirrors PortAPI._createKey in the client."""
    now_sec = int(time.time()) if now_sec is None else now_sec
    r1, r2, r3, d, e, f = draws or (
        random.randrange(9),
        random.randrange(8999),
        random.randrange(32767),
        random.randrange(10),
        random.randrange(10),
        random.randrange(10),
    )
    seed = PORT_API_SEED[member_id % 10]
    a = 1000 * (r1 + 1) + member_id % 1000
    b, c = r2 + 1000, r3 + 32768
    g = ((4132653 + c) * (int(str(member_id)[:4]) + 1000) - now_sec + (1875979 + 9 * c) - member_id) * seed
    s = f"{d}{a}{g}{b}"
    s = s[:8] + str(e) + s[8:]
    s = s[:18] + str(f) + s[18:]
    return s + str(c)


def selftest():
    """Same vectors as the Rust test, so the two cannot drift apart."""
    cases = [
        (12345678, 1790070509, (0, 0, 0, 0, 0, 0), "016787170357072736044100032768"),
        (12345678, 1790070509, (8, 8998, 32766, 9, 9, 9), "996787249381642446948999865534"),
        (1000, 1600000000, (4, 1234, 10000, 1, 2, 3), "150002152221357266317223442768"),
        (99999999, 1790070509, (3, 777, 20000, 5, 0, 7), "5499940903377151197664177752768"),
        (7, 1, (0, 0, 0, 0, 0, 0), "010071070604665641020100032768"),
    ]
    for member_id, now, draws, expected in cases:
        got = port_api_key(member_id, now, draws)
        assert got == expected, f"{member_id}: {got} != {expected}"
    print("port_api_key selftest ok")


def call(host, token, endpoint, extra=""):
    payload = f"api_token={token}&api_verno=1{extra}"
    referer = (
        f"{host}/kcs2/index.php?api_root=/kcsapi&voice_root=/kcs/sound"
        f"&osapi_root=osapi.dmm.com&api_token={token}"
    )
    req = urllib.request.Request(
        f"{host}/kcsapi/{endpoint}",
        data=payload.encode(),
        headers={**HEADERS, "Origin": host, "Referer": referer},
        method="POST",
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        body = resp.read().decode("utf-8")
    return json.loads(body.removeprefix("svdata="))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--member-id", type=int, help="needed for api_port/port")
    ap.add_argument("--out", default=f"z/snapshot/{date.today()}")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()

    if args.selftest:
        selftest()
        return 0

    token = os.environ.get("EMUKC_LIVE_TOKEN")
    host = os.environ.get("EMUKC_LIVE_HOST")
    if not token or not host:
        print("set EMUKC_LIVE_TOKEN and EMUKC_LIVE_HOST", file=sys.stderr)
        return 1

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    targets = list(READ_ONLY)
    if args.member_id:
        key = port_api_key(args.member_id)
        targets.append(("api_port/port", f"&api_sort_key=5&spi_sort_order=2&api_port={key}"))

    for endpoint, extra in targets:
        name = endpoint.replace("/", "_")
        if extra:
            tab = urllib.parse.parse_qs(extra.lstrip("&")).get("api_tab_id")
            if tab:
                name += f"_tab{tab[0]}"
        try:
            data = call(host, token, endpoint, extra)
        except Exception as err:  # a dead session or a 404 is data, not a crash
            print(f"{endpoint:40} FAILED {err}")
            continue
        (out / f"{name}.json").write_text(json.dumps(data, ensure_ascii=False))
        result = data.get("api_result")
        note = "" if result == 1 else f"  <- {data.get('api_result_msg', '')}"
        print(f"{endpoint:40} result={result}{note}")
        time.sleep(0.3)  # keep the cadence closer to a client than to a scraper

    print(f"\nwrote {args.out}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
