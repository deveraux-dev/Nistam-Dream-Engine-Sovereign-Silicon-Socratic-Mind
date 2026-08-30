# Plan: Align Crate and Build Cleanroom Ports

This plan details the surgical modification of all documentation to "Swissafy" the systems-level engineering precision, remove proprietary technology references, and preserve the unique "Flavor Town" narrative (Cree sovereignty, Edmonton Walterdale Bridge, NACE Level 2 inspector). It also details building new, runnable cleanroom Rust ports of the somatic tokenizer, DSP primitives, Mixture of Musicians (MoM) audio routing, and safety router inside the `forge-envelope` crate.

---

## 1. Objective

1. **Sovereign Alignment:** Standardize on a single-person singular narrative voice ("I", "my", "me") across all documents to highlight independent developer craftsmanship.
2. **Proprietary Tech Clean-up:** Eliminate mentions of "moe-dsp-gpu", "vixitic", and other external proprietary tech.
3. **Swissafy the Docs:** Rewrite the architectural, submission, and test documentation to have razor-sharp, zero-fluff, mathematical, and high-density systems engineering rigor.
4. **Preserve Flavor Town:** Maintain the authentic and cinematic narrative elements (Edmonton Walterdale Bridge, NACE inspecting, Canadian freeze-thaw degradation, Cree spatial concepts, Fredholm-Dante attention, and the Laughter damping kernel).
5. **Cleanroom Rust Ports:** Build four separate, runnable, and thoroughly tested cleanroom modules inside `src/`:
   - `somatic_tokenizer.rs` (tactile bitfield unpacking & kinematics)
   - `cognitive_heal.rs` (f64 DSP primitives: Lowpass/Highpass Biquad, DelayLine, DampedComb, Schroeder Allpass, EnvelopeFollower, Lfo, and a complete Freeverb-style Reverb)
   - `mom.rs` (Mixture of Musicians routing: UmpWord, 49-slot MoeRouter XOR+POPCNT, Musician trait, Conductor automation, and MomBus i24 TPDF dithered fold)
   - `safety_router.rs` (Grammar-guided S13 token safety gate)

---

## 2. Key Files & Action Items

### 2.1 Documentation & Metadata Alignment

| File Path | Strategy | Changes / Action Items |
| :--- | :--- | :--- |
| **`surfaceledger/SUBMISSION_ENTRY.md`** | **Master Merge & Refine** | Merge high-level pitch, origin story, systems architecture, the 3-Gemma ternary MoE, MoM routing, and performance baseline. Use first-person singular ("I"). |
| **`surfaceledger/ARCHITECTURE.md`** | **Swissafy & Dual-Track Integration** | Expand on the dual-flywheel (Physical + Cognitive) design. Formalize Pararity math ($n=2m+k$, $k=1$) and the Photometric Stereo Lambertian model. |
| **`GEMINI.md`** | **Context Synchronization** | Update paths, tables, and principles to align with the new cleanroom ports (`somatic_tokenizer`, `cognitive_heal`, `mom`, `safety_router`). |
| **`README.md`** | **Crate Introduction** | Rewrite to introduce the crate as the zero-retention container underlying the physical-cognitive Surface Ledger. |
| **`docs/SCALE_TESTING.md`** | **Baseline Refinement** | Clean up municipal deployment summaries to focus purely on the systems metrics and active sabotage defense. |
| **`docs/HANDOFF-2026-08-17-GOOGLE-HACKATHON-AVT-GOVERNOR.md`** | **Archive Refinement** | Ensure all plural expressions are converted to first-person singular and proprietary references are cleaned. |

### 2.2 New Cleanroom Rust Ports

I will create four new files in `src/` to house the cleanroom ports of the physical and cognitive engines:

#### 1. `src/somatic_tokenizer.rs`
* **Purpose:** Somatic and photometric tokenization completely offline.
* **Content:**
  * `SomaticKinematics` struct (tick, position, velocity, acceleration, trit state).
  * `EmergentSomaticTokenizer` implementing zero-heap, branchless coordinate unpacking with L2 normalization and $[-15, 15]$ clamping to enforce strict $[-10,000, 10,000]$ Permyriad invariants.

#### 2. `src/cognitive_heal.rs`
* **Purpose:** Pure f64 Faust-free DSP core replacing all GPL-translated elements.
* **Content:**
  * `Biquad` (lowpass, highpass, bandpass, peaking filter via TDF-II RBJ cookbook).
  * `DelayLine` (fractional interp).
  * `DampedComb` (Freeverb LBCF).
  * `Allpass` (Schroeder magnitude-flat diffuser).
  * `EnvelopeFollower` (attack/release envelope detection).
  * `Lfo` (sine and triangle wave generator).
  * `Freeverb` Reverb engine composed purely from 4 parallel `DampedComb` and 2 series `Allpass` filters.

#### 3. `src/mom.rs`
* **Purpose:** Real-time Mixture of Musicians audio routing and lossless mixing.
* **Content:**
  * `UmpWord` wrapper representing 16-byte universal MIDI packet structures.
  * `RoutingTag` triad.
  * `MoeRouter` supporting POPCNT / XOR bit-parallel distance lookups.
  * `Musician` trait.
  * `Conductor` (manages automation weight matrices and active slots).
  * `MomBus` summing bus with high-fidelity, PCG-seeded Triangular Probability Density Function (TPDF) dither, preventing truncation distortion.

#### 4. `src/safety_router.rs`
* **Purpose:** High-security runtime guard.
* **Content:**
  * `SafetyRouter` validating S13 token transitions.
  * Coordinates local 2-expert debate fallbacks and enforces grammar-guided logit boundaries.

#### 5. `src/lib.rs`
* **Purpose:** Expose the new modules cleanly.
* **Content:**
  * Expose `somatic_tokenizer`, `cognitive_heal`, `mom`, and `safety_router` as public modules.
  * Verify all tests run flawlessly.

---

## 3. Verification & Testing Plan

1. **Compilation Check:** Run `cargo check` to verify `#![no_std]` compliance and syntactic correctness.
2. **Unit & Scale Tests:** Run `cargo test` to execute both the crate unit tests and the 10,000-inspector scale stress test. Ensure everything remains completely green.
3. **Audit Proof:** Validate that the final documentation compiles cleanly and displays the beautiful, unified, "Swissafied-with-flavor" engineering specs.
