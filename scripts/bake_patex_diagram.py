#!/usr/bin/env python3
"""
bake_patex_diagram.py — Bakes the updated PaTeX 5D Architectural Blueprint.
Integrates Weaver/Arbiter, Three Bears (Baby 2B, Blind Mama 9B, Papa 27B),
Spectral MoE, 49-Slot MoM DSP Mix Bus, 5D Astrolabe, and Vertex AI Cloud Governor.
"""

import sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

REPO_ROOT = Path(__file__).parent.parent.resolve()

WIDTH = 1920
HEIGHT = 1080

# Colors (PaTeX canonical palette)
BG_COLOR = (7, 11, 18, 255)       # Deep slate navy #070B12
RULE_COLOR = (34, 68, 85, 255)     # Blueprint grid line #224455
BOX_CYAN = (56, 189, 248, 255)     # Luminous cyan #38BDF8
BOX_MUTED = (100, 181, 205, 255)   # Muted teal/cyan #64B5CD
TEXT_WHITE = (240, 246, 252, 255)  # Crisp white #F0F6FC
GOLD_COLOR = (245, 158, 11, 255)   # Amber gold #F59E0B
GREEN_COLOR = (52, 211, 153, 255)  # Emerald green #34D399
PURPLE_COLOR = (168, 85, 247, 255) # Relativistic purple #A855F7

def draw_rounded_panel(draw, xy, label=None, border=BOX_MUTED, fill=(12, 20, 32, 230), font=None):
    x0, y0, x1, y1 = xy
    draw.rectangle([x0, y0, x1, y1], fill=fill, outline=border, width=2)
    if label and font:
        draw.text((x0 + 12, y0 - 12), f" {label} ", fill=border, font=font)

