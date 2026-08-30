#!/usr/bin/env python3
"""Vertex AI (Imagen) character gen — sibling to genai_gen.py (Gemini Developer API),
same STYLE anchor + palette-lock, different backend/auth (gcloud ADC, not GEMINI_API_KEY).
Usage: python vertex_gen.py "<prompt>" <out_basename>
"""
import sys, json, os
import vertexai
from vertexai.preview.vision_models import ImageGenerationModel
from PIL import Image

PROJECT = "nde1-493505"
LOCATION = "us-central1"
OUT = r"F:\v3\web\13forge.com\assets"

from genai_gen import STYLE, palette_lock  # reuse the same style anchor + palette-lock

def main():
    prompt = sys.argv[1] if len(sys.argv) > 1 else "character concept"
    base = sys.argv[2] if len(sys.argv) > 2 else "vertex-test"
    vertexai.init(project=PROJECT, location=LOCATION)
    model = ImageGenerationModel.from_pretrained("imagen-3.0-generate-002")
    full = STYLE + "\n\n" + prompt
    print(f"[vertex] project={PROJECT} model=imagen-3.0-generate-002 base={base}")
    resp = model.generate_images(prompt=full, number_of_images=1)
    raw_path = os.path.join(OUT, f"{base}-raw.png")
    resp.images[0].save(location=raw_path, include_generation_parameters=False)
    img = Image.open(raw_path)
    locked = palette_lock(img, os.path.join(OUT, f"{base}.png"))
    print(f"[vertex] wrote {raw_path} ({img.size}) + palette-locked {locked}")

if __name__ == "__main__":
    main()
