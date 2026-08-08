# sce-nostd-build-probe

SCE Protocol-Synthesis RFC §5.J.2 — a compile gate proving that **generated** SCE state
machines (not just the `backends/rust/runtime` crate) are allocator-free under
`--no-std`.

`thumbv7em-none-eabihf` has no `std`, and this crate declares no
`#[global_allocator]`, so any reachable `alloc` symbol (`String`, `Vec`, `Box`,
`[T]::sort_by`'s merge buffer, …) is a hard compile error. A template
regression that reintroduces `alloc` into emitted code fails here even though
the runtime-only no_std build still passes.

The probe `[lib]` compiles a generated machine from
`sce-build/tests/fixtures/no_std/parallel_history_probe.scxml`, chosen to
exercise the constructs that were historically std-coupled: nested `<parallel>`
(conflict-resolution buffers + the `FnvIndexSet` dedup set), shallow + deep
`<history>` (`StateChain` fields), and many transitions (`stable_sort_by`).

## Run locally

```sh
# 1. Build the code generator.
cargo build --bin sce-codegen --features cli -p sce-build

# 2. Generate the probe machine (git-ignored output).
target/debug/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/nostd-build/src \
  sce-build/tests/fixtures/no_std/parallel_history_probe.scxml

# 3. Compile it for the MCU target with no allocator.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/nostd-build/Cargo.toml \
  --target thumbv7em-none-eabihf
```

A clean exit means the generated machine links with zero `alloc` dependency.
The generated `src/*_sm.rs` is git-ignored and produced fresh on each run, so it
never drifts stale against the templates.
