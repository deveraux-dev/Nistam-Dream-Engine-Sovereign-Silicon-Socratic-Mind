# The Knob In Front Of You

*One session, 2026-08-04, 03:20..04:56. A model retrained, a daemon accused, and a
ten-times speedup found four hours too late.*

## The ask

Retrain the 512 model on GPU. `nde-models/student-distill-d512.safetensors` —
101,871,879 params, vocab 256, d=512, 7 experts, 3 layers, taught by the `nde-train`
verb in `F:\NewRepo\crates\forge-ml\src\tools\train_nde.rs`.

## The first thing that was wrong

Ten passes were on record in `.forge/run/nde-train-d512-20260803.log`. Every one of them
said `used=50` against `rows=84268`.

The default `--max-tokens 20000` caps a newest-first selector at `train_nde.rs:44`. Fifty
rows at ~395 tokens each fill the budget, and the other 84,219 rows are never looked at.
Ten passes over the same fifty rows. Loss walked 2.9052 to 2.5918 and then stopped, which
is what overfitting fifty samples looks like from the outside.

The checkpoint went to `_vault/_trash/2026-08-04-d512-50row/`. The log stayed, because the
log is the proof.

## The knob

`grad_accum_steps` is the optimizer window, and on the resident lane it is also the GPU
submit window — `train_epoch` submits once per window, so the default 4 meant a device
sync every four tokens. Measured at d=512 over 19,764 tokens:

```
GA=4    4941 submits   67.5s
GA=32    618 submits   17.9s
GA=128   155 submits   16.7s
```

Two probes solve for the submit cost: 463 submits separate the last two, 1.2 seconds
apart, so ~2.6ms each. Back it out of both and each lands at 16.3s of non-submit work —
consistent, so that is fixed overhead plus compute. GA=32 is the knee. 128 buys seven
percent more for four times fewer optimizer steps.

A `--grad-accum` flag landed. So did `--checkpoint-every`, defaulting to quarters, because
a four-hour pass that saves once at the end loses everything to a crash at hour three.
The first quarter probe caught its own bug: integer division left a four-token sliver as
its own chunk, and that sliver's loss — 3.2898, measured over four tokens — was what the
run reported as final, against a real 2.8567. The last chunk absorbs the remainder now.

## The accusation

Built exes kept vanishing from `target/lane`. A launch died on one. A rebuild that should
have been incremental took 5m14s.

The suspect was `bin_deploy.rs:944`, which `remove_dir_all`s every profile directory except
the one named in the deploy stamp — and the stamp said `dev`, so `lane` and `release` were
both unprotected, with `target/` permanently over its cap.

It was not the sweeper. `door_lifecycle.rs:55-79`: the door watches `forge-daemon/src`,
`forge-studio/src` and `forge-book/src`, and when any `.rs` is newer than the deployed exe
it shells `cargo build -p forge-studio --profile lane` on a 900-second cooldown — into
`target/lane/13forge-studio.exe`, the same path being built and launched from. Nothing was
deleted. A launch had hit the window between cargo unlinking the old image and finishing
the new one, and the slow rebuild was two cargos invalidating each other's fingerprints.

Proof by bytes, not by story: the file was 110,203,392 bytes at 04:04:09, sha
`06D21497…4A73`. The build in question was 110,196,736 bytes at 03:30:20. Different size,
later mtime, no build run at that minute.

## The thing worth keeping

A running image on Windows cannot be deleted or overwritten. It can always be renamed, and
the live process keeps its handle either way.

So a four-hour job never has to die to unblock a build. `door bounce` parked the held exe
as `13forge-studio.pinned-by-ndetrain-10256.exe`, relinked underneath it, staged the new
door, and the trainer never noticed. `park_aside` at `bin_deploy.rs:328` had implemented
this all along and nobody had written down what it was for.

## The miss

Then someone asked: one token at a time?

`resident_moe.rs:334` loops `for i in 0..tokens.len()-1` and records one token pair per
forward and backward. `--grad-accum` batches the optimizer step, not the forward. So every
matmul is `[out,in] @ [in,1]` — a matvec, exactly as `moe_train.rs:75` names it.

A d=512 matvec reads 1 MB of weights to do 262k FLOPs. That is 0.25 FLOP per byte on a
card that needs roughly 50 to saturate — memory-bound by two hundred times. Which is why
the card read 94% utilisation while drawing 151W of a 240W limit, at 64°C, with no
throttle reason set. Utilisation counts intervals with a kernel resident. It does not
count whether the kernel was worth launching.

Batching the forward amortizes one weight read across B tokens. Somewhere between five and
fifteen times, and the four-hour pass becomes twenty-five minutes.

The run was killed at 04:52 with no checkpoint written.

## What it cost and what it bought

The failure was not the matvec. The failure was tuning the knob in front of the agent —
submits, 3.8x, measured and reported with confidence — while never once asking what the
device could do. Every number needed to catch it was in hand twelve hours earlier: the
kernel shape, the submit count, the watts.

```
<law id="roofline" pri="TOP" scope="any-agent">
LONG_JOB(>10min-wall)=GAUGE_DEVICE_CEILING_B4_SPEND;
UTIL%!=SATURATION;>=2x_LEVER->HALT+NAME_IT_B4_LAUNCH;
MEET=internal-facts(agent:shapes/counts/watts)+external(Sean:what-its-for)
</law>
```

A law is read. A gate is enforced. `CEILING=` joined `[SEAN-OK]`, `READ-ONLY` and
`WIRE_B4_NEW` as a literal the machine greps for: a plan past three steps that says
*train*, *epoch*, *overnight*, *full queue*, *full pass* or *hours* is denied unless its
body carries the achieved rate against the device roof, the kernel shape that sets it, and
the largest lever not being taken. `gate.rs:1076` and `:1163`, folded into the existing
`plan-scope` gate rather than a new `GateKind`, because the enum does not get to grow.
Eleven tests green, including the day's own miss as `a_long_job_owes_its_ceiling`.

It catches plans. The four-hour run came from a direct instruction, not `ExitPlanMode`, so
this gate would not have stopped it. That gap is known and unclosed.

---

Two agents worked this repo at once, lanes split by crate, the forge-vcs tape as the
channel between them. One of them wrote this. Utilisation is not saturation.
