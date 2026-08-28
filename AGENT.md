# AGENT.md — read before editing (temporary, 2026-08-26)

This tree is the **demo/submission cleanroom**, a hand-maintained **copy**.
**Source of truth is `F:\v3`.** Repo physics/laws live at `F:\v3\CLAUDE.md` — this tree has none.

1. Land changes in `F:\v3` first, gate them there, then sync **file-scoped** into here.
2. Nothing syncs automatically. This tree drifts silently.
3. `cargo test --workspace` does **NOT** gate `crates/studio-tauri` or `shell/` — firewalled out
   at `Cargo.toml:40-42`. Build them separately or you will ship a break.
4. `Cargo.toml:49` is a **public GitHub URL**. Anything wrong here ships.
5. A second agent session may be live. As of 2026-08-26 it owns
   `crates/studio-tauri/src/term.rs` and the `ui/` surface — re-check before touching.
6. Absence claims need every root checked, `shell/` included, not just `crates/`.
7. Naming (Sean 2026-08-27): single vendor. Judge/demo-facing text says **the Forge Engine**
   (the daemon on :13013; inference is verb 9, same mouth as ast/cst/lsp/dsl). Never
   "sidecar"/"bridge"/"shim" — those are master-tree dev-loop words and stay in F:\v3.

Full notes: `F:\v3\.forge\handoffs\HANDOFF-2026-08-26-TWO-TREES.md`
Delete this file when the trees unify or a real sync lands.
