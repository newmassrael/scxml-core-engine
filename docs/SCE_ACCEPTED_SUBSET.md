# SCE Accepted Subset

**Positive-form counterpart to `SCE_ERROR_CONTRACT.md`.** The error
contract catalogues the signals SCE emits when it *rejects* an input;
this document catalogues what SCE *accepts*. Upstream automation (LLM
drafters, IDE drafters, repair loops) should consult this doc to
determine whether a given SCXML document is in the accepted subset
before invoking `sce-codegen` — turning acceptance into a static
property instead of a trial-and-error loop.

Audience: authors and tooling producing SCXML for SCE, not runtime
consumers of generated code. Runtime semantics (W3C execution order,
event routing, datamodel behaviour) are documented in `ARCHITECTURE.md`
and the W3C SCXML recommendation; this doc is **build-time acceptance
only**.

The appendix at the bottom enumerates every `DiagnosticCode` the
toolchain can emit and partitions them by whether the author can
prevent them by writing better SCXML (*Acceptance boundary*) or not
(*Diagnostic-only*, i.e. I/O and infrastructure failures). The
enumeration is kept honest by the `acceptance_doc_covers_every_code`
test in `sce-build/src/forge/diagnostic.rs` — adding a new
`DiagnosticCode` variant without listing it here breaks the build.

---

## §1 W3C SCXML inclusions

SCE-codegen accepts the W3C SCXML 1.0 subset that all five backends
(C++, Kotlin, Rust, Go, Python) currently ship at **202/202** parity
against the W3C IRP suite. See the per-backend memoranda for the
concrete pass sets — `rust_backend_next_steps.md`,
`python_bindings_progress.md`, `go_backend_status.md`,
`kotlin_lua_engine.md` (JVM/Android), and the C++ test targets in
`tests/CMakeLists.txt`.

The accepted surface comprises:

- **Core constructs**: `<state>`, `<parallel>`, `<final>`, `<history>`
  (shallow and deep), `<initial>`, nested compound states.
- **Transitions**: event triggers, cond guards, targets, target sets,
  `type="internal"`, eventless transitions, wildcard event
  descriptors (`*`, `event.*`).
- **Executable content**: `<onentry>`, `<onexit>`, `<if>`/`<elseif>`/
  `<else>`, `<foreach>`, `<raise>`, `<send>` (internal targets,
  `#_internal`, `#_parent`, `#_invokeid`, delayed sends),
  `<cancel>` (W3C §6.3 — MUST carry `sendid` or `sendidexpr`; the
  both-empty shape is rejected at parse time, wire
  `validation/require-either`), `<assign>`, `<log>`, `<script>`.
- **Datamodel**: `<datamodel>` / `<data>` with expression-language
  assignment. The generated code runs a Lua 5.4 datamodel by default
  (see `lua_engine_default.md`); ECMAScript data-model documents are
  accepted and rewritten through the Lua engine at generation time.
- **Invoke**: `<invoke type="scxml">` with inline `<content>` or
  external `src`, static param binding via `<param>` and `<finalize>`.
- **HTTP event processor**: `<send type="BasicHTTPEventProcessor">`
  for W3C §C.2 conformance (all five backends tested against the
  `HttpAotTest` harness).
- **Communication**: `_ioprocessors`, `_sessionid`, `_name`,
  `_event` (excluding the exclusions listed in §3).

The **AOT code generator** is the default path; the Interpreter exists
as a fallback for documents that cannot be statically generated. At
HEAD, `tests/CMakeLists.txt` lists every W3C IRP test in
`W3C_AOT_TESTS` and `W3C_INTERPRETER_ONLY_TESTS` is empty — i.e. the
full IRP suite generates statically on every supported backend. The
categories in §3 describe the *kinds* of constructs that would route a
document to the interpreter fallback if it contained them, independent
of whether the IRP suite happens to exercise those categories today.

The acceptance boundary for each specific rejection is linked from the
appendix. Codes are grouped by pipeline stage, matching the Stage
taxonomy in `SCE_ERROR_CONTRACT.md` §4.

---

## §2 SCE extensions grammar

SCE adds a small set of extension elements and attributes for
functionality beyond plain SCXML. All SCE extensions live under the
namespace URI `https://sce.example/ns/1` (constant `SCE_NAMESPACE` in
`sce-build`). Unqualified or wrongly-namespaced attributes are rejected
as schema violations (`xml/schema-validation`).

### §2.1 Forge kinds — `sce:kind`

The `sce:kind` attribute on the root `<scxml>` element selects the
forge kind the document compiles to. The closed value set is the
eleven variants of `ForgeKind` (source of truth:
`sce-build/src/forge/model.rs`; see `forge_kinds_catalog.md` for the
stateful/stateless/inline-eligible matrix):

```
Statechart   Procedure   Transform   Lookup      Condition
Codec        Validator   Filter      Interpolation
Timer        Observer
```

