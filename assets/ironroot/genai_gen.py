#!/usr/bin/env python3
"""Fire Gemini image gen to match the Ironroot zodiac aesthetic, then palette-lock
the output to the shared 64-colour atlas palette so it stays consistent.
Usage: python genai_gen.py "<prompt>" <out_basename>
"""
import sys, json, os
from google import genai
from google.genai import types
from PIL import Image
import io

OUT = r"F:\v3\web\13forge.com\assets"
STYLE = ("Gothic pixel-art concept art, Blasphemous / Castlevania Symphony of the Night "
         "aesthetic (NOT Dark Souls), ornate suffering, painterly sprite-work, ink-wash "
         "dramatic shadow like Ninja Scroll, ancient decayed-civilization detail like EQ "
         "Kunark ruins, isometric painterly fantasy grime like Pathfinder Kingmaker, "
         "oppressive dungeon dread like Diablo, but with clean readable silhouettes like "
         "Overwatch / Valorant character read. PALETTE RULE (hard constraint): near-void-black "
         "and greyscale monochrome across ~97-98% of the frame; colour is a SPARSE accent only "
         "— no more than 2-3% colour coverage within any 40% region of the image at a time "
         "(thin ember/flame lines, a single glowing eye or rune, one accent hue max). "
         "Never a broad wash of colour. High contrast edges, NOT anime NOT chibi NOT cartoon.")

def palette_lock(img, out_png):
    pal = json.load(open(os.path.join(OUT,"ironroot-palette.json")))
    cols = [tuple(int(h[i:i+2],16) for i in (1,3,5)) for h in pal["colors"]]
    flat = [v for c in cols for v in c] + [0]*(3*(256-len(cols)))
    pmap = Image.new("P",(1,1)); pmap.putpalette(flat)
    q = img.convert("RGB").quantize(palette=pmap, dither=Image.FLOYDSTEINBERG).convert("RGB")
    q.save(out_png)
    return out_png

def main():
    prompt = sys.argv[1] if len(sys.argv)>1 else "key art banner"
    base = sys.argv[2] if len(sys.argv)>2 else "genai-test"
    client = genai.Client(api_key=os.environ["GEMINI_API_KEY"])
    full = STYLE + "\n\n" + prompt
    model = "gemini-2.5-flash-image"
    print(f"[genai] model={model} prompt={prompt[:70]}...")
    try:
        resp = client.models.generate_content(model=model, contents=full)
    except Exception as e:
        print(f"[genai] {model} failed: {e}\n[genai] retrying gemini-2.0-flash-preview-image-generation")
        model = "gemini-2.0-flash-preview-image-generation"
        resp = client.models.generate_content(
            model=model, contents=full,
            config=types.GenerateContentConfig(response_modalities=["IMAGE","TEXT"]))
    got = 0
    for part in resp.candidates[0].content.parts:
        if getattr(part,"inline_data",None) and part.inline_data.data:
            img = Image.open(io.BytesIO(part.inline_data.data))
            raw = os.path.join(OUT, f"{base}-raw.png"); img.save(raw)
            locked = palette_lock(img, os.path.join(OUT, f"{base}.png"))
            print(f"[genai] wrote {raw} ({img.size}) + palette-locked {locked}")
            got += 1
    if not got: print("[genai] NO IMAGE in response — text was:", getattr(resp,"text","(none)")[:200])

if __name__=="__main__": main()
