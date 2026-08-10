# SCE Forge AST Export

`sce-codegen generate --emit-ast=<path>` writes the parsed Forge
document as JSON conforming to
[`apis/forge-ast.v1.schema.json`](../apis/forge-ast.v1.schema.json).

This document is the consumer contract for that file: what the
envelope means, which guarantees v1 carries, how the schema evolves
across versions, and the worked shapes for each `sce:kind`. The
authoritative *implementation* lives in
[`sce-build/src/forge/ast_export.rs`](../sce-build/src/forge/ast_export.rs)
and [`sce-build/src/forge/model.rs`](../sce-build/src/forge/model.rs);
this file is what a downstream tool author reads before depending on
the output.

## 0. CLI surface

Two entry points produce the v1 envelope:

* **`sce-codegen generate --emit-ast=<path>`** — single-document
  emit. Writes one envelope file at `<path>`. Covers both forge
  documents (Pipeline::Forge) and W3C SCXML statecharts
  (Pipeline::Scxml); the `oneOf` discriminator on `ast.document.kind`
  distinguishes the two. Documents rejected by W3C SCXML 5.8
  (`document_rejected`) skip emit and fall through to the existing
  rejection-stub codegen path; the absence of `<path>` is the
  consumer signal.
* **`sce-codegen orchestrate --emit-ast-dir=<dir>`** — multi-document
  batch emit. Writes one `<doc_stem>.ast.json` per `--forge` input
  AND per `--scxml` input — the v1 envelope's `oneOf` arm covers
  statechart documents (`ast.document.kind = "statechart"`)
  alongside the 15 forge kinds. Useful for batch tools that consume
  the IR across an entire multi-doc build without invoking SCE
  codegen per file.

Both forms produce byte-identical envelopes for the same input —
the orchestrate form is purely a batch convenience.

## 1. Why this exists

SCE's main pipeline turns Forge documents into target-language code
through `sce-codegen` itself. External tooling — DB schema
generators, event-store adapters, UI mirror compilers, GM consoles —
needs the *parsed IR*, not the generated code. The choices are:

1. Re-implement an SCXML+`sce:` parser per consumer (drift risk).
2. Link `sce-build` as a Rust library (forces consumers into the
   Rust toolchain).
3. **Have SCE emit the IR as JSON over a stable wire schema.** ← this
   document.

Option 3 keeps SCE the single source of truth for parsing while
letting consumers be written in any language with a JSON parser.

## 2. Envelope shape

```json
{
  "v": 1,
  "generator": "896629cf07d4",
  "ast": {
    "document": { "kind": "...", "name": "...", ... },
    "imports": [ { "src": "...", "kind": "...", "alias": "..." } ],
    "externs": [ { "name": "...", "sig": "...", "abi": "...", "crate_name": "..." } ]
  }
}
```

* **`v` (integer, const `1`)** — wire-format version. A consumer
  written against v1 may reject any other value outright.
* **`generator` (string, required)** — commit of the SCE build that
  produced this payload, matching `^([0-9a-f]{7,40}|unknown)$`. Same
  value the stdout manifest carries as `generator` and `--version`
  reports in parentheses, so a consumer reading both surfaces from one
  run gets one answer under one key. `unknown` only on a build with no
  git checkout to read (vendored crate, release tarball).

  This is what discharges `SCE_WIRE_CONTRACTS.md` policy 1, which
  tells consumers to pin a specific SCE commit while the surface is
  `pre-release`. The field previously held the `sce-build` crate
  version under the name `sce_producer_version` and was optional; that
  version is frozen pre-1.0, so it identified nothing and no consumer
  could pin from it. It matters most on the failure path: a rejected
  run writes no manifest at all, so an export sitting next to a
  rejection has nothing else alongside it to be attributed by.
