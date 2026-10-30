# ADR 0074 — The engine's per-turn state belongs to its shared core

- Status: Accepted (2026-10-30), on the owner's instruction to optimise the footprint structurally
  until no optimisation point is left, and a consumer's request (watching-zenoh).
- Amends: ADR 0068 (the main loop's state as numbers in a non-generic `EngineCore`).
- Evidence: watching-zenoh's fixed client image, where each of its two machines carries its own
  `Engine<P>::new_in_place`, `step`, `process_event` and `initialize`, 144 to 160 bytes each; and
  `backends/rust/probes/nostd-footprint`.

## Context

ADR 0068 moved what the main event loop reads and writes into `EngineCore`, so the code that drives
the queues and keeps the counts is one copy for every machine. The bookkeeping that opens and closes
a host call's turn stayed in `Engine<P>`: the clock and the reading latched for the turn
(§scxml-3.13: one host call, one reading), whether `tick` has run, and the count of macrosteps a
machine with delayed sends ran without one. None of it depends on the machine, but fields of
`Engine<P>` sit at offsets that do, so every machine compiled its own copy of the code that reads
them. On the probe, `Engine<P>::step` was 104 bytes: a clock read through the HAL, a saturating
count, a flag test, and the call into the turn.

## Decision

1. `clock`, `turn_now`, `tick_has_run` and `unattended_scheduler_steps` are fields of `EngineCore`.
   `EngineCore::new` starts them at the values `Engine` gave them, so the constant construction of
   ADR 0070 is unchanged.
2. Opening a turn, reading the scheduler's time and noting an unattended macrostep are non-generic
   methods of `EngineCore` (`open_turn`, `sched_now`, `note_unattended_step`), out of line. The
   machine's HAL reading is the one part that is the machine's: it is passed in as a function
   pointer (`Engine::HAL_NOW`).
3. What stays generic is what is the machine's: handing a host-owned clock reading to the sessions
   a machine invoked (`follow_clock_in_children`), and `run_turn`, which reserves the machine-sized
   workspace on its own stack.

## Consequences

- `step`, `initialize`, `tick` and `process_event` are, per machine, a call into the core, the call
  into the turn, and what the policy itself adds.
- The HAL reading is an indirect call where it was inlined; the semantics are the same.
- The footprint probe counts `EngineCore`'s methods as shared core (`CORE_SYMBOLS`).

## Measured (2026-10-30)

`thumbv7em-none-eabihf`, `opt-level = "s"`, the probe's figures
(`backends/rust/probes/nostd-footprint/footprint.budget`), with ADR 0073 landed beside it:

| | one more machine | shared core | worst-case stack |
|---|---|---|---|
| diagnostics on | 1,692 -> 1,516 B | 7,366 -> 7,524 B | 952 -> 952 B |
| diagnostics off | 1,638 -> 1,468 B | 7,258 -> 7,416 B | 952 -> 952 B |
| two machines without `<history>` or `<parallel>` | 1,460 -> 1,290 B | 5,338 -> 5,496 B | 760 -> 760 B |

A machine's `step` fell from 104 bytes to 44 and its `initialize` from 104 to 50. The core pays
158 bytes once where each machine paid about 170, so two machines are 182 bytes smaller from this
decision, and 340 with ADR 0073's, on the plain image (192,346 -> 192,006).
