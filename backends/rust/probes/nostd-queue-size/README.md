# sce-nostd-queue-size-probe

Downstream MCU footprint report — a compile-time `size_of` gate over a
generated state machine's no_std `Engine`. It now guards three per-machine
footprint levers: the per-machine `<scxml sce:capacity="N">` queue sizing (so
the runtime does not fall back to the crate-global depth-64 default), the
`_event.*` metadata-string elision, and the scheduler-ring `event_data`
elision. Each is a *silent* regression — the reverted form still compiles — so
only a size assertion catches it.

RFC: `claudedocs/rfc-nostd-per-machine-event-queue-sizing.md`.

## Why a size gate (not just a compile gate)

`backends/rust/probes/nostd-build` proves generated machines are *allocator-free*. It cannot
catch a regression where the per-machine `StatePolicy::EventQueue` silently
reverts to the depth-64 default — because the depth-64 bare form **still
compiles**. That reversion is exactly the downstream MCU blocker: each `Engine`'s
two W3C Appendix D queues balloon by `2 × 64 × size_of::<EventWithMetadata<_>>()`
(~205 KiB for this machine), making a per-slot `Engine` pool infeasible on MCU
SRAM. Only a *size* assertion distinguishes the depth-2 machine from a depth-64
reversion.

## How it works

- `machine/` is a crate whose `[lib]` is a generated `--no-std` machine from
  `sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml`, which
  declares `<scxml sce:capacity="2">`. The generated `machine/src/*_sm.rs` is
  git-ignored and produced fresh on each run, so it never drifts stale.
- `src/lib.rs` const-asserts (evaluated at compile time, so **building the crate
  is the gate**) the three per-machine no_std footprint levers, each at its own
  type so the bound is robust against unrelated machine state:
  1. **Queue lever** — `size_of::<StatePolicy::EventQueue>()`: catches a
     `<sce:capacity>` reversion to the depth-64 default.
  2. **Metadata lever** — `size_of::<EventMetadata>()`: catches a `_event.*`
     `SceString` re-added to the queued metadata (1 B under no_std).
  3. **Scheduler lever** — `size_of::<PullScheduler<E>>()`: catches the
     per-entry delayed-send `event_data` string re-added to `ScheduledEntry`
     under no_std.
- Plus a loose whole-`Engine` sanity bound (32 KiB) for gross regressions that
  miss all three precise bounds.

Using the real generated machine (rather than a hand-written policy) keeps the
gate in lock-step with the templates with zero maintenance.

Measured on `thumbv7em-none-eabihf`: `size_of::<Engine<P>>()` is ~9.0 KiB
(down from ~17.5 KiB before the scheduler lever and ~23.7 KiB before the
metadata lever); a depth-64 queue reversion would balloon it past ~222 KiB.

## Run locally

```bash
# 1. Build the codegen binary (release).
cargo build --bin sce-codegen --features cli -p sce-build

# 2. Generate the capacity=2 probe machine (git-ignored output).
target/debug/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/nostd-queue-size/machine/src \
  sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml

# 3. Compile the gate for the MCU target — the const assertion runs here.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/nostd-queue-size/Cargo.toml \
  --target thumbv7em-none-eabihf
```

A clean exit means all three levers are wired through. A failed `const`
assertion (`E0080: evaluation panicked: ... exceeds its bound`) names the
regressed lever: a queue-depth reversion to the depth-64 default, a `_event.*`
metadata string re-added, or the scheduler `event_data` string re-added under
no_std.
