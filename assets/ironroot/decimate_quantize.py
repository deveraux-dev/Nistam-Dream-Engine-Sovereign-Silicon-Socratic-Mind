#!/usr/bin/env python3
"""Decimate + quantize Ironroot maps/blueprints to the shared 64-colour palette.
Decimate: downsample longest side to MAX_SIDE (detail/size reduction).
Quantize: lock to ironroot-palette.json so maps match the character atlas.
Outputs to assets/maps/ as <name>.q.png plus a manifest.
"""
import os, glob, json
from PIL import Image

SRC = r"F:\v3\assets\ironroot\Good"
PAL = r"F:\v3\web\13forge.com\assets\ironroot-palette.json"
OUT = r"F:\v3\web\13forge.com\assets\maps"
MAX_SIDE = 1024
KEYWORDS = ("blueprint","map","dungeon","fortress","parish","arena","hub","orchard","cathedral")

def load_pmap():
    cols = [tuple(int(h[i:i+2],16) for i in (1,3,5)) for h in json.load(open(PAL))["colors"]]
    flat = [v for c in cols for v in c] + [0]*(3*(256-len(cols)))
    p = Image.new("P",(1,1)); p.putpalette(flat); return p

def slug(name):
    base = os.path.splitext(os.path.basename(name))[0].lower()
    for ch in " _-.":
        base = base.replace(ch,"-")
    base = "".join(c for c in base if c.isalnum() or c=="-")
    while "--" in base: base = base.replace("--","-")
    return base.strip("-")[:48]

def main():
    os.makedirs(OUT, exist_ok=True)
    pmap = load_pmap()
    files = [f for f in glob.glob(os.path.join(SRC,"*.png"))
             if any(k in os.path.basename(f).lower() for k in KEYWORDS)]
    seen, manifest = set(), []
    for f in sorted(files):
        s = slug(f)
        if s in seen: continue
        seen.add(s)
        im = Image.open(f).convert("RGB")
        w,h = im.size
        scale = MAX_SIDE/max(w,h)
        if scale < 1: im = im.resize((round(w*scale), round(h*scale)), Image.LANCZOS)
        q = im.quantize(palette=pmap, dither=Image.FLOYDSTEINBERG).convert("RGB")
        out = os.path.join(OUT, f"{s}.q.png")
        q.save(out, optimize=True)
        kb = os.path.getsize(out)//1024
        manifest.append({"src":os.path.basename(f),"out":f"maps/{s}.q.png","w":q.size[0],"h":q.size[1],"kb":kb})
        print(f"  {w}x{h} -> {q.size[0]}x{q.size[1]}  {kb}KB  {s}")
    json.dump({"palette":"ironroot-palette.json","max_side":MAX_SIDE,"maps":manifest},
              open(os.path.join(OUT,"maps.json"),"w"), indent=1)
    print(f"[maps] {len(manifest)} decimated + quantized -> {OUT}")

if __name__=="__main__": main()
