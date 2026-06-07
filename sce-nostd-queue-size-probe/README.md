# sce-nostd-queue-size-probe

Watching-zenoh MCU footprint report — a compile-time `size_of` gate proving that
a generated state machine's **per-machine** `<scxml sce:capacity="N">` actually
sizes its no_std event queues, instead of the runtime falling back to the
crate-global depth-64 default.

RFC: `claudedocs/rfc-nostd-per-machine-event-queue-sizing.md`.

## Why a size gate (not just a compile gate)

`sce-nostd-build-probe` proves generated machines are *allocator-free*. It cannot
catch a regression where the per-machine `StatePolicy::EventQueue` silently
reverts to the depth-64 default — because the depth-64 bare form **still
compiles**. That reversion is exactly watching-zenoh's blocker: each `Engine`'s
two W3C Appendix D queues balloon by `2 × 64 × size_of::<EventWithMetadata<_>>()`
(~205 KiB for this machine), making a per-slot `Engine` pool infeasible on MCU
SRAM. Only a *size* assertion distinguishes the depth-2 machine from a depth-64
reversion.

## How it works

- `machine/` is a crate whose `[lib]` is a generated `--no-std` machine from
  `sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml`, which
  declares `<scxml sce:capacity="2">`. The generated `machine/src/*_sm.rs` is
  git-ignored and produced fresh on each run, so it never drifts stale.
- `src/lib.rs` const-asserts `size_of::<Engine<EventQueueCapacityProbePolicy>>()
  <= 64 KiB`. The assertion is evaluated at compile time, so **building the crate
  is the gate**. Using the real generated machine (rather than a hand-written
  policy) keeps the gate in lock-step with the templates with zero maintenance.

Measured at the RFC landing: `size_of::<Engine<P>>()` is ~23.7 KiB at depth 2
versus ~222 KiB if reverted to depth 64. The 64 KiB bound sits comfortably
between them.

## Run locally

```bash
# 1. Build the codegen binary (release).
cargo build --bin sce-codegen --features cli --release -p sce-build

# 2. Generate the capacity=2 probe machine (git-ignored output).
target/release/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir sce-nostd-queue-size-probe/machine/src \
  sce-build/tests/fixtures/no_std/event_queue_capacity_probe.scxml

# 3. Compile the gate for the MCU target — the const assertion runs here.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path sce-nostd-queue-size-probe/Cargo.toml \
  --target thumbv7em-none-eabihf
```

A clean exit means the per-machine queue depth is wired through. A failed
`const` assertion (`E0080: evaluation panicked: ... exceeds the per-machine
no_std queue-size bound`) means the capacity regressed to the depth-64 default.
