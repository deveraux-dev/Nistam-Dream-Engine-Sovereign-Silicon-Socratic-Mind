# UMP alpha fixes applied

This bundle folds in the verification feedback from the first alpha pass.

## Applied

- Fixed `invalid_status_in_known_mt_errors` test vector from `0x4700_0000` to `0x4070_0000`.
  - `0x4700_0000` encodes group `0x7`, status `0x0`.
  - `0x4070_0000` encodes group `0x0`, status `0x7`.
- Removed duplicate `bytemuck` entry from `[dev-dependencies]` in `crates/forge-ump/Cargo.toml`.
- Added `crates/forge-core` minimal spine/hash surface for standalone testing.
- Added `forge_core::spine::CarrierKind::UmpTicketPack = 10`.
- Added `forge_ump::ticket::UmpAuthorityTicket`:
  - `#[repr(C)]`
  - `bytemuck::Pod + Zeroable`
  - exactly 16 bytes
  - `Lane` integration
  - deterministic hash via `BrutalHashInput`
- Registered `crates/forge-core` in the root workspace.

## Flagged for v0.2

- ProgramChange bank byte positions should be checked against the canonical MIDI 2.0 packet layout.
- ProgramChange bank-valid bit should be decoded instead of assuming bank fields are valid.

## Repo integration note

If your real repo already has `forge-core`, do not add this minimal crate as a duplicate. Merge:

- `crates/forge-core/src/spine.rs` additions into the existing spine module.
- `CarrierKind::UmpTicketPack = 10` into the existing carrier enum.
- `BrutalHashInput` usage in `forge-ump/src/ticket.rs` against the canonical BrutalHash implementation.
