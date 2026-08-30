#!/usr/bin/env python3
"""Ironroot atlas + shared-palette builder.
Extracts ONE 64-colour palette from all 12 zodiac sprites (the consistency anchor),
then quantizes every sprite to it and packs a single atlas PNG + JSON manifest.
Any future art — hand-drawn or genai — is locked to palette.json to stay consistent.
"""
import json, os, glob
from PIL import Image

ROOT = r"F:\v3\assets\ironroot\Good"
OUT  = r"F:\v3\web\13forge.com\assets"
SIGNS = ["aries","taurus","gemini","cancer","leo","virgo","libra",
         "scorpio","sagittarius","capricorn","aquarius","pisces"]
# 8 UI/accent colours from the 2dak spec (VOID..ACCENT), appended after 56 sprite colours.
UI = [(16,12,23),(0,0,0),(255,255,255),(60,220,60),(255,60,60),(140,60,220),(224,173,76),(127,208,138)]

def pick(sign):
    d = os.path.join(ROOT, sign)
    files = glob.glob(os.path.join(d, "*.png"))
    # prefer an idle/tpose/cast full-body frame; else the largest file
    for kw in ("idle","tpose","cast","ascended"):
        hit = [f for f in files if kw in os.path.basename(f).lower()]
        if hit: return sorted(hit, key=os.path.getsize, reverse=True)[0]
    return sorted(files, key=os.path.getsize, reverse=True)[0] if files else None

def main():
    os.makedirs(OUT, exist_ok=True)
    picks = {s: pick(s) for s in SIGNS}
    picks = {s: p for s, p in picks.items() if p}
    print(f"[atlas] sources: {len(picks)}/12 signs")

    # 1) SHARED PALETTE: montage every sprite (RGB over black), adaptive-quantize to 56.
    thumbs = []
    for s, p in picks.items():
        im = Image.open(p).convert("RGBA")
        bg = Image.new("RGBA", im.size, (0,0,0,255)); bg.alpha_composite(im)
        thumbs.append(bg.convert("RGB").resize((192, 352)))
    W = 192*len(thumbs); montage = Image.new("RGB", (W, 352))
    for i,t in enumerate(thumbs): montage.paste(t, (i*192, 0))
    pal_img = montage.quantize(colors=56, method=Image.MEDIANCUT)
    raw = pal_img.getpalette()[:56*3]
    colours = [(raw[i],raw[i+1],raw[i+2]) for i in range(0,56*3,3)] + UI  # 64 total

    # write palette.json + a swatch strip
    with open(os.path.join(OUT,"ironroot-palette.json"),"w") as f:
        json.dump({"name":"2dak-64","colors":["#%02x%02x%02x"%c for c in colours]}, f, indent=1)
    sw = Image.new("RGB",(64*16,48))
    for i,c in enumerate(colours):
        for x in range(16):
            for y in range(48): sw.putpixel((i*16+x,y), c)
    sw.save(os.path.join(OUT,"ironroot-palette.png"))

    # fixed-palette image for quantizing everything to the same 64
    flat = [v for c in colours for v in c] + [0]*(3*(256-len(colours)))
    pmap = Image.new("P",(1,1)); pmap.putpalette(flat)

    # 2) ATLAS: 4x3 grid, each sprite quantized to the shared palette, alpha preserved.
    CW, CH, COLS = 384, 704, 4
    rows = (len(picks)+COLS-1)//COLS
    atlas = Image.new("RGBA",(CW*COLS, CH*rows),(0,0,0,0))
    manifest = {"atlas":"ironroot-atlas.png","cell":[CW,CH],"palette":"ironroot-palette.json","frames":{}}
    for i,(s,p) in enumerate(picks.items()):
        im = Image.open(p).convert("RGBA").resize((CW,CH))
        alpha = im.getchannel("A")
        q = im.convert("RGB").quantize(palette=pmap, dither=Image.NONE).convert("RGBA")
        q.putalpha(alpha)
        cx,cy = (i%COLS)*CW, (i//COLS)*CH
        atlas.paste(q,(cx,cy))
        manifest["frames"][s] = {"x":cx,"y":cy,"w":CW,"h":CH}
    atlas.save(os.path.join(OUT,"ironroot-atlas.png"))
    with open(os.path.join(OUT,"ironroot-atlas.json"),"w") as f: json.dump(manifest,f,indent=1)
    print(f"[atlas] wrote ironroot-atlas.png ({CW*COLS}x{CH*rows}), palette (64), manifest ({len(picks)} frames)")

if __name__=="__main__": main()