Omitting `sce:kind` defaults to `Statechart`. Values outside this set
are rejected as `validation/unsupported-kind`. The phase 2/3 runtime
packages for the stateful kinds (Validator, Filter, Timer, Observer)
are described in `forge_phase3_complete.md`; inline-eligible kinds
(`is_inline_eligible()` → true) may be embedded in a `<data>` element
of an outer statechart.

### §2.2 Typed fields — `<sce:field>`

Structured data carriers used by codec / validator / filter / etc.
kinds. Required attributes:

- `id` — unique within the enclosing kind (duplicates are rejected as
  `validation/duplicate-id`).
- `sce:type` — closed value set of integer and fixed-width types
  (e.g. `u8`, `u16`, `u32`, `u64`, `i8`, `i16`, `i32`, `i64`, `f32`,
  `f64`, `bool`, `string`). Values outside the recognised set are
  rejected as `validation/invalid-attribute`.

Field cardinality and direction constraints are enforced per-kind —
e.g. `Transform` requires at least one input and one output field
(`validation/empty-collection`, `validation/invalid-direction`).

### §2.3 Context objects — `<sce:context>`

Per-kind context objects carrying stateful scratch data. Rules:

- The `id` is unique across all context objects in the document
  (`validation/duplicate-context-object`).
- The `id` does not collide with a type alias the C++ codegen emits
  on the generated state-machine class. At HEAD the reserved set is
  `{ policy }` — comparison is case-insensitive because Jinja2's
  `capitalize` filter maps `policy`, `Policy`, and `POLICY` to the
  same `PolicyType` alias (`validation/reserved-context-id`). The
  set is not maintained by hand: `RESERVED_CONTEXT_IDS` in
  `sce-build/src/parser.rs` is a `LazyLock` that scans
  `tools/codegen/templates/state_machine.jinja2` for literal
  `using {Id}Type =` aliases at first access. Adding a new class-
  scope alias to the template therefore extends the reserved set
  automatically — no parallel const to update, and no drift window
  between template and parser.
- Kinds that require a context object (e.g. Validator for history
  tracking) reject documents that omit it
  (`validation/missing-context`).

### §2.4 Cross-file composition — `<sce:import>`

Imports a standalone SCXML document that declares a non-Statechart
kind, for use in an outer statechart via `<invoke>` or inline via
`<data>`. Required attribute:

- `src` — filesystem path to the imported SCXML file (resolved
  relative to the importing file). The path must resolve
  (`import/file-not-found`) and parse (`import/not-forge`) to a
  document with a recognised `sce:kind`.

Optional attribute:

- `kind` — if present, the importer asserts the imported document's
  `sce:kind` matches. Mismatches are rejected as
  `import/kind-mismatch`.

Circular imports across the manifest graph are rejected as
`manifest/circular-dependency`.

### §2.5 Communication patterns — `sce:pattern`

For distributed deployments under `--deploy`, the `sce:pattern`
attribute on `<send>` declares the communication pattern the send
follows. The enum carries seven variants defined in
`sce-build/src/mesh/pattern.rs` (source of truth):

```
FireForget   Request   Reply   Notify   Subscribe   Publish   Field
```

Per-transport capability tables constrain which patterns each
transport (`local`, `shm`, `someip`, `zenoh`) can realise
(`mesh/topology-pattern-capability-violation`). **Realization status:**
at HEAD only `FireForget` is fully realized end-to-end across all
transports; the remaining six patterns have partial codegen and are
tracked in `mesh_pattern_realization_gap.md`. Documents using
non-`FireForget` patterns are accepted by the validator (they match the
enum) but may generate code that panics or under-realises the
pattern semantics. Treat non-`FireForget` patterns as **experimental**
until the pattern-realization session lands.

### §2.6 Mesh-RPC invoke — `<invoke type="sce:mesh-rpc">`

Explicit extension for RPC-style cross-machine invokes under
`--deploy`. Documented in `SCE_MESH.md` §9.5. At HEAD this extension is
recognised by the parser but end-to-end realization is in progress —
see `next_session_task6_mesh_rpc_invoke.md` for the current state.
Acceptance is conditional on the deploy topology resolving both ends
of the RPC pair (`mesh/topology-receiver-not-declared`,
`mesh/topology-unresolved-targets`).

### §2.7 XInclude composition — `<xi:include>`

Multi-file SCXML composition via W3C XInclude (namespace
`http://www.w3.org/2001/XInclude`) is processed at parse time so
the AOT code generator consumes the same effective document as
the C++ runtime (`PugiXMLDocument::processXInclude`). Accepted
shape:

```xml
<scxml xmlns:xi="http://www.w3.org/2001/XInclude">
  <xi:include href="guards.xml"/>
</scxml>
```

Semantics match the minimal subset the runtime implements: the
children of the included document's root element are spliced
in place of the `<xi:include>` node — the root element itself
is discarded, so authors bundle N top-level fragments under any
XML wrapper (e.g. `<fragment>…</fragment>`) without affecting
SCXML validity, and a single `<xi:include>` composes them all.
`href` resolves absolute-first, then relative to the including
file, then relative to the current working directory; recursion
is bounded by a documented depth limit (mirrored from the
runtime), and cycles are detected.

