#!/usr/bin/env python3
"""Fetch api_req_map/start data for maps 1-1 ~ 7-5 and save responses."""

import ssl
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

URL = "https://localhost:27666/kcsapi/api_req_map/start"
HEADERS = {
    "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:148.0) Gecko/20100101 Firefox/148.0",
    "Accept": "application/json, text/plain, */*",
    "Accept-Language": "en-US,en;q=0.9",
    "Content-Type": "application/x-www-form-urlencoded",
    "Origin": "https://localhost:27666",
    "Sec-GPC": "1",
    "Connection": "keep-alive",
    "Referer": "https://localhost:27666/kcs2/index.php?api_root=/kcsapi&voice_root=/kcs/sound&osapi_root=osapi.dmm.com&version=6.2.7.0&api_token=YOUR_TOKEN&api_starttime=1775127806409",
    "Sec-Fetch-Dest": "empty",
    "Sec-Fetch-Mode": "cors",
    "Sec-Fetch-Site": "same-origin",
    "DNT": "1",
    "Priority": "u=0",
    "TE": "trailers",
}

API_TOKEN = "YOUR_TOKEN"

# 部分旧服务器 TLS 协商不稳定，放宽安全级别
SSL_CTX = ssl.create_default_context()
SSL_CTX.minimum_version = ssl.TLSVersion.TLSv1_2
SSL_CTX.set_ciphers("DEFAULT:@SECLEVEL=1")


# api_serial_cid 通常是一次性的时间戳+随机数，这里每次请求重新生成
def make_serial_cid() -> str:
    return f"{int(time.time() * 1000)}{int(time.time() * 1000000) % 1000000}"


def fetch_map(area_id: int, map_no: int, retries: int = 3) -> str:
    payload = urllib.parse.urlencode(
        {
            "api_token": API_TOKEN,
            "api_verno": "1",
            "api_maparea_id": str(area_id),
            "api_mapinfo_no": str(map_no),
            "api_deck_id": "1",
            "api_serial_cid": make_serial_cid(),
        }
    )
    req = urllib.request.Request(
        URL,
        data=payload.encode("utf-8"),
        headers=HEADERS,
        method="POST",
    )
    last_err = None
    for attempt in range(1, retries + 1):
        try:
            with urllib.request.urlopen(req, context=SSL_CTX, timeout=30) as resp:
                return resp.read().decode("utf-8")
        except (urllib.error.URLError, ssl.SSLError, TimeoutError) as e:
            last_err = e
            print(f"  Retry {attempt}/{retries} for {area_id}-{map_no}: {e}")
            time.sleep(2**attempt)
    raise last_err  # type: ignore[misc]


def main() -> None:
    out_dir = Path(__file__).with_name("map_start_data")
    out_dir.mkdir(exist_ok=True)

    for area_id in range(1, 7):
        for map_no in range(1, 6):
            print(f"Fetching {area_id}-{map_no} ...")
            raw = fetch_map(area_id=area_id, map_no=map_no)

            # KanColle API 返回 "svdata=" 前缀的 JSONP / 类 JSON 数据
            # 去掉前缀后再存为纯 JSON
            if raw.startswith("svdata="):
                body = raw[len("svdata=") :]
            else:
                body = raw

            file_path = out_dir / f"map_{area_id}-{map_no}.json"
            file_path.write_text(body, encoding="utf-8")
            print(f"  Saved -> {file_path}")

            # 稍作延迟，避免请求过快
            time.sleep(3)


if __name__ == "__main__":
    main()
