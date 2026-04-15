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
  `<cancel>`, `<assign>`, `<log>`, `<script>`.
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

### §2.7 Deploy manifest (`deploy.yaml`)

Accepted when `--deploy <path>` is passed. Schema is enforced by
serde with `deny_unknown_fields` (`mesh/deploy-parse` on unknown
keys). Only `version: 1` is accepted
(`mesh/deploy-unsupported-version`); machine names are globally unique
across all devices (`mesh/deploy-duplicate-machine`). The
`transports:` block is device-level; `bindings:` is per-target
(see `mesh_phase3_patterns.md`). External event/group/field catalogues
referenced from `bindings:` follow the rules in §2.5 and must resolve
in full (`mesh/external-unresolved-names`).

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

## Appendix — `DiagnosticCode` index (84 codes)

This appendix is the **drift-guarded coverage target** for the
`acceptance_doc_covers_every_code` test. Every slash-path string in
`DiagnosticCode::as_str` appears in exactly one of the two tables
below. The prose sections §1–§3 above reference a subset of these
codes inline, but the appendix is the coverage contract.

### Acceptance boundary

Codes that the author can avoid by writing a better SCXML /
`deploy.yaml` / CLI invocation. Listed in pipeline-stage order.

| Code | Stage | Section |
|---|---|---|
| `xml/parse` | Xml | §1 / XML well-formedness |
| `xml/schema-validation` | Xml | §2 / SCE XSD |
| `validation/missing-element` | Validation | §3.2 / §2 cardinality |
| `validation/missing-attribute` | Validation | §2 required attrs |
| `validation/invalid-attribute` | Validation | §2.2 typed attrs |
| `validation/unsupported-kind` | Validation | §2.1 closed enum |
| `validation/duplicate-id` | Validation | §2.2 field ids |
| `validation/duplicate-context-object` | Validation | §2.3 context ids |
| `validation/empty-collection` | Validation | §2 per-kind cardinality |
| `validation/count-mismatch` | Validation | §2 per-kind cardinality |
| `validation/incompatible-attributes` | Validation | §2 mutual exclusion |
| `validation/missing-context` | Validation | §2.3 |
| `validation/invalid-reference` | Validation | §2 ref integrity |
| `validation/invalid-direction` | Validation | §2 field direction |
| `validation/numeric-parse` | Validation | §3.4 literals |
| `validation/empty-value` | Validation | §2 non-empty attrs |
| `validation/singleton-violation` | Validation | §2 cardinality = 1 |
| `validation/require-either` | Validation | §3.2 required pairs |
| `validation/wrong-pipeline` | Validation | §2 kind/pipeline match |
| `validation/dynamic-features` | Validation | §3.1 dynamic I/O |
| `expression/empty` | Expression | §3.4 |
| `expression/lex` | Expression | §3.4 |
| `expression/unsupported-construct` | Expression | §3.3 / §3.4 |
| `expression/strict-equality` | Expression | §3.4 strict eq |
| `expression/parse-mismatch` | Expression | §3.4 |
| `expression/unexpected-token` | Expression | §3.4 |
| `expression/invalid-lvalue` | Expression | §3.4 |
| `expression/type-coercion` | Expression | §3.4 |
| `expression/go-ternary-unsupported` | Expression | §3.4 go backend |
| `import/file-not-found` | Import | §2.4 `src` resolves |
| `import/kind-mismatch` | Import | §2.4 `kind` assertion |
| `import/not-forge` | Import | §2.4 imported doc shape |
| `manifest/circular-dependency` | Manifest | §2.4 cyclic import graph |
| `cli/unknown-language` | Cli | §1 `-l <lang>` closed set |
| `cli/unsupported-language` | Cli | §1 backend parity |
| `cli/missing-metadata-field` | Cli | CLI metadata args |
| `cli/not-a-directory` | Cli | CLI path shape |
| `cli/invalid-format-option` | Cli | `--error-format` closed set |
| `cli/format-style-not-found` | Cli | format style name |
| `cli/no-scxml-tag` | Cli | input is SCXML |
| `mesh/deploy-parse` | Mesh Deploy | §2.7 deploy.yaml schema |
| `mesh/deploy-unsupported-version` | Mesh Deploy | §2.7 `version: 1` |
| `mesh/deploy-duplicate-machine` | Mesh Deploy | §2.7 global uniqueness |
| `mesh/external-parse` | Mesh External | §2.7 external config parse |
| `mesh/external-unresolved-names` | Mesh External | §2.7 name resolution |
| `mesh/external-ambiguous-event-group` | Mesh External | §2.7 event-group rules |
| `mesh/external-empty-event-group` | Mesh External | §2.7 event-group rules |
| `mesh/external-named-reference-without-config` | Mesh External | §2.7 |
| `mesh/external-reserved-someip-id-keys` | Mesh External | §2.7 someip naming |
| `mesh/external-someip-field-on-non-someip-transport` | Mesh External | §2.7 |
| `mesh/external-conflicting-event-schema` | Mesh External | §2.7 |
| `mesh/external-conflicting-event-field-kinds` | Mesh External | §2.7 |
| `mesh/external-empty-event-entry` | Mesh External | §2.7 |
| `mesh/topology-unresolved-targets` | Mesh Topology | §2.6 / §2.7 |
| `mesh/topology-machine-not-found` | Mesh Topology | §2.7 |
| `mesh/topology-receiver-not-declared` | Mesh Topology | §2.6 |
| `mesh/topology-absolute-source-path` | Mesh Topology | §2.7 relative paths |
| `mesh/topology-uncovered-events` | Mesh Topology | §2.7 event coverage |
| `mesh/topology-pattern-capability-violation` | Mesh Topology | §2.5 pattern/transport |
| `mesh/topology-missing-binding-field` | Mesh Topology | §2.7 binding schema |
| `mesh/topology-invalid-binding-field` | Mesh Topology | §2.7 binding schema |
| `mesh/topology-event-binding-unused` | Mesh Topology | §2.7 binding usage |
| `mesh/codegen-unsupported-language` | Mesh Codegen | §1 backend parity |
| `mesh/codegen-unsupported-transport` | Mesh Codegen | §2.5 transport set |
| `mesh/codegen-event-name-collision` | Mesh Codegen | §2.7 event naming |

### Diagnostic-only

I/O and infrastructure failures that the author cannot prevent by
editing the SCXML document. Consumers routing repairs should not
attempt an SCXML-level fix for these; they indicate build-environment
or SCE-internal issues.

| Code | Stage | Reason diagnostic-only |
|---|---|---|
| `import/read-error` | Import | Filesystem read failure on imported file |
| `manifest/io` | Manifest | Filesystem failure during manifest resolution |
| `generate/invalid-config` | Generate | SCE-internal codegen config |
| `generate/template-load` | Generate | SCE template asset load failure |
| `generate/template-render` | Generate | SCE template rendering failure |
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
