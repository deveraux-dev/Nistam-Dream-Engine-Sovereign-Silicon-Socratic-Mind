#!/usr/bin/env python3
"""Match any folder of art to the shared Ironroot 64-colour palette.
Decimate (downsample longest side to MAX_SIDE) + quantize (Floyd-Steinberg) so
loose concept art becomes consistent with the character atlas and maps.
Usage: python match_palette.py "<src_dir>" "<out_subdir>" [max_side]
"""
import os, sys, glob, json
from PIL import Image

PAL = r"F:\v3\web\13forge.com\assets\ironroot-palette.json"
OUTROOT = r"F:\v3\web\13forge.com\assets"

def pmap():
    cols = [tuple(int(h[i:i+2],16) for i in (1,3,5)) for h in json.load(open(PAL))["colors"]]
    flat = [v for c in cols for v in c] + [0]*(3*(256-len(cols)))
    p = Image.new("P",(1,1)); p.putpalette(flat); return p

def slug(name):
    b = os.path.splitext(os.path.basename(name))[0].lower()
    for ch in " _-.,'": b = b.replace(ch,"-")
    b = "".join(c for c in b if c.isalnum() or c=="-")
    while "--" in b: b = b.replace("--","-")
    return b.strip("-")[:48]

def main():
    src = sys.argv[1]
    outdir = os.path.join(OUTROOT, sys.argv[2] if len(sys.argv)>2 else "concept")
    max_side = int(sys.argv[3]) if len(sys.argv)>3 else 1024
    os.makedirs(outdir, exist_ok=True)
    pm = pmap()
    files = []
    for ext in ("*.png","*.jpg","*.jpeg","*.webp"):
        files += glob.glob(os.path.join(src,"**",ext), recursive=True)
    seen, manifest = set(), []
    for f in sorted(files):
        s = slug(f)
        if s in seen: continue
        seen.add(s)
        try: im = Image.open(f).convert("RGB")
        except Exception as e: print(f"  skip {os.path.basename(f)}: {e}"); continue
        w,h = im.size; sc = max_side/max(w,h)
        if sc < 1: im = im.resize((round(w*sc), round(h*sc)), Image.LANCZOS)
        q = im.quantize(palette=pm, dither=Image.FLOYDSTEINBERG).convert("RGB")
        out = os.path.join(outdir, f"{s}.q.png"); q.save(out, optimize=True)
        kb = os.path.getsize(out)//1024
        manifest.append({"src":os.path.basename(f),"out":f"{os.path.basename(outdir)}/{s}.q.png","w":q.size[0],"h":q.size[1],"kb":kb})
        print(f"  {w}x{h} -> {q.size[0]}x{q.size[1]}  {kb}KB  {s}")
    json.dump({"palette":"ironroot-palette.json","count":len(manifest),"items":manifest},
              open(os.path.join(outdir,"index.json"),"w"), indent=1)
    print(f"[match] {len(manifest)} matched -> {outdir}")

if __name__=="__main__": main()
