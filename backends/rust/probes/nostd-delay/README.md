# sce-nostd-delay-probe

A compile gate proving that a **generated** SCE machine whose states keep deadlines computed
by `delayexpr` is allocator-free under `--no-std` (docs/adr/0134).

Each state of the probe arms its deadline with a delayed `<send>` in its `onentry` and cancels
it by the same id in its `onexit`, and the compound state keeps an overall limit. Its delays
take every form a `delayexpr` does: an integer joined to its unit (`limit_s + 's'`), computed
from the integer by checked arithmetic with no text; one whose integer can fail; a join of
texts (`idle + unit`), written into a `DelayText` buffer on the stack the build sizes from what
the data model declares; and a conditional one of whose branches joins. The machine builds for
`thumbv7em-none-eabihf`, which has no `std`, with no `#[global_allocator]`, so a lowering that
spelled `format!` or a `String` into a delay is a hard compile error here, which the host-target
tests, where an allocator is, cannot show.

The probe `[lib]` compiles the machine generated from
`sce-build/tests/fixtures/no_std/session_deadline_probe.scxml`.

## Run locally

```sh
# 1. Build the code generator.
cargo build --bin sce-codegen --features cli -p sce-build

# 2. Generate the probe machine (git-ignored output).
target/debug/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/nostd-delay/src \
  sce-build/tests/fixtures/no_std/session_deadline_probe.scxml

# 3. Compile it for the MCU target with no allocator.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/nostd-delay/Cargo.toml \
  --target thumbv7em-none-eabihf
```

A clean exit means the generated machine links with zero `alloc` dependency.