Unsupported W3C XInclude features are rejected explicitly rather
than silently ignored — accepting them at build time would
produce state machines diverging from runtime parse:

- `parse="text"` — `xml/xinclude-unsupported`.
- `xpointer=` — `xml/xinclude-unsupported`.
- `<xi:fallback>` — `xml/xinclude-unsupported`.

Rejections the AOT pipeline hard-errors on (the C++ runtime
warns-and-skips the same inputs; matching behaviour at
build-time is preferable to silent divergence): missing or empty
`href` (`xml/xinclude-missing-href`, fixable), unresolvable
`href` (`xml/xinclude-not-found`), filesystem read failures
(`xml/xinclude-read-error`), cycles (`xml/xinclude-cycle`),
depth overflow (`xml/xinclude-too-deep`), and malformed
included files (`xml/xinclude-malformed`).

### §2.8 Deploy manifest (`deploy.yaml`)

Accepted when `--deploy <path>` is passed. Schema is enforced by
serde with `deny_unknown_fields` (`mesh/deploy-parse` on unknown
keys). Only `version: 1` is accepted
(`mesh/deploy-unsupported-version`); machine names are globally unique
across all devices (`mesh/deploy-duplicate-machine`). The
`transports:` block is device-level; `bindings:` is per-target
(see `mesh_phase3_patterns.md`). External event/group/field catalogues
referenced from `bindings:` follow the rules in §2.5 and must resolve
in full (`mesh/external-unresolved-names`).

### §2.9 Composition extensions — `<sce:template>`

`<sce:template>` / `<sce:use>` / `<sce:param>` add parameterised XML
composition adjacent to XInclude (§2.7). XInclude handles
byte-identical reuse; `sce:template` handles fragments that differ by
a small closed set of constants. Both paths expand templates: the
AOT pipeline in sce-build (`crate::template::expand`) per RFC §6.5
Phase A, and the C++ Interpreter runtime in
`SCE::PugiXMLDocument::processSceTemplate` per RFC §3 Phase B M5 —
documents produced by each path are byte-equivalent after
canonicalisation, pinned by the CTest harness under
`tests/w3c_phase_b_parity/`. Each failure mode raises a typed
`SCE::parsing::Template<Variant>` subtype agreeing 1:1 with the Rust
`xml/template-*` DiagnosticCode set (pinned by
`cpp_template_subtypes_match_rust_diagnostic_codes`).

Expansion semantics (RFC §3):

- A template declaration is a standalone XML file whose root is
  `<sce:template name="...">`. Children `<sce:param name="..."
  required="true"|default="...">` declare parameters; remaining
  children form the template body.
- `<sce:use template="relative/path.xml" ...>` at the call site
  resolves the template file with XInclude precedence
  (absolute-first, then base-directory, then cwd), binds every
  non-reserved attribute as a parameter value, and splices the
  rendered body in place of the `<sce:use>` node. Attributes named
  `template` are reserved.
- `{$name}` tokens inside the template body (attribute values and
  text nodes) are replaced by the parameter's bound string in a
  single lexical pass. Substitution does not cascade — a bound
  value that itself contains `{$other}` is emitted verbatim.
- Nesting is bounded by `MAX_TEMPLATE_DEPTH = 10` (mirrors
  XInclude). Cycles are detected via the same path-stack mechanism.

Rejections the AOT pipeline hard-errors on: unresolvable template
path (`xml/template-not-found`), filesystem read failures
(`xml/template-read-error`, Diagnostic-only), malformed template
file or malformed `<sce:param>` declaration
(`xml/template-malformed`), `<sce:use>` missing the required
`template` attribute (`xml/template-missing-attribute`, fixable),
omitted `required="true"` parameter
(`xml/template-missing-param`, fixable), unknown attribute on
`<sce:use>` (`xml/template-unknown-param`), cycles
(`xml/template-cycle`), and depth overflow
(`xml/template-too-deep`).

Post-expansion diagnostic attribution (RFC §6.3 Q3 depth-1 rule, as
implemented by `crate::position_map::Origin::CallSite` and
`Origin::File` emitted during `template::expand`):