* **`ast.document`** — per-kind parsed body. The closed set of
  `kind` discriminator values is given in
  [§5](#5-supported-kinds-v1).
* **`ast.imports`** — resolved `<sce:import>` declarations in document
  order. Always present (`[]` when the document declares no imports).
* **`ast.externs`** — `<sce:extern>` declarations validated against
  the §5.I baseline registry. Always present (`[]` when the document
  declares no externs). The wire shape is uniform: consumers can
  read `ast.externs[]` without branching on presence.

`additionalProperties` at the envelope top level is `false`:
consumers may safely reject unknown top-level keys as a v2 signal.
Inside `ast.document`, the per-kind shape is mechanically derived
from the Rust `ForgeDocument` enum via `schemars` (~3300 lines of
JSON Schema across 110 type definitions). Every IR field is part of
the contract — adding a new optional Rust field automatically
extends the schema in an additive way; renaming or removing a field
forces a non-additive regen that the drift guard catches in CI. See
[§3](#3-versioning-policy) for the lifecycle policy and
`sce-build/src/forge/ast_export.rs` `mod schema_drift` for the
mechanically-enforced producer ↔ schema invariant.

### Schema lifecycle marker — not on the wire

The schema file at `apis/forge-ast.v1.schema.json` carries an
`x-sce-schema-status` header (`pre-release` until SCE 1.0). This is
**not** a wire field — the payload itself does not carry the schema
status. Consumers that need to discriminate `pre-release` from
`stable` MUST inspect the schema file's header rather than the
emitted JSON. Rationale: lifecycle is metadata about the schema, not
metadata about the payload; encoding it in both invites the question
"which one wins on a mismatch?" with no satisfying answer.

This mirrors the diagnostic schema (`schemas/sce-diagnostic.v1.schema.json`),
which also carries `x-sce-schema-status` in the file header alone.

## 3. Versioning policy

This surface is one row in the cross-surface stability registry
[`SCE_WIRE_CONTRACTS.md`](../SCE_WIRE_CONTRACTS.md), which states the
shared `pre-release` policy and flip-to-`stable` procedure across every
wire surface. The rules below are the forge-AST specifics.

* **`pre-release` window (until SCE 1.0).** Non-additive changes
  permitted within `v=1`. Consumers should pin to a specific SCE
  release while the schema status is `pre-release`. The producer's
  `FORGE_AST_SCHEMA_STATUS` constant and the schema file's
  `x-sce-schema-status` header must be flipped to `stable` in the
  same commit; the test
  [`envelope_constants_match_schema_header`](../sce-build/tests/forge_ast_export.rs)
  guards this.

* **Additive (non-breaking) within `v=1` once stable.**
  * Adding a new optional field on any object.
  * Adding a new variant to the `kind` discriminator (and the
    accompanying `oneOf` arm).
  * Adding a new optional field to `imports[]` or
    `externs[]` entries.
  * Broadening a value enum (e.g. accepting a new `abi` value).

* **Breaking, requires `v=2` schema file alongside the v1 file.**
  * Renaming any field.
  * Removing any field.
  * Narrowing a value enum.
  * Changing a kind discriminator string (e.g.
    `"bounded-collection"` → `"bounded_collection"`).
  * Changing the cardinality of an existing field (single → array
    or vice versa).

Both schema files stay checked in across the deprecation window;
producers may emit either by version negotiation (TBD when the first
v2 lands).

## 4. Consumer compatibility checklist

A consumer that follows this checklist survives any v1-compatible
producer evolution:

1. **Pin `v == 1`.** Reject other values.
2. **Ignore unknown fields** at every level. Do not assert
   `additionalProperties: false` on `ast.document` or inner shapes.
3. **Read `x-sce-schema-status` from the schema file header, not the
   payload.** The status is `pre-release` until SCE 1.0; pin to a
   specific SCE release as the only firm guarantee in that window.
4. **Dispatch on `ast.document.kind`.** The string is closed (§5);
   an unknown value is a v2 signal — either upgrade or fail loudly.
5. **Read `imports` / `externs` as arrays unconditionally.** The
   producer emits `[]` for empty collections so consumers do not need
   to branch on presence vs `[]`. Tolerating absence is still safe
   (older producers may have stripped empty arrays) but the current
   contract is uniform.
6. **Use `name` as the document identifier.** It is derived from
   the input file stem by the CLI, not from `<scxml name=...>` —
   safe to use as a language identifier root (snake_case, PascalCase
   transformations apply uniformly).
7. **`source_location` is best-effort.** Present when the parser
   attached spatial provenance. Consumers that need it for IDE
   tooling should still tolerate `None` — XSD-level failures and
   synthesised IR nodes have no source position.
8. **`source_location.file` path semantics are producer-defined.**
   See [§10](#10-source_locationfile-path-semantics) for the full
   contract; consumers MUST NOT assume any particular root.

## 5. Supported kinds (v1)

The closed enum, in declaration order:

| Discriminator | Model | Producer notes |
|---|---|---|
| `statechart` | `SCXMLModel` | W3C SCXML state machine — emitted post-analyzer, pre-deploy-mutation. See [§8](#8-worked-example--statechart-kind). |
| `transform` | `TransformModel` | Pure formula. `inputs` + `outputs` arrays of `ForgeField`. |
| `lookup` | `LookupModel` | Key-value mapping with `miss_policy` (internally-tagged: `{kind: "default", value: "..."}` or `{kind: "error"}`). |
| `condition` | `ConditionModel` | Named boolean expression. |
| `codec` | `CodecModel` | Byte-level encoder/decoder. Large surface — see [`model.rs`](../sce-build/src/forge/model.rs) `struct CodecModel`. |
| `procedure` | `ProcedureModel` | Stateful procedure with guarded transitions. |
| `validator` | `ValidatorModel` | Range / rate-of-change / plausibility rules. |
| `filter` | `FilterModel` | Signal filter (moving average, low-pass, debounce). |
| `interpolation` | `InterpolationModel` | 1D/2D table interpolation. |
| `timer` | `TimerModel` | Periodic / delayed task. |
| `observer` | `ObserverModel` | Threshold monitor with hysteresis. |
| `algorithm` | `AlgorithmModel` | Pure synchronous function. |
| `link` | `LinkModel` | Byte-stream link endpoint (MCU-class). |
| `buffer-pool` | `BufferPoolModel` | SRAM-placed DMA-aligned slot table (MCU-class). |
| `worker` | `WorkerModel` | Concurrent execution context driven by a `<sce:link-rx>` source. |
| `bounded-collection` | `BoundedCollectionModel` | Build-time capacity container with runtime occupancy. |

The 16 kinds cover SCE's full IR surface: the 15 forge kinds plus
the W3C SCXML statechart arm. `imports` and `externs` are always
empty for `statechart` documents — W3C SCXML carries no
`<sce:import>` or `<sce:extern>` declarations of its own — so the
discriminator on `ast.document.kind` is the single signal that
distinguishes the statechart arm from forge arms.

## 6. Worked example — `transform` kind

Input:

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       xmlns:sce="http://sce.dev/ext"
       sce:kind="transform" name="unit_converter">
  <datamodel>
    <data id="celsius" sce:type="float64" sce:direction="in"/>
    <data id="fahrenheit" sce:type="float64" sce:direction="out"
          expr="celsius * 9 / 5 + 32"/>
  </datamodel>
</scxml>
```

`sce-codegen generate --emit-ast=out.json ...` writes:

```json
{
  "v": 1,
  "ast": {
    "document": {
      "kind": "transform",
      "name": "unit_converter",
      "inputs": [
        { "id": "celsius", "sce_type": "float64", "direction": "in" }
      ],
      "outputs": [
        { "id": "fahrenheit", "sce_type": "float64",
          "direction": "out", "expr": "celsius * 9 / 5 + 32" }
      ],
      "source_location": { "file": "...", "line": 1, "col": 1 }
    },
    "imports": []
  }
}
```

## 7. Worked example — `lookup` kind

Input:

```xml
<scxml ... sce:kind="lookup" name="gear_position_lookup">
  <datamodel>
    <data id="gearRaw" sce:type="uint8" sce:direction="in"/>
    <data id="gear" sce:type="string" sce:direction="out"/>
    <data id="mapping" sce:default="NEUTRAL">
      <sce:entry key="0" value="PARK"/>
      ...
    </data>
  </datamodel>
</scxml>
```

Emit:

```json
{
  "v": 1,
  "ast": {
    "document": {
      "kind": "lookup",
      "name": "gear_position_lookup",
      "input": { "id": "gearRaw", "sce_type": "uint8", "direction": "in" },
      "output": { "id": "gear", "sce_type": "string", "direction": "out" },
      "entries": [
        { "key": "0", "value": "PARK" }
      ],
      "miss_policy": { "kind": "default", "value": "NEUTRAL" },
      "source_location": { "file": "...", "line": 1, "col": 1 }
    },
    "imports": []
  }
}
```

Note the internally-tagged `miss_policy` enum: the discriminator
field is also `kind` (different from the document-level `kind`),
matching serde's `tag = "kind"` convention used throughout the IR.

## 8. Worked example — `statechart` kind

Input (minimal W3C SCXML state machine, no `sce:kind` attribute):

```xml
<scxml xmlns="http://www.w3.org/2005/07/scxml"
       version="1.0" datamodel="ecmascript"
       initial="s0" name="example_machine">
  <state id="s0">
    <transition event="go" target="done"/>
  </state>
  <final id="done"/>
</scxml>
```

Emit (truncated — the full envelope mirrors `SCXMLModel`'s
serialised form, ~50 keys; only the load-bearing surface is shown
here):

```json
{
  "v": 1,
  "ast": {
    "document": {
      "kind": "statechart",
      "name": "example_machine",
      "scxml_name": "example_machine",
      "initial": "s0",
      "initial_leaf": "s0",
      "datamodel_type": "ecmascript",
      "binding": "early",
      "states": {
        "s0": {
          "id": "s0", "is_final": false, "is_parallel": false,
          "document_order": 0,
          "transitions": [
            {
              "event": "go", "target": "done",
              "type": "external",
              "matching_enum_values": ["go"],
              "prefix_matching_events": ["go"]
            }
          ]
        },
        "done": {
          "id": "done", "is_final": true, "is_parallel": false,
          "document_order": 1
        }
      },
      "events": ["go"],
      "external_ingress_events": ["go"],
      "has_history_states": false,
      "has_parallel_states": false,
      "source_location": { "file": "...", "line": 20, "col": 1 }
    },
    "imports": [],
    "externs": []
  }
}
```

The shape is the post-analyzer `SCXMLModel` IR: every derived
field the analyzer enriched (`has_parent_communication`,
`needs_event_data`, `is_remote_invoke_target`, etc.) appears at
the top level, and the per-state `transitions[]` array carries
every analyzed metadata field (matching-enum values, prefix
matchers, source location) needed by downstream consumers to
mirror what the codegen pipeline sees.

`imports` and `externs` are always `[]` for the statechart arm —
W3C SCXML carries no cross-doc references of its own. Consumers
that need to walk a multi-doc statechart graph (an `<invoke>`
parent + child) should emit AST for each document separately
(orchestrate's per-doc fanout handles this) and join on
`invokes[].src`.

### `external_ingress_events` — the event-injection contract

Two top-level event sets serve different consumers:

* **`events`** is the kitchen-sink union of *every* event token the
  document references — transition triggers, the events it emits via
  `<send>`/`<raise>`, and engine-synthesized platform events
  (`error.*`, `done.invoke.*`, `done.state.*`, `cancel.invoke`, the
  `Wildcard` marker). It exists for codegen (enum population) and is
  **not** a statement of what the machine accepts from outside.
* **`external_ingress_events`** is the precise set of event
  descriptors that appear as `<transition event="...">` triggers with
  the engine-reserved families and the wildcard/eventless sentinels
  removed. Omitted from the envelope when empty.

A tool that injects events into a running machine — a transport
switchboard mapping a pub/sub key to a domain event, an external
command router, a test driver — validates its targets against
`external_ingress_events`, **not** `events`. An injected name is
accepted by the machine iff it matches a member per W3C SCXML 3.12.1
event-descriptor matching (exact match in the common case where the
trigger is a full event name). Validating against `events` would
false-accept names the machine only *emits* or that the engine
reserves; validating against `external_ingress_events` is drift-proof
because SCE owns the reserved-family filter — consumers do not
re-implement the platform-event taxonomy.

## 9. Reference

* Authoritative producer:
  [`sce-build/src/forge/ast_export.rs`](../sce-build/src/forge/ast_export.rs)
* IR type tree (field-by-field shape — every type derives
  `cfg_attr(test, derive(schemars::JsonSchema))`):
  [`sce-build/src/forge/model.rs`](../sce-build/src/forge/model.rs)
* Wire schema (mechanically derived from the IR via schemars; do
  not hand-edit — regenerate with
  `UPDATE_EXPECT=1 cargo test -p sce-build schema_drift`):
  [`apis/forge-ast.v1.schema.json`](../apis/forge-ast.v1.schema.json)
* Drift guards (schema ↔ Rust type-tree invariant + per-kind
  round-trip coverage):
  [`sce-build/src/forge/ast_export.rs`](../sce-build/src/forge/ast_export.rs)
  (`mod schema_drift`) and
  [`sce-build/tests/forge_ast_export.rs`](../sce-build/tests/forge_ast_export.rs)
* General `apis/` contract:
  [`apis/README.md`](../apis/README.md)

## 10. `source_location.file` path semantics

The `source_location.file` string carries whatever the producer was
given as the document's diagnostic label. The SCE CLI today passes
the SCXML file's basename (file stem with extension) — *not* an
absolute path, *not* a path relative to a project root, and *not*
guaranteed unique across an invocation. The semantics are:

* **Producer contract.** The producer puts the
  `DocumentLabel.diagnostic_label` value verbatim. That value is
  whatever the caller picked when constructing the label — for the
  CLI it's the input file's basename; for in-process callers
  (`build.rs` users) it can be anything the build script chose.
* **Consumer contract.** Treat `source_location.file` as an opaque
  identifier scoped to the current emit. It is suitable for:
  * IDE / editor "jump to file" features when the consumer already
    knows how to resolve the producer's roots.
  * Anchoring diagnostic messages back to a human-readable label.

  It is NOT suitable for:
  * Asserting that the file exists on the consumer's filesystem.
  * Joining onto a project-root path without explicit knowledge of
    the producer's working directory.
  * Cross-emit deduplication (two different SCXML inputs may share
    a basename if the producer didn't disambiguate).
* **Recommended producer hygiene.** When emitting AST for a
  multi-file build, pre-normalise the label to a stable form
  (workspace-relative path) before invoking SCE so the emitted
  envelope's `file` field is unambiguously locatable. The CLI's
  `--emit-ast` path uses basenames today; a future flag may
  introduce explicit normalisation.

Future schema revisions may add a richer `source_location` with a
canonical-path field, but v1 stays minimal — the line/col fields
are the load-bearing part for IDE consumers.

## 11. Canonical form

Consumers that hash or sign envelopes for content addressing should
treat the SCE-emitted form as the canonical baseline for v1:

* **Encoding.** UTF-8, no BOM.
* **Whitespace.** Pretty-printed via `serde_json::to_writer_pretty`
  — two-space indentation, newlines between siblings, trailing
  newline at end of file (so `git diff` does not flag a missing
  terminator).
* **Field ordering.** `serde_json`'s default ordering preserves
  struct field declaration order at each level; nested `Map`-typed
  values (e.g. inside generated definitions) are sorted
  alphabetically by serde_json's `BTreeMap` representation.
  Consumers MUST NOT assume any particular order beyond what the
  schema declares.
* **Empty arrays.** `imports` and `externs` always emit as `[]`
  (never omitted). See [§4 item 5](#4-consumer-compatibility-checklist).
* **Optional `None` fields.** Skipped from the wire entirely (no
  `"key": null`). Consumers that need a uniform shape should treat
  absent keys as `None`.

Consumers requiring a stricter canonical form (e.g. RFC 8785) MUST
re-serialise through a canonicalisation library; SCE does not
guarantee RFC 8785 byte-equality across releases.

