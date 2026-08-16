# SCE Error Contract

Machine-readable error format produced by `sce-codegen --error-format=json`.
Consumed by upstream automation — LangGraph triage nodes, IDE language
servers, CI repair bots, and any other consumer that needs to branch on
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

Consumers **must** split the two streams by fd. A parser that reads
stdout looking for errors is reading the wrong stream.

**A diagnostic does not imply failure.** Most do — the record is emitted
and the process exits non-zero — but some verdicts are reached at
generation time and are obliged to let the run finish: W3C SCXML §5.9.1
requires an unevaluable `cond` to raise `error.execution` at runtime
rather than be refused at build time, so the document must still
generate. Such a run emits its records on stderr **and** its manifest on
stdout, and exits `0`. No severity field carries this: [§10.2](#102-stream-discipline)
already makes the exit code the thing that says whether stdout is valid,
so `records + exit 0 + manifest` and `records + non-zero exit + empty
stdout` are already distinguishable. A consumer that treats any record
as fatal will refuse documents the W3C conformance suite requires.
`sce-codegen --lint` is the opt-in that promotes the reportable ones to
fatal, for authors who have no conformance excuse for carrying them.

## 2. Record shape

```json
{
  "v": 1,
  "id": "fnv1a:dd04a37de468ffb4",
  "generator": "2be6e02c2c2c",
  "code": "validation/invalid-attribute",
  "stage": "validation",
  "message": "sce:field: unknown sce:type value 'blob' (expected: u8, u16, u32)",
  "location": {"file": "checkout.scxml", "line": 42, "col": 3},
  "actual": "blob",
  "fix": {"kind": "replace_one_of", "candidates": ["u8", "u16", "u32"]}
}
```

All fields except `v`, `id`, `generator`, `code`, `stage`, and `message`
are optional. Omitted fields are absent from the JSON entirely (not
`null`). Consumers **must** ignore unknown fields for forward
compatibility.

### 2.1 Field semantics

| Field | Type | Guarantee |
|---|---|---|
| `v` | integer | Schema version. Currently `1`. First key in every record. |
| `id` | `fnv1a:<16hex>` | Content hash over `(code, stage, location.file, key_fragments)`. Same semantic error → same id, **independent of message rewording**. Use for dedup, caching, "seen this before" checks. Shared across producers — see [§2.1.1](#211-key-fragments-and-the-id-namespace). |
| `generator` | short commit, or `unknown` | Commit of the generator that emitted the record — the same value [§10](#10-stdout-manifest)'s manifest carries as `generator` and `--version` reports in parentheses. Present on **every** record, because a rejected run writes no manifest (stdout is empty, [§1](#1-streams)): on the failure path this record is the only thing the consumer receives. [§8.1](#81-stability) tells consumers to pin a specific commit rather than rely on `v1` while the schema is `pre-release`, and that is unfollowable if the payload does not name the commit it came from. `unknown` when the build had no git checkout to read (vendored crate, release tarball). |
| `code` | slash-path string | Closed enum. See [§5 Code Catalog](#5-code-catalog). Consumers dispatch on `code`, never on `message`. |
| `stage` | lowercase / kebab-case string | Pipeline stage. Routes to the correct repair loop. See [§4 Stage Taxonomy](#4-stage-taxonomy). |
| `spec` | string | Specification anchor (e.g. `"W3C SCXML 3.13"`). Present when the rule is spec-derived. Enables LLM grounding. |
| `message` | English prose | Human-readable one-liner. **Not** machine-parsed. Not part of `id`. |
| `location` | object | Source location when known. See [§2.2](#22-location-object). |
| `expanded_from` | object | The `<sce:use>` whose parameters synthesised `actual`. Present only when a preprocessor assembled the rejected value. See [§2.3](#23-expanded-values). |
| `expected` | array of strings | Non-repair expectation metadata (parser expectations like `"identifier"`, cardinality constraints like `"1"`). **Never** carries a candidate list for substitution — that role belongs to `fix`. The two fields are disjoint by contract (see [§3.2](#32-no-overlap-between-fix-and-expected)). |
| `actual` | string | The observed value that triggered rejection. |
| `fix` | object | Structured repair proposal. The sole channel for repair signals. See [§3 Fixes](#3-fixes). |
| `spec_provenance` | array of objects | NL→IR Mapping Roadmap Item 6 — spec-document anchors that justify the rejected node (`doc_id` + optional `rev`/`section`/`page`). SCE never infers this; IR generators (NL→IR pipelines, ARXML transcoders) populate it when they know the spec origin. Pass-through field on the wire — absent when the upstream did not record provenance. |
| `question_kind` | string (enum) | NL→IR Mapping Roadmap Item 6 — coarse routing label so IDE / triage tooling can dispatch on the *kind* of question the diagnostic raises (`implicit_default` / `ambiguous_mapping` / `cross_doc_conflict` / `unit_unspecified` / `unknown_vocabulary` / `structural`). Extensible — consumers must treat unknown values as `structural`. Absent on purely structural rejections that map cleanly onto `code` alone. |

### 2.1.1 Key fragments and the id namespace

`id` is a **shared namespace, not a per-producer one**. Two producers
emit records for the same document — the Rust `sce-codegen` CLI and the
C++ runtime parser, which conforms to this same schema — and a consumer
reading both (the W3C harness generates with one and loads with the
other) must fold one logical error into one entry. That only holds if
both derive the id from the same inputs, so `key_fragments` carry a
rule:

> A key fragment must be a value **SCE itself determined** — a name
> from the document, a resolved path, a search trail, an enforced
> limit, an SCE-authored classification.

Text produced by a third-party parser is not such a value. It differs
between the two XML engines (roxmltree here, pugixml in the runtime
parser) and shifts when either is upgraded, so hashing it makes `id`
unreproducible for the same document — across producers, and across a
dependency bump within one producer. `xml/parse` therefore carries **no**
fragments (a document has one parse failure; `code|stage|file` names
it), and `xml/xinclude-malformed` / `xml/template-malformed` key on the
href or template name alone, with the engine's reason travelling in
`message`. The same rule empties `generate/template-load` and
`generate/template-render`, whose only payload is the template
renderer's own text — an audit of all 212 fragment-bearing arms found
those two, and every other `detail` in the set is a sentence SCE
writes itself.

`tests/parsing/CrossProducerDiagnosticId_test.cpp` is what enforces
this: it runs both producers over one fixture document and compares the
records. A leaf that hashes something the other producer cannot
reproduce reds it.

### 2.2 Location object

```json
{"file": "checkout.scxml", "line": 42, "col": 3}
```

`line` and `col` are optional; `file` is required when the object is present.

`file` names the document **as the caller named it** — the path passed
to `check` / `generate`, not its basename. A consumer opens it to apply
a fix, and `file` is one of the id's hash inputs, so a producer that
shortens it cannot share a dedup key with one that does not. Artifacts
are the other way round: an SCE-MAP marker or a provenance record lands
in generated source, so it carries the basename rather than baking one
machine's checkout into the tree. Same document, two spellings, chosen
by who reads them.

Mesh errors currently omit `location` — their coordinates are the
machine / binding / target names carried by the error fields themselves.

### 2.3 Expanded values

`<xi:include>` and `<sce:use>` run before parsing, so a rejection can
name a value the pipeline assembled rather than one an author typed.
`location` still points at real source — but at the *template* row,
which carries the parameterised shape:

```
counter.scxml:12   <transition event="tick" target="tick_{$n}"/>
main.scxml:19        <sce:use template="counter.scxml" n="1"/>
```

Rejecting the expansion `tick_1` gives `actual: "tick_1"`,
`location: counter.scxml:12`, and `expanded_from: main.scxml:19`. All
three are true and none is redundant: `actual` is what failed to
resolve, `location` is where that value takes its shape, and
`expanded_from` is what chose the parameters — the only coordinate that
distinguishes this expansion from its siblings.

Two consequences for consumers:

- **`actual` need not occur in `location`'s row when `expanded_from`
  is present.** That row holds `tick_{$n}`. Outside this case it does
  occur there, and [§3.1.1](#311-locating-the-edit) is what enforces
  it.
- **No substitution `fix` accompanies such a record.** Rewriting the
  template row would change every expansion of the template, so no
  local edit repairs the failure — the case [§3](#3-fixes) already
  spells as "`fix` absent". The repair is the author's judgment
  between fixing the template, the call site's parameters, or the
  document the expansion refers into, and the three coordinates are
  what let a consumer state that choice.

## 3. Fixes

`fix` is the **sole channel** for repair signals. Whenever the producer
can name a change that would satisfy the rejected constraint — single
value, closed candidate list, attribute to add — the payload lives on
this field, never on `expected`. Consumers drive repair by dispatching on
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
| `replace_one_of` | `candidates[]` | Replace `actual` with one of `candidates`. Emitted when the producer knows the closed set of legal values (attribute-value enums, cross-reference resolution, supported language list) but cannot pick a single answer — the consumer or human chooses from the list. |
| `add_one_of` | `element`, `attrs[]` | Add one of the listed `attrs` to `element`. Used for "require either X or Y" constraints (e.g. `<send>` needs `event` or `eventexpr`). Choice-based. |

The variant name encodes the *shape* of the repair: deterministic
variants can be applied without further judgment; choice variants
(`replace_one_of`, `add_one_of`) require the consumer or human to pick
from the closed candidate set.

A choice variant's set is never empty. There is no choosing from
nothing, so a producer that reaches that state has found no repair
rather than a degenerate one, and the record carries no `fix` at all —
the spelling [§3](#3-fixes) already defines for that case. Consumers
therefore never have to decide what `"candidates": []` means, and the
producer cannot ship the two readings of it from different sites.

### 3.1.1 Locating the edit

Every `fix` names an edit the consumer performs on the document in
[`location.file`](#22-location-object). Two properties make that
possible, and both are enforced against the emitted corpus by
`sce-build/tests/diagnostic_fix_is_applicable.rs` rather than asserted
here:

- When the record carries `location.line`, the value in `actual` occurs
  on that line. A coordinate on an enclosing element — the `<send>`
  around the offending `<param>`, say — sends the consumer to a line
  the token is absent from.
- When it does not, `actual` occurs exactly once in the document, since
  a whole-file search is then the only locating strategy the wire
  offers.

The same target replays each substitution proposal against the CLI:
applying it must clear the record's `id`.

Consumers holding a dispatch table keyed on `fix.kind` may safely
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

Consumers that want "the closed set of legal values" should always read
`fix`. Consumers that want "what the producer was grammatically expecting
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

`stage` is the repair-routing key for consumers. Its value is determined
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
mis-route repair consumers and is explicitly forbidden.

Malformed XML that never reaches the root attribute list falls back
to the SCXML parser: intent cannot be inferred, and the SCXML
parser's XML-level diagnostic is the least-wrong answer.

The on-disk enforcement of this rule is
`tests/error_format_json.rs::json_mode_routes_unknown_sce_kind_through_forge_pipeline`.
The routing primitive is `sce_build::classify_document` in
`sce-build/src/lib.rs`.

## 5. Code catalog

The full enumeration of `code` values, grouped by the pipeline that
produces them. The set is extended additively — a code is never renamed
or repurposed without a schema bump ([§8](#8-evolution-policy)).

**The tables below are generated**, from the golden record each code
emits plus `DiagnosticCode::spec_anchor`. They were hand-maintained
until 2026-08-12, and by then named 96 of 346 codes while opening with
the sentence above: fifteen stages had no row at all, and no test read
this file in either direction, so the claim had nothing holding it up.
Completing the table by hand would have created a second enumeration to
keep true beside `ALL_DIAGNOSTIC_CODES`, which is the shape that went
stale in the first place. Regenerate with:

```
UPDATE_EXPECT=1 cargo test -p sce-build error_contract_catalog
```

The `Stage` column is the record's `stage` field, which is **not** the
code's prefix — 250 of the 346 codes differ, so a consumer must branch
on `stage` rather than on the text before the slash.

The `Fix?` column names the `fix.kind` the code's golden carries, or
`no` where the record has none. A code emitted from sites that name
different repairs lists each, separated by `/`: `fix` is a property of
the failure, not of the code, and a single value would be a claim the
goldens do not support.

The `Spec` column names the authoritative section that defines the rule
being enforced. An empty `Spec` column means the code records an
operational failure (I/O, template render, argument parsing) rather
than a specification violation. Section references come from
`DiagnosticCode::spec_anchor` in `sce-build/src/forge/diagnostic.rs` and
must point at a real section — adding a plausible-looking anchor for a
rule that is not actually documented there is strictly worse than
leaving the column empty, because consumers ground hallucinated
references against a real document and drift silently.

<!-- BEGIN GENERATED: code catalog -->

### 5.1 Forge

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `algorithm/append-target-not-buffer` | `validation` | no | SCE Forge §4.12 |
| `algorithm/append-type-mismatch` | `validation` | no | SCE Forge §4.12 |
| `algorithm/bc-mutation-forbidden` | `validation` | no | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/call-arg-count-mismatch` | `validation` | no | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/call-target-method-unknown` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/call-target-unknown` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/const-fold-budget-exceeded` | `generate` | no | SCE Protocol-Synthesis RFC §5.F |
| `algorithm/const-not-foldable` | `generate` | no | SCE Protocol-Synthesis RFC §5.F |
| `algorithm/const-yield-type-mismatch` | `generate` | no | SCE Protocol-Synthesis RFC §5.F |
| `algorithm/foreach-source-bc-with-bytes-item-type` | `validation` | no | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/foreach-source-not-iterable` | `validation` | no | SCE Protocol-Synthesis RFC §5.A + §5.L |
| `algorithm/local-shadows-param` | `validation` | no | SCE Protocol-Synthesis RFC §5.A |
| `algorithm/lvalue-unsupported` | `validation` | no | SCE Protocol-Synthesis RFC §5.A |
| `algorithm/return-missing` | `validation` | no | SCE Protocol-Synthesis RFC §5.A |
| `algorithm/test-vector-unsupported-kind` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/dma-alignment-unsatisfiable` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-bind-carrier-after-embed` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-bind-duplicate-input` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-bind-input-not-declared` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-bind-source-not-resolved` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-bind-width-mismatch` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/flag-input-unbound` | `validation` | `add_attribute` | SCE Protocol-Synthesis RFC §5.B |
| `codec/peek-byte-flag-layout-mismatch` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/present-if-refs-later-field` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/repeat-count-refs-later-field` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/tlv-chain-depth-unspecified` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/tlv-chain-truncate-under-entry-flag` | `validation` | `replace_with` | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-arm-body-caller-tag-unsupported` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-arm-inner-mid-undeclared` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-arm-mid-mismatch` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-arm-unreachable` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-default-overlay-arm-not-declared` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-dispatch-arms-not-distinguishable-without-default` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-dispatch-bit-width-mismatch` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-dispatch-carrier-after-embed` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-dispatch-flag-has-static-value` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-dispatch-flag-not-resolved` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-duplicate-default-arm` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codec/variant-no-default-arm` | `validation` | no | SCE Protocol-Synthesis RFC §5.B |
| `codegen/generic-kind-backend-emit-missing` | `generate` | no |  |
| `codegen/mcu-class-kind-on-non-mcu-language` | `generate` | no |  |
| `codegen/no-std-fs-load-not-supported` | `generate` | no | SCE Protocol-Synthesis RFC §5.J.2 |
| `codegen/no-std-http-not-supported` | `generate` | no | SCE Protocol-Synthesis RFC §5.J.2 |
| `codegen/no-std-invoke-not-supported` | `generate` | no | SCE Protocol-Synthesis RFC §5.J.2 |
| `codegen/no-std-script-not-supported` | `generate` | no | SCE Protocol-Synthesis RFC §5.J.2 |
| `collection/capacity-unresolved` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.L |
| `collection/element-type-not-a-kind` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.L |
| `collection/index-by-field-missing` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.L |
| `collection/multi-writer-without-atomics` | `validation` | no | SCE Protocol-Synthesis RFC §5.L |
| `collection/ordering-sorted-requires-index-by` | `validation` | no | SCE Protocol-Synthesis RFC §5.L |
| `collection/overflow-policy-oldest-wins-requires-ordering-insertion` | `validation` | no | SCE Protocol-Synthesis RFC §5.L |
| `expression/empty` | `expression` | no | SCE Forge §3.4 |
| `expression/go-ternary-unsupported` | `expression` | no | SCE Forge §3.4 |
| `expression/invalid-lvalue` | `expression` | no | SCE Forge §3.4 |
| `expression/lex` | `expression` | no | SCE Forge §3.4 |
| `expression/parse-mismatch` | `expression` | no | SCE Forge §3.4 |
| `expression/property-not-callable` | `expression` | `replace_with` / no | W3C SCXML §B.2 |
| `expression/strict-equality` | `expression` | `replace_with` | SCE Forge §3.4 |
| `expression/type-coercion` | `expression` | no | SCE Forge §3.4 |
| `expression/unexpected-token` | `expression` | no | SCE Forge §3.4 |
| `expression/unknown-identifier` | `expression` | `replace_one_of` / no | W3C SCXML §B.2 |
| `expression/unsupported-builtin` | `expression` | `replace_one_of` | W3C SCXML §B.2 |
| `expression/unsupported-construct` | `expression` | no | SCE Forge §3.4 |
| `extern/abi-mismatch` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.I |
| `extern/ordering-unspecified` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.I |
| `extern/signature-mismatch` | `validation` | `replace_with` | SCE Protocol-Synthesis RFC §5.I |
| `extern/symbol-not-in-whitelist` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.I |
| `extern/target-plugin-symbol-conflict` | `validation` | no | SCE Protocol-Synthesis RFC §5.I |
| `generate/invalid-config` | `generate` | no |  |
| `generate/template-load` | `generate` | no |  |
| `generate/template-render` | `generate` | no |  |
| `generate/unsupported-feature` | `generate` | no |  |
| `import/file-not-found` | `import` | no |  |
| `import/kind-mismatch` | `import` | `replace_with` |  |
| `import/not-forge` | `import` | no |  |
| `import/read-error` | `import` | no |  |
| `io/filesystem` | `io` | no |  |
| `link/backpressure-undeclared` | `validation` | no | SCE Protocol-Synthesis RFC §5.C |
| `link/class-unsupported-on-target` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.C |
| `link/deploy-role-listener-without-scxml-accept-side-role` | `validation` | no |  |
| `link/framer-missing` | `validation` | no | SCE Protocol-Synthesis RFC §5.C |
| `link/framer-ref-not-declared` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.C |
| `link/inbound-event-queue-unsized` | `validation` | no | SCE Protocol-Synthesis RFC §5.N |
| `link/link-class-unknown` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.C |
| `link/listener-link-not-paired-with-established-sibling` | `validation` | no | SCE Protocol-Synthesis RFC §5.C |
| `link/pool-ref-not-declared` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.C |
| `link/pool-slot-smaller-than-framer-max` | `validation` | no | SCE Protocol-Synthesis RFC §5.C |
| `link/role-listener-with-non-session-arming-trust-class` | `validation` | no |  |
| `manifest/circular-dependency` | `manifest` | no |  |
| `manifest/io` | `manifest` | no |  |
| `mcu/driver-header-not-found` | `validation` | no | SCE Protocol-Synthesis RFC §5.2 |
| `mcu/section-attribute-name-invalid` | `generate` | no | SCE Protocol-Synthesis RFC §5.2 |
| `mcu/section-attribute-on-non-mcu-target` | `generate` | no | SCE Protocol-Synthesis RFC §5.2 |
| `mem/alignment-not-power-of-two` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mem/cache-line-alignment` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `mem/cache-policy-unsupported-on-no-dcache-core` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mem/dcache-line-size-not-power-of-two` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mem/inter-pool-padding-not-emitted` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `mem/pool-section-conflict` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mem/pool-too-large` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `mem/reassembly-pool-variant-missing-max-fragments` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `mem/reassembly-pool-variant-missing-timeout` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `mem/reassembly-slot-size-below-declared-mtu` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `mem/slot-size-not-alignment-multiple` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `pool/cache-maintenance-misplaced` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `pool/cache-pre-arm-invalidate-missing-on-speculative-core` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `pool/sample-callback-signature-non-borrow` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `pool/sample-take-without-stage-pool` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `pool/sample-typestate-attributes-disabled` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `pool/speculative-prefetch-flag-missing` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `pool/stage-copy-accept-rejected-under-forbid` | `validation` | no | SCE Protocol-Synthesis RFC §5.K |
| `pool/stage-copy-policy-error` | `validation` | no | SCE Protocol-Synthesis RFC §5.K |
| `reassembly/binding-on-unpaired-listener` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/expected-fragmentation-rate-high` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/max-fragments-insufficient-for-mtu` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/peer-id-not-zid-on-established-session` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/per-peer-quota-build-invariant-violated` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/stage-copy-wcet-exceeds-slot-budget` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/trust-class-missing-on-fragmenting-link` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `reassembly/untrusted-link-binding` | `validation` | no | SCE Protocol-Synthesis RFC §5.M |
| `scxml/accept-side-role-without-listener-link` | `validation` | no |  |
| `scxml/accept-side-states-without-role-declaration` | `validation` | no |  |
| `scxml/always-false-guard` | `validation` | no |  |
| `scxml/contradictory-unhandled-declaration` | `validation` | no |  |
| `scxml/dead-transition` | `validation` | no |  |
| `scxml/duplicate-session-role-declaration` | `validation` | no |  |
| `scxml/non-exhaustive-event-handling` | `validation` | no |  |
| `scxml/null-datamodel-forbids-construct` | `validation` | no | W3C SCXML §B.1 |
| `scxml/on-sample-event-name-conflict` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `scxml/on-sample-invalid-parent` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `scxml/on-sample-link-duplicate-in-state` | `validation` | no | SCE Protocol-Synthesis RFC §5.E |
| `scxml/on-sample-link-not-declared` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `scxml/on-sample-link-wrong-kind` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `scxml/shadowed-transition` | `validation` | no |  |
| `scxml/stale-unhandled-declaration` | `validation` | no |  |
| `scxml/top-level-script-unloaded` | `validation` | no | W3C SCXML §5.8 |
| `scxml/unknown-session-role-kind` | `validation` | `replace_one_of` |  |
| `scxml/unreachable-state` | `validation` | no |  |
| `scxml/unsupported-datamodel` | `validation` | `replace_one_of` | W3C SCXML §3.2 |
| `timer/period-below-tick-rate` | `validation` | no | SCE Protocol-Synthesis RFC §5.D |
| `traceability/meta-generated-source-line-marker-missing` | `generate` | no | SCE Protocol-Synthesis RFC §5.O |
| `traceability/sce-map-attribute-stripped` | `generate` | no | SCE Protocol-Synthesis RFC §5.O |
| `traceability/scxml-line-range-missing` | `generate` | no | SCE Protocol-Synthesis RFC §5.O |
| `traceability/sourcemap-source-hash-mismatch` | `generate` | no | SCE Protocol-Synthesis RFC §5.O |
| `traceability/state-id-collision` | `generate` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.O |
| `traceability/symbol-name-exceeds-c-identifier-limit` | `generate` | no | SCE Protocol-Synthesis RFC §5.O |
| `validation/bytes-comparison-not-equality` | `validation` | no |  |
| `validation/bytes-max-size-violation` | `validation` | no |  |
| `validation/count-mismatch` | `validation` | no |  |
| `validation/cross-kind-circular-dependency` | `validation` | no |  |
| `validation/cross-kind-field-not-found` | `validation` | `replace_one_of` |  |
| `validation/cross-kind-type-mismatch` | `validation` | no |  |
| `validation/duplicate-context-object` | `validation` | `rename_duplicate` |  |
| `validation/duplicate-id` | `validation` | `rename_duplicate` |  |
| `validation/duplicate-requirement-id` | `validation` | no |  |
| `validation/dynamic-features` | `validation` | no |  |
| `validation/empty-collection` | `validation` | no |  |
| `validation/empty-value` | `validation` | `add_attribute` |  |
| `validation/enum-no-variants` | `validation` | no |  |
| `validation/enum-unsupported-underlying-type` | `validation` | no |  |
| `validation/enum-variant-duplicate-name` | `validation` | no |  |
| `validation/enum-variant-duplicate-value` | `validation` | no |  |
| `validation/enum-variant-value-overflows-underlying` | `validation` | no |  |
| `validation/event-payload-field-unknown` | `validation` | `replace_one_of` |  |
| `validation/event-schema-on-builtin-event` | `validation` | no |  |
| `validation/incompatible-attributes` | `validation` | no |  |
| `validation/invalid-attribute` | `validation` | `replace_one_of` |  |
| `validation/invalid-direction` | `validation` | `replace_one_of` | SCE Forge §3.3 |
| `validation/invalid-reference` | `validation` | `replace_one_of` |  |
| `validation/mesh-rpc-duplicate-target` | `validation` | no | SCE Mesh §9.5 |
| `validation/mesh-rpc-missing-target` | `validation` | no | SCE Mesh §9.5 |
| `validation/mesh-rpc-reserved-param` | `validation` | no | SCE Mesh §9.5 |
| `validation/missing-attribute` | `validation` | `add_attribute` |  |
| `validation/missing-context` | `validation` | no |  |
| `validation/missing-element` | `validation` | no |  |
| `validation/native-action-argument` | `validation` | no |  |
| `validation/native-action-placement` | `validation` | no |  |
| `validation/native-action-signature-conflict` | `validation` | no |  |
| `validation/numeric-parse` | `validation` | no |  |
| `validation/removed-attribute` | `validation` | `remove_fields` | SCE Mesh §13 |
| `validation/require-either` | `validation` | `add_one_of` |  |
| `validation/reserved-context-id` | `validation` | no |  |
| `validation/singleton-violation` | `validation` | no |  |
| `validation/unresolved-placeholder` | `validation` | no |  |
| `validation/unsupported-kind` | `validation` | `replace_one_of` | SCE Forge §3.2 |
| `validation/wrong-pipeline` | `validation` | no | SCE Forge §4 |
| `worker/inbox-ordering-relaxed-across-cores` | `validation` | no | SCE Protocol-Synthesis RFC §5.I |
| `worker/inbox-ordering-unspecified` | `validation` | no | SCE Protocol-Synthesis RFC §5.I |
| `worker/link-rx-ref-unknown` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.D |
| `worker/outbox-ref-unknown` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.D |
| `worker/outbox-target-suffix-invalid` | `validation` | `replace_with` | SCE Protocol-Synthesis RFC §5.D |
| `worker/outbox-target-wrong-kind` | `validation` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.D |
| `worker/scheduler-unsupported` | `validation` | no | SCE Protocol-Synthesis RFC §5.D |
| `worker/shared-mutable-state` | `validation` | no | SCE Protocol-Synthesis RFC §5.D |
| `xml/file-not-found` | `xml` | no |  |
| `xml/parse` | `xml` | no |  |
| `xml/preprocessor-not-run` | `xml` | no |  |
| `xml/schema-validation` | `xml` | no | SCE Forge XSD |
| `xml/template-cycle` | `xml` | no |  |
| `xml/template-malformed` | `xml` | no |  |
| `xml/template-missing-attribute` | `xml` | `add_attribute` |  |
| `xml/template-missing-param` | `xml` | `add_attribute` |  |
| `xml/template-not-found` | `xml` | no |  |
| `xml/template-read-error` | `xml` | no |  |
| `xml/template-too-deep` | `xml` | no |  |
| `xml/template-unknown-param` | `xml` | no |  |
| `xml/wrong-root-element` | `xml` | no |  |
| `xml/xinclude-cycle` | `xml` | no |  |
| `xml/xinclude-malformed` | `xml` | no |  |
| `xml/xinclude-missing-href` | `xml` | `add_attribute` |  |
| `xml/xinclude-not-found` | `xml` | no |  |
| `xml/xinclude-read-error` | `xml` | no |  |
| `xml/xinclude-too-deep` | `xml` | no |  |
| `xml/xinclude-unsupported` | `xml` | no |  |

### 5.2 CLI

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `cli/create-output-dir` | `cli` | no |  |
| `cli/format-style-not-found` | `cli` | no |  |
| `cli/generator-source-drift` | `cli` | no |  |
| `cli/generator-source-unverifiable` | `cli` | no |  |
| `cli/invalid-format-option` | `cli` | `replace_one_of` |  |
| `cli/invalid-suite-package` | `cli` | no |  |
| `cli/json-serialization` | `cli` | no |  |
| `cli/missing-metadata-field` | `cli` | no |  |
| `cli/no-scxml-tag` | `cli` | no |  |
| `cli/not-a-directory` | `cli` | no |  |
| `cli/project-root-not-found` | `cli` | no |  |
| `cli/query-no-match` | `cli` | no |  |
| `cli/read-input` | `cli` | no |  |
| `cli/scxml-generate` | `cli` | no |  |
| `cli/unknown-language` | `cli` | `replace_one_of` |  |
| `cli/unsupported-language` | `cli` | `replace_one_of` |  |
| `cli/usage` | `cli` | no |  |
| `cli/write-output` | `cli` | no |  |
| `forge/source-hash-input-uncovered` | `cli` | no | SCE Protocol-Synthesis RFC §6.2.6 |
| `forge/source-hash-mismatch` | `cli` | no | SCE Protocol-Synthesis RFC §6.2.6 |
| `forge/source-hash-walk-unbounded` | `cli` | no | SCE Protocol-Synthesis RFC §6.2.6 |

### 5.3 Mesh

| Code | Stage | Fix? | Spec |
|---|---|---|---|
| `deploy/accept-rate-config-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/keepalive-jitter-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-burst-absorption-insufficient` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-burst-pps-missing-on-isr-dispatch` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-driver-class-mismatch` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-driver-unknown` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-expected-p99-exceeds-mtu` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-mtu-below-driver-floor` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-mtu-missing-on-fragmenting-link` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-not-declared-in-deploy` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-not-declared-in-forge` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/link-rx-dispatch-worker-tick-on-high-burst` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/scheduler-incompatible-with-worker-count` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/session-arming-fields-on-non-arming-link` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/session-arming-quota-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/session-arming-quota-vs-peer-table-invariant-violated` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/stage-copy-policy-unknown` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/stateless-accept-extern-not-whitelisted` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.K |
| `deploy/stateless-accept-key-rotation-shorter-than-lifetime` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/stateless-accept-required-on-untrusted-source` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/worker-slot-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `deploy/worker-stack-budget-missing` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.K |
| `link/concurrent-count-exceeds-scheduler-slots` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.N |
| `link/per-link-budget-exceeds-tick-period` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.N |
| `mesh/codegen-event-name-collision` | `mesh-codegen` | no |  |
| `mesh/codegen-pool-with-rpc-client-unsupported` | `mesh-codegen` | no |  |
| `mesh/codegen-template-read` | `mesh-codegen` | no |  |
| `mesh/codegen-template-render` | `mesh-codegen` | no |  |
| `mesh/codegen-unsupported-language` | `mesh-codegen` | `replace_one_of` | SCE Mesh §7 |
| `mesh/codegen-unsupported-transport` | `mesh-codegen` | `replace_one_of` | SCE Mesh §8 |
| `mesh/deploy-cross-target-reply-not-supported` | `mesh-deploy` | no | SCE Mesh §14.6 |
| `mesh/deploy-discovery-not-supported` | `mesh-deploy` | no | SCE Mesh §3.3 |
| `mesh/deploy-duplicate-machine` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-invalid-auth-policy` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-invalid-custom-tcp-socket` | `mesh-deploy` | no | SCE Mesh §16.8.3 |
| `mesh/deploy-invalid-dds-qos` | `mesh-deploy` | no | SCE Mesh §8.2 |
| `mesh/deploy-invalid-dedup-window` | `mesh-deploy` | no | SCE Mesh §10.5 |
| `mesh/deploy-invalid-liveliness` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-invalid-ordering-timings` | `mesh-deploy` | no | SCE Mesh §10.6 |
| `mesh/deploy-invalid-outbound-buffer` | `mesh-deploy` | no | SCE Mesh §10.10 |
| `mesh/deploy-invalid-reply-from` | `mesh-deploy` | no | SCE Mesh §14.6 |
| `mesh/deploy-invalid-retry-policy` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-invalid-server-response-deadline` | `mesh-deploy` | no | SCE Mesh §9.5 |
| `mesh/deploy-parse` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-barrier-timeout-invalid` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-duplicate-name` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-empty` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-machine-not-listed` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-multi-device` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-name-not-identifier` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-partial-coverage-requires-default` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-pool-machine` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-synth-infix-collision` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-transport-binding-unsupported` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-uncovered-unit` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-partition-unit-duplicate` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-platform-class-os-mismatch` | `mesh-deploy` | no | SCE Mesh §14 |
| `mesh/deploy-pool-binding-field-not-supported` | `mesh-deploy` | `remove_fields` | SCE Mesh §14.4 |
| `mesh/deploy-pool-dispatch-without-member` | `mesh-deploy` | `remove_fields` | SCE Mesh §14.4 |
| `mesh/deploy-pool-empty-member-list` | `mesh-deploy` | no | SCE Mesh §14.4 |
| `mesh/deploy-pool-invalid-placeholder` | `mesh-deploy` | no | SCE Mesh §14.4 |
| `mesh/deploy-pool-missing-member-list` | `mesh-deploy` | no | SCE Mesh §14.4 |
| `mesh/deploy-pool-not-supported-by-transport` | `mesh-deploy` | no | SCE Mesh §14.4 |
| `mesh/deploy-read` | `mesh-deploy` | no |  |
| `mesh/deploy-scxml-invoke-cross-device-transport` | `mesh-deploy` | no | SCE Mesh §9.6 L1393 |
| `mesh/deploy-scxml-invoke-target-conflict` | `mesh-deploy` | no | SCE Mesh §9.6 |
| `mesh/deploy-server-pool-not-supported` | `mesh-deploy` | `remove_fields` | SCE Mesh §14.4 |
| `mesh/deploy-someip-liveness-service-id-overflow` | `mesh-deploy` | no | SCE Mesh §16.4 |
| `mesh/deploy-someip-liveness-service-id-pin-collision` | `mesh-deploy` | no | SCE Mesh §16.4 |
| `mesh/deploy-someip-liveness-service-id-pin-out-of-range` | `mesh-deploy` | no | SCE Mesh §16.4 |
| `mesh/deploy-someip-machine-liveness-service-id-overflow` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-someip-machine-liveness-service-id-pin-collision` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range` | `mesh-deploy` | no | SCE Mesh §16.7 |
| `mesh/deploy-someip-scxml-invoke-service-id-overflow` | `mesh-deploy` | no | SCE Mesh §9.6 |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-collision` | `mesh-deploy` | no | SCE Mesh §9.6 |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range` | `mesh-deploy` | no | SCE Mesh §9.6 |
| `mesh/deploy-stage-pool-not-declared` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mesh/deploy-stage-pool-transport-mismatch` | `mesh-deploy` | `remove_fields` | SCE Protocol-Synthesis RFC §5.E |
| `mesh/deploy-stage-pool-wrong-kind` | `mesh-deploy` | `replace_one_of` | SCE Protocol-Synthesis RFC §5.E |
| `mesh/deploy-unknown-binding-field` | `mesh-deploy` | `replace_one_of` | SCE Mesh §14 |
| `mesh/deploy-unsupported-version` | `mesh-deploy` | `replace_one_of` | SCE Mesh §14 |
| `mesh/distributability-r1-shared-write` | `mesh-deploy` | no | SCE Mesh §16.3 |
| `mesh/distributability-r2-cross-region-transition` | `mesh-deploy` | no | SCE Mesh §16.3 |
| `mesh/event-schema-mismatch` | `mesh-deploy` | no |  |
| `mesh/external-ambiguous-event-group` | `mesh-external` | no |  |
| `mesh/external-conflicting-event-field-kinds` | `mesh-external` | no |  |
| `mesh/external-conflicting-event-schema` | `mesh-external` | no |  |
| `mesh/external-empty-event-entry` | `mesh-external` | no |  |
| `mesh/external-empty-event-group` | `mesh-external` | no |  |
| `mesh/external-named-reference-without-config` | `mesh-external` | no |  |
| `mesh/external-parse` | `mesh-external` | no |  |
| `mesh/external-read` | `mesh-external` | no |  |
| `mesh/external-reserved-someip-id-keys` | `mesh-external` | `remove_fields` |  |
| `mesh/external-someip-field-on-non-someip-transport` | `mesh-external` | `replace_with` |  |
| `mesh/external-unresolved-names` | `mesh-external` | no |  |
| `mesh/io` | `io` | no |  |
| `mesh/partition-barrier-timeout-without-root` | `mesh-deploy` | no | SCE Mesh §14 rule 12 |
| `mesh/partition-parallel-root-ambiguous` | `mesh-deploy` | no | SCE Mesh §14 rule 12 |
| `mesh/partition-parallel-root-non-host` | `mesh-deploy` | no | SCE Mesh §14 rule 12 |
| `mesh/partition-parallel-root-not-in-machines` | `mesh-deploy` | no | SCE Mesh §14 rule 12 |
| `mesh/partition-parallel-root-undesignated` | `mesh-deploy` | no | SCE Mesh §14 rule 12 |
| `mesh/partition-wire21-custom-tcp-unimplemented` | `mesh-deploy` | no | SCE Mesh §16.5 |
| `mesh/topology-absolute-source-path` | `mesh-topology` | no |  |
| `mesh/topology-event-binding-unused` | `mesh-topology` | `remove_fields` | SCE Mesh §14 |
| `mesh/topology-invalid-binding-field` | `mesh-topology` | no | SCE Mesh §14 |
| `mesh/topology-machine-lifetime-subscription-unsupported` | `mesh-topology` | no | SCE Mesh §13 |
| `mesh/topology-machine-not-found` | `mesh-topology` | `replace_one_of` | SCE Mesh §14 |
| `mesh/topology-missing-binding-field` | `mesh-topology` | `add_attribute` | SCE Mesh §14 |
| `mesh/topology-ordering-cannot-be-guaranteed` | `mesh-topology` | no | SCE Mesh §10.6 |
| `mesh/topology-pattern-capability-violation` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-pool-param-name-missing` | `mesh-topology` | no | SCE Mesh §14.4 |
| `mesh/topology-receiver-not-declared` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-receiver-source-parse` | `mesh-topology` | no |  |
| `mesh/topology-receiver-source-read` | `mesh-topology` | no |  |
| `mesh/topology-subscription-source-unbound` | `mesh-topology` | `replace_one_of` | SCE Mesh §13 |
| `mesh/topology-uncovered-events` | `mesh-topology` | no | SCE Mesh §9 |
| `mesh/topology-unresolved-targets` | `mesh-topology` | no | SCE Mesh §9 |
| `timer/slot-overflow` | `mesh-deploy` | no | SCE Protocol-Synthesis RFC §5.D |

<!-- END GENERATED: code catalog -->

## 6. Exit codes

Exit status is a coarse routing signal; `code` is the finer one.
A non-zero exit with no NDJSON record is a contract violation.

That last sentence is a claim about *every* way this process can
end, including the ones no pipeline stage produces: an unparseable
command line and a query that matched nothing both leave through
here, and both carry a record. `1` and `20` are the two statuses
that exist for those, and the table below is the whole set — a
status outside it is a defect, not an undocumented convention.
Pinned by `sce-build/tests/exit_status_contract.rs`, which reads
this table and probes the real binary.

Two rows can name one code — an exact code and the `family/*` that
contains it. **The exact row wins.** Without that rule the table is
not a function from code to status, and a consumer branching on it
would have to guess; `cli/query-no-match` is the case that exists.

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | `cli/query-no-match` — a well-formed query against a well-formed artifact that matched nothing. Not a failure of the run: the tool looked and the answer was "none". Separate from `20` so a build gate can assert "this state lowered to something" without a JSON parser. |
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
| `20` | `cli/*` (CLI-boundary errors), including `cli/usage` — the command line itself did not parse |

## 7. Determinism guarantees

- **No timestamps, no wall-clock**, no PIDs, no absolute paths other
  than those the user passed in.
- **No ANSI / color escapes** in JSON mode — ever.
- **Field order** within a record is fixed: `v`, `id`, `generator`,
  `code`, `stage`, `spec`, `message`, `location`, `expected`, `actual`,
  `fix`.
- **One record per line.** A record never contains a raw `\n`.
  Consumers may split stderr on `\n` without a JSON parser lookahead.
- **`id` stability**: rewording a `thiserror` `#[error]` template
  does not shift `id`. Only changing the hashed semantic fields
  (code, stage, file, key_fragments) does — and those fields are
  producer-independent by [§2.1.1](#211-key-fragments-and-the-id-namespace),
  so the same document yields the same id whichever producer read it.

## 8. Evolution policy

**Additive-only at v1**:

- Adding a new `code` ✔ (consumers must treat unknown codes as "unknown;
  inspect `stage` for routing, fall back to `exit_code` family").
- Adding a new `Fix` variant ✔ (consumers must ignore unknown `kind`).
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
on `v1` stability. Every record names the commit that produced it in
`generator` ([§2.1](#21-field-semantics)), so a consumer can check a
payload against the commit it pinned instead of assuming they match;
without that field the instruction in this paragraph would be one a
consumer had no way to act on, since a rejected run writes no manifest
to read the commit from. The flip to `stable` is a deliberate editorial act,
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
and flip-to-`stable` procedure for every wire surface.

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

On success, `sce-codegen generate`, `sce-codegen check` and
`sce-codegen orchestrate` each write exactly one JSON line to stdout —
nothing more, nothing less. The wire schema is
`schemas/sce-manifest.v1.schema.json`; the surface's stability status
is registered in `SCE_WIRE_CONTRACTS.md`. The shape is:

```json
{
  "v": 1,
  "kind": "generate",
  "generator": "b497eacf7d94",
  "artifacts": [
    {"path": "/abs/path/foo_sm.rs"}
  ],
  "needs_script_engine": true,
  "needs_event_scheduler": true,
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
| `needs_event_scheduler` | bool | Which driving entry point the machine requires of its host: `true` means the runtime's `tick()`, `false` means `step()` is enough. `tick()` drains the delayed-send scheduler **and** ticks invoked child sessions; `step()` does neither. A machine carrying a `<send delay>` / `<cancel>`, or a session-bearing `<invoke>`, driven by `step()` alone loses those events with no error and no diagnostic — the symptom is events that never arrive, which is why the answer is published rather than left to be read off the runtime's source. Always present, so `false` is an answer rather than an absent field. |
| `script_engine_causes` | optional array of objects | **Why** `needs_script_engine` is `true` — one record per construct that forced the engine in. Present exactly when the flag is `true`; omitted (not `[]`) otherwise, so a pure-static manifest carries no new bytes. See [§10.4](#104-script-engine-causes). |
| `rejected` | optional object | Present only when the input triggered a W3C-spec rejection (currently `W3C SCXML 5.8`, "untestable manifest") and stub files were written in place of generated code. Absence means clean generation. Fields: `spec` (e.g. `"W3C SCXML 5.8"`) and `name` (the document's `name` attribute). |
| `deploy` | optional object | Declarations read out of `--deploy` that SCE records without acting on. Omitted whole when the run had no deploy or the deploy declared none of them, so a deploy-unaware manifest carries no new bytes. See [§10.5](#105-deploy-declarations). |

### 10.2 Stream discipline

- On **failure** the manifest is not emitted; stdout is empty and the
  NDJSON diagnostic on stderr is the sole signal. Consumers must
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

The document-set route is also the only one that can hold a
cross-document **name** to account. `<sce:rx-pool ref>`,
`<sce:tx-pool ref>` and `<sce:stage-pool ref>` on a link kind name a
buffer-pool document, and every consumer of those refs — the
§synth-5-K burst-absorption check, the §synth-5-M reassembly check,
the §synth-5-C slot-size-vs-framer check — resolves them by joining on
that name and **skips the link when the join misses**. Skipping is
right for a partial topology and wrong for a typo, which looks
identical at those layers; the result was that a one-character slip in
a pool ref switched the MCU capacity validators off instead of failing
the build. Only this route is handed the whole build, so only it can
tell the two apart: it refuses an unresolved ref with
`link/pool-ref-not-declared` before the deploy validators run, which
is the same join `deploy.yaml`'s `stage_pool:` has always been held to
(`mesh/deploy-stage-pool-not-declared`). A ref resolves either through
the build's input documents or through the link's own `<sce:import>`.
Single-document `generate` keeps its tolerance — it is handed one file
and cannot know what else the build declares.

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
