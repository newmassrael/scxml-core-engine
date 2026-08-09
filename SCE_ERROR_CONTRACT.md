# SCE Error Contract

Machine-readable error format produced by `sce-codegen --error-format=json`.
Consumed by upstream automation — LangGraph triage nodes, IDE language
servers, CI repair bots, and any other agent that needs to branch on
SCE's rejection signals without parsing human text.

This document is the **wire contract**. It must move only in the directions
defined in [§8 Evolution Policy](#8-evolution-policy). The on-disk
enforcement is `forge::diagnostic::tests::diagnostic_goldens_are_byte_stable`
— a byte-level golden check that fires on any accidental drift.

## 0. Counterpart documents

- **Positive-form acceptance spec**: `docs/SCE_ACCEPTED_SUBSET.md`
  enumerates the SCXML subset SCE accepts and partitions every
  `DiagnosticCode` into author-preventable (acceptance boundary) vs
  I/O / infrastructure (diagnostic-only). Adding a new `DiagnosticCode`
  variant without listing it there fails the
  `acceptance_doc_covers_every_code` test. Read it together with this
  contract when deciding whether a new rejection signal is earned.

## 1. Streams

| Stream | Content |
|---|---|
| **stdout** | On success: a single JSON manifest line per run (see [§10](#10-stdout-manifest)). On failure: empty. Never carries diagnostics. |
| **stderr** | Exactly one NDJSON diagnostic per line when `--error-format=json`. In `human` mode, free-form text. |

Agents **must** split the two streams by fd. A parser that reads
stdout looking for errors is reading the wrong stream.

## 2. Record shape

```json
{
  "v": 1,
  "id": "fnv1a:dd04a37de468ffb4",
  "code": "validation/invalid-attribute",
  "stage": "validation",
  "message": "sce:field: unknown sce:type value 'blob' (expected: u8, u16, u32)",
  "location": {"file": "checkout.scxml", "line": 42, "col": 3},
  "actual": "blob",
  "fix": {"kind": "replace_one_of", "candidates": ["u8", "u16", "u32"]}
}
```

All fields except `v`, `id`, `code`, `stage`, and `message` are optional.
Omitted fields are absent from the JSON entirely (not `null`). Consumers
**must** ignore unknown fields for forward compatibility.

### 2.1 Field semantics

| Field | Type | Guarantee |
|---|---|---|
| `v` | integer | Schema version. Currently `1`. First key in every record. |
| `id` | `fnv1a:<16hex>` | Content hash over `(code, stage, location.file, key_fragments)`. Same semantic error → same id, **independent of message rewording**. Use for dedup, caching, "seen this before" checks. |
| `code` | slash-path string | Closed enum. See [§5 Code Catalog](#5-code-catalog). Agents dispatch on `code`, never on `message`. |
| `stage` | lowercase / kebab-case string | Pipeline stage. Routes to the correct repair loop. See [§4 Stage Taxonomy](#4-stage-taxonomy). |
| `spec` | string | Specification anchor (e.g. `"W3C SCXML 3.13"`). Present when the rule is spec-derived. Enables LLM grounding. |
| `message` | English prose | Human-readable one-liner. **Not** machine-parsed. Not part of `id`. |
| `location` | object | Source location when known. See [§2.2](#22-location-object). |
| `expected` | array of strings | Non-repair expectation metadata (parser expectations like `"identifier"`, cardinality constraints like `"1"`). **Never** carries a candidate list for substitution — that role belongs to `fix`. The two fields are disjoint by contract (see [§3.2](#32-no-overlap-between-fix-and-expected)). |
| `actual` | string | The observed value that triggered rejection. |
| `fix` | object | Structured repair proposal. The sole channel for repair signals. See [§3 Fixes](#3-fixes). |
| `spec_provenance` | array of objects | NL→IR Mapping Roadmap Item 6 — spec-document anchors that justify the rejected node (`doc_id` + optional `rev`/`section`/`page`). SCE never infers this; IR generators (NL→IR pipelines, ARXML transcoders) populate it when they know the spec origin. Pass-through field on the wire — absent when the upstream did not record provenance. |
| `question_kind` | string (enum) | NL→IR Mapping Roadmap Item 6 — coarse routing label so IDE / triage tooling can dispatch on the *kind* of question the diagnostic raises (`implicit_default` / `ambiguous_mapping` / `cross_doc_conflict` / `unit_unspecified` / `unknown_vocabulary` / `structural`). Extensible — consumers must treat unknown values as `structural`. Absent on purely structural rejections that map cleanly onto `code` alone. |

### 2.2 Location object

```json
{"file": "checkout.scxml", "line": 42, "col": 3}
```

`line` and `col` are optional; `file` is required when the object is present.
Mesh errors currently omit `location` — their coordinates are the
machine / binding / target names carried by the error fields themselves.

## 3. Fixes

`fix` is the **sole channel** for repair signals. Whenever the producer
can name a change that would satisfy the rejected constraint — single
value, closed candidate list, attribute to add — the payload lives on
this field, never on `expected`. Agents drive repair by dispatching on
`fix.kind` and reading the variant's payload.

`fix` is absent only when no structured repair can be named — e.g. an
`Io` failure, a template-render crash, or a cardinality violation that
requires a redesign rather than a local edit. Absence means "no
structured repair on the wire"; it does **not** mean "look elsewhere".
There is no fallback from `fix` to `expected`.

### 3.1 Fix variants

| `kind` | Payload | Semantics |
|---|---|---|
| `add_attribute` | `element`, `attr` | Add the named attribute to the named element. For deploy.yaml errors, `element` is a dotted path (`machines.x.bindings.y`). Deterministic. |
| `rename_duplicate` | `what`, `id` | The id `id` of kind `what` appears more than once; rename one occurrence. Deterministic. |
| `remove_fields` | `location`, `fields[]` | At the config path `location`, remove every key in `fields`. Deterministic. |
| `replace_with` | `to` | Replace the value carried in the record's `actual` field with `to`. Emitted when exactly one legal replacement exists — e.g. `==` → `===` under strict equality, or a `<sce:import kind>` declaration corrected to the imported file's real kind. Deterministic. |
| `replace_one_of` | `candidates[]` | Replace `actual` with one of `candidates`. Emitted when the producer knows the closed set of legal values (attribute-value enums, cross-reference resolution, supported language list) but cannot pick a single answer — the agent or human chooses from the list. |
| `add_one_of` | `element`, `attrs[]` | Add one of the listed `attrs` to `element`. Used for "require either X or Y" constraints (e.g. `<send>` needs `event` or `eventexpr`). Choice-based. |

The variant name encodes the *shape* of the repair: deterministic
variants can be applied without further judgment; choice variants
(`replace_one_of`, `add_one_of`) require the agent or human to pick
from the closed candidate set.

Agents holding a dispatch table keyed on `fix.kind` may safely
enumerate these — the set only grows in backward-compatible ways
([§8](#8-evolution-policy)).

### 3.2 No overlap between `fix` and `expected`

`fix` and `expected` are **disjoint**: no diagnostic record ever
carries the same information in both fields. A consumer that needs a
repair signal reads `fix`; a consumer that needs to know what the
producer was grammatically expecting reads `expected`. The two fields
describe orthogonal aspects of a rejection and are interpreted
independently.

Concretely:

- `validation/invalid-attribute`, `validation/invalid-reference`,
  `validation/unsupported-kind`, `validation/invalid-direction`,
  `cli/unknown-language`, `mesh/topology-machine-not-found`,
  `mesh/codegen-unsupported-transport`, and similar codes with a
  closed legal-value enumeration populate `fix.candidates` (or
  `fix.attrs`) and leave `expected` absent. The candidate list is
  never duplicated across both fields.
- `expression/parse-mismatch` populates `expected` with a grammar
  production name (e.g. `"identifier"`) and leaves `fix` absent — the
  parser expectation is diagnostic metadata, not a substitution
  candidate.
- `mesh/external-ambiguous-event-group` populates `expected` with the
  required cardinality (e.g. `["1"]`) and leaves `fix` absent — the
  number is a rule description, not a replacement value for `actual`.

Agents that want "the closed set of legal values" should always read
`fix`. Agents that want "what the producer was grammatically expecting
at this position" should read `expected`. These needs never coincide.

## 4. Stage taxonomy

| Stage | Source | Role |
|---|---|---|
| `xml` | XML / XSD parser | Document well-formedness and schema validation. |
| `validation` | Forge semantic validator | `sce:kind`-specific structural rules. |
| `expression` | Expression transpiler | Stateless-subset rejections, ECMAScript unsupported constructs. |
| `import` | `<sce:import>` resolver | Cross-file dependency resolution. |
| `manifest` | Dependency graph builder | Cycle detection and directory scans. |
| `generate` | Template engine | Jinja2 load/render failures. |
| `io` | Filesystem boundary | Read/write errors that are not associated with a specific pipeline stage. |
| `cli` | Command-line driver | Argument parsing, workspace layout, unsupported languages. |
| `mesh-deploy` | deploy.yaml parser | YAML syntax, schema version, duplicate machines. |
| `mesh-external` | vsomeip.json / zenoh.json5 resolver | Name resolution, schema conflicts. |
| `mesh-topology` | Target / binding resolver | Send-target coverage, pattern capability, binding field validation. |
| `mesh-codegen` | Mesh template engine | Transport-specific rendering and collision checks. |

### 4.1 Pipeline routing

`stage` is the repair-routing key for agents. Its value is determined
by the `sce:kind` attribute on the `<scxml>` root, *not* by whether
the document happens to parse successfully:

| `sce:kind` value                       | Pipeline         | Diagnostic source          |
|----------------------------------------|------------------|----------------------------|
| absent                                 | SCXML parser     | `xml/parse` / `validation/*` |
| `"statechart"`                         | SCXML parser     | `xml/parse` / `validation/*` |
| known forge kind (e.g. `"lookup"`)     | Forge            | `xml` / `validation` / ... |
| unknown value (e.g. `"bogus"`)         | Forge            | `xml/schema-validation`    |

The last row is a contract guarantee. An author who wrote
`sce:kind="bogus"` intended a forge document, so the failure must
surface through the forge pipeline — where the bundled XSD identifies
the violation and the `message` field enumerates the legal values.
Reporting such a failure through the SCXML parser's XML stage would
mis-route repair agents and is explicitly forbidden.

Malformed XML that never reaches the root attribute list falls back
to the SCXML parser: intent cannot be inferred, and the SCXML
parser's XML-level diagnostic is the least-wrong answer.

The on-disk enforcement of this rule is
`tests/error_format_json.rs::json_mode_routes_unknown_sce_kind_through_forge_pipeline`.
The routing primitive is `sce_build::classify_document` in
`sce-build/src/lib.rs`.

## 5. Code catalog

The full enumeration of `code` values, grouped by stage. The set is
extended additively — a code is never renamed or repurposed without
a schema bump ([§8](#8-evolution-policy)).

The `Spec` column names the authoritative section that defines the rule
being enforced. An empty `Spec` column means the code records an
operational failure (I/O, template render, argument parsing) rather
than a specification violation. Section references follow
`DiagnosticCode::spec_anchor` in `sce-build/src/forge/diagnostic.rs` and
must point at a real section — adding a plausible-looking anchor for a
rule that is not actually documented there is strictly worse than
leaving the column empty, because consumers ground hallucinated
references against a real document and drift silently.

### 5.1 Forge

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `xml/parse` | `xml` | no | |
| `xml/schema-validation` | `xml` | no | SCE Forge XSD |
| `validation/missing-element` | `validation` | no | |
| `validation/missing-attribute` | `validation` | `add_attribute` | |
| `validation/invalid-attribute` | `validation` | `replace_one_of` | |
| `validation/unsupported-kind` | `validation` | `replace_one_of` | SCE Forge §3.2 |
| `validation/duplicate-id` | `validation` | `rename_duplicate` | |
| `validation/duplicate-context-object` | `validation` | `rename_duplicate` | |
| `validation/empty-collection` | `validation` | no | |
| `validation/count-mismatch` | `validation` | no | |
| `validation/incompatible-attributes` | `validation` | no | |
| `validation/missing-context` | `validation` | no | |
| `validation/invalid-reference` | `validation` | `replace_one_of` | |
| `validation/invalid-direction` | `validation` | `replace_one_of` | SCE Forge §3.3 |
| `validation/duplicate-requirement-id` | `validation` | no | |
| `validation/unresolved-placeholder` | `validation` | no | |
| `validation/numeric-parse` | `validation` | no | |
| `validation/empty-value` | `validation` | `add_attribute` | |
| `validation/singleton-violation` | `validation` | no | |
| `validation/require-either` | `validation` | `add_one_of` | |
| `validation/wrong-pipeline` | `validation` | no | SCE Forge §4 |
| `validation/dynamic-features` | `validation` | no | |
| `expression/empty` | `expression` | no | SCE Forge §3.4 |
| `expression/lex` | `expression` | no | SCE Forge §3.4 |
| `expression/unsupported-construct` | `expression` | no | SCE Forge §3.4 |
| `expression/strict-equality` | `expression` | `replace_with` | SCE Forge §3.4 |
| `expression/parse-mismatch` | `expression` | no | SCE Forge §3.4 |
| `expression/unexpected-token` | `expression` | no | SCE Forge §3.4 |
| `expression/invalid-lvalue` | `expression` | no | SCE Forge §3.4 |
| `expression/type-coercion` | `expression` | no | SCE Forge §3.4 |
| `expression/go-ternary-unsupported` | `expression` | no | SCE Forge §3.4 |
| `import/file-not-found` | `import` | no | |
| `import/kind-mismatch` | `import` | `replace_with` | |
| `import/not-forge` | `import` | no | |
| `import/read-error` | `import` | no | |
| `manifest/circular-dependency` | `manifest` | no | |
| `manifest/io` | `manifest` | no | |
| `generate/invalid-config` | `generate` | no | |
| `generate/template-load` | `generate` | no | |
| `generate/template-render` | `generate` | no | |
| `generate/unsupported-feature` | `generate` | no | |
| `codegen/mcu-class-kind-on-non-mcu-language` | `generate` | no | |
| `codegen/generic-kind-backend-emit-missing` | `generate` | no | |
| `io/filesystem` | `io` | no | |

### 5.2 CLI

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `cli/unknown-language` | `cli` | `replace_one_of` | |
| `cli/unsupported-language` | `cli` | no | |
| `cli/read-input` | `cli` | no | |
| `cli/write-output` | `cli` | no | |
| `cli/create-output-dir` | `cli` | no | |
| `cli/scxml-generate` | `cli` | no | |
| `cli/missing-metadata-field` | `cli` | no | |
| `cli/not-a-directory` | `cli` | no | |
| `cli/invalid-format-option` | `cli` | `replace_one_of` | |
| `cli/json-serialization` | `cli` | no | |
| `cli/project-root-not-found` | `cli` | no | |
| `cli/format-style-not-found` | `cli` | no | |
| `cli/no-scxml-tag` | `cli` | no | |

### 5.3 Mesh

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `mesh/deploy-read` | `mesh-deploy` | no | |
| `mesh/deploy-parse` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-unsupported-version` | `mesh-deploy` | `replace_one_of` | SCE Mesh §14 |
| `mesh/deploy-duplicate-machine` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-platform-class-os-mismatch` | `mesh-deploy` | no | SCE Mesh §14 |
| `deploy/worker-stack-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/worker-slot-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/keepalive-jitter-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/scheduler-incompatible-with-worker-count` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `timer/slot-overflow` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.D |
| `mesh/external-read` | `mesh-external` | no | |
| `mesh/external-parse` | `mesh-external` | no | |
| `mesh/external-unresolved-names` | `mesh-external` | no | |
| `mesh/external-ambiguous-event-group` | `mesh-external` | no | |
| `mesh/external-empty-event-group` | `mesh-external` | no | |
| `mesh/external-named-reference-without-config` | `mesh-external` | no | |
| `mesh/external-reserved-someip-id-keys` | `mesh-external` | `remove_fields` | |
| `mesh/external-someip-field-on-non-someip-transport` | `mesh-external` | `replace_with` | |
| `mesh/external-conflicting-event-schema` | `mesh-external` | no | |
| `mesh/external-conflicting-event-field-kinds` | `mesh-external` | no | |
| `mesh/external-empty-event-entry` | `mesh-external` | no | |
| `mesh/topology-unresolved-targets` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-machine-not-found` | `mesh-topology` | `replace_one_of` | SCE Mesh §14 |
| `mesh/topology-receiver-not-declared` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-absolute-source-path` | `mesh-topology` | no | |
| `mesh/topology-receiver-source-read` | `mesh-topology` | no | |
| `mesh/topology-receiver-source-parse` | `mesh-topology` | no | |
| `mesh/topology-uncovered-events` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-pattern-capability-violation` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-missing-binding-field` | `mesh-topology` | `add_attribute` | SCE Mesh §14 |
| `mesh/topology-invalid-binding-field` | `mesh-topology` | no | SCE Mesh §14 |
| `mesh/topology-event-binding-unused` | `mesh-topology` | `remove_fields` | SCE Mesh §14 |
| `mesh/topology-ordering-cannot-be-guaranteed` | `mesh-topology` | no | SCE Mesh §10.6 |
| `mesh/codegen-unsupported-language` | `mesh-codegen` | `replace_one_of` | SCE Mesh §7 |
| `mesh/codegen-unsupported-transport` | `mesh-codegen` | `replace_one_of` | SCE Mesh §8 |
| `mesh/codegen-template-read` | `mesh-codegen` | no | |
| `mesh/codegen-template-render` | `mesh-codegen` | no | |
| `mesh/codegen-event-name-collision` | `mesh-codegen` | no | |
| `mesh/io` | `io` | no | |

## 6. Exit codes

Exit status is a coarse routing signal; `code` is the finer one.
A non-zero exit with no NDJSON record is a contract violation.

| Code | Meaning |
|---|---|
| `0` | Success. |
| `2` | `xml/*` |
| `3` | `validation/*` |
| `4` | `expression/*` |
| `5` | `import/*` |
| `6` | `manifest/*` |
| `7` | `generate/*` |
| `8` | `io/filesystem` (forge I/O) |
| `10` | `mesh/deploy-*` |
| `11` | `mesh/topology-*` |
| `12` | `mesh/codegen-*` |
| `13` | `mesh/io` |
| `14` | `mesh/external-*` |
| `20` | `cli/*` (CLI-boundary errors) |

## 7. Determinism guarantees

- **No timestamps, no wall-clock**, no PIDs, no absolute paths other
  than those the user passed in.
- **No ANSI / color escapes** in JSON mode — ever.
- **Field order** within a record is fixed: `v`, `id`, `code`,
  `stage`, `spec`, `message`, `location`, `expected`, `actual`, `fix`.
- **One record per line.** A record never contains a raw `\n`.
  Consumers may split stderr on `\n` without a JSON parser lookahead.
- **`id` stability**: rewording a `thiserror` `#[error]` template
  does not shift `id`. Only changing the hashed semantic fields
  (code, stage, file, key_fragments) does.

## 8. Evolution policy

**Additive-only at v1**:

- Adding a new `code` ✔ (agents must treat unknown codes as "unknown;
  inspect `stage` for routing, fall back to `exit_code` family").
- Adding a new `Fix` variant ✔ (agents must ignore unknown `kind`).
- Adding a new optional field ✔ (consumers must ignore unknown keys).
- Adding a new `Stage` variant ✔ (unknown stage → inspect `code` prefix).

**Requires `v` bump**:

- Renaming or removing any code, stage, or Fix variant.
- Changing the semantics of an existing field.
- Making a previously-optional field required or vice versa.
- Changing `id` hash inputs or algorithm (including FNV → successor).

When `v` bumps, the previous format stays available for at least one
minor release behind a compatibility flag.

### 8.1 Stability

Schema `v1` is currently marked `pre-release`. While `pre-release`,
non-additive shape changes are permitted without a `v` bump —
downstream consumers should pin to a specific commit rather than rely
on `v1` stability. The flip to `stable` is a deliberate editorial act,
not an automated threshold: a maintainer decides the schema has
settled (e.g. an external consumer has committed to the format, or
the surface has been stable long enough that further churn is
unlikely) and lands a single commit that updates both `SCHEMA_STATUS`
and the schema file's `x-sce-schema-status`. Once `stable`, §8's
evolution rules apply strictly — any non-additive change requires a
`v2` schema coexisting with `v1` for at least one minor release
cycle.

The current status is encoded in `SCHEMA_STATUS` at
`sce-build/src/forge/diagnostic.rs` and emitted as `x-sce-schema-status`
at the top of `schemas/sce-diagnostic.v1.schema.json`, so downstream
consumers can read the signal without linking the crate. The
`schema_file_declares_status` test asserts the two agree; any change
to the const must update the schema file (or vice versa) in the same
commit.

This surface is one row in the cross-surface stability registry
`SCE_WIRE_CONTRACTS.md`, which states the shared `pre-release` policy
and flip-to-`stable` procedure for every agent-facing wire surface.

## 9. Reference implementation

- Trait: `sce_build::forge::diagnostic::ToDiagnostics`
- Record: `sce_build::forge::diagnostic::Diagnostic`
- JSON Schema: `schemas/sce-diagnostic.v1.schema.json` — draft-07
  validator input for consumers that do not link the sce-build
  library. Drift is caught by
  `json_schema_enums_match_rust_source_of_truth` in
  `sce-build/src/forge/diagnostic.rs`: the test reads the schema at
  compile time (via `include_str!`) and asserts that `properties.code.enum`
  and `properties.stage.enum` match `DiagnosticCode::as_str` and
  `Stage::as_str` in source order.
- Spec anchors: `DiagnosticCode::spec_anchor` in
  `sce-build/src/forge/diagnostic.rs` maps each code to the
  authoritative section. Emission sites route through the method;
  the per-code table here is the documented counterpart.
- Goldens: `sce-build/src/forge/diagnostic.rs` →
  `diagnostic_goldens_are_byte_stable` test.
- Non-overlap invariant: `sce-build/src/forge/diagnostic.rs` →
  `non_overlap_class`, `fix_carries_candidates_emitters_obey_non_overlap`,
  `expected_is_metadata_emitters_obey_non_overlap`. The classification
  `match` is exhaustive over `DiagnosticCode`, so adding a new code
  fails the build until the author places it in one of the three
  buckets — the contract cannot drift from the implementation.
- CLI integration: `sce-build/tests/error_format_json.rs` (forge/CLI
  boundary) and `sce-build/tests/mesh_error_format_json.rs` (mesh
  codegen via `--deploy`).
- Emitter: `sce-build/src/bin/sce_codegen.rs` →
  `ErrorFormat::emit_and_exit` and `cli_exit`.
- Flag naming rationale: `docs/adr/0001-error-format-flag-naming.md`
  documents the alternatives considered for the `--error-format`
  flag and the reason the current spelling is compatible with future
  wire shapes (SARIF, protobuf).

## 10. Stdout manifest

On success, `sce-codegen generate` and `sce-codegen check` each write
exactly one JSON line to stdout — nothing more, nothing less. The wire
schema is `schemas/sce-manifest.v1.schema.json`; the surface's
stability status is registered in `SCE_WIRE_CONTRACTS.md`. The shape
is:

```json
{
  "v": 1,
  "kind": "generate",
  "generator": "b497eacf7d94",
  "artifacts": [
    {"path": "/abs/path/foo_sm.rs"}
  ],
  "needs_script_engine": true,
  "script_engine_causes": [
    {"kind": "transition-guard", "state": "idle",
     "location": {"file": "m.scxml", "line": 7, "col": 5}},
    {"kind": "datamodel-variable-init", "var": "retry",
     "location": {"file": "m.scxml", "line": 5, "col": 15}}
  ],
  "rejected": {"spec": "W3C SCXML 5.8", "name": "untestable_doc"},
  "deploy": {"static_analyzer": "coverity"}
}
```

### 10.1 Fields

| Field | Type | Semantics |
|---|---|---|
| `v` | integer | Schema version. Currently `1`. Bumped under the same policy as the error contract ([§8](#8-evolution-policy)). |
| `kind` | string | Which subcommand produced this manifest. Consumers branch on this before deserialising into a subcommand-specific shape. `"generate"` or `"check"`; the enumeration is `ManifestKind` and the schema's `kind.enum` is pinned to it. |
| `generator` | string | Commit of the `sce-codegen` build that produced these artifacts, or `"unknown"` when the build had no git checkout to read (vendored crate, release tarball). The crate version is frozen pre-1.0 and identifies nothing, so this is the field to record when attributing a committed artifact to a generator — reading it here costs no extra invocation and replaces a hand-maintained version sidecar. Identifies the committed state the generator was built from; uncommitted edits to the generator are not reflected (the §6.2.6 hashes cover the inputs, not the binary). Also available as `sce-codegen --version`. |
| `artifacts` | array of `{path}` objects | Absolute path of every file written during the run, in emission order. Entries are objects (not bare strings) so the schema can grow additively — future fields (`size`, `hash`, `artifact_kind`) must extend the object without a `v` bump. |
| `needs_script_engine` | bool | Whether the compiled machine embeds ECMAScript requiring a runtime engine. |
| `script_engine_causes` | optional array of objects | **Why** `needs_script_engine` is `true` — one record per construct that forced the engine in. Present exactly when the flag is `true`; omitted (not `[]`) otherwise, so a pure-static manifest carries no new bytes. See [§10.4](#104-script-engine-causes). |
| `rejected` | optional object | Present only when the input triggered a W3C-spec rejection (currently `W3C SCXML 5.8`, "untestable manifest") and stub files were written in place of generated code. Absence means clean generation. Fields: `spec` (e.g. `"W3C SCXML 5.8"`) and `name` (the document's `name` attribute). |
| `deploy` | optional object | Declarations read out of `--deploy` that SCE records without acting on. Omitted whole when the run had no deploy or the deploy declared none of them, so a deploy-unaware manifest carries no new bytes. See [§10.5](#105-deploy-declarations). |

### 10.2 Stream discipline

- On **failure** the manifest is not emitted; stdout is empty and the
  NDJSON diagnostic on stderr is the sole signal. Agents must
  treat stdout as valid only when the process exit code is `0`.
- No legacy prose is emitted. Anything grepping for `Generated:`,
  `Needs ScriptEngine:`, `Document rejected:`, or `Reason:` in
  stdout is reading a format that was removed in v1 and will never
  return — pinned by
  `tests/error_format_json.rs::stdout_does_not_emit_human_prose`.
- The manifest is one line — no pretty-printing, no trailing
  whitespace, terminated by a single `\n`.

### 10.3 On-disk enforcement

- Schema: `schemas/sce-manifest.v1.schema.json`, registered as a wire
  surface in `SCE_WIRE_CONTRACTS.md`.
- Structs: `Manifest`, `ManifestKind`, `ArtifactEntry`, `RejectedInfo`,
  `DeployInfo`, `LanguageVerdict` in `sce-build/src/manifest.rs`;
  `ScriptEngineCauseRecord` in `sce-build/src/script_engine_analyzer.rs`.
  They live in the library rather than beside the CLI so the
  schema-lockstep guards run where the cross-surface registry test can
  reach them.
- Emitter: `build_manifest` in `sce-build/src/bin/sce_codegen.rs`, the
  single construction point for both subcommands, reached from
  `emit_generate_manifest` at every `cmd_generate` exit and from
  `cmd_check` / `cmd_check_document_set`.
- Tests: `manifest.rs::tests` pins the producer constants to the schema
  file and validates produced records against it, positively and
  negatively;
  `tests/error_format_json.rs::stdout_emits_single_json_manifest_on_success`
  (positive pin) and `::stdout_does_not_emit_human_prose` (negative
  pin); `tests/cli_check.rs` and `tests/cli_check_cross_doc.rs`
  validate emitted `check` manifests — single-document and
  document-set — against the schema through the real binary.

### 10.3.1 `check` manifests

`sce-codegen check` reaches the verdict `generate` would and writes
nothing. Two fields carry that difference:

- `artifacts` is always `[]`. This is the subcommand's contract, not a
  property of the input — pinned by
  `tests/cli_check.rs::check_writes_no_file_anywhere`, which also
  re-lists the working directory and the input's directory to confirm
  no file appeared in either.
- `languages` is present only on a `check` manifest: one verdict per
  backend, each `{"language", "status"}` plus a `code` when the status
  is `"rejected"`.

The exit code splits the two refusal axes. A **document-axis** refusal
(`xml/*`, `validation/*`, `scxml/*`) is fatal — the document is wrong
under every backend, stdout stays empty per [§10.2](#102-stream-discipline).
A **backend-axis** refusal (`generate/*`, `codegen/*`) is fatal only
when the operator named the backend with `--language`, so `check -l X`
and `generate -l X` always agree; that agreement is swept over the
fixture corpus by
`tests/cli_check.rs::check_and_generate_agree_on_every_document_and_backend`.
Without `--language` every backend is checked, the per-backend verdict
rides `languages`, and the exit is `0`: "only the Rust backend can
lower this document" is an answer, not a failure.

#### 10.3.1.1 Document sets

The producer a `check` run mirrors is the one its invocation shape
names. A lone document is checked against `generate`; a **document
set** — any `--scxml`, `--forge`, or `--deploy` — is checked against
`orchestrate`, so the cross-doc registry is built and the
§synth-5-K / §synth-5-M deploy validators fire instead of
silent-skipping. Both routes emit the same `kind: "check"` record.

This matters because `orchestrate` has no no-write mode: it requires an
`--output-dir` and materialises the whole build into it, so asking
"is this system valid?" used to cost a tree of artifacts. Two claims
keep the routes honest, both swept in
`tests/cli_check_cross_doc.rs`:

- `check … ≡ orchestrate …` on exit code and diagnostic code, over
  every deploy variant × backend
  (`check_and_orchestrate_agree_on_every_document_set_and_backend`).
  The sweep asserts that the refusing variants refuse on every backend
  that lowers the set, so agreement cannot be satisfied by two commands
  that both skip the validators.
- Nothing is written
  (`check_over_a_document_set_writes_no_file_anywhere`), asserted
  against a control run of `orchestrate` over the same set that does
  write.

On this route `needs_script_engine` describes the **set**: the union
over its statechart inputs, which is the form the question takes for a
build system deciding whether to link an engine. `-I`,
`--strict-unresolved` and `--no-std` are refused rather than accepted
and ignored — the multi-doc compile entry point resolves includes
relative to each document with no search path and renders no `no_std`
variant, so honouring them would answer a question no producer can
reproduce.

### 10.4 Script-engine causes

`needs_script_engine` alone tells a consumer that a machine lost its
pure-static lowering, but not what cost it. A build that gates on the
flag — an MCU target with no engine to embed, a deployment that must
stay deterministic and replayable — then fails with nothing to act on.
`script_engine_causes` names the construct, so the gate can point at a
line of SCXML.

This is a **non-fatal degradation report**, the same role `rejected`
plays: generation succeeded, but the output is weaker than the author
may have intended. It is carried here rather than as a diagnostic
because diagnostics have no severity — every record on stderr is a
rejection by construction ([§1](#1-streams)) — and because a clean run
is pinned to an empty stderr. Falling back to the script engine is a
supported outcome (`static_hybrid`); falling back *silently* is what
this field ends.

Each record is `{"kind": "<token>", …anchors, "location": {…}}`. `kind`
is a stable kebab-case token; consumers dispatch on it and **must**
tolerate unknown values, since new constructs may be added.

`location` is the offending element's own `{file, line, col}` — the same
[location object](#22-location-object) a diagnostic carries, so tooling
anchors a degradation exactly as it anchors a rejection. It is absent
only for a cause with no single element to blame (a `<script src=…>` the
parser could not read).

The identifier anchors are optional and flat — at most one per record,
so a consumer reads `cause.state` / `cause.invoke` without matching on a
nested union:

| Anchor | Present on |
|---|---|
| `state` | Causes owned by a state or one of its transitions. |
| `var` | `datamodel-variable-init` — the `<data id>`. |
| `param` | `send-param-expr` — the `<param name>` (alongside `state`). |
| `invoke` | Invoke-anchored causes — the `<invoke id>`. |

| `kind` | Construct |
|---|---|
| `datamodel-variable-init` | `<data>` with an `expr` / `src` / content initializer. |
| `global-script` | Top-level `<script>`. |
| `unresolved-external-script` | `<script src=…>` the parser could not load (WASM `parse_string`). |
| `transition-guard` | `<transition cond=…>` evaluated by the engine. **A typed `_event.data` guard that did not lower natively lands here** — see below. |
| `send-namelist` | `<send namelist=…>`. |
| `send-param-expr` | `<send><param expr=…>` that is not a static literal. |
| `send-dynamic-attr` | `<send>` with `eventexpr` / `targetexpr` / `delayexpr` / `typeexpr` / `contentexpr` / `idlocation`. |
| `if-condition`, `elseif-condition` | `<if cond=…>` / `<elseif cond=…>`. |
| `assign-action` | `<assign>`. |
| `log-expr` | `<log expr=…>`. |
| `inline-script-action` | Inline `<script>` executable content. |
| `cancel-expr` | `<cancel sendidexpr=…>`. |
| `foreach-action` | `<foreach>`. |
| `hybrid-invoke` | `<invoke srcexpr=…>` / `contentexpr`. |
| `static-invoke-namelist` | `<invoke namelist=…>`. |
| `mesh-rpc-srcexpr` | `<invoke type="sce:mesh-rpc">` with an `srcexpr` target. |
| `donedata-param`, `donedata-content` | `<donedata>` `<param>` / `<content expr=…>`. |
| `child-invoke-needs-script-engine` | A statically-invoked child whose own analysis required an engine. |

**Typed guards.** A `cond` reading `_event.data.<field>` from an
imported EventSchema lowers to a native payload comparison and needs no
engine ([`docs/SCE_ACCEPTED_SUBSET.md`](docs/SCE_ACCEPTED_SUBSET.md)
§3.4). When it *cannot* — the guard also references a datamodel
identifier or a function call, which has no binding inside the generated
`matches!`, or the schema declares an enum-typed field — the document
stays legal and keeps the engine, and the fallback appears here as
`transition-guard`. A guard that does not parse as a Forge expression at
all (e.g. `==` instead of `===`) is a *rejection*, not a cause: it never
reaches the manifest.

The producer is `script_engine_analyzer::analyze`. The parser stores its
result on the model in the same statement that sets
`needs_script_engine`, which is that result's boolean projection — so the
flag and the cause list cannot disagree, and neither is recomputed
downstream from a model a later pass has touched
(`script_engine_analyzer::tests::model_flag_agrees_with_stored_causes`).
`ScriptEngineCauseKind::to_wire` is an exhaustive match: a new cause
variant does not compile until its wire `kind` is chosen.

### 10.5 Deploy declarations

`script_engine_causes` and `rejected` report what the *document* cost.
`deploy` reports what the *deployment* declared — keys under
`deploy.yaml`'s `build:` block that change no emission and gate no
build.

| Field | Type | Semantics |
|---|---|---|
| `static_analyzer` | optional string | Which commercial analyzer the deployment declares it relies on for the ownership contract: `"pc-lint-plus"` or `"coverity"`. Omitted when unstated. |

These exist here because a declaration nothing reads is
indistinguishable from a typo. SCE Protocol-Synthesis RFC §synth-5-E is explicit that
`build.static_analyzer` is "descriptive rather than load-bearing" and
that SCE "does not verify or gate on the claim" — so no diagnostic
fires, no emission changes, and no build fails for its absence. What
would otherwise be left is a key the author writes and the build never
acknowledges, which is the silently-inert shape §2.4 forbids. Echoing
it here is the whole of SCE's part: deploy review reads the declaration
off the build rather than off the author's word.

**When the key is present.** `deploy` appears on a run that applied the
deploy to a state machine. It is absent — even with `--deploy` and a
declaration — when the run never reached that point: a forge document
(`sce:kind="codec"` and friends) takes a pipeline where deploy-derived
model mutations do not apply, and a document rejected under §scxml-5.8
exits before them. In both cases SCE processed no deployment, and
saying otherwise would attribute a declaration to a build that did not
consult it.

The vocabulary is closed, and the closure is the spec's. §synth-5-E
rules out Polyspace (its in-source comments justify findings rather than
describe function behaviour) and Clang-Tidy (the typestate macros expand
to nothing on a C build, so it cannot report the violations it would be
mandated for). Both are refused at parse time with the recognized names
in the diagnostic — vocabulary validation, not adjudication of the
claim.

Producer: `DeployInfo::from_facts` in `sce-build/src/bin/sce_codegen.rs`,
fed by `sce_build::DeployFacts` from `apply_deploy_model_mutations`.
Tests: `tests/deploy_static_analyzer.rs` — each recognized value is
asserted to reach the manifest *and* to differ from the other, so a
reader reporting a constant fails; and the emitted code is asserted
identical to a run whose deploy gained only a YAML comment, so nothing
may come to depend on the key.
