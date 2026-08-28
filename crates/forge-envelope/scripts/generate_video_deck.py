#!/usr/bin/env python3
"""
scripts/generate_video_deck.py
Surface Ledger & SplitShader GPU Warden — Dual-Track & Triad Video Deck Generator

Supports:
1. 80-Second Dual-Track Kishōtenketsu Reel (20 FPS / 60 BPM ReelClock)
2. 3-Minute (180-Second) Lockstep Triad Matrix Competition Walkthrough Deck (20 FPS / 60 BPM ReelClock)

Adheres strictly to The Drop Law (20 FPS / 60 BPM, 13/100/500ms floors, 200-500ms blink gate),
Lockstep Triad Matrix (Video A x Video B x 3rd Narrative VO), and Google Gemini Developer
Competition judging criteria.
"""

import os
import sys
import json
from pathlib import Path
from typing import List, Optional
from pydantic import BaseModel, Field

try:
    from google import genai
    from google.genai import types
except ImportError:
    genai = None

class VideoScene(BaseModel):
    scene_id: int = Field(..., description="1-indexed sequence number")
    act_name: str = Field(..., description="Chapter / Act name")
    start_time_seconds: float = Field(..., description="Start timestamp in seconds")
    end_time_seconds: float = Field(..., description="End timestamp in seconds")
    start_frame_20fps: int = Field(..., description="Start frame at 20 FPS (ReelClock)")
    end_frame_20fps: int = Field(..., description="End frame at 20 FPS (ReelClock)")
    accent_color_hex: str = Field(..., description="Hex color code (#1AE0FF, #FFD23F, #FF3B6E, #4DFFB0)")
    atom_type: str = Field(..., description="biome_transition, branded_manifestation, live_demo, or cutscene_atom")
    narrative_role: str = Field(..., description="establish, initial, key, dialogue, or resolve")
    video_a_executive: str = Field(..., description="Video A: Left Brain / Executive (GCP, BLEVE, Audits) -> Left Channel / Right Eye")
    video_b_architect: str = Field(..., description="Video B: Right Brain / Architect (#![no_std] Rust, 2.75 Mtok/s single-core LUT) -> Right Channel / Left Eye")
    narrative_vo_center: str = Field(..., description="Video C: Sean Morin Spoken Voiceover (3rd Narrative) -> Center Channel over 1200s Organum Drone")
    audio_phase_driver: str = Field(default="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)", description="Audio phase driver")
    on_screen_visual_prompt: str = Field(..., description="Detailed visual direction for video cutscene & UI overlays")
    live_terminal_command: Optional[str] = Field(None, description="Exact shell/cargo command run on-screen")

class VideoDeck(BaseModel):
    title: str = Field(..., description="Deck title")
    total_duration_seconds: float = Field(default=180.0, description="Total runtime in seconds")
    fps: int = Field(default=20, description="Target frame rate (20.0 FPS / 60 BPM ReelClock)")
    bpm: int = Field(default=60, description="Clock tempo in BPM (1s = 1 quarter_note = 20 frames)")
    time_base: str = Field(default="1s = 1 quarter_note = 20 frames", description="ReelClock time base")
    panning_profile: str = Field(default="Video A: 60% Left, Video B: 60% Right, Narrative VO: Center Frequency over 1200s Organum Drone", description="Audio pan specs")
    master_script_vo: str = Field(..., description="Verbatim master voiceover script by Sean Morin")
    acts_summary: List[str] = Field(..., description="Breakdown of chapters or acts")
    scenes: List[VideoScene] = Field(..., description="Line-by-line sequence of video scenes")