- When a diagnostic fires in bytes produced by `{$param}`
  substitution, `location.{file, row, col}` points at the offending
  `<sce:use>` element in the **caller document** — the call site
  that supplied the parameter bindings (attributes on `<sce:use>`,
  per the XSD in `schemas/sce-forge-ext.xsd`). Column precision
  inside the substituted value is deliberately collapsed (every
  byte of every substituted region shares the same single (row,
  col) — the `<sce:use>`'s element position).
- When a diagnostic fires in template-body bytes (regions copied
  1:1 from the template file during expansion, i.e. not produced
  by `{$param}` substitution), `location.{file, row, col}` points
  at the template file's own (row, col). This lets template authors
  navigate to the body they wrote rather than to a caller that did
  nothing wrong.

Other XML meta-processing primitives (parameter entities,
conditional inclusion, computed attributes, Turing-complete
templating) remain out of scope — see `ARCHITECTURE.md` → "Scope &
Composition" for the discipline gate.

---

## §3 Exclusions (cannot be statically generated)

Documents using any of the following constructs fall outside the
statically-generated subset. At HEAD these would be routed to the
Interpreter fallback by `sce-codegen generate`; a document that
otherwise targets the statically-generated AOT path
(`sce-codegen` with `-l <lang>`) is rejected rather than silently
downgraded. The current W3C IRP pass matrix contains no such
documents, so `W3C_INTERPRETER_ONLY_TESTS` in `tests/CMakeLists.txt`
is empty at HEAD — but the rejection categories remain load-bearing
for arbitrary author-supplied input.

The categories and their primary rejection signals:

### §3.1 Dynamic file I/O at the invoke boundary

`<invoke srcexpr="pathVar"/>` — target file path determined at
runtime. Rejected when compiling under `--deploy` or when generating
AOT code, because the set of reachable invoke targets must be known at
build time to drive codegen. Signaled as
`validation/dynamic-features`.

### §3.2 Documents without an initial state

Documents that rely on W3C SCXML §3.4 "default initial state"
semantics (i.e. neither `initial=` attribute nor `<initial>` child,
expecting runtime selection of the first child) are rejected at
generation time. A document intended for AOT must name its entry
state explicitly (signaled as `validation/missing-element`
or `validation/require-either` depending on the anchor).

### §3.3 Runtime event metadata references

Guards or data-model expressions that read `_event.origintype` (the
W3C-defined event origin-type slot) are rejected as
`expression/unsupported-construct`. The generated code has no
mechanism to populate this runtime-only metadata at AOT targets.
Other `_event.*` fields (`name`, `type`, `data`, `origin`, `sendid`,
`invokeid`) are supported.

### §3.4 Unsupported expression-language constructs

The forge expression language (SCE_FORGE.md §3.4) is a typed subset of
ECMAScript with a Lua-compatible runtime. Constructs that either have
no typed interpretation or are explicitly excluded:

- Triple-equals / strict-inequality (`===`, `!==`) —
  `expression/strict-equality`. Use `==` / `!=`.
- Go ternary target (source-language-specific restriction for Go
  codegen) — `expression/go-ternary-unsupported`. Restructure with
  `if`/`else`.
- Free-form tokens not part of the grammar —
  `expression/unsupported-construct`, `expression/unexpected-token`,
  `expression/invalid-lvalue`, `expression/type-coercion`,
  `expression/parse-mismatch`, `expression/lex`,
  `expression/empty`, `expression/numeric-parse` (on integer literal
  overflow, dispatched as `validation/numeric-parse`).

---

## Appendix — `DiagnosticCode` index (264 codes)

This appendix is the **drift-guarded coverage target** for the
`acceptance_doc_covers_every_code` test. Every slash-path string in
`DiagnosticCode::as_str` appears in exactly one of the two tables
below. The prose sections §1–§3 above reference a subset of these
codes inline, but the appendix is the coverage contract.

The guard matches each code against the table-row anchor
`` | `code` | ``. Reformatting the tables (extra whitespace between
`|` and the code, changing to bullet lists, wrapping across lines)
will break the anchor and fire the test — the markdown shape above
is the load-bearing format. Keep rows on a single line with exactly
one space on either side of the leading backtick-wrapped code.

### Acceptance boundary

Codes that the author can avoid by writing a better SCXML /
`deploy.yaml` / CLI invocation. Listed in pipeline-stage order.

| Code | Stage |
|---|---|
| `xml/parse` | Xml |
| `xml/schema-validation` | Xml |
| `xml/file-not-found` | Xml |
| `xml/wrong-root-element` | Xml |
| `xml/xinclude-missing-href` | Xml |
| `xml/xinclude-not-found` | Xml |
| `xml/xinclude-cycle` | Xml |
| `xml/xinclude-too-deep` | Xml |
| `xml/xinclude-malformed` | Xml |
| `xml/xinclude-unsupported` | Xml |
| `xml/template-not-found` | Xml |
| `xml/template-malformed` | Xml |
| `xml/template-missing-attribute` | Xml |
| `xml/template-missing-param` | Xml |
| `xml/template-unknown-param` | Xml |
| `xml/template-cycle` | Xml |
| `xml/template-too-deep` | Xml |
| `validation/missing-element` | Validation |
| `validation/missing-attribute` | Validation |
| `validation/invalid-attribute` | Validation |
| `validation/unsupported-kind` | Validation |
| `validation/duplicate-id` | Validation |
| `validation/duplicate-context-object` | Validation |
| `validation/reserved-context-id` | Validation |
| `validation/empty-collection` | Validation |
| `validation/count-mismatch` | Validation |
| `validation/incompatible-attributes` | Validation |
| `validation/missing-context` | Validation |
| `validation/invalid-reference` | Validation |
| `validation/invalid-direction` | Validation |
| `validation/numeric-parse` | Validation |
| `validation/empty-value` | Validation |
| `validation/singleton-violation` | Validation |
| `validation/require-either` | Validation |
| `validation/wrong-pipeline` | Validation |
| `validation/dynamic-features` | Validation |
| `validation/mesh-rpc-reserved-param` | Validation |
| `validation/mesh-rpc-missing-target` | Validation |
| `validation/mesh-rpc-duplicate-target` | Validation |
| `validation/removed-attribute` | Validation |
| `validation/bytes-max-size-violation` | Validation |
| `algorithm/local-shadows-param` | Validation |
| `algorithm/lvalue-unsupported` | Validation |
| `algorithm/return-missing` | Validation |
| `algorithm/foreach-source-not-iterable` | Validation |
| `algorithm/call-target-unknown` | Validation |
| `algorithm/call-target-method-unknown` | Validation |
| `algorithm/bc-mutation-forbidden` | Validation |
| `algorithm/foreach-source-bc-with-bytes-item-type` | Validation |
| `algorithm/call-arg-count-mismatch` | Validation |
| `algorithm/const-not-foldable` | Generate |
| `algorithm/const-fold-budget-exceeded` | Generate |
| `algorithm/const-yield-type-mismatch` | Generate |
| `codec/variant-arm-unreachable` | Validation |
| `codec/present-if-refs-later-field` | Validation |
| `codec/repeat-count-refs-later-field` | Validation |
| `algorithm/test-vector-unsupported-kind` | Validation |
| `codec/tlv-chain-depth-unspecified` | Validation |
| `codec/dma-alignment-unsatisfiable` | Validation |
| `codec/parent-flag-mismatch` | Validation |
| `link/framer-missing` | Validation |
| `link/link-class-unknown` | Validation |
| `link/backpressure-undeclared` | Validation |
| `link/class-unsupported-on-target` | Validation |
| `link/pool-slot-smaller-than-framer-max` | Validation |
| `mem/pool-section-conflict` | Validation |
| `mem/pool-too-large` | Validation |
| `mem/inter-pool-padding-not-emitted` | Validation |
| `mem/cache-line-alignment` | Validation |
| `mem/slot-size-not-cache-line-multiple` | Validation |
| `mem/cache-policy-unsupported-on-no-dcache-core` | Validation |
| `pool/cache-maintenance-misplaced` | Validation |
| `pool/speculative-prefetch-flag-missing` | Validation |
| `pool/cache-pre-arm-invalidate-missing-on-speculative-core` | Validation |
| `pool/sample-typestate-attributes-disabled` | Validation |
| `pool/sample-take-without-stage-pool` | Validation |
| `pool/sample-callback-signature-non-borrow` | Validation |
| `worker/shared-mutable-state` | Validation |
| `worker/link-rx-ref-unknown` | Validation |
| `worker/inbox-ordering-unspecified` | Validation |
| `worker/inbox-ordering-relaxed-across-cores` | Validation |
| `worker/scheduler-unsupported` | Validation |
| `worker/outbox-ref-unknown` | Validation |
| `worker/outbox-target-wrong-kind` | Validation |
| `worker/outbox-target-suffix-invalid` | Validation |
| `mem/reassembly-pool-variant-missing-max-fragments` | Validation |
| `mem/reassembly-pool-variant-missing-timeout` | Validation |
| `mem/reassembly-slot-size-below-declared-mtu` | Validation |
| `reassembly/max-fragments-insufficient-for-mtu` | Validation |
| `reassembly/expected-fragmentation-rate-high` | Validation |
| `reassembly/untrusted-link-binding` | Validation |
| `reassembly/trust-class-missing-on-fragmenting-link` | Validation |
| `reassembly/stage-copy-wcet-exceeds-slot-budget` | Validation |
| `reassembly/peer-id-not-zid-on-established-session` | Validation |
| `link/listener-link-not-paired-with-established-sibling` | Validation |
| `reassembly/binding-on-unpaired-listener` | Validation |
| `link/concurrent-count-exceeds-scheduler-slots` | Mesh Deploy |
| `link/per-link-budget-exceeds-tick-period` | Mesh Deploy |
| `link/inbound-event-queue-unsized` | Validation |
| `collection/ordering-sorted-requires-index-by` | Validation |
| `collection/overflow-policy-oldest-wins-requires-ordering-insertion` | Validation |
| `collection/element-type-not-a-kind` | Validation |
| `collection/index-by-field-missing` | Validation |
| `collection/multi-writer-without-atomics` | Validation |
| `collection/capacity-unresolved` | Validation |
| `timer/period-below-tick-rate` | Validation |
| `timer/slot-overflow` | Mesh Deploy |
| `extern/symbol-not-in-whitelist` | Validation |
| `extern/abi-mismatch` | Validation |
| `extern/signature-mismatch` | Validation |
| `extern/ordering-unspecified` | Validation |
| `extern/target-plugin-symbol-conflict` | Validation |
| `scxml/top-level-script-unloaded` | Validation |
| `scxml/on-sample-invalid-parent` | Validation |
| `scxml/on-sample-link-duplicate-in-state` | Validation |
| `scxml/on-sample-event-name-conflict` | Validation |
| `scxml/on-sample-link-not-declared` | Validation |
| `scxml/on-sample-link-wrong-kind` | Validation |
| `codegen/no-std-script-not-supported` | Generate |
| `codegen/no-std-http-not-supported` | Generate |
| `codegen/no-std-fs-load-not-supported` | Generate |
| `codegen/no-std-invoke-not-supported` | Generate |
| `expression/empty` | Expression |
| `expression/lex` | Expression |
| `expression/unsupported-construct` | Expression |
| `expression/strict-equality` | Expression |
| `expression/parse-mismatch` | Expression |
| `expression/unexpected-token` | Expression |
| `expression/invalid-lvalue` | Expression |
| `expression/type-coercion` | Expression |
| `expression/go-ternary-unsupported` | Expression |
| `import/file-not-found` | Import |
| `import/kind-mismatch` | Import |
| `import/not-forge` | Import |
| `manifest/circular-dependency` | Manifest |
| `cli/unknown-language` | Cli |
| `cli/unsupported-language` | Cli |
| `cli/missing-metadata-field` | Cli |
| `cli/not-a-directory` | Cli |
| `cli/invalid-format-option` | Cli |
| `cli/format-style-not-found` | Cli |
| `cli/no-scxml-tag` | Cli |
| `mesh/deploy-parse` | Mesh Deploy |
| `mesh/deploy-unsupported-version` | Mesh Deploy |
| `mesh/deploy-duplicate-machine` | Mesh Deploy |
| `mesh/deploy-invalid-ordering-timings` | Mesh Deploy |
| `mesh/deploy-invalid-liveliness` | Mesh Deploy |
| `mesh/deploy-invalid-server-query-timeout` | Mesh Deploy |
| `mesh/deploy-invalid-outbound-buffer` | Mesh Deploy |
| `mesh/deploy-discovery-not-supported` | Mesh Deploy |
| `mesh/deploy-pool-not-supported-by-transport` | Mesh Deploy |
| `mesh/deploy-pool-missing-instance-list` | Mesh Deploy |
| `mesh/deploy-pool-empty-instance-list` | Mesh Deploy |
| `mesh/deploy-pool-invalid-placeholder` | Mesh Deploy |
| `mesh/deploy-server-pool-not-supported` | Mesh Deploy |
| `mesh/deploy-stage-pool-not-declared` | Mesh Deploy |
| `mesh/deploy-stage-pool-wrong-kind` | Mesh Deploy |
| `mesh/deploy-stage-pool-transport-mismatch` | Mesh Deploy |
| `mesh/deploy-scxml-invoke-target-conflict` | Mesh Deploy |
| `mesh/deploy-partition-duplicate-name` | Mesh Deploy |
| `mesh/deploy-partition-multi-device` | Mesh Deploy |
| `mesh/deploy-partition-unit-duplicate` | Mesh Deploy |
| `mesh/deploy-partition-machine-not-listed` | Mesh Deploy |
| `mesh/deploy-partition-empty` | Mesh Deploy |
| `mesh/deploy-partition-synth-infix-collision` | Mesh Deploy |
| `mesh/deploy-partition-uncovered-unit` | Mesh Deploy |
| `mesh/deploy-partition-partial-coverage-requires-default` | Mesh Deploy |
| `mesh/deploy-partition-pool-machine` | Mesh Deploy |
| `mesh/deploy-partition-transport-binding-unsupported` | Mesh Deploy |
| `mesh/deploy-scxml-invoke-cross-device-transport` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-scxml-invoke-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-liveness-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-overflow` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-pin-out-of-range` | Mesh Deploy |
| `mesh/deploy-someip-machine-liveness-service-id-pin-collision` | Mesh Deploy |
| `mesh/deploy-partition-barrier-timeout-invalid` | Mesh Deploy |
| `mesh/partition-parallel-root-undesignated` | Mesh Deploy |
| `mesh/partition-parallel-root-ambiguous` | Mesh Deploy |
| `mesh/partition-parallel-root-not-in-machines` | Mesh Deploy |
| `mesh/partition-parallel-root-non-host` | Mesh Deploy |
| `mesh/partition-barrier-timeout-without-root` | Mesh Deploy |
| `mesh/partition-wire21-custom-tcp-unimplemented` | Mesh Deploy |
| `mesh/distributability-r1-shared-write` | Mesh Deploy |
| `mesh/distributability-r2-cross-region-transition` | Mesh Deploy |
| `mesh/deploy-platform-class-os-mismatch` | Mesh Deploy |
| `deploy/worker-stack-budget-missing` | Mesh Deploy |
| `deploy/worker-slot-budget-missing` | Mesh Deploy |
| `deploy/keepalive-jitter-budget-missing` | Mesh Deploy |
| `deploy/scheduler-incompatible-with-worker-count` | Mesh Deploy |
| `deploy/link-driver-unknown` | Mesh Deploy |
| `deploy/link-mtu-missing-on-fragmenting-link` | Mesh Deploy |
| `deploy/link-mtu-below-driver-floor` | Mesh Deploy |
| `deploy/link-expected-p99-exceeds-mtu` | Mesh Deploy |
| `deploy/link-burst-pps-missing-on-isr-dispatch` | Mesh Deploy |
| `deploy/link-not-declared-in-deploy` | Mesh Deploy |
| `deploy/link-not-declared-in-forge` | Mesh Deploy |
| `deploy/link-burst-absorption-insufficient` | Mesh Deploy |
| `deploy/link-rx-dispatch-worker-tick-on-high-burst` | Mesh Deploy |
| `pool/stage-copy-policy-error` | Validation |
| `pool/stage-copy-accept-rejected-under-forbid` | Validation |
| `deploy/stage-copy-policy-unknown` | Mesh Deploy |
| `deploy/session-arming-quota-missing` | Mesh Deploy |
| `deploy/accept-rate-config-missing` | Mesh Deploy |
| `deploy/session-arming-fields-on-non-arming-link` | Mesh Deploy |
| `deploy/stateless-accept-required-on-untrusted-source` | Mesh Deploy |
| `deploy/stateless-accept-key-rotation-shorter-than-lifetime` | Mesh Deploy |
| `deploy/session-arming-quota-vs-peer-table-invariant-violated` | Mesh Deploy |
| `deploy/stateless-accept-extern-not-whitelisted` | Mesh Deploy |
| `mesh/external-parse` | Mesh External |
| `mesh/external-unresolved-names` | Mesh External |
| `mesh/external-ambiguous-event-group` | Mesh External |
| `mesh/external-empty-event-group` | Mesh External |
| `mesh/external-named-reference-without-config` | Mesh External |
| `mesh/external-reserved-someip-id-keys` | Mesh External |
| `mesh/external-someip-field-on-non-someip-transport` | Mesh External |
| `mesh/external-conflicting-event-schema` | Mesh External |
| `mesh/external-conflicting-event-field-kinds` | Mesh External |
| `mesh/external-empty-event-entry` | Mesh External |
| `mesh/topology-unresolved-targets` | Mesh Topology |
| `mesh/topology-machine-not-found` | Mesh Topology |
| `mesh/topology-receiver-not-declared` | Mesh Topology |
| `mesh/topology-absolute-source-path` | Mesh Topology |
| `mesh/topology-uncovered-events` | Mesh Topology |
| `mesh/topology-pattern-capability-violation` | Mesh Topology |
| `mesh/topology-missing-binding-field` | Mesh Topology |
| `mesh/topology-invalid-binding-field` | Mesh Topology |
| `mesh/topology-event-binding-unused` | Mesh Topology |
| `mesh/topology-ordering-cannot-be-guaranteed` | Mesh Topology |
| `mesh/topology-pool-param-name-missing` | Mesh Topology |
| `mesh/topology-subscription-source-unbound` | Mesh Topology |
| `mesh/topology-machine-lifetime-subscription-unsupported` | Mesh Topology |
| `mesh/codegen-unsupported-language` | Mesh Codegen |
| `mesh/codegen-unsupported-transport` | Mesh Codegen |
| `mesh/codegen-event-name-collision` | Mesh Codegen |
| `mesh/codegen-pool-with-rpc-client-unsupported` | Mesh Codegen |

### Diagnostic-only

I/O and infrastructure failures that the author cannot prevent by
editing the SCXML document. Consumers routing repairs should not
attempt an SCXML-level fix for these; they indicate build-environment
or SCE-internal issues.

| Code | Stage | Reason diagnostic-only |
|---|---|---|
| `xml/xinclude-read-error` | Xml | Filesystem read failure on an `<xi:include>` target |
| `xml/template-read-error` | Xml | Filesystem read failure on a `<sce:use>` template target |
| `import/read-error` | Import | Filesystem read failure on imported file |
| `manifest/io` | Manifest | Filesystem failure during manifest resolution |
| `generate/invalid-config` | Generate | SCE-internal codegen config |
| `generate/template-load` | Generate | SCE template asset load failure |
| `generate/template-render` | Generate | SCE template rendering failure |
| `generate/unsupported-feature` | Generate | SCXML construct exists in the model but the requested target language has no codegen path for it (e.g. `<invoke type="sce:mesh-rpc">` with `--lang rust`) |
| `codegen/mcu-class-kind-on-non-mcu-language` | Generate | Shell-only at PR-0; producer + matrix walker land with the algorithm kind in Phase A3. MCU-class kind authored against a non-MCU language target (watching-zenoh RFC §5.J.4) |
| `codegen/generic-kind-backend-emit-missing` | Generate | Shell-only at PR-0; producer lands with the matrix walker. Generic-class kind expected to emit on a backend per the parity matrix but the per-kind template is absent (SCE bug, watching-zenoh RFC §5.J.5) |
| `io/filesystem` | Io | Generic filesystem failure |
| `cli/read-input` | Cli | Input file read error |
| `cli/write-output` | Cli | Output file write error |
| `cli/create-output-dir` | Cli | Output directory creation error |
| `cli/scxml-generate` | Cli | SCE-internal codegen dispatch |
| `cli/json-serialization` | Cli | SCE-internal serde failure |
| `cli/project-root-not-found` | Cli | Build-environment discovery |
| `mesh/deploy-read` | Mesh Deploy | `deploy.yaml` read error |
| `mesh/external-read` | Mesh External | External catalog read error |
| `mesh/topology-receiver-source-read` | Mesh Topology | Receiver SCXML read error |
| `mesh/topology-receiver-source-parse` | Mesh Topology | Receiver SCXML parse error (I/O-adjacent; the receiver file is not authored by the producing machine) |
| `mesh/codegen-template-read` | Mesh Codegen | Mesh template asset read |
| `mesh/codegen-template-render` | Mesh Codegen | Mesh template rendering failure |
| `mesh/io` | Mesh Io | Generic mesh codegen filesystem failure |
| `forge/source-hash-mismatch` | Cli | `sce-codegen verify` detected drift between an emitted file's embedded §6.2.6 header hash and the recomputed value over current source + template state; not preventable by authoring SCXML (regenerate via `sce-codegen` to repair) |
| `traceability/scxml-line-range-missing` | Generate | watching-zenoh RFC §5.O Atomic 0 IR provenance pre-emit guard: a node eligible for SCE-MAP marker emission reaches the codegen pre-emit walker with `source_location: None`. Codegen-internal invariant — authors never see this signal in practice; the fix lives in the parser site that produced the IR node. |
| `traceability/state-id-collision` | Generate | watching-zenoh RFC §5.O Atomic 1 — symbol mangling collision. Two distinct IR nodes mangle to the same `<machine>__<state_path>__<artifact>` identifier (typically XInclude or `sce:template` composition importing a state fragment whose id collides with a top-level state). The repair (rename one of the two ids) is author-facing, but the wire payload also carries the two colliding `<file>:<line>` sites as candidates. |
| `traceability/symbol-name-exceeds-c-identifier-limit` | Generate | watching-zenoh RFC §5.O Atomic 1 — mangled symbol exceeds C99 §5.2.4.1 external-identifier limit (31 chars). Default rendering is warn (sourcemap still emits); `platform.strict_c99_identifiers: true` in deploy.yaml escalates to hard-error. Repair is to shorten any of the contributing names (machine id, state id, artifact suffix) or relax the strict flag. |
| `traceability/sourcemap-source-hash-mismatch` | Generate | watching-zenoh RFC §5.O Atomic 1 — sourcemap `source_hash` field drifted from the §6.2.6 header's `source-hash`. Codegen-invariant — every `sce_sourcemap.json`'s top-level `source_hash` MUST be byte-equal to the per-file header (spec lines 3321-3324). Not preventable by authoring SCXML; regenerate via `sce-codegen generate` to repair. |
| `traceability/sce-map-attribute-stripped` | Generate | watching-zenoh RFC §5.O Atomic 1 — Rust SCE-MAP `#[doc]` preservation heads-up (OQ-W16 b). The dual-emit fallback (`// SCE-MAP:` line comment, default since Atomic 0c) covers the strip, so this is a warn-only signal that the `#[doc]` form was not preserved by rustdoc under the named profile / target. |
| `traceability/meta-generated-source-line-marker-missing` | Generate | watching-zenoh RFC §5.O Atomic 1 follow-up — codegen-internal traceability invariant: every SCE-emitted file (one carrying a §6.2.6 drift header) must contain at least one `SCE-MAP:` marker line. Fires from `forge::sourcemap::validate_emitted_files_have_markers` walking `out_dir` after every successful `cmd_generate` / `cmd_generate_w3c`. ARCHITECTURE.md "Traceability Ownership Boundary" pins the scope: external meta-generator output (protoc, bindgen, cbindgen) carries no drift header and is silently out-of-scope. Not author-preventable — fix lives in the template that lost its `sce_map_marker` macro call. |

---

## Maintenance

- When a new `DiagnosticCode` variant is added, the compile-time
  guard `all_diagnostic_codes_is_exhaustive` fires first (in
  `sce-build/src/forge/diagnostic.rs`). Add the variant to
  `ALL_DIAGNOSTIC_CODES`, then place its slash-path in exactly one
  of the two appendix tables. The `acceptance_doc_covers_every_code`
  runtime check will otherwise fail with the specific missing code
  name.
- If a code's *classification* changes (boundary ↔ diagnostic-only),
  move the row between tables. `acceptance_doc_covers_every_code`
  asserts exactly one appendix row per code, so duplication or
  deletion during the move is caught.
- When §1 / §2 / §3 prose gains a new construct, link it from the
  relevant appendix row's "Section" column so reviewers can trace
  prose ↔ code coverage. Inline prose mentions are not guarded; only
  the appendix substring is.
- Accurate reflection of partial realization is explicit policy: if a
  §1/§2/§3 feature is in flight, state its status (see §2.5
  communication patterns / §2.6 mesh-rpc). Do not under- or
  over-claim.
