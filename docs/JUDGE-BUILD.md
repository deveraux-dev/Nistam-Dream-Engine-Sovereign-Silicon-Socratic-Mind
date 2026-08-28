# Judge Build & Verification Instructions

Prerequisites: Rust stable (`rustup`), and on Windows, WebView2 (preinstalled on Windows 10 21H2+ / Windows 11). No Node.js required anywhere in this workspace.

## 1. Fastest path — one-click demo

```cmd
:: Windows
run_demo.bat
```
```bash
# Linux / macOS
./run_demo.sh
```

Runs the 180-second hands-off competition demo end to end. Equivalent direct call: `python scripts/hands_off_demo_driver.py`.

## 2. Build and run the Tauri demo shell

```bash
cargo run --manifest-path crates/studio-tauri/Cargo.toml
```

or, for a release build:

```bash
cd crates/studio-tauri
cargo build --release
```

This is the playable face: 5D free-flight star navigation over the 119,625-star HYG catalog, a birth-rite CYOA arc, an M5 geodesic worldbuilder canvas, and a ConPTY glass terminal.

## 3. Run the test suites

```bash
./test.bat                                                          # 1-click master suite: CPU + GPU + WebGPU + Oracle + Airgap + Vertex AI
cargo test --workspace                                              # Rust workspace tests
```

`cargo test --workspace` does **not** reach `crates/studio-tauri` or `shell/` — those crates are gated separately because they carry native WebView2/WebGPU dependencies the workspace-wide test runner doesn't firewall through. Gate them explicitly:

```bash
cargo test --manifest-path crates/studio-tauri/Cargo.toml
cargo test --manifest-path crates/gemma-s13/Cargo.toml              # S13 ternary + WebGPU compute
```

## 4. Sovereign airgap and cloud governor verification

```bash
python crates/forge-envelope/scripts/test_sovereign_airgap_red_green.py    # 3-wave airgap red/green (5/5 red vectors blocked)
python scripts/test_vertex_cache_strict.py                                 # Vertex AI context-cache token census + strict verification
```

The Vertex AI checks require your own `GOOGLE_CLOUD_PROJECT` and bill at Vertex AI list rates — they are not mocked.

## 5. Live autonomous cloud agent

```powershell
.\scripts\demo_cloud_agent.ps1
```

Invokes `crates/forge-envelope/scripts/agent_loop.py` with `--require-cloud`, which disables every offline fallback. A green run is proof of real Google Cloud traffic (Vertex AI + Firestore + Cloud Storage); a failure is an honest failure rather than a mock dressed up as a result.

## 6. Reproduce the benchmark numbers

See [`docs/BENCHMARKS.md`](BENCHMARKS.md) for the exact command behind every number in `README.md` and `docs/DEVPOST.md`. These are cache-resident CPU microbenchmarks that move with CPU model, thermal state, and compiler version — the only trustworthy figure is the one your own run prints.
