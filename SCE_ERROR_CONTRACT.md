# SCE Error Contract

Machine-readable error format produced by `sce-codegen --error-format=json`.
Consumed by upstream automation — LangGraph triage nodes, IDE language
servers, CI repair bots, and any other agent that needs to branch on
SCE's rejection signals without parsing human text.

This document is the **wire contract**. It must move only in the directions
defined in [§8 Evolution Policy](#8-evolution-policy). The on-disk
enforcement is `forge::diagnostic::tests::diagnostic_goldens_are_byte_stable`
— a byte-level golden check that fires on any accidental drift.

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
| absent                                 | SCXML parser     | `cli/scxml-parse`, etc.    |
| `"statechart"`                         | SCXML parser     | `cli/scxml-parse`, etc.    |
| known forge kind (e.g. `"lookup"`)     | Forge            | `xml` / `validation` / ... |
| unknown value (e.g. `"bogus"`)         | Forge            | `xml/schema-validation`    |

The last row is a contract guarantee. An author who wrote
`sce:kind="bogus"` intended a forge document, so the failure must
surface through the forge pipeline — where the bundled XSD identifies
the violation and the `message` field enumerates the legal values.
Reporting such a failure as `cli/scxml-parse` would mis-route repair
agents and is explicitly forbidden.

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
| `validation/numeric-parse` | `validation` | no | |
| `validation/empty-value` | `validation` | `add_attribute` | |
| `validation/singleton-violation` | `validation` | no | |
| `validation/require-either` | `validation` | `add_one_of` | |
| `validation/wrong-pipeline` | `validation` | no | SCE Forge §4 |
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
| `io/filesystem` | `io` | no | |

### 5.2 CLI

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `cli/unknown-language` | `cli` | `replace_one_of` | |
| `cli/unsupported-language` | `cli` | no | |
| `cli/read-input` | `cli` | no | |
| `cli/write-output` | `cli` | no | |
| `cli/create-output-dir` | `cli` | no | |
| `cli/scxml-parse` | `cli` | no | |
| `cli/scxml-generate` | `cli` | no | |
| `cli/dynamic-features` | `cli` | no | |
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

On success, `sce-codegen generate` writes exactly one JSON line to
stdout — nothing more, nothing less. The shape is:

```json
{
  "v": 1,
  "kind": "generate",
  "artifacts": [
    {"path": "/abs/path/foo_sm.rs"}
  ],
  "needs_script_engine": false,
  "rejected": {"spec": "W3C SCXML 5.8", "name": "untestable_doc"}
}
```

### 10.1 Fields

| Field | Type | Semantics |
|---|---|---|
| `v` | integer | Schema version. Currently `1`. Bumped under the same policy as the error contract ([§8](#8-evolution-policy)). |
| `kind` | string | Which subcommand produced this manifest. Agents branch on this before deserialising into a subcommand-specific shape. Currently only `"generate"`. |
| `artifacts` | array of `{path}` objects | Absolute path of every file written during the run, in emission order. Entries are objects (not bare strings) so the schema can grow additively — future fields (`size`, `hash`, `artifact_kind`) must extend the object without a `v` bump. |
| `needs_script_engine` | bool | Whether the compiled machine embeds ECMAScript requiring a runtime engine. |
| `rejected` | optional object | Present only when the input triggered a W3C-spec rejection (currently `W3C SCXML 5.8`, "untestable manifest") and stub files were written in place of generated code. Absence means clean generation. Fields: `spec` (e.g. `"W3C SCXML 5.8"`) and `name` (the document's `name` attribute). |

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

- Structs: `GenerateManifest`, `ArtifactEntry`, `RejectedInfo` in
  `sce-build/src/bin/sce_codegen.rs`.
- Emitter: `emit_generate_manifest` in the same file, called at every
  `cmd_generate` exit point.
- Tests: `tests/error_format_json.rs::stdout_emits_single_json_manifest_on_success`
  (positive pin) and `::stdout_does_not_emit_human_prose` (negative pin).
