#!/usr/bin/env python3
"""One-time migration: merge real KC API cell data into codex map_catalog.json.

Updates boss_cell_no and color_no from real KC API captures.
Preserves existing event_id/event_kind for ambiguous colors (4, 6, 9+).
Only updates the default variant ("") to avoid cross-contamination.
"""

import json
import glob
import os

# Unambiguous color → (event_id, event_kind) mappings.
# color_no=4 is NOT mapped because it could be battle (eid=4),
# air battle (eid=6), ld_airbattle (eid=7), etc.
UNAMBIGUOUS_EVENTS = {
    0: (0, 0),   # start
    2: (2, 0),   # resource
    3: (3, 0),   # maelstrom
    5: (5, 1),   # boss
}


def main():
    repo_root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    catalog_path = os.path.join(repo_root, ".data", "codex", "map_catalog.json")
    real_data_dir = os.path.join(repo_root, "docs", "real_data", "map_start_data")

    with open(catalog_path, "r") as f:
        catalog = json.load(f)

    maps = catalog["maps"]
    updated = 0
    skipped = 0

    for filepath in sorted(glob.glob(os.path.join(real_data_dir, "map_*.json"))):
        basename = os.path.basename(filepath)
        parts = basename.replace("map_", "").replace(".json", "").split("-")
        area, stage = int(parts[0]), int(parts[1])
        map_key = f"{area}{stage}"

        if map_key not in maps:
            print(f"  SKIP {basename}: key {map_key} not in catalog")
            skipped += 1
            continue

        with open(filepath, "r") as f:
            real_data = json.load(f)

        api = real_data.get("api_data", real_data)
        real_boss = api.get("api_bosscell_no")
        real_cells = api.get("api_cell_data", [])

        if not real_cells:
            print(f"  SKIP {basename}: no api_cell_data")
            skipped += 1
            continue

        real_cell_map = {rc["api_no"]: rc for rc in real_cells}

        map_entry = maps[map_key]
        boss_changed = False
        cells_changed = 0

        # Only update default variant to avoid cross-contamination
        variant = map_entry.get("variants", {}).get("")
        if variant is None:
            print(f"  SKIP {basename}: no default variant")
            skipped += 1
            continue

        # Update boss_cell_no
        if real_boss is not None and variant.get("boss_cell_no") != real_boss:
            variant["boss_cell_no"] = real_boss
            boss_changed = True

        # Update cells: color_no always, event_id/event_kind only for unambiguous colors
        for cell in variant.get("cells", []):
            cell_no = cell["cell_no"]
            if cell_no not in real_cell_map:
                continue

            rc = real_cell_map[cell_no]
            new_color = rc["api_color_no"]

            if cell["color_no"] != new_color:
                cell["color_no"] = new_color
                cells_changed += 1

            # Only overwrite event_id/event_kind for unambiguous colors
            if new_color in UNAMBIGUOUS_EVENTS:
                eid, ekind = UNAMBIGUOUS_EVENTS[new_color]
                if cell["event_id"] != eid or cell["event_kind"] != ekind:
                    cell["event_id"] = eid
                    cell["event_kind"] = ekind
                    cells_changed += 1

        if boss_changed or cells_changed > 0:
            print(f"  FIX  {basename} (key={map_key}): boss={'CHANGED' if boss_changed else 'ok'}, cells={cells_changed}")
            updated += 1
        else:
            print(f"  OK   {basename} (key={map_key}): already correct")
            skipped += 1

    with open(catalog_path, "w") as f:
        json.dump(catalog, f, indent=2, ensure_ascii=False)
        f.write("\n")

    print(f"\nDone: {updated} maps updated, {skipped} skipped/unmodified")


if __name__ == "__main__":
    main()
