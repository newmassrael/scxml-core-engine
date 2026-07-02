# sce-portable-emit-probe

Watching-zenoh RFC §5.J.2 — a compile gate proving that a **single** `--no-std`
generated machine compiles against **both** runtime profiles:

- the **std** runtime (the AP profile — `cargo build`, host target), and
- the **no_std** runtime (the MCU profile — `--features mcu`, thumb target).

## Why

The runtime owns the std-vs-heapless collection choice through profile-resolving
aliases — `SceBytes<N>`, `SceString`, `SceTransitionBuf<T>`, `SceIndexBuf`,
`SceDedupSet<K>`, `StateChain<S>`. Generated code names only those aliases, so
the runtime's own `#[cfg(feature = "no_std")]` picks the concrete type and one
emission is portable across profiles.

Before that alias layer was complete, a `--no-std` emit hard-coded
`sce_rust_runtime::heapless::*` (a `no_std`-only re-export) for its `bytes`
payload field, parallel-state transition buffers, and microstep dedup set. Built
against the **std** runtime (`no_std` OFF) those references failed with
`error[E0433]: cannot find heapless in sce_rust_runtime`. This crate's default
(std-runtime) build is the regression gate for that bug; the `mcu` build proves
the same emission stays allocator-free on bare metal.

## Coverage

The probe `[lib]` compiles a generated machine from
`sce-build/tests/fixtures/no_std/portable_probe.scxml`, chosen to exercise every
profile-resolving alias in one emission:

- a typed `_event.data.raw` **bytes** guard → `SceBytes` + `SceString` + scalar
  payload struct, and
- nested `<parallel>` regions → `SceTransitionBuf` / `SceIndexBuf` /
  `SceDedupSet` + `StateChain`.

## Run locally

```sh
# 1. Build the code generator.
cargo build --bin sce-codegen --features cli --release -p sce-build

# 2. Generate the probe machine (git-ignored output).
target/release/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/portable-emit/src \
  sce-build/tests/fixtures/no_std/portable_probe.scxml

# 3a. AP gate — compile the no_std emit against the STD runtime (host).
cargo build --manifest-path backends/rust/probes/portable-emit/Cargo.toml

# 3b. MCU gate — compile the SAME emit against the no_std runtime (thumb).
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/portable-emit/Cargo.toml \
  --target thumbv7em-none-eabihf --features mcu
```

A clean exit from both means the one emission is portable across both runtime
profiles. The generated `src/*_sm.rs` is git-ignored and produced fresh on each
run, so it never drifts stale against the templates.