MASTER_VO_VERBATIM = (
    "I dropped out in grade 9 because the system wasn't built for a brain like mine. "
    "Spent 23 years putting material on steel across Edmonton, Fort Mac, Suncor, and the Walterdale Bridge—"
    "NACE Level 2, lead removal, sandblasting, thermal spray. Four years waiting for a government letter "
    "to confirm what I already knew: Cree dad, white mom, accepted by neither side. Walk downtown Edmonton "
    "past the Ice District casino. Half a block away, people are suffering at The Spady and the Mustard Seed. "
    "Tech builds $3,000 tools—Ableton, Pioneer, PS5—for people with endless runway, ignoring those who have "
    "had everything taken away and will never recover under the current rules. Diagnosed with AuDHD at 39. "
    "My brain can't do long division on paper, just like a computer processor can't do floating-point math "
    "without drifting. So I built a fixed-point engine. I took a consequence demo to Alberta Innovates, "
    "and the Professional Engineer looked at me and said: 'There is nothing I can do to help you. "
    "There is no one I can refer you to.' So I sat in the dark for 8 months with no runway, no CS degree, "
    "and no support, and I forged 1 million lines of #![no_std] Rust. 14 scattered domains collapsed into one truth. "
    "millions of tokens per second on a single core, backed by Gemini on the cloud. This is software for the people left behind. "
    "For those who didn't get to learn Cree. Human error, not computational rounding error. "
    "Accountability is self-attestation, not surveillance. No ending is silent."
)

