# feature-unification probe

One workspace holding a bare-metal crate (`mcu`) and an application crate
(`ap`), each with the manifest a downstream crate of its kind writes
(docs/adr/0053):

```toml
# mcu/Cargo.toml: the bare-metal profile is the absence of `std`
sce-rust-runtime = { path = "...", default-features = false }

# ap/Cargo.toml: the defaults, `std`, `http-send` and `macrostep-diagnostics`
sce-rust-runtime = { path = "..." }
```

Cargo builds one copy of `sce-rust-runtime` for every package of one build,
with the union of the features they ask for. The runtime's features add and
never refuse, so the union compiles, and each crate still gets what it asked
for:

- `cargo check --workspace` on the host builds the `std` profile, the union.
  `mcu` compiles in it because it names only what the bare-metal profile
  declares, which `std` declares too; `ap` holds code that compiles only where
  `SceString` is `String`, the HTTP processor exists and the macrostep report
  (`truncated_macrosteps`) can be read.
- `cargo build -p sce-feature-unification-mcu --target thumbv7em-none-eabihf`
  selects `mcu` alone, and Cargo unifies features only across the packages one
  command builds, so the runtime is its bare-metal profile. The thumb target
  has no `std`; a feature of `ap` leaking into this build is a hard error.

`mcu` depends on [`portable-emit`](../portable-emit/README.md), the generated
`--no-std` machine, so both builds compile a real emission.

Before the runtime's features were additive, the first command stopped at the
runtime: its `no_std` feature refused `http-send` with `compile_error!`. And a
bare-metal crate that named `no_macrostep_diagnostics` removed the report's
accessors from the whole build, so an application crate beside it that read
them stopped compiling; the report is now the feature `macrostep-diagnostics`,
which adds them.

## Run locally

```sh
# 1. Build the code generator.
cargo build --bin sce-codegen --features cli -p sce-build

# 2. Generate the machine `mcu` holds (git-ignored output).
target/debug/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/portable-emit/src \
  sce-build/tests/fixtures/no_std/portable_probe.scxml

# 3a. Both crates in one host build.
cargo check --manifest-path backends/rust/probes/feature-unification/Cargo.toml --workspace

# 3b. The bare-metal crate alone, for its target.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/feature-unification/Cargo.toml \
  -p sce-feature-unification-mcu --target thumbv7em-none-eabihf
```

`scripts/gates/nostd-mcu.sh` runs these steps.
