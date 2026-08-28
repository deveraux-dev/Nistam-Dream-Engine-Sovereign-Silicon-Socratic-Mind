# ADR — forge-vcs-v3 is our version control, it is just not git; and the real gap it leaves open

- **Status:** ACCEPTED (2026-08-18)
- **Date:** 2026-08-18

## Context

This session hit a real bug twice: the compiled `.forge/bin/foreman.exe`
was stale — built before a fix already present in `crates/forge-foreman-v3/
src/hook.rs` — and nothing in the repo caught it. The symptoms were:

1. `pre_edit`'s L25 phase-zero gate deadlocked on the documented arming
   order (`current.json` write blocked while `.loop-active` was armed,
   even though `hook.rs:187-196` already exempts that exact write).
2. `foreman.exe drift` reported all seven hooks `MISSING` for an entire
   session even though `.claude/settings.json` was, and had been the
   whole time, correctly wired.

Both were fixed by nothing more than `cargo build --release -p
forge-foreman-v3 --bin foreman` and redeploying the binary — the source
was already right. Framed loosely in chat as "we don't have version
control." That framing was wrong and Sean corrected it: **we do have
version control — `forge-vcs-v3` — it is simply not git.**

## What forge-vcs-v3 actually is

Per its own crate doc (`crates/forge-vcs-v3/src/lib.rs:1-32`):

> "the VCS flight recorder. Content-addressed, append-only, and behind an
> airgap. There is no git under this and there will not be: lineage is the
> tape, and the tape is the ref."

Concretely: `root::VcsRoot::open` on a tape directory, `commit_bytes` to
record, `restore` to get bytes back by hash. No head-pointer file, no ref
directory — "the head for a path is the last row mentioning it." A hash
firewall isolates the one place in the tree allowed to link `blake3`
(`forge-core-v3` stays zero-dependency and blake3-free by law). This is a
real, deliberate, working design — not a stand-in for git, a different
answer to the same problem: how does a change get a durable, addressable
record.

## The actual gap

`forge-vcs-v3` records history. It has never been asked to answer a
different question: **is the binary I am about to trust the same as the
source that's checked in right now?** Nothing in this repo currently
computes "does `.forge/bin/foreman.exe`'s behavior match
`crates/forge-foreman-v3/src/**` as of this moment" and refuses/flags when
the answer is no. `cargo build`'s own `Finished` message does not mean the
resulting artifact was deployed anywhere a hook reads from — confirmed
directly this session: a `cargo build --release` run reported success with
no `target/release/` output at all on its first attempt (a real, separate,
already-fixed anomaly, not this ADR's subject), and the deployed
`.forge/bin/foreman.exe` sat stale for an unmeasured number of hours with
zero signal that it had drifted from source.

This is exactly the class of problem a content-addressed store like
`forge-vcs-v3` is well-shaped to answer, once asked: hash the source tree
that fed a build, hash (or otherwise identify) the deployed binary, and
compare. Nothing here proposes replacing `forge-vcs-v3` with git — it
proposes pointing `forge-vcs-v3`'s existing hash machinery (`hash::
BrutalHash`, already the one place in the tree allowed to compute a real
content hash) at this specific, currently-unanswered question.

## Decision

Documented, not yet built. No code changes ride with this ADR — it is the
named floor for a future one. The shape a fix would most plausibly take:

- A build step (or `xtask` verb) that computes a `BrutalHash` over
  `crates/forge-foreman-v3/src/**` and embeds it in the compiled
  `foreman.exe` (via `build.rs` + `env!`/`include!`, the standard Rust
  pattern for a build-time constant).
- `foreman.exe drift` (or a new verb) re-hashes the live source tree at
  run time and compares against the embedded value, reporting `STALE
  BINARY: rebuild + redeploy` as its own distinct verdict — not silently
  folded into the existing `PASS`/`FAIL` hook-wiring check, since it's a
  different failure mode with a different fix (rebuild, not re-edit
  `settings.json`).
- This generalizes to any other `.forge/bin/*.exe` the harness trusts, not
  only `foreman.exe` — named here as the pattern, not scoped to one binary.

## Consequences

- Until built, a source fix to any compiled hook can silently sit
  un-deployed indefinitely, exactly as happened tonight — twice, in the
  same session, undetected until manually investigated.
- This ADR's existence is itself a receipt against that recurring: the
  next person (or agent) hitting the same "the hook is doing the wrong
  thing but the source looks right" confusion has a named diagnosis and a
  named fix shape to reach for, instead of re-deriving it from scratch.
