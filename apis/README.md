# `apis/` — External Consumer Wire Schemas

This directory holds JSON Schema definitions that pin the wire format
of artefacts SCE emits for *external* consumers — downstream tooling
that sits outside the SCE repository and depends on a stable contract
to consume SCE outputs. Anything here is part of SCE's public surface;
a change that breaks consumers requires a major version bump on the
affected envelope.

Schemas that live elsewhere:

* `schemas/sce-diagnostic.v1.schema.json` — diagnostic NDJSON records
  emitted on stderr. *Internal* in the sense that the SCE CLI itself
  produces the records and SCE tooling consumes them; the schema is
  still wire-stable but lives next to the runtime-format definitions.
* `schemas/sce-forge.xsd` / `schemas/sce-forge-ext.xsd` — input
  validators, not output formats.

## Files

| File | Envelope | Producer | Consumer |
|---|---|---|---|
| [`forge-ast.v1.schema.json`](forge-ast.v1.schema.json) | `{v, ast}` | `sce-codegen generate --emit-ast=<path>` (single-doc) / `sce-codegen orchestrate --emit-ast-dir=<dir>` (multi-doc batch) | Downstream tools that derive artefacts from the parsed IR (DB schema, event-store adapters, UI mirrors) without invoking SCE codegen. Covers all 16 kinds (15 forge + statechart). See [`docs/SCE_FORGE_AST.md`](../docs/SCE_FORGE_AST.md). |

## Stability contract

* **Pre-release status.** Each schema carries an `x-sce-schema-status`
  header. Until SCE 1.0 the field reads `pre-release`, signalling that
  non-additive changes are permitted within the major version line
  (mirrors `SCE_ERROR_CONTRACT.md` §8.1). Flipping to `stable`
  requires synchronising the schema file header and the producer's
  status constant in one commit; the drift guards under
  `sce-build/tests/forge_ast_export.rs` catch divergence.
* **Additive evolution within `v`.** New optional fields, new optional
  kind variants, and broadened value enums are non-breaking. Consumers
  MUST ignore unknown fields.
* **Major version bumps.** Renaming or removing a field, narrowing a
  value enum, or changing a kind discriminator requires authoring
  `<name>.vN+1.schema.json` alongside the v`N` file; both files
  remain checked in for the deprecation window.
* **Drift guards.** Every envelope ships with a regression test under
  `sce-build/tests/` that asserts the producer's emit shape matches
  the checked-in schema. CI fails on drift.

## Adding a new schema

1. Author `<name>.v1.schema.json` here.
2. Add an entry to the file table above.
3. Land the producer hook (CLI flag, library helper) under
   `sce-build/`.
4. Add a drift-guard test under `sce-build/tests/<name>_export.rs`
   that round-trips a representative input through the producer and
   asserts the emitted envelope matches the schema's frozen
   invariants.
5. Add a per-envelope companion doc under `docs/` describing the
   consumer contract, version-bump policy, and worked examples.
