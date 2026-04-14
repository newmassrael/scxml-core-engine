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
| **stdout** | Success artifacts: generated file paths, progress lines, manifest JSON produced by `manifest` / `list-fixtures` subcommands. Never carries diagnostics. |
| **stderr** | Exactly one NDJSON diagnostic per line when `--error-format=json`. In `human` mode, free-form text. |

Agents **must** split the two streams by fd. A parser that reads
stdout looking for errors is reading the wrong stream.

## 2. Record shape

```json
{
  "v": 1,
  "id": "fnv1a:1c56b923b2b2b87f",
  "code": "validation/missing-attribute",
  "stage": "validation",
  "spec": "W3C SCXML 3.13",
  "message": "sce:field must have an 'id' attribute",
  "location": {"file": "checkout.scxml", "line": 42, "col": 3},
  "expected": ["u8", "u16", "u32"],
  "actual": "blob",
  "fix": {"kind": "add_attribute", "element": "sce:field", "attr": "id"}
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
| `expected` | array of strings | Legal values the producer would have accepted. |
| `actual` | string | The observed value that triggered rejection. |
| `fix` | object | Structured repair proposal. See [§3 Fixes](#3-fixes). |

### 2.2 Location object

```json
{"file": "checkout.scxml", "line": 42, "col": 3}
```

`line` and `col` are optional; `file` is required when the object is present.
Mesh errors currently omit `location` — their coordinates are the
machine / binding / target names carried by the error fields themselves.

## 3. Fixes

`fix` is present **only** when the repair is deterministic and requires
no further judgment. When a legitimate repair exists but demands a
human decision (e.g. "rename the event OR remove the binding"), the
field is absent — agents then use `expected` / `actual` as the
less-structured signal.

### 3.1 Fix variants

| `kind` | Payload | Semantics |
|---|---|---|
| `add_attribute` | `element`, `attr` | Add the named attribute to the named element. For deploy.yaml errors, `element` is a dotted path (`machines.x.bindings.y`). |
| `rename_duplicate` | `what`, `id` | The id `id` of kind `what` appears more than once; rename one occurrence. |
| `remove_fields` | `location`, `fields[]` | At the config path `location`, remove every key in `fields`. The only well-defined repair for reserved-key and unused-entry errors. |

Agents holding a dispatch table keyed on `fix.kind` may safely
enumerate these — the set only grows in backward-compatible ways
([§8](#8-evolution-policy)).

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

## 5. Code catalog

The full enumeration of `code` values, grouped by stage. The set is
extended additively — a code is never renamed or repurposed without
a schema bump ([§8](#8-evolution-policy)).

### 5.1 Forge

| Code | Stage | Fix? |
|---|---|---|
| `xml/parse` | `xml` | no |
| `xml/schema-validation` | `xml` | no |
| `validation/missing-element` | `validation` | no |
| `validation/missing-attribute` | `validation` | `add_attribute` |
| `validation/invalid-attribute` | `validation` | no (use `expected`) |
| `validation/unsupported-kind` | `validation` | no |
| `validation/duplicate-id` | `validation` | `rename_duplicate` |
| `validation/empty-collection` | `validation` | no |
| `validation/count-mismatch` | `validation` | no |
| `validation/incompatible-attributes` | `validation` | no |
| `validation/invalid-reference` | `validation` | no (use `expected`) |
| `validation/invalid-direction` | `validation` | no |
| `validation/numeric-parse` | `validation` | no |
| `validation/empty-value` | `validation` | `add_attribute` |
| `validation/singleton-violation` | `validation` | no |
| `validation/require-either` | `validation` | no (use `expected`) |
| `validation/wrong-pipeline` | `validation` | no |
| `expression/empty` | `expression` | no |
| `expression/lex` | `expression` | no |
| `expression/unsupported-construct` | `expression` | no |
| `expression/strict-equality` | `expression` | no (use `expected`) |
| `expression/parse-mismatch` | `expression` | no |
| `expression/unexpected-token` | `expression` | no |
| `expression/invalid-lvalue` | `expression` | no |
| `expression/type-coercion` | `expression` | no |
| `expression/go-ternary-unsupported` | `expression` | no |
| `import/file-not-found` | `import` | no |
| `import/kind-mismatch` | `import` | no |
| `import/not-forge` | `import` | no |
| `import/read-error` | `import` | no |
| `manifest/circular-dependency` | `manifest` | no |
| `manifest/io` | `manifest` | no |
| `generate/invalid-config` | `generate` | no |
| `generate/template-load` | `generate` | no |
| `generate/template-render` | `generate` | no |
| `io/filesystem` | `io` | no |

### 5.2 CLI

| Code | Stage | Fix? |
|---|---|---|
| `cli/unknown-language` | `cli` | no (use `expected`) |
| `cli/unsupported-language` | `cli` | no |
| `cli/read-input` | `cli` | no |
| `cli/write-output` | `cli` | no |
| `cli/create-output-dir` | `cli` | no |
| `cli/scxml-parse` | `cli` | no |
| `cli/scxml-generate` | `cli` | no |
| `cli/dynamic-features` | `cli` | no |
| `cli/missing-metadata-field` | `cli` | no |
| `cli/not-a-directory` | `cli` | no |
| `cli/invalid-format-option` | `cli` | no (use `expected`) |
| `cli/json-serialization` | `cli` | no |
| `cli/project-root-not-found` | `cli` | no |
| `cli/format-style-not-found` | `cli` | no |
| `cli/no-scxml-tag` | `cli` | no |

### 5.3 Mesh

| Code | Stage | Fix? |
|---|---|---|
| `mesh/deploy-read` | `mesh-deploy` | no |
| `mesh/deploy-parse` | `mesh-deploy` | no |
| `mesh/deploy-unsupported-version` | `mesh-deploy` | no (use `expected`) |
| `mesh/deploy-duplicate-machine` | `mesh-deploy` | no |
| `mesh/external-read` | `mesh-external` | no |
| `mesh/external-parse` | `mesh-external` | no |
| `mesh/external-unresolved-names` | `mesh-external` | no |
| `mesh/external-ambiguous-event-group` | `mesh-external` | no |
| `mesh/external-empty-event-group` | `mesh-external` | no |
| `mesh/external-named-reference-without-config` | `mesh-external` | no |
| `mesh/external-reserved-someip-id-keys` | `mesh-external` | `remove_fields` |
| `mesh/external-someip-field-on-non-someip-transport` | `mesh-external` | no |
| `mesh/external-conflicting-event-schema` | `mesh-external` | no |
| `mesh/external-conflicting-event-field-kinds` | `mesh-external` | no |
| `mesh/external-empty-event-entry` | `mesh-external` | no |
| `mesh/topology-unresolved-targets` | `mesh-topology` | no |
| `mesh/topology-machine-not-found` | `mesh-topology` | no (use `expected`) |
| `mesh/topology-receiver-not-declared` | `mesh-topology` | no |
| `mesh/topology-absolute-source-path` | `mesh-topology` | no |
| `mesh/topology-receiver-source-read` | `mesh-topology` | no |
| `mesh/topology-receiver-source-parse` | `mesh-topology` | no |
| `mesh/topology-uncovered-events` | `mesh-topology` | no |
| `mesh/topology-pattern-capability-violation` | `mesh-topology` | no |
| `mesh/topology-missing-binding-field` | `mesh-topology` | `add_attribute` |
| `mesh/topology-invalid-binding-field` | `mesh-topology` | no |
| `mesh/topology-event-binding-unused` | `mesh-topology` | `remove_fields` |
| `mesh/codegen-unsupported-language` | `mesh-codegen` | no |
| `mesh/codegen-unsupported-transport` | `mesh-codegen` | no |
| `mesh/codegen-template-read` | `mesh-codegen` | no |
| `mesh/codegen-template-render` | `mesh-codegen` | no |
| `mesh/codegen-event-name-collision` | `mesh-codegen` | no |
| `mesh/io` | `io` | no |

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

- Trait: `sce_build::forge::diagnostic::ToDiagnostic`
- Record: `sce_build::forge::diagnostic::Diagnostic`
- Goldens: `sce-build/src/forge/diagnostic.rs` →
  `diagnostic_goldens_are_byte_stable` test.
- CLI integration: `sce-build/tests/error_format_json.rs`.
- Emitter: `sce-build/src/bin/sce_codegen.rs` →
  `ErrorFormat::emit_and_exit` and `cli_exit`.
