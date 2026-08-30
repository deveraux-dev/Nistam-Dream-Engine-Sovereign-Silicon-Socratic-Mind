#!/usr/bin/env python3
"""Ironroot CURATED atlas — ~39 hand-picked images spanning every bucket
(blueprint/character/env/faction/key-art/location/portrait/misc/world-map/sprite),
quantized to the SAME shared ironroot-palette.json the zodiac atlas already uses.
Usage: python build_atlas_curated.py
"""
import json, os
from PIL import Image

ROOT = r"F:\v3\assets\ironroot\Good"
OUT  = r"F:\v3\web\13forge.com\assets"

FILES = [
    "Blueprint of Ironroot Cathedral-Fortress-Palace.png",
    "Ironroot dungeon blueprint_ The Under-Orchard.png",
    "C01_protagonist_normal.png", "C02_protagonist_ascended.png", "C03_protagonist_mega.png",
    "C06_bandit_boss.png", "C07_corrupted_wolf.png", "C08_bandit_swordsman.png",
    "E01_forest_entry.png", "E02_forest_deep.png", "E03_camp_exterior.png",
    "E05_spirit_forest.png", "E06_boss_arena.png", "E07_celestial_bastion.png",
    "hollowden_pack_banner_golden.png", "ironmoor_trade_banner_golden.png",
    "murkveil_cult_banner_decay.png", "thornhaven_guard_banner_golden.png",
    "Ironroot Dominion_ The Faithful Order.png",
    "thornhaven_full.png", "thornhaven_isometric.png", "thornhaven_overhead.png",
    "thornhaven_gate_golden.png", "thornhaven_market_golden.png",
    "2Q.png", "2Q-1.png", "1772136310270.png",
    "Moon oracle of the silver veil.png", "Regina Viriditas, Queen of Glenrealm.png",
    "River fae messenger_ herald of currents.png", "Twilight scout of the Gloamwild.png",
    "Angelic-demonic warrior in gothic haze.png", "Mystic armored assassin in dark fantasy armor.png",
    "The Verdant Court of Glenrealm.png",
    "Welcome to Ironroot's hidden wonders.png", "Ironroot_ tales beneath the surface.png",
    "Ironroot MVP system map.png", "Ironroot camera strategy system map.png",
    "sheet1.png",
]

def _index_good():
    """name -> full path, now that Good/ is sorted into bucket subfolders."""
    idx = {}
    for dirpath, _, names in os.walk(ROOT):
        for n in names:
            idx.setdefault(n, os.path.join(dirpath, n))
    return idx

def main():
    os.makedirs(OUT, exist_ok=True)
    good_idx = _index_good()
    pal = json.load(open(os.path.join(OUT, "ironroot-palette.json")))
    cols = [tuple(int(h[i:i+2], 16) for i in (1, 3, 5)) for h in pal["colors"]]
    flat = [v for c in cols for v in c] + [0] * (3 * (256 - len(cols)))
    pmap = Image.new("P", (1, 1)); pmap.putpalette(flat)

    CW, CH, COLS = 256, 256, 8
    n = len(FILES)
    rows = (n + COLS - 1) // COLS
    atlas = Image.new("RGBA", (CW * COLS, CH * rows), (0, 0, 0, 0))
    manifest = {"atlas": "ironroot-atlas-curated.png", "cell": [CW, CH],
                "palette": "ironroot-palette.json", "frames": {}}

    missing = []
    for i, name in enumerate(FILES):
        p = good_idx.get(name)
        if not p or not os.path.exists(p):
            missing.append(name); continue
        im = Image.open(p).convert("RGBA")
        bg = Image.new("RGBA", im.size, (0, 0, 0, 255)); bg.alpha_composite(im)
        bg = bg.convert("RGB").resize((CW, CH))
        q = bg.quantize(palette=pmap, dither=Image.FLOYDSTEINBERG).convert("RGB")
        cx, cy = (i % COLS) * CW, (i // COLS) * CH
        atlas.paste(q, (cx, cy))
        key = os.path.splitext(name)[0]
        manifest["frames"][key] = {"x": cx, "y": cy, "w": CW, "h": CH, "src": name}

    atlas.save(os.path.join(OUT, "ironroot-atlas-curated.png"))
    with open(os.path.join(OUT, "ironroot-atlas-curated.json"), "w") as f:
        json.dump(manifest, f, indent=1)
    print(f"[atlas-curated] wrote ironroot-atlas-curated.png ({CW*COLS}x{CH*rows}), "
          f"{n-len(missing)}/{n} frames, palette=ironroot-palette.json (64 shared colours)")
    if missing:
        print("[atlas-curated] MISSING (skipped):", missing)

if __name__ == "__main__":
    main()