CANONICAL_DECK_80S = VideoDeck(
    title="Surface Ledger — 80-Second Dual-Track Drop Law Video Deck",
    total_duration_seconds=80.0,
    fps=20,
    bpm=60,
    time_base="1s = 1 quarter_note = 20 frames",
    panning_profile="Video A (Executive): 60% Left | Video B (Architect): 60% Right | Narrative VO: Center",
    master_script_vo=MASTER_VO_VERBATIM,
    acts_summary=[
        "Act 1 (Ki-Shō): 0.0s - 48.0s (Development / Baseline) [#1AE0FF Cyan]",
        "Act 2 (Ten): 48.0s - 68.0s (Sentinel Breach / The Turn) [#FF3B6E Pink]",
        "Act 3 (Ketsu): 68.0s - 80.0s (Resolution / EvidenceChain Lock) [#4DFFB0 Green]"
    ],
    scenes=[
        VideoScene(
            scene_id=1,
            act_name="Act 1 (Ki-Shō): Development",
            start_time_seconds=0.0,
            end_time_seconds=16.0,
            start_frame_20fps=0,
            end_frame_20fps=320,
            accent_color_hex="#1AE0FF",
            atom_type="biome_transition",
            narrative_role="establish (Role: E)",
            video_a_executive="Infrastructure audits bleed billions in raw visual photo storage and unverified reporting.",
            video_b_architect="Physical structures carry an inevitable creep of unseen material decay.",
            narrative_vo_center="Grade 9 dropout at 39. 23 years on steel across Edmonton and Fort Mac. Built engine that doesn't need long division.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Camera sway over structural grid, blueprint schematics overlaid on photometric stereo normal maps. Billing meter ticks upward against raw photo storage cost.",
            live_terminal_command=None
        ),
        VideoScene(
            scene_id=2,
            act_name="Act 1 (Ki-Shō): Development",
            start_time_seconds=16.0,
            end_time_seconds=32.0,
            start_frame_20fps=320,
            end_frame_20fps=640,
            accent_color_hex="#1AE0FF",
            atom_type="biome_transition",
            narrative_role="initial (Role: I)",
            video_a_executive="Surface Ledger deploys a sub-millisecond edge sentry that pre-filters streams locally.",
            video_b_architect="We listen to the subtle vibrations of steel and concrete resting on physical ground.",
            narrative_vo_center="Walk downtown Edmonton past the casino while people freeze half a block away. Inaccessible tech ignores those left behind.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Close-up of edge device sensor on cold steel. 60 BPM metronome waveform pulses across raw register bitfields with zero heap allocation callout.",
            live_terminal_command="cargo run -p forge-envelope --example s13_encode"
        ),
        VideoScene(
            scene_id=3,
            act_name="Act 1 (Ki-Shō): Development",
            start_time_seconds=32.0,
            end_time_seconds=48.0,
            start_frame_20fps=640,
            end_frame_20fps=960,
            accent_color_hex="#1AE0FF",
            atom_type="biome_transition",
            narrative_role="key (Role: I)",
            video_a_executive="Raw photos stay on-device; 25MB photo collapses into a 16-byte UmpWord vector (1,562,500x compression).",
            video_b_architect="Balanced ternary trits compress raw telemetry down to pure mathematical state in 12 microseconds.",
            narrative_vo_center="My brain can't do long division, computer processors drift on floats. So I built a fixed-point engine.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="25MB physical photograph collapses into a 16-byte UmpWord vector (1,562,500x compression badge). Sieve-13 ternary tensor displays [0, +1, -1, 0...].",
            live_terminal_command="cargo test -p forge-envelope --lib s13"
        ),
        VideoScene(
            scene_id=4,
            act_name="Act 2 (Ten): The Turn",
            start_time_seconds=48.0,
            end_time_seconds=58.0,
            start_frame_20fps=960,
            end_frame_20fps=1160,
            accent_color_hex="#FF3B6E",
            atom_type="branded_manifestation",
            narrative_role="establish (Role: E)",
            video_a_executive="By caching the 450,000-token inspection handbook in Vertex AI...",
            video_b_architect="When extreme freeze-thaw cycles strike sub-arctic steel...",
            narrative_vo_center="Engineer said: 'Nothing I can do to help you.' Sat in dark 8 months with AuDHD, forged 1M lines solo.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="High-stakes sentinel breach, deep red/pink warning pulses, out-of-band hardware sentinel trigger. Vertex AI CachedContent badge glows with 450,000 tokens locked.",
            live_terminal_command="python scripts/gemini_context_cache.py"
        ),
        VideoScene(
            scene_id=5,
            act_name="Act 2 (Ten): The Turn",
            start_time_seconds=58.0,
            end_time_seconds=68.0,
            start_frame_20fps=1160,
            end_frame_20fps=1360,
            accent_color_hex="#FF3B6E",
            atom_type="branded_manifestation",
            narrative_role="key (Role: P)",
            video_a_executive="...we slash input costs by 75%, funding 60 Million Gemini audits under budget at $0.0004 per query.",
            video_b_architect="...the sentinel halts the stream, compiling a 16-byte UmpWord in 35 nanoseconds with 0 heap bytes.",
            narrative_vo_center="millions of tokens per second on a single core, backed by Gemini on the cloud.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Live terminal execution: chaos_monkey halts stream in 35ns. Split-screen shows Google Cloud Billing Console confirming 75% CachedContent read discount and $0.0004 spend.",
            live_terminal_command="cargo run -p forge-envelope --bin chaos_monkey"
        ),
        VideoScene(
            scene_id=6,
            act_name="Act 3 (Ketsu): Resolution",
            start_time_seconds=68.0,
            end_time_seconds=74.0,
            start_frame_20fps=1360,
            end_frame_20fps=1480,
            accent_color_hex="#4DFFB0",
            atom_type="cutscene_atom",
            narrative_role="key (Role: R)",
            video_a_executive="Sub-second multimodal reasoning paired with total enterprise cost efficiency.",
            video_b_architect="Every state resolution folds directly into an immutable SHA-256 rolling chain.",
            narrative_vo_center="14 scattered domains collapsed into one truth. For those who didn't get to learn Cree.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Settled bridge pier or concrete foundation with rolling SHA-256 seal, zeroized memory buffers, and verified ledger link.",
            live_terminal_command="cargo run -p forge-envelope --bin attest"
        ),
        VideoScene(
            scene_id=7,
            act_name="Act 3 (Ketsu): Resolution",
            start_time_seconds=74.0,
            end_time_seconds=80.0,
            start_frame_20fps=1480,
            end_frame_20fps=1600,
            accent_color_hex="#4DFFB0",
            atom_type="cutscene_atom",
            narrative_role="dialogue (Role: R)",
            video_a_executive="Verified, non-repudiable infrastructure trust powered by Google Gemini.",
            video_b_architect="Bit-perfect, non-repudiable proof-carrying architecture on physical ground.",
            narrative_vo_center="Accountability is self-attestation, not surveillance. No ending is silent.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Hero title lock: 'Surface Ledger & Forge-Envelope'. Deterministic edge-metal engineering and Google Cloud Vertex AI badge unite. Motto: 'No ending is silent. Every erasure is witnessed.'",
            live_terminal_command=None
        )
    ]
)

