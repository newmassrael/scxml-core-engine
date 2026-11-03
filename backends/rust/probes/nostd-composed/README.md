# sce-nostd-composed-probe

A compile gate proving that a **generated** SCE machine that reads a composed payload
(docs/adr/0086, sections 6 and 7.4) is allocator-free under `--no-std`.

Every list, text and byte string such a machine holds, in its variables, its records and
the payload struct of the event it reads, is `sce_portable_bytes::BoundedList<T, N>`,
`SceString<N>` or `SceBytes<N>`. Those hold their elements inline (`heapless`) where no
allocator is, so the machine builds for `thumbv7em-none-eabihf`, which has no `std`, with
no `#[global_allocator]`. A lowering that spelled a `Vec` or a `String` into the machine is
a hard compile error here, which the host-target tests, where an allocator is, cannot show.

The probe `[lib]` compiles the machine generated from
`sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml`: a list of records
that hold a list of texts, a list of lists, a list of optional values, a byte string inside
a record, and a list taken whole into a list variable of a larger bound.

## Run locally

```sh
# 1. Build the code generator.
cargo build --bin sce-codegen --features cli -p sce-build

# 2. Generate the probe machine (git-ignored output).
target/debug/sce-codegen --workspace-root . generate -l rust --no-std \
  --output-dir backends/rust/probes/nostd-composed/src \
  sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml

# 3. Compile it for the MCU target with no allocator.
rustup target add thumbv7em-none-eabihf   # once
cargo build --manifest-path backends/rust/probes/nostd-composed/Cargo.toml \
  --target thumbv7em-none-eabihf
```

A clean exit means the generated machine links with zero `alloc` dependency.
