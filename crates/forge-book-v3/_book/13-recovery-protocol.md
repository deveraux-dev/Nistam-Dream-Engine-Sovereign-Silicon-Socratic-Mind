# 13. Recovery Protocol: The Goldminer Engine

This chapter defines the protocol for recovering "lost" capability gold—orphan logic detected in snapshots but absent from the live codebase.

## Overview
The recovery process is governed by the `goldmine.py` engine, a double-oracle system designed for exhaustive deduplication across fragmented storage snapshots.

## The Double-Oracle Approach
1. **Oracle A (Name-Diff):** Identifies symbols present in the quarry/snapshots but absent from the `F:/NewRepo/crates` baseline.
2. **Oracle B (5D Tractor Beam):** Uses the `tractor-beam` skill primitives (`forge_ml::nearest_neighbor` 5D raycasting) to analyze orphans. By firing a 5D ray from an orphan symbol through its subsystem heading over a codebook of live subsystem functions, we distinguish between truly lost logic and renamed twins.

## Dynamic Deduplication Link
The recovery ledger is dynamically linked to the `tractor-beam` skill's 5D raycasting engine. When an orphan is classified as **LOST**, its isolated proximity in 5D balanced trinary space dictates its priority for reconstruction.

For protocol details and the 5D implementation, refer to the [tractor-beam skill](../.agents/skills/tractor-beam/SKILL.md).
