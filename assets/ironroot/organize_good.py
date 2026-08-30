#!/usr/bin/env python3
"""One-time reorg: move the 174 flat files in assets/ironroot/Good/ into
bucket subfolders per .forge/ironroot-assets.csv. Leaves the 12 zodiac sign
folders (aries/, taurus/, ...) and doc/spec folders untouched.
"""
import csv, os, shutil

ROOT = r"F:\v3\assets\ironroot\Good"
CSV = r"F:\v3\.forge\ironroot-assets.csv"

BUCKET_DIR = {
    "BLUEPRINT": "blueprint",
    "CHARACTER": "character",
    "ENV-backdrop": "env-backdrop",
    "FACTION-banner": "faction-banner",
    "KEY-ART/dossier": "key-art",
    "LOCATION": "location",
    "MISC/uncategorized": "misc",
    "PORTRAIT/concept": "portrait",
    "SPRITE/ITEM-sheet": "sprite-sheet",
    "WORLD-MAP": "world-map",
}

def main():
    moved, skipped_missing, skipped_nested = 0, 0, 0
    with open(CSV, newline="", encoding="utf-8") as f:
        for row in csv.DictReader(f):
            bucket, name = row["bucket"], row["name"]
            dest_dir = BUCKET_DIR.get(bucket)
            if not dest_dir:
                print(f"[skip] unknown bucket {bucket!r} for {name}")
                continue
            src = os.path.join(ROOT, name)
            if not os.path.exists(src):
                skipped_missing += 1  # lives in a sign-subfolder already, not flat
                continue
            dst_dir = os.path.join(ROOT, dest_dir)
            os.makedirs(dst_dir, exist_ok=True)
            dst = os.path.join(dst_dir, name)
            if os.path.exists(dst):
                print(f"[skip] already at dest: {dest_dir}/{name}")
                skipped_nested += 1
                continue
            shutil.move(src, dst)
            moved += 1
    print(f"[organize] moved={moved} skipped_missing(sign-subfolder)={skipped_missing} skipped_dup={skipped_nested}")

if __name__ == "__main__":
    main()
