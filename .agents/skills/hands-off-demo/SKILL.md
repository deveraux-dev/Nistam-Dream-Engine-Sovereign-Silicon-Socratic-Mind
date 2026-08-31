---
name: hands-off-demo
description: >-
  Automated, hands-off 180-second live competition demo driver for the Nistam Dream Engine.
  Executes the complete 5-Act showcase with zero human intervention: 5D Astrolabe,
  Blindfolded Cybernetic Autopilot, Resident 3-Model Gemma Fleet (2.71 GB VRAM),
  Vertex AI Gemini 2.5 Flash with 3-Wave Airgap Defense, GPU Singularity Tikhonov
  Self-Healing Test, and 5,213 compiled Rust tests (0 failed, 11 ignored;
receipt: docs/RECEIPT-cargo-test-workspace-2026-08-29.txt).
---

# 🎬 Hands-Off Competition Demo Driver

This skill runs the complete, timed **180-second hands-off competition demo** for Devpost ("All Things Agentic").

---

## 🚀 Instant Launch Command

To execute the entire 180-second demo completely hands-off while recording:

```powershell
python scripts/hands_off_demo_driver.py
```

*(Or via PowerShell wrapper: `powershell -ExecutionPolicy Bypass -File scripts/hands_off_demo_driver.ps1`)*

---

## ⏱️ What the Driver Showcases (5-Act Execution Matrix)

```
 ┌─────────────────────────────────────────────────────────────────────────────┐
 │   ACT I [0:00 - 0:35]: 5D ASTROLABE & RELATIVISTIC SO(5) MANIFOLD           │
 │   • 119,625 Real HYG Stars rendered at 44.45M stars/sec with zero heap      │
 │   • SO(5) Givens Hyperplane Rotations across (Z,W) and (W,V) planes         │
 │   • Relativistic Lorentz aberration & Doppler shift in OKLCH space          │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │   ACT II [0:35 - 1:10]: THE BLINDFOLDED CYBERNETIC AUTOPILOT                │
 │   • Headless background PrintWindow(PW_RENDERFULLCONTENT) capture           │
 │   • 60-Bit Morton 5D Z-order saliency lock in sub-milliseconds              │
 │   • PostMessageW input injection with ZERO OS foreground focus              │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │   ACT III [1:10 - 1:45]: RESIDENT 3-MODEL GEMMA FLEET IN TERMINAL           │
 │   • Baby Bear (2B - 446 MB):   M5 Geodesic Manifold & VIXI Shaders          │
 │   • Blind Mama Bear (9B - 1.72 GB): S13 Dual-Stream Arbiter (500k passes)   │
 │   • Papa Bear (M2 sentry - 765 MB): 7-Domain BQ MetaRouter (363 ns)         │
 │   • Total Resident VRAM: 2,710 MB (Fits in 8 GB consumer GPU)               │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │   ACT IV [1:45 - 2:20]: GOOGLE CLOUD VERTEX AI & GEMINI 2.5 FLASH           │
 │   • Model: gemini-2.5-flash @ temp 0.0 (75% context cache discount)         │
 │   • Context Caching: >= 32,768 tokens VARS knowledge base pre-indexed       │
 │   • 3-Wave Cultural Airgap Sentry: Zero Cree on Cloud (ADR-0026 zeroize)    │
 ├─────────────────────────────────────────────────────────────────────────────┤
 │   ACT V [2:20 - 3:00]: LIVE GPU PANIC TEST & HARDWARE RECEIPTS              │
 │   • Stress Trigger: β -> 0.99999 spikes N x IPR metric to singularity       │
 │   • Self-Healing: Dynamic Tikhonov Clamp (ε = 1e-4) locks 120 FPS           │
 │   • 5,068 Rust tests passed clean (0 failed, 6 ignored)                    │
 └─────────────────────────────────────────────────────────────────────────────┘
```

---

## 📊 Measured Physical Hardware Receipts

- **500,000 Blind Dual-Stream Arbitrations:** 11.56 Million decisions/s (`86.51 ns/eval`)
- **512-bit BQ MetaRouter Centroid Routing:** 2.75 Million decisions/s (`363.40 ns/decision`)
- **AVX2 Conjugate Sign Inversion:** `37.06 Gtrits/s`
- **Host Staging Double-Buffer Memcpy:** `59.62 GB/s`
- **Resident VRAM Footprint:** `2,710 MB` on local NVIDIA RTX 3070
- **Cloud Governor Ceiling:** `$0.0004 / call` on Google Cloud Vertex AI
