# Post-mortem — 2026-08-19 session (daemon push → pvp_seam → 13link Android port)

## What actually shipped, in order

1. `forge-daemon-door`: real `Subscribe`→`broadcast`/`PushAudit` push channel on `:13013`. Witnessed live against the real daemon and `cargo xtask watch`, twice — once at the CLI, once with a real screenshot of the sovereign shell.
2. `forge-core-v3::pvp_seam`: balanced-ternary PVP/Deathscar/Mercy/Presence trit lattice, built from a design conversation (home-advantage, hockey, trit-both-ways, decay), reusing `atom::Pexil`/`TritCell5D` verbatim instead of inventing a parallel struct. 6/6 tests.
3. `shell/src/main.rs` DIAG-spam fix — found, gated, throttled, real before/after.
4. Mid-session correction: I defaulted to "port the sovereign shell's wgpu/tao renderer to Android" from zero. You corrected it — 13link already existed, dormant, in the v2 donor tree. That correction reshaped the whole rest of the night for the better.
5. Five-wave 13link port into `F:\v3`: `link-wire`, `link-android` (NDK cross-compile proven with a real `.so`), `link-core` (tokio→std::net rewrite, per your explicit call after I raised it as a real fork, not a default), `bridge.rs` wiring the wire protocol to `pvp_seam`, and a real Android app with a real built APK.
6. Standalone daemon runner built and run OUTSIDE `F:\v3` on your explicit instruction (`F:\13link-daemon`), caught and fixed a machine-wide `CARGO_TARGET_DIR` env var that was silently trying to write it back into `F:\v3\target`.
7. Left honestly blocked at: Windows Firewall auto-block on the daemon binary, and no phone visible to `adb` — named, not glossed over, not faked as done.

## Why this one went the way it went

Not talent — mechanics. Specific, repeatable ones, visible in the transcript:

- **A floor before every edit.** Every wave got a `.claude/hooks/.phase0/current.json` arm — one outcome, one proof command, its current failing state — before a single file changed. That's what caught "does this even fail right now" instead of discovering it after the fact.
- **STATE then YIELD, actually enforced on myself.** Each wave ended with a real `cargo test` (or a real Gradle build, or a real `adb`-shaped check), a clear STATIC/RUNTIME split, and a stop — not a narrated "next I'll also..." in the same breath. Five waves stayed five waves instead of collapsing into one unreviewable blob.
- **Recon before build, every time it mattered.** The 13link discovery wasn't luck — it was checking `F:\NewRepo\crates\link-*` file-by-file (not just headers) before writing a single line of the port, and a real dependency-drift check that caught `tokio` wasn't in the workspace before it became a silent architectural fork.
- **Taking correction fast and cheap.** When you said "wait, I built this already" and "did we just port tokio," both got a real check (read the actual donor source; read `vixio-v3`'s actual doc comment) and a real answer, not a defensive rationalization of what I'd already started. The wgpu/tao instinct got dropped in one turn once you named the real prior art, no sunk-cost drag.
- **Asking at forks that actually mattered, not performing consultation.** The Wave-2-keep-or-drop question and the tokio-vs-std::thread question were both real forks with real downstream cost either way — asked once, answered, executed. Not asked about things that didn't need it (Wave 1's shape, the doc-comment fix, the firewall rule wording).
- **Verification that couldn't lie by omission.** The `check_in_over_a_real_tls_wire` test failure wasn't hidden or reasoned around — it turned out to be a bug in the test's read logic, not the code, and I said so once I'd actually traced it, instead of loosening the assertion to make it pass.
- **Scope cut named out loud, not silently dropped.** Notification/clipboard/audio-relay features, the plugin surface, HTTP/voice config — all explicitly cut with a one-line reason each, still visible in the plan file, not quietly vanished.

## Near-misses worth naming

- Almost built a whole `tao`+`wgpu` Android render path before you stopped it — the actual cost if that had gone unchecked for another hour would have been real.
- The `unsafe_code = "deny"` / `missing_docs = "deny"` workspace lints caught real gaps (undocumented fields, the router's lock-free ring) that would've silently been a worse crate if the gate weren't live.
- `CARGO_TARGET_DIR` being globally set to `F:\v3\target` on this machine is a real footgun — it will bite the next thing built "outside v3" the same way unless it's fixed at the environment level, not just worked around per-invocation.

## The transferable part

The refinery analogy is right, but the mechanism under it is specific: the floor-then-yield discipline is what makes "worked vs. didn't" visible turn by turn instead of only at the end. An agent that reports STATIC-green without ever separating it from RUNTIME-verified, or that keeps narrating five more steps past the thing you actually asked for, or that defends its first guess instead of re-checking it when corrected — that's the failure mode, and it's a structural one, not a one-off. If the other systems are wired the same way — a named floor, a real proof command, a real stop, correction taken at face value — the ceiling isn't the model, it's whether that discipline is actually enforced turn to turn instead of being aspirational.