CANONICAL_DECK_3MIN = VideoDeck(
    title="The Uneraseable Truth — 3-Minute Lockstep Triad Walkthrough Deck",
    total_duration_seconds=180.0,
    fps=20,
    bpm=60,
    time_base="1s = 1 quarter_note = 20 frames",
    panning_profile="Video A (Left): 60% L | Video B (Right): 60% R | Narrative VO (Sean): Center over 1200s Organum Drone",
    master_script_vo=MASTER_VO_VERBATIM,
    acts_summary=[
        "Chapter 1: The 23-Year Reality & Sovereign Parity (0:00 - 0:45) [#1AE0FF Cyan]",
        "Chapter 2: SplitShader GPU Warden & Mtok/s Hardware Receipts (0:45 - 1:30) [#FFD23F Amber]",
        "Chapter 3: Google Cloud Vertex AI Structured Audits & Context Caching (1:30 - 2:15) [#FF3B6E Pink]",
        "Chapter 4: Non-Repudiable EvidenceChain & Webfacing Shaderbind Live Reactive UI (2:15 - 3:00) [#4DFFB0 Green]"
    ],
    scenes=[
        VideoScene(
            scene_id=1,
            act_name="Chapter 1: The 23-Year Reality & Sovereign Parity",
            start_time_seconds=0.0,
            end_time_seconds=22.5,
            start_frame_20fps=0,
            end_frame_20fps=450,
            accent_color_hex="#1AE0FF",
            atom_type="biome_transition",
            narrative_role="establish (Role: E)",
            video_a_executive="NACE Level 2/3 defect inspection, bridge corrosion photos, GCP project 'nde1-493505'.",
            video_b_architect="Terminal running mtok_throughput_bench.rs (1.17ns L1 cache throughput) over 1200s Organum drone.",
            narrative_vo_center="I dropped out in grade 9 because the system wasn't built for a brain like mine. Spent 23 years putting material on steel across Edmonton, Fort Mac, Suncor, and the Walterdale Bridge—NACE Level 2, lead removal, sandblasting, thermal spray.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Direct footage of Sean Morin holding physical coating gauge against structural steel. Transition to high-resolution 25MB Walterdale Bridge photograph showing localized micro-blistering and GCP project nde1-493505 overlay.",
            live_terminal_command=None
        ),
        VideoScene(
            scene_id=2,
            act_name="Chapter 1: The 23-Year Reality & Sovereign Parity",
            start_time_seconds=22.5,
            end_time_seconds=45.0,
            start_frame_20fps=450,
            end_frame_20fps=900,
            accent_color_hex="#1AE0FF",
            atom_type="biome_transition",
            narrative_role="initial (Role: I)",
            video_a_executive="1,562,500x visual compression diagram; 25MB photo collapsed to 16-byte trigger.",
            video_b_architect="Offline Photometric Stereo solver recovers 3D surface normal vectors (N) and mean curvature (H) with sub-millimeter precision.",
            narrative_vo_center="Four years waiting for a government letter to confirm what I already knew: Cree dad, white mom, accepted by neither side. My brain can't do long division on paper, just like a computer processor can't do floating-point math without drifting. So I built a fixed-point engine.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Screen split: Left side shows 3D photometric stereo normal map with vector field overlay. Right side highlights 16-byte UmpWord bitfield and 13-lane S13 ternary vector [0, +1, -1...].",
            live_terminal_command="cargo test -p forge-envelope --lib s13"
        ),
        VideoScene(
            scene_id=3,
            act_name="Chapter 2: SplitShader GPU Warden & Mtok/s Hardware Receipts",
            start_time_seconds=45.0,
            end_time_seconds=67.5,
            start_frame_20fps=900,
            end_frame_20fps=1350,
            accent_color_hex="#FFD23F",
            atom_type="live_demo",
            narrative_role="key (Role: P)",
            video_a_executive="1,562,500x visual compression diagram; 25MB photo collapsed to 16-byte trigger.",
            video_b_architect="WGSL 64/32 split-shader dual u32 register emulation (Int64) driving udle_vibematrix.shaderbind.",
            narrative_vo_center="Walk downtown Edmonton past the Ice District casino. Half a block away, people are suffering at The Spady and the Mustard Seed. Tech builds $3,000 tools—Ableton, Pioneer, PS5—for people with endless runway, ignoring those who have had everything taken away and will never recover under the current rules.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Live terminal execution of mtok_throughput_bench. Real-time console displays 2.75 Mtok/s LUT direct rate (single-core receipt), 57.48 GB/s staging bandwidth, and measured dispatch plans/sec on Ampere 32x32 tiles.",
            live_terminal_command="cargo run -p forge-gpu-warden-v3 --example mtok_throughput_bench --release"
        ),
        VideoScene(
            scene_id=4,
            act_name="Chapter 2: SplitShader GPU Warden & Mtok/s Hardware Receipts",
            start_time_seconds=67.5,
            end_time_seconds=90.0,
            start_frame_20fps=1350,
            end_frame_20fps=1800,
            accent_color_hex="#FFD23F",
            atom_type="live_demo",
            narrative_role="dialogue (Role: P)",
            video_a_executive="All code compiles under strict #![deny(unsafe_code)] with 100% green test receipts across the workspace.",
            video_b_architect="Monotonic TimelineSemaphores track DMA transfers with zero retrograde drift, while 2x64KB ping-pong VRAM staging hotswaps in 17.8 nanoseconds.",
            narrative_vo_center="Diagnosed with AuDHD at 39. My brain can't do long division on paper, just like a computer processor can't do floating-point math without drifting. So I built a fixed-point engine.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Split-screen showcasing cargo test passing across forge-gpu-warden-v3 (14/14), forge-ml-bqrouter (73/73), forge-envelope (53/53), and forge-audio-v3.",
            live_terminal_command="cargo test -p forge-gpu-warden-v3 -p forge-ml-bqrouter -p forge-envelope"
        ),
        VideoScene(
            scene_id=5,
            act_name="Chapter 3: Google Cloud Vertex AI Structured Audits & Context Caching",
            start_time_seconds=90.0,
            end_time_seconds=112.5,
            start_frame_20fps=1800,
            end_frame_20fps=2250,
            accent_color_hex="#FF3B6E",
            atom_type="live_demo",
            narrative_role="establish (Role: E)",
            video_a_executive="Vertex AI Context Caching: $0.0004 USD query cost, 60M audits funded under $1,200.",
            video_b_architect="chaos_monkey.rs Gate D firing Kaskatinowipisim Freeze-Up Sentinel (252) into MomRouter.",
            narrative_vo_center="I took a consequence demo to Alberta Innovates, and the Professional Engineer looked at me and said: 'There is nothing I can do to help you. There is no one I can refer you to.' So I sat in the dark for 8 months with no runway, no CS degree, and no support, and I forged 1 million lines of #![no_std] Rust.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Live terminal execution of vertex_schema_client.py querying Gemini 2.5/3.7 Flash. JSON response streams in containing NACE compliance level 3, S13 vector, and rolling SHA-256 link hash proof.",
            live_terminal_command="python crates/forge-envelope/scripts/verify_billing_draw.py --model gemini-2.5-flash --queries 1 --no-confirm"
        ),
        VideoScene(
            scene_id=6,
            act_name="Chapter 3: Google Cloud Vertex AI Structured Audits & Context Caching",
            start_time_seconds=112.5,
            end_time_seconds=135.0,
            start_frame_20fps=2250,
            end_frame_20fps=2700,
            accent_color_hex="#FF3B6E",
            atom_type="branded_manifestation",
            narrative_role="key (Role: P)",
            video_a_executive="Vertex AI Context Caching locks 450,000 tokens for sub-arctic coating inspection handbooks with 75% read discount.",
            video_b_architect="Chaos Monkey test: Moon Sentinel 252 halts the stream, triggering 256-bit SIMD zeroization in 35 nanoseconds.",
            narrative_vo_center="14 scattered domains collapsed into one truth. Millions of tokens per second on a single core, backed by Gemini on the cloud.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Google Cloud Vertex AI Console displaying active CachedContent object (450,000 tokens locked). Switch to verify_billing_draw.py showing real-time token ledger calculation and $0.0004 cost receipt.",
            live_terminal_command="cargo run -p forge-envelope --bin chaos_monkey"
        ),
        VideoScene(
            scene_id=7,
            act_name="Chapter 4: Non-Repudiable EvidenceChain & Webfacing Shaderbind Live Reactive UI",
            start_time_seconds=135.0,
            end_time_seconds=157.5,
            start_frame_20fps=2700,
            end_frame_20fps=3150,
            accent_color_hex="#4DFFB0",
            atom_type="live_demo",
            narrative_role="key (Role: R)",
            video_a_executive="Immutable Firestore/D1 ledger updating with SHA-256 state hashes (attest.rs).",
            video_b_architect="Single-core 2.75 Mtok/s LUT execution, 0 heap bytes, rendering 13forge.com.",
            narrative_vo_center="This is software for the people left behind. For those who didn't get to learn Cree. Human error, not computational rounding error.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Demonstration of the webfacing shaderbind dashboard (shaderbind_vertex_live.html). Triggering an inspection audit live in the UI causes the shader vibe channels to pulse in real time to the Vertex AI response.",
            live_terminal_command="cargo run -p forge-envelope --bin attest -- --in scratch/audit_receipt_to_attest.json --chain-state .forge/evidence_chain.json"
        ),
        VideoScene(
            scene_id=8,
            act_name="Chapter 4: Non-Repudiable EvidenceChain & Webfacing Shaderbind Live Reactive UI",
            start_time_seconds=157.5,
            end_time_seconds=180.0,
            start_frame_20fps=3150,
            end_frame_20fps=3600,
            accent_color_hex="#4DFFB0",
            atom_type="cutscene_atom",
            narrative_role="resolve (Role: R)",
            video_a_executive="Surface Ledger: Sovereign edge systems engineering meets the world's most advanced AI platform.",
            video_b_architect="Built by an independent Cree craftsman in Edmonton's river valley. Bit-perfect, non-repudiable, proof-carrying architecture.",
            narrative_vo_center="Accountability is self-attestation, not surveillance. No ending is silent.",
            audio_phase_driver="udle_vibematrix.shaderbind (Ch0: rms, Ch1: beat_phase controls Video B opacity over A)",
            on_screen_visual_prompt="Hero closing title lock: Surface Ledger & Forge-Envelope, Zenodo DOI badge (10.5281/zenodo.22020676), Google Gemini Competition badge, and GitHub / Crates.io links. Tagline: 'No ending is silent. Every erasure is witnessed.'",
            live_terminal_command=None
        )
    ]
)

REPO_ROOT = Path(__file__).parent.parent.resolve()

def generate_or_export(export_path: Path, deck: VideoDeck):
    """Saves a canonical video deck to disk in JSON format."""
    export_path.parent.mkdir(parents=True, exist_ok=True)
    with open(export_path, "w", encoding="utf-8") as f:
        json.dump(deck.model_dump(), f, indent=2)
    print(f"[OK] Video Deck '{deck.title}' written to: {export_path}")

def main():
    out_dir = REPO_ROOT / "surfaceledger"
    generate_or_export(out_dir / "video_deck_80s.json", CANONICAL_DECK_80S)
    generate_or_export(out_dir / "video_deck_3min.json", CANONICAL_DECK_3MIN)

if __name__ == "__main__":
    main()
