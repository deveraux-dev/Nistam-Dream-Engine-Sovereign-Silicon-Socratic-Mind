#!/usr/bin/env python3
"""Trit-6: the 'less is more' palette. A trit is {-,0,+}; flip it -> 6 poles.
Six anchors, each reusable for six different roles. Posterize (no/low dither)
so art reads as one graphic style, not a dithered AI render.
Usage: python trit6.py <src.png> <out_basename>
"""
import sys, os, json
import numpy as np
from PIL import Image

# 8x8 Bayer ordered-dither matrix — the "rendering glaze". Structured, grid-aligned;
# reads as intentional print/retro style, not random AI-render noise.
BAYER8 = np.array([
 [ 0,32, 8,40, 2,34,10,42],[48,16,56,24,50,18,58,26],
 [12,44, 4,36,14,46, 6,38],[60,28,52,20,62,30,54,22],
 [ 3,35,11,43, 1,33, 9,41],[51,19,59,27,49,17,57,25],
 [15,47, 7,39,13,45, 5,37],[63,31,55,23,61,29,53,21]],dtype=np.float32)/64.0 - 0.5

def bayer_quantize(im, cols, spread=48):
    """Nearest-palette mapping with an 8x8 ordered-dither bias per channel."""
    a = np.asarray(im.convert("RGB"),dtype=np.float32)
    H,W,_ = a.shape
    tile = np.tile(BAYER8,(H//8+1,W//8+1))[:H,:W][...,None]*spread
    a = np.clip(a+tile,0,255)
    pal = np.asarray(cols,dtype=np.float32)                      # (6,3)
    d = ((a[:,:,None,:]-pal[None,None,:,:])**2).sum(-1)          # (H,W,6)
    idx = d.argmin(-1)
    out = pal[idx].astype(np.uint8)
    return Image.fromarray(out,"RGB")

OUT = r"F:\v3\web\13forge.com\assets"
# Astrolabe-verdigris six: an aged brass instrument on a dark ground.
# void/iron = the two dark neutrals (iron catches fog/greys so verdigris only
# appears on true green/cyan); patina = verdigris; brass = aged gold; bone =
# parchment highlight; blood = oxblood accent. Each family reusable for 6 roles.
TRIT6 = {
  "void":   (10, 13, 16),    # ground, night, the absent moon, death, deep, ui-bg
  "iron":   (58, 66, 72),    # neutral cool-dark: armor, fog-shadow, stone, rain, ui-frame
  "patina": (74,142,124),    # verdigris: brass-age, water-life, harmony, growth, secret-glow, ui-accent
  "brass":  (198,150, 74),   # aged gold: fire, toll-light, craft, wealth, sacred, ui-highlight
  "bone":   (224,214,190),   # parchment highlight: light, snow, revelation, bone, ui-fg, edge
  "blood":  (150, 42, 40),   # oxblood: danger, wound, ember-core, combat, discord, alarm
}

def build_pmap(anchors):
    cols = list(anchors.values())
    flat = [v for c in cols for v in c] + [0]*(3*(256-len(cols)))
    p = Image.new("P",(1,1)); p.putpalette(flat); return p, cols

def main():
    src, base = sys.argv[1], sys.argv[2]
    pm, cols = build_pmap(TRIT6)
    # save the 6-swatch + json once
    sw = Image.new("RGB",(6*80,80))
    for i,c in enumerate(cols):
        for x in range(80):
            for y in range(80): sw.putpixel((i*80+x,y), c)
    sw.save(os.path.join(OUT,"trit6-palette.png"))
    json.dump({"name":"trit6","roles":list(TRIT6.keys()),
               "colors":["#%02x%02x%02x"%c for c in cols]},
              open(os.path.join(OUT,"trit6-palette.json"),"w"), indent=1)
    im = Image.open(src).convert("RGB")
    # flat = hard posterize (no dither); bayer8 = the 8x8 ordered glaze
    im.quantize(palette=pm, dither=Image.NONE).convert("RGB").save(os.path.join(OUT,f"{base}-trit6-flat.png"))
    bayer_quantize(im, cols).save(os.path.join(OUT,f"{base}-trit6-bayer8.png"))
    print(f"  flat + bayer8 -> {base}-trit6-*.png")

if __name__=="__main__": main()
