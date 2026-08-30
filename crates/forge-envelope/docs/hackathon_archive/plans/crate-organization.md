# Plan: Crate-Native RAMUSPRIME Organization

## Objective
To organize all competition assets, documentation, and web releases **directly within the `forge-envelope` repository** under a clean, dedicated folder structure (`/surfaceledger`). This keeps the Rust crate root pristine, organizes all competition submissions, and ensures the repo is 100% submission-ready.

---

## The Target Folder Layout inside `forge-envelope`

We will organize all competition assets under a new directory: `crates/forge-envelope/surfaceledger/`.

```
crates/forge-envelope/
├── Cargo.toml
├── LICENSE-MIT
├── README.md
├── GEMINI.md
├── src/
│   └── lib.rs
└── surfaceledger/              <-- Unified Competition Directory
    ├── index.html              <-- Main Web Release (renamed from surfaceledger_landing.html)
    ├── ARCHITECTURE.md         <-- Clean copy of GEMINI.md for judges
    └── SUBMISSION_ENTRY.md     <-- Complete independent developer entry form answers
```

---

## Action Plan

1.  **Create `surfaceledger/index.html`:** Move/write the polished standalone web landing page directly into the `surfaceledger/` folder.
2.  **Create `surfaceledger/ARCHITECTURE.md`:** Write a clean copy of the S13/VARS architectural context here for easy access by competition judges.
3.  **Create `surfaceledger/SUBMISSION_ENTRY.md`:** Compile and write the complete, finalized independent developer entry form fields (Title, Tagline, Story, Tech, and AI usage details).
4.  **Verification:** Exit plan mode and execute these file writes. (We will also clean up/remove any duplicate loose files from the root of the crate to keep it mathematically beautiful).