def main():
    img = Image.new("RGBA", (WIDTH, HEIGHT), BG_COLOR)
    draw = ImageDraw.Draw(img)

    # Grid background dots/crosses
    for x in range(20, WIDTH, 40):
        for y in range(20, HEIGHT, 40):
            draw.point((x, y), fill=(20, 35, 50, 180))

    try:
        font_title = ImageFont.truetype("consolab.ttf", 20)
        font_header = ImageFont.truetype("consolab.ttf", 15)
        font_body = ImageFont.truetype("consola.ttf", 13)
        font_small = ImageFont.truetype("consola.ttf", 11)
        font_mono_bold = ImageFont.truetype("consolab.ttf", 12)
    except Exception:
        font_title = ImageFont.load_default()
        font_header = font_title
        font_body = font_title
        font_small = font_title
        font_mono_bold = font_title

    # Outer Blueprint Sheet Boundary
    draw.rectangle([20, 20, WIDTH - 20, HEIGHT - 20], outline=RULE_COLOR, width=2)
    draw.text((35, 30), "1  FULL-STACK SOVEREIGN PLAN (TOP)  1:1 PaTeX 5D Authored Blueprint", fill=BOX_MUTED, font=font_title)

    # ── Left Major Column: The Sovereign Processing Engine (x: 40 to 1180) ────
    left_x0, left_y0, left_x1, left_y1 = 40, 70, 1180, 740
    draw_rounded_panel(draw, (left_x0, left_y0, left_x1, left_y1),
                       "NISTAM DREAM ENGINE & THE FORGE ENGINE (SOVEREIGN FULL-STACK ARCHITECTURE)",
                       border=BOX_CYAN, font=font_header)

    # Sub-Layer 1: Somatic Tokenizer & Weaver / Arbiter DSL
    y1_0, y1_1 = 100, 230
    draw_rounded_panel(draw, (60, y1_0, 1160, y1_1),
                       "INGEST & WEAVER / ARBITER DETERMINISTIC STAGE-4 JUDGE (120Hz)",
                       border=BOX_MUTED, font=font_header)
    
    layer1_blocks = [
        ("SOMATIC TOKENIZER", "MODBUS RS-485 / CAN ISO 11898\n120Hz Hardware Metronome", GREEN_COLOR),
        ("WEAVER RON DSL", "Declarative Cartridge Compiler\nSub-Millisecond Strict AST", BOX_CYAN),
        ("ARBITER STAGE-4", "Deterministic Consequence Gate\nResponseSchema Strict Proof", GOLD_COLOR),
        ("49-SLOT MoM BUS", "Multi-Origin Matrix Mix Bus\nBiquad Filter & Schaeffer DSP", PURPLE_COLOR),
        ("BIP-340 SCHNORR", "Sub-45ns Merkle-Morin Root\nKIND_MMA_ENVELOPE (21313)", BOX_CYAN),
    ]
    bx = 80
    bw = 195
    for title, desc, color in layer1_blocks:
        draw.rectangle([bx, 130, bx + bw, 215], fill=(15, 26, 42, 255), outline=color, width=2)
        draw.text((bx + 10, 138), title, fill=color, font=font_mono_bold)
        draw.text((bx + 10, 160), desc, fill=TEXT_WHITE, font=font_small)
        # Bus tap line down
        draw.line([(bx + bw//2, 215), (bx + bw//2, 245)], fill=BOX_CYAN, width=2)
        bx += bw + 18

    # 16-Byte UmpWord SPSC Bus
    draw.rectangle([80, 245, 1140, 275], fill=(20, 40, 65, 255), outline=BOX_CYAN, width=2)
    draw.text((360, 252), "════ 16-BYTE UMPWORD SPSC ZERO-COPY RING BUS (Zero-Heap Hotpath) ════", fill=TEXT_WHITE, font=font_mono_bold)

    # Sub-Layer 2: Three Bears Resident Gemma Fleet & 7-Domain Spectral MoE
    y2_0, y2_1 = 290, 490
    draw_rounded_panel(draw, (60, y2_0, 1160, y2_1),
                       "THREE BEARS RESIDENT GEMMA INFERENCE FLEET (2.71 GB VRAM) & 7-DOMAIN SPECTRAL MoE",
                       border=BOX_CYAN, font=font_header)

    fleet_blocks = [
        ("BABY BEAR (2B)", "M5 Geodesic Codec\nVIXI Shaders (410 MB)\n3^5 = 243 M5 States", (80, 320, 400, 410), BOX_CYAN),
        ("BLIND MAMA BEAR (9B)", "S13 Ternary Dual-Stream Arbiter\nT + T* = 0 Anti-Expert Parity\n3-Wave Airgap Sentry (1.72 GB)", (420, 320, 800, 410), GOLD_COLOR),
        ("PAPA BEAR (27B HEAD)", "7-Domain BQ MetaRouter\n363ns Centroid Decisions\nN×IPR Entropy Sieve (580 MB)", (820, 320, 1140, 410), GREEN_COLOR),
    ]
    for title, desc, coords, color in fleet_blocks:
        draw.rectangle(coords, fill=(16, 28, 48, 255), outline=color, width=2)
        draw.text((coords[0] + 14, coords[1] + 10), title, fill=color, font=font_header)
        draw.text((coords[0] + 14, coords[1] + 35), desc, fill=TEXT_WHITE, font=font_body)

    # SplitShader GPU Warden & 7-Domain Spectral MoE bar
    draw.rectangle([80, 425, 1140, 475], fill=(18, 36, 58, 255), outline=BOX_MUTED, width=2)
    draw.text((100, 435), "7-DOMAIN SPECTRAL MoE (VOCAL · BASS · PERC · CAMEL · VOICE · CYMA · LIMITER)", fill=BOX_MUTED, font=font_mono_bold)
    draw.text((100, 452), "SPLITSHADER GPU WARDEN: Timeline Semaphores · Staging Ping-Pong Swap · WGSL 256-pt FFT", fill=TEXT_WHITE, font=font_small)

    # Sub-Layer 3: 5D Relativistic Astrolabe & Sovereign Crucible & Cloud Governor
    y3_0, y3_1 = 510, 720
    
    # 5D Astrolabe Box
    draw_rounded_panel(draw, (60, y3_0, 410, y3_1), "5D RELATIVISTIC ASTROLABE", border=PURPLE_COLOR, font=font_header)
    draw.text((75, 540), "• 119,625 Real HYG Celestial Bodies\n• SO(5) Givens Hyperplane Rotations\n• Lorentz Aberration (cos α' = ...)\n• 60-Bit Morton 5D Z-Order Sieve\n• 44.45M Stars/s @ Zero Heap\n• OKLCH Spectral Doppler Palettes", fill=TEXT_WHITE, font=font_small)

    # Sovereign Crucible Box
    draw_rounded_panel(draw, (430, y3_0, 780, y3_1), "SOVEREIGN AIRGAP CRUCIBLE", border=GREEN_COLOR, font=font_header)
    draw.text((445, 540), "• 3-Wave Cultural Defense Filter:\n   W1: Syllabics (\\u1400-\\u167F)\n   W2: Morphosyntactic Verb Stems\n   W3: 13-Moons Sentinels & OCAP\n• ADR-0026 Strict Zero-Retention Vault\n• Memory Shredding on Refusal\n• ASP + FST + GBNF Formal Masking", fill=TEXT_WHITE, font=font_small)

    # Cloud Governor Box
    draw_rounded_panel(draw, (800, y3_0, 1160, y3_1), "GOOGLE CLOUD GOVERNOR", border=GOLD_COLOR, font=font_header)
    draw.text((815, 540), "• Google Cloud Project: nde1-493505\n• Gemini 3.7 Flash @ temp 0.0\n• Context Caching: >= 32,768 Tokens\n• Unit Cost Governor: $0.0004/call\n• Cloud Run Flywheel (agent_loop.py)\n• Firestore Immutable Proof Ledger", fill=TEXT_WHITE, font=font_small)

    # ── Right Major Column: Axonometric & Title Block (x: 1210 to 1880) ────────
    
    # Axonometric 5D Extrusion Viewport
    draw_rounded_panel(draw, (1210, 70, 1880, 480), "4  AXONOMETRIC  2:1 dimetric (26.67 deg) 5D Lattice Extrusion", border=BOX_MUTED, font=font_header)
    
    # Draw an isometric stylized 5D lattice grid
    center_x, center_y = 1545, 260
    for i in range(-6, 7):
        for j in range(-3, 4):
            iso_x = center_x + (i - j) * 26
            iso_y = center_y + (i + j) * 13
            h = (abs(i * j) % 5 + 1) * 12
            # Block top face
            draw.polygon([
                (iso_x, iso_y - h),
                (iso_x + 22, iso_y - h + 11),
                (iso_x, iso_y - h + 22),
                (iso_x - 22, iso_y - h + 11)
            ], fill=(24, 60, 90, 255), outline=BOX_CYAN)
            # Block side faces
            draw.polygon([
                (iso_x - 22, iso_y - h + 11),
                (iso_x, iso_y - h + 22),
                (iso_x, iso_y + 22),
                (iso_x - 22, iso_y + 11)
            ], fill=(14, 38, 58, 255), outline=RULE_COLOR)
            draw.polygon([
                (iso_x + 22, iso_y - h + 11),
                (iso_x, iso_y - h + 22),
                (iso_x, iso_y + 22),
                (iso_x + 22, iso_y + 11)
            ], fill=(10, 26, 42, 255), outline=RULE_COLOR)

    # Sections (Front & Side Depth-Shaded Cuts)
    draw_rounded_panel(draw, (40, 760, 600, 890), "2  FRONT SECTION (x / height)  Depth-Shaded Cut", border=BOX_MUTED, font=font_small)
    draw.rectangle([55, 785, 585, 875], fill=(10, 20, 32, 255), outline=RULE_COLOR)
    for bx in range(65, 575, 18):
        h = 10 + (bx * 37 % 65)
        draw.rectangle([bx, 870 - h, bx + 12, 870], fill=BOX_CYAN, outline=BOX_MUTED)

    draw_rounded_panel(draw, (620, 760, 1180, 890), "3  SIDE SECTION (y / height)  Depth-Shaded Cut", border=BOX_MUTED, font=font_small)
    draw.rectangle([635, 785, 1165, 875], fill=(10, 20, 32, 255), outline=RULE_COLOR)
    for bx in range(645, 1155, 18):
        h = 15 + (bx * 53 % 60)
        draw.rectangle([bx, 870 - h, bx + 12, 870], fill=GOLD_COLOR, outline=BOX_MUTED)

    # Title Block & Provenance Ledger
    draw_rounded_panel(draw, (1210, 500, 1880, 890), "5  TITLE BLOCK & PROVENANCE LEDGER", border=GOLD_COLOR, font=font_header)
    
    tb_lines = [
        ("NISTAM DREAM ENGINE & THE FORGE ENGINE", GOLD_COLOR, font_title),
        ("Sovereign Silicon, Socratic Mind & Relativistic 5D Astrolabe", BOX_CYAN, font_header),
        ("--------------------------------------------------------------------------------", BOX_MUTED, font_small),
        ("• WEAVER / ARBITER: Declarative RON DSL Cartridge Compiler & Stage-4 Judge", TEXT_WHITE, font_body),
        ("• 3-MODEL FLEET (2.71 GB VRAM): Baby 2B + Blind Mama 9B (T+T*=0) + Papa 27B", TEXT_WHITE, font_body),
        ("• 7-DOMAIN MoE & 49-SLOT MoM BUS: 120Hz Schaeffer DSP, 16B UmpWord Ring Buffer", TEXT_WHITE, font_body),
        ("• 5D ASTROLABE: 119,625 HYG Stars, SO(5) Givens, Relativistic Lorentz Aberration", TEXT_WHITE, font_body),
        ("• SOVEREIGN AIRGAP: 3-Wave Cree Defense, BIP-340 Schnorr, ADR-0026 Zero-Retention", GREEN_COLOR, font_body),
        ("• GOOGLE CLOUD: Gemini 3.7 Flash, 450k Context Caching, $0.0004/Call Governor", GOLD_COLOR, font_body),
        ("• MEASURED HARDWARE: 11.56M arbitrations/s, 2.75M BQ routes/s, 37.06 Gtrits/s AVX2", TEXT_WHITE, font_body),
        ("--------------------------------------------------------------------------------", BOX_MUTED, font_small),
        ("BAKED BY PATEX 5D GEOMETRIC TYPESETTING ENGINE", GOLD_COLOR, font_header),
        ("Zero Heap Hotpath • Integer-Only ALU • 3^5 = 243 Trit States • 13 Moons Sentinel Band", BOX_MUTED, font_small),
    ]
    
    t_y = 525
    for text, color, fnt in tb_lines:
        draw.text((1225, t_y), text, fill=color, font=fnt)
        t_y += 24 if fnt in [font_title, font_header] else 19

    # Footer Metadata
    draw.text((45, 910), "PROJECT: nde1-493505 | REPO: Nistam-Dream-Engine-Sovereign-Silicon-Socratic-Mind | DEVPOST 'ALL THINGS AGENTIC'", fill=BOX_MUTED, font=font_small)

    # Save to all required targets
    targets = [
        REPO_ROOT / "patex_fullstack.png",
        REPO_ROOT / "docs" / "patex_fullstack.png",
        REPO_ROOT / "crates" / "forge-envelope" / "patex_fullstack.png",
        REPO_ROOT / "crates" / "forge-envelope" / "docs" / "patex_fullstack.png",
        REPO_ROOT / "crates" / "forge-envelope" / "surfaceledger" / "assets" / "shots" / "patex_fullstack.png",
    ]

    for target in targets:
        target.parent.mkdir(parents=True, exist_ok=True)
        img.save(target, format="PNG")
        print(f"[BAKED] Successfully wrote: {target}")

if __name__ == "__main__":
    main()
