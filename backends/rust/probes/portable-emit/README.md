# sce-portable-emit-probe

SCE Protocol-Synthesis RFC §5.J.2 — a **single** `--no-std` generated machine
that must compile against **both** runtime profiles:

- the **std** profile (the AP profile, host target), and
- the bare-metal profile, the runtime without its `std` feature (the MCU
  profile, thumb target).

This crate is the machine and nothing else: its manifest asks the runtime for
no features, as a bare-metal crate does. Both builds of it are made by the
[`feature-unification`](../feature-unification/README.md) probe, whose
bare-metal crate depends on this one: the host `cargo check --workspace` there
compiles it inside the `std` profile an application crate beside it turns on,
and the thumb build of that crate alone compiles it allocator-free.

## Why

The runtime owns the std-vs-heapless collection choice through profile-resolving
aliases — `SceBytes<N>`, `SceString`, `StateChain<S>`. Generated code names only
those aliases, so the runtime's own `#[cfg(feature = "std")]` picks the
concrete type and one emission is portable across profiles. The microstep's own
buffers (`SceTransitionBuf<T>` and friends) never appear in an emission at all:
Appendix D's procedures live in the runtime.

Before that alias layer was complete, a `--no-std` emit hard-coded
`sce_rust_runtime::heapless::*` (a `no_std`-only re-export) for its `bytes`
payload field, parallel-state transition buffers, and microstep dedup set. Built
against the **std** runtime those references failed with
`error[E0433]: cannot find heapless in sce_rust_runtime`. The host build is the
regression gate for that bug, and since docs/adr/0053 it is also what makes a
workspace holding a bare-metal crate and an application crate sound: the
bare-metal crate's machine compiles in the profile the union of their features
gives the runtime. The thumb build proves the same emission stays
allocator-free on bare metal.

## Coverage

The probe `[lib]` compiles a generated machine from
`sce-build/tests/fixtures/no_std/portable_probe.scxml`, chosen to exercise every
profile-resolving alias in one emission:

- a typed `_event.data.raw` **bytes** guard → `SceBytes` + `SceString` + scalar
  payload struct, and
- nested `<parallel>` regions → the `StateChain` active set, and the runtime's
  microstep monomorphised over it.

## Run locally

See [`../feature-unification/README.md`](../feature-unification/README.md). The
generated `src/*_sm.rs` is git-ignored and produced fresh on each run, so it
never drifts stale against the templates.
