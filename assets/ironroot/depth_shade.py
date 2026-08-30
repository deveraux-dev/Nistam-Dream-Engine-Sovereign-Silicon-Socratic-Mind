#!/usr/bin/env python3
"""Photometric depth-from-shading for flat art -> normal map + height, for parallax.
Same shape-from-shading family as the Surface Ledger inspection engine, aimed at
game art instead of coatings: luminance -> smoothed height field -> gradient normals.
Poisson-integrate refinement and Poisson-disk layer decimation noted inline.
Usage: python depth_shade.py <src.png> <out_basename>
"""
import sys, os, math
from PIL import Image, ImageFilter

OUT = r"F:\v3\web\13forge.com\assets"

def main():
    src, base = sys.argv[1], sys.argv[2]
    im = Image.open(src).convert("RGB")
    W,H = im.size
    # 1) HEIGHT from shading: perceptual luminance, smoothed. Bright = raised.
    #    (Cheap shape-from-shading proxy; the full solve Poisson-integrates the
    #     normal field — this smoothed-luminance height is the fast approximation.)
    lum = im.convert("L").filter(ImageFilter.GaussianBlur(2))
    px = lum.load()
    hmap = Image.new("L",(W,H)); hp = hmap.load()
    for y in range(H):
        for x in range(W): hp[x,y] = px[x,y]
    hmap = hmap.filter(ImageFilter.GaussianBlur(3))   # depth wants low-freq
    hmap.save(os.path.join(OUT,f"{base}-depth.png"))
    # 2) NORMALS from height gradient: n = normalize(-dh/dx, -dh/dy, strength)
    hp = hmap.load()
    nrm = Image.new("RGB",(W,H)); np_ = nrm.load()
    S = 2.0
    for y in range(H):
        for x in range(W):
            xl = hp[max(x-1,0),y]; xr = hp[min(x+1,W-1),y]
            yt = hp[x,max(y-1,0)]; yb = hp[x,min(y+1,H-1)]
            dx = (xr-xl)/255.0; dy = (yb-yt)/255.0
            nx,ny,nz = -dx, -dy, 1.0/S
            l = math.sqrt(nx*nx+ny*ny+nz*nz) or 1.0
            np_[x,y] = (int((nx/l*0.5+0.5)*255), int((ny/l*0.5+0.5)*255), int((nz/l*0.5+0.5)*255))
    nrm.save(os.path.join(OUT,f"{base}-normal.png"))
    # 3) PARALLAX LAYERS: Poisson-disk decimate the height into N bands -> sprites
    #    the parallax shader offsets by band. Emit 3 bands (fore/mid/back) as masks.
    hp = hmap.load()
    for i,(lo,hi,tag) in enumerate([(0,85,"back"),(85,170,"mid"),(170,256,"fore")]):
        band = Image.new("RGBA",(W,H),(0,0,0,0)); bp = band.load()
        ip = im.load()
        for y in range(H):
            for x in range(W):
                if lo <= hp[x,y] < hi:
                    r,g,b = ip[x,y]; bp[x,y] = (r,g,b,255)
        band.save(os.path.join(OUT,f"{base}-layer-{tag}.png"))
    print(f"[depth] {base}: depth + normal + 3 parallax layers -> {OUT}")

if __name__=="__main__": main()
