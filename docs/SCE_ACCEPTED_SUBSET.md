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
  (shallow and deep — W3C §3.10.2 requires the default `<transition>`
  child; see [state-reference
  resolution](#statechart-state-reference-resolution)), `<initial>`,
  nested compound states.
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
eighteen variants of `ForgeKind`, written in the `sce:kind` attribute
as the kebab-case tokens below (source of truth:
`sce-build/src/forge/model.rs` `ForgeKind::from_attr`; see
`forge_kinds_catalog.md` for the stateful/stateless/inline-eligible
matrix):

```
statechart   procedure   transform   lookup       condition
codec        validator   filter      interpolation
timer        observer    algorithm   link         worker
buffer-pool  bounded-collection      enum         event-schema
```

Omitting `sce:kind` defaults to `Statechart`. Values outside this set
are rejected as `validation/unsupported-kind`. The phase 2/3 runtime
packages for the stateful kinds (Validator, Filter, Timer, Observer)
are described in `forge_phase3_complete.md`; inline-eligible kinds
(`is_inline_eligible()` → true) may be embedded in a `<data>` element
of an outer statechart.

**Document name — the file stem, not the `name` attribute, except for
`sce:kind="algorithm"`.** The compiled model's name, which every
backend derives its type and file names from, is the document's file
stem. A `name` attribute on the root `<scxml>` element of a forge
document is accepted and ignored: `enum_hex_values.scxml` declaring
`name="opcode"` compiles to `EnumHexValues`, and a document with no
`name` attribute at all compiles the same way. Only the root element's
attribute is inert — `name` on a child (`<sce:variant name>`,
`<sce:flag name>`, `<sce:link name>`) names the thing it sits on and is
used normally.

`sce:kind="algorithm"` is the one exception, because its artifact is a
function rather than a type: the root `name` names the emitted
function, and the file stem does not appear in the output at all.
`algorithm_bytes_equal.scxml` declaring `name="bytes_equal"` emits
`bytes_equal.rs` with `pub fn bytes_equal`, the C11 symbol
`bytes_equal`, and the C++ namespace `SCE::Generated::BytesEqual` —
which is what a cross-document caller resolves against, so the name is
load-bearing rather than decorative there. The rule as stated above
carried no exception until the corpus was measured against it:
`the_identity_rule_holds_for_every_kind_the_corpus_declares` now
generates every committed document whose root `name` disagrees with its
stem and checks which one the artifact takes, so a kind cannot leave
the rule silently.

The grammar admits the attribute (`name` is optional on the root) and
many in-tree documents carry one, so this is stated rather than
enforced: rejecting it would redefine the language against its own
corpus instead of fixing a defect. Examples in this document that
show a root `name` — the `sce:kind="enum"` opt-out sample under
"Opt-out for open-set vocabularies" among them — are naming the
document for the reader, not selecting the emitted type.

This is the one place a forge kind departs from
`sce:kind="statechart"`, where W3C SCXML 5.10 requires the root `name`
attribute to be bound to the `_name` system variable (W3C tests
323/324/329/346).

### §2.2 Typed fields — `<sce:field>`

Structured data carriers used by codec / validator / filter / etc.
kinds. Required attributes:

- `id` — unique within the enclosing kind (duplicates are rejected as
  `validation/duplicate-id`).
- `sce:type` — closed value set of fixed-width scalar tokens
  (source of truth: `SceType::from_attr` in
  `sce-build/src/forge/model.rs`): `uint8`, `uint16`, `uint32`,
  `uint64`, `int8`, `int16`, `int32`, `int64`, `float32`, `float64`,
  `bool`, `string`, `bytes`. An enum-typed field uses the
  `enum:<alias>` form referencing an imported `sce:kind="enum"`
  document (§2 EventSchema / NL→IR Item C1). Values outside this set
  are rejected as `validation/invalid-attribute`.

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

**Name references between documents must resolve.** `<sce:import>` is
the path-based route; a `sce:kind="link"` document also names sibling
documents by *name*, and those names are joined against the build:

- `<sce:framer ref>` names a `sce:kind="codec"` document
  (`link/framer-ref-not-declared` when it names none).
- `<sce:rx-pool ref>`, `<sce:tx-pool ref>` and `<sce:stage-pool ref>`
  name a `sce:kind="buffer-pool"` document
  (`link/pool-ref-not-declared`).

A ref resolves either way it can be spelled: the named document may be
one of the build's inputs, or an `<sce:import>` alias of the matching
kind on the link document itself. Both diagnostics carry the reachable
names of that kind as `Fix::ReplaceOneOf` candidates.

These joins fire from the multi-document entry points — `orchestrate`
and `check` over a document set — and not from a single-document
`generate`, which is handed one file and cannot tell a name declared
elsewhere in the build from one declared nowhere. The distinction is
load-bearing rather than lenient: downstream checks that follow these
refs (`link/pool-slot-smaller-than-framer-max`, the deploy-time
burst-absorption and reassembly validators) skip the link when a ref
does not resolve, which is right for a partial topology and would
otherwise let a misspelt ref switch them off silently.

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
file, then against any operator-configured include directories
(the repeatable `--include-dir` / `-I` flag, in declaration
order), then relative to the current working directory; recursion
is bounded by a documented depth limit (mirrored from the
runtime), and cycles are detected. The include-directory search
path lets a fragment be referenced by bare name independent of
the including file's directory depth; the C++ runtime mirrors the
same precedence via `PugiXMLDocument::setIncludeDirs`.

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

#### §2.8.1 `variant_defaults:` (RFC variant-default-overlay Atomic A)

Optional top-level map carrying per-codec default-arm overrides for
`<sce:variant>` peek-byte dispatch. Wire-spec invariants (bit
positions and `<sce:flag value=...>` MID constants) stay in the
SCXML — they are shared by every consumer. The *choice* of which
arm a freshly-constructed `Default::default()` dispatches to is
per-consumer convention and lives here instead:

```yaml
variant_defaults:
  codec_zenoh_request: 0x03    # client convention: query is the default
  codec_zenoh_response: 0x04   # reply is the default response body
```

Resolution order at codegen time:
1. If `variant_defaults` names the codec, the overlay value selects
   the default arm. `<sce:arm value="V"/>` matching `V == overlay
   value` becomes the Default-trait body; all peer arms have any
   SCXML-side `default="true"` marker cleared.
2. Otherwise the SCXML's own `<sce:arm default="true"/>` marker
   selects the default arm (legacy Atomic α-γ path, unchanged).
3. Otherwise `codec/variant-no-default-arm` fires at the cross-doc
   gate (§5.B Atomic γ-3 contract).

Overlay entries naming a value that no `<sce:arm value=...>`
declares fire `codec/variant-default-overlay-arm-not-declared`;
the `Fix::ReplaceOneOf` candidate set is the codec's declared
arm values (sorted, hex-formatted).

Backward-compat: deploy paths that omit `variant_defaults` (or
omit a specific codec entry) preserve the SCXML's existing
`default="true"` markers byte-identically. The 107 existing
`compile_forge_with_imports` call sites (no deploy) are unaffected.

#### §2.8.2 `<sce:variant-dispatch>` import-site dispatch (RFC §5.B B5-ν inversion)

B5-ν inversion places dispatch ownership at the composition root:
the parent codec declares — at its `<sce:import>` site — which of
its own flags drives an imported variant codec's arm selection.
The leaf codec describes only its body (variant arms + their wire
shapes); it carries no `tag=` attribute and no
`<sce:requires-parent-flags>` block for B5-ν purposes.

```xml
<!-- Leaf: pure body, no parent reference -->
<scxml sce:kind="codec" sce:codec-id="codec_zenoh_keyexpr">
  <datamodel>
    <sce:variant>
      <sce:arm value="0x00" type="codec_keyexpr_nonlocal" default="true"/>
      <sce:arm value="0x01" type="codec_keyexpr_local"/>
    </sce:variant>
  </datamodel>
</scxml>

<!-- Parent declares dispatch at the import site -->
<scxml sce:kind="codec" sce:codec-id="codec_zenoh_push">
  <sce:import src="codec_zenoh_keyexpr.scxml" kind="codec" as="key">
    <sce:variant-dispatch flag="header.M"/>
  </sce:import>
  <datamodel>
    <sce:flags id="header" sce:type="uint8">
      <sce:flag name="mid" bit="0" width="5" value="0x1d"/>
      <sce:flag name="M" bit="6"/>
    </sce:flags>
    <sce:embed id="key" type="key" sce:byte="1"/>
  </datamodel>
</scxml>
```

The leaf's decode signature gains a `tag: u8` parameter; the leaf
matches `tag` directly to pick the arm. Encode is unchanged — the
active arm is the language-level enum discriminant. The parent's
decode extracts the dispatch tag from its own flag carrier
(`(carrier >> bit) & mask`) and passes it to the leaf. The parent's
encode pre-computes the carrier's bit value from the embedded
variant's active arm and ORs it into the carrier before emitting
the carrier bytes.

Parents importing a variant codec **without** `<sce:variant-dispatch>`
fall back to the leaf's `<sce:arm default="true"/>` arm as the
construction-time tag input (Q-D-3 (a)). This is the case when arm
bodies happen to be wire-distinguishable by other means, or when the
author selects the arm at construction.

Cross-doc constraints (parent-local validator):

- `<sce:variant-dispatch flag="X.Y"/>` must resolve against the
  parent's own fields — both the carrier `X` and the flag `Y` must
  exist on the parent →
  `codec/variant-dispatch-flag-not-resolved`
  (`Fix::ReplaceOneOf` candidates = available carriers / flags).
- The named flag's `width` must fit the imported variant's arm count →
  `codec/variant-dispatch-bit-width-mismatch`.
- A parent without `<sce:variant-dispatch>` importing a variant codec
  without a `default="true"` arm cannot resolve the dispatch tag →
  `codec/variant-dispatch-arms-not-distinguishable-without-default`.
- The named flag must not carry a static `<sce:flag value="V"/>`
  constant (the bit is derived, not constant) →
  `codec/variant-dispatch-flag-has-static-value`.
- The flag carrier field must precede the embed field in the parent's
  `<datamodel>` declaration order →
  `codec/variant-dispatch-carrier-after-embed`.

Multi-bit dispatch: B5-ν inversion preserves B5-β's bit-range width
semantics. A `flag="X.Y"` form on a 3-bit flag dispatches over 8
arm values.

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
  (absolute-first, then base-directory, then operator-configured
  include directories via `--include-dir` / `-I`, then cwd), binds
  every non-reserved attribute as a parameter value, and splices
  the rendered body in place of the `<sce:use>` node. Attributes
  named `template` are reserved. With an include directory on the
  search path a case file can reference a shared template by bare
  name (`template="guard.sce-template.xml"`) instead of a
  depth-coupled relative path.
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

Those eight name ways expansion can fail. Expansion *not having been
attempted* is named separately by `xml/preprocessor-not-run`, raised
when a `<sce:use>` or `<xi:include>` survives into parsing. Both
pipelines hold the precondition: the file-based entries
(`SCXMLParser::parse_file`, `compile_forge_file`) run the expander
themselves, but the in-memory entries take already-read content, so a
caller that drives the pipeline itself can hand them unexpanded bytes.
Both parsers then select children by tag name with no else-branch and
skip the directive in silence. In a `lookup` with `sce:default` that
turns a dropped row into a plausible answer rather than a visible
failure; in a statechart it drops whole states from a model that
reports no error.

The check cannot live in the XSD. `<sce:use>` is a declared element
whose containers are `xs:any processContents="lax"`, so the schema
calls an unexpanded document valid by construction — and it must keep
doing so, since template authoring and editor integrations both work
on documents that have not been expanded yet. Only the document tree
can tell "not yet expanded" from "not expandable".

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

- Loose equality / inequality (`==`, `!=`) —
  `expression/strict-equality`. Use `===` / `!==`. Extended SCXML is a
  typed language and admits no implicit coercion, so the operator whose
  ECMAScript meaning *is* coercion has no interpretation here
  (SCE_FORGE.md §3.4). The rejection carries a
  `replace_with` fix, and both lowerings agree on the result: codegen
  emits the target language's `==`, and the script-engine path rewrites
  `===` → Lua `==` / `!==` → `~=`.

  This applies to Extended SCXML expressions — a transition `cond` that
  reads typed `_event.data.<field>` from an imported EventSchema. A
  `cond` on an un-schema'd event is plain ECMAScript evaluated by the
  script engine, where `==` is legal and stays legal (the W3C corpus
  depends on it).
- Go ternary target (source-language-specific restriction for Go
  codegen) — `expression/go-ternary-unsupported`. Restructure with
  `if`/`else`.
- Free-form tokens not part of the grammar —
  `expression/unsupported-construct`, `expression/unexpected-token`,
  `expression/invalid-lvalue`, `expression/type-coercion`,
  `expression/parse-mismatch`, `expression/lex`,
  `expression/empty`, and numeric-parse failures on integer literal
  overflow, which dispatch as `validation/numeric-parse`.

### §2.10 Metadata annotations — `sce:req` / `sce:provenance` / `sce:unresolved`

NL→IR Mapping Roadmap Items 1, 5, and 6 add three metadata
attribute families that any IR generator (NL→IR pipeline,
hand-authored DSL, ARXML transcoder) may attach to `<state>`,
`<parallel>`, `<final>`, `<transition>`, `<onentry>`, `<onexit>`,
the executable-content actions inside those blocks, and
`<invoke>`. The annotations are pure metadata — emitted code is
byte-identical to the unannotated form, byte-stable goldens stay
unchanged.

**`sce:req`** — whitespace-separated requirement IDs.

```xml
<state id="armed" sce:req="REQ_AB_12345 REQ_CD_67890">
  <transition event="go" target="firing" sce:req="REQ_AB_12346"/>
</state>
```

Tokens are opaque to SCE (no shape enforcement — IR generators
own the semantic layer). Duplicates on a single node fail at
parse time with `validation/duplicate-requirement-id`. Block
annotations on `<onentry>` / `<onexit>` inherit onto every
action inside the block, appended after any per-action ids.
`sce-codegen requirements <file>` emits one NDJSON record per
annotated node for downstream req-coverage tooling.

**`sce:provenance`** — spec-document anchors.

Two equivalent forms:

```xml
<state id="armed" sce:provenance="OEM-SPEC-01@23#4.4.2"/>

<state id="armed">
  <sce:provenance doc="OEM-SPEC-01" rev="23" section="4.4.2"/>
  <sce:provenance doc="ISO-14229-1" section="11.2.1"/>
</state>
```

The compact URI form is `doc_id[@rev][#section[:page]]`. The
element form allows multi-document anchoring on a single node.
Pass-through to the diagnostic `spec_provenance` field — SCE
never infers it.

**`sce:unresolved`** — explicit "revisit later" markers.

Two equivalent forms (attribute carries one marker; element
form allows multiple per node):

```xml
<state id="armed"
       sce:unresolved="tbd_threshold"
       sce:unresolved-reason="awaiting calibration"
       sce:unresolved-candidates="42 50 65"/>

<state id="armed">
  <sce:unresolved id="tbd_target" reason="route TBD" candidates="left right"/>
</state>
```

Default builds carry the marker silently in the model — the
`sce-codegen unresolved <file>` NDJSON report surfaces it for IDE
/ linter / dashboard consumers. `--strict-unresolved` on
`generate` lifts the marker to a build-failing
`validation/unresolved-placeholder` so production CI gates
cannot merge unresolved IR.

The three families compose freely on a single node — `sce:req`,
`sce:provenance`, and `sce:unresolved` are orthogonal axes.

### §2.11 Native host actions — `<sce:action>` (W3C SCXML G.7)

A `<sce:action>` is a W3C SCXML §G.7 Custom Action Element that
names a host operation dispatched **without a runtime script
engine**. It is the engine-free counterpart, for *effects*, of the
typed `_event.data` guard lowering: the statechart keeps the
operation symbolic (language-neutral SSOT), each argument flows
through the imported EventSchema's typed-payload channel, and the
host supplies the behaviour by implementing a generated trait.

```xml
<transition event="fragment.received" target="assembling">
  <sce:action name="append_fragment_payload">
    <sce:arg expr="_event.data.payload"/>
    <sce:arg expr="_event.data.offset"/>
  </sce:action>
</transition>
```

The Rust backend emits a `<Machine>Actions` trait (one method per
distinct `name`, parameter types derived from the schema field
types — `bytes → &[u8]`, `uint32 → u32`, …) and a `Policy<A: …>`
generic over the host implementation; the constructor takes the
`actions` value and carries **no `IScriptEngine`**, so a statechart
whose only effects are `<sce:action>`s compiles under `#![no_std]`.

v1 acceptance contract (enforced at the validation stage):

- A `<sce:action>` is a **direct child** of a `<transition>`, an
  `<onentry>` / `<onexit>` block, or initial executable content (an
  `<initial>` transition or a history state's default transition).
  Nesting inside `<if>` / `<foreach>` is rejected — that call site is
  conditional or iterated, which v1 does not lower
  (`validation/native-action-placement`).
- Arguments require the triggering event's typed payload in scope,
  which happens only on a `<transition>`. An `<onentry>` / `<onexit>` /
  initial position has no triggering event, so only a **no-argument**
  `<sce:action>` is admissible there; an arg-bearing one is rejected
  (`validation/native-action-argument`).
- On a transition, each `<sce:arg>` is a bare `_event.data.<field>`
  reference (the `name` attribute, when present, names the trait
  parameter). A literal or derived argument, or one whose triggering
  event imports no EventSchema or whose schema is not all-primitive (an
  enum-typed field — the same eligibility rule as the typed-guard
  channel), is rejected (`validation/native-action-argument`).
- An argument's `<field>` must exist on the triggering event's
  imported EventSchema (`validation/invalid-reference` via the
  cross-kind field resolver).
- A `name` that recurs on more than one transition must carry the
  same argument types each time, so one generated trait method
  serves every call site; a divergence is rejected
  (`validation/native-action-signature-conflict`).
- A no-argument `<sce:action>` (e.g. `reset_slot()`) needs no
  schema and lowers to a bare trait call.

An arg-bearing action reads its values from the event's typed
payload, so the triggering event must be raised via its generated
typed inject; an event raised by name carries no payload and a
debug build `debug_assert!`s rather than silently skipping the
effect (release builds compile the check away).

Only the Rust backend lowers `<sce:action>` natively today; the
other backends reject the construct
(`generate/unsupported-feature`) rather than silently routing it
through a script engine. The construct is engine-free by
definition — it never degrades to a runtime fallback.

### §2.12 Unsupported `<invoke type>` (W3C SCXML 6.4.1)

An `<invoke>` whose `type` names no processor this platform
implements is **accepted, not rejected as malformed**: §scxml-6.4.1
defines the case — the processor "MUST place error.execution in the
internal event queue" — so the document is valid SCXML with a
defined meaning. Its entire observable is that one raise at invoke
time. No child session starts, `done.invoke.<id>` never fires, and
state exit has nothing to cancel.

```xml
<state id="probe">
  <invoke type="urn:example:no-such-processor"/>
  <transition event="error.execution" target="handled"/>
</state>
```

The supported set is closed: the `scxml` shorthand, the SCXML
processor URI with and without its trailing slash, and
`sce:mesh-rpc` (SCE_MESH.md §9.5). `typeexpr` resolves the type at
runtime and is therefore never classified at build time.

Both engines raise the same event, by different routes:

| Engine | §6.4.1 behaviour |
|---|---|
| Interpreter | `InvokeHandlerFactory::createHandler` returns null for a type outside the supported set and `InvokeExecutor::executeInvoke` raises `error.execution`. |
| AOT — C++ / Rust / Kotlin / Go / Python / C11 | The `<invoke>` is deferred at entry and the raise fires at macrostep end, preserving §scxml-6.4 ordering. Leaving the state first cancels the pending entry along with every other deferred invoke, so a machine that exits before the macrostep boundary raises nothing. |

No backend refuses the document. Refusing it would be a conformance
break rather than a coverage boundary: §6.4.1 assigns the construct a
meaning instead of declaring it malformed, so an AOT target that
rejected it would reject valid SCXML. The `<sce:action>` refusal above
is not a precedent here — that construct is an SCE extension with no
W3C meaning to preserve.

Accepting the document while emitting no raise is the failure this
subset clause guards against, because it reproduces the original
defect, in which the `<invoke>` vanished from the model entirely and
AOT produced no observable at all where the Interpreter produced one.
Wiring one backend does not close it for the rest: the model variant
that carries the unsupported invoke is skipped by each template's
`scxml`-family filter unless that backend is wired explicitly, so the
silent drop moves from the parser into the templates rather than
disappearing.

The runtime witness is
`integration_resources/invoke_unsupported_type/invoke_unsupported_type.scxml`,
driven on all seven channels (C++ Interpreter + AOT, Rust, Kotlin, Go,
Python, C11). It rests in its `probe` state and never completes on any
channel that drops the `<invoke>`.

### Cross-kind typed binding (NL→IR Mapping Roadmap Item 2)

When an importing kind references an imported kind's field via
`<sce:import as="alias"/>` + `alias.field` syntax in an expression,
the parser walks every expression site after import enrichment and
validates the reference against the imported kind's declared member
surface. Three rejection codes:

- `validation/cross-kind-field-not-found` — alias resolves but field
  does not. Diagnostic carries a closed `Fix::ReplaceOneOf` set =
  the imported kind's full member surface (sorted, deduplicated) so
  consumers see the legal alternatives for `did_you_mean`-style
  typo repair.
- `validation/cross-kind-type-mismatch` — field resolves but its
  declared type is incompatible with the surrounding use-site
  contract (signature return type, `<sce:param type=...>`, …).
  Silent when the use site does not constrain the expected type
  (`Unknown` context). NL→IR Mapping Roadmap Item 4 also routes
  physical-quantity unit mismatches in arithmetic to this same code
  (typed payload `ValidationError::QuantityUnitMismatch`) — adding
  `<sce:param sce:quantity="celsius"/>` + `<sce:param sce:quantity="kelvin"/>`
  inputs to a Transform whose body combines them arithmetically
  triggers the code with the operator and both unit names in the
  diagnostic message.
- `validation/cross-kind-circular-dependency` — the `<sce:import>`
  graph contains a cycle. Defensive check; without it, the enrichment
  pass recurses into infinite open-file work or surfaces as an opaque
  stack-overflow at codegen.

Today the validator is wired only on the Forge→Forge path (a Forge
document's expressions reference another Forge document imported via
`<sce:import>`) — the silent-broken pattern the
`infer_types`-returns-`Unknown` fall-through historically allowed.
The diagnostic codes themselves are kind-agnostic: a future
Statechart→Forge binding would wire through the same validator
without renaming codes or extending payload shape.

---

### Statechart state-reference resolution

Every id an SCXML document uses to name a state must resolve to a
`<state>`, `<parallel>`, `<final>`, or `<history>` declared in that
document. Four reference positions carry the rule:

| Position | Spec | Rule |
|---|---|---|
| `<transition target>` | W3C SCXML §3.5, §3.13 | Every whitespace-separated token resolves independently. A targetless transition (no `target` attribute) is not a reference. |
| `<state initial>` | W3C SCXML §3.3 | Every token names a child of the owning state. |
| `<initial>` child | W3C SCXML §3.6 | The initial element's transition target resolves; the parser folds it into the owning state's `initial`. |
| `<history>` default | W3C SCXML §3.10.2 | The default `<transition>` child is **required**, and its target resolves. |

Rejection codes:

- `validation/invalid-reference` — a token names nothing. `actual`
  carries the unresolved id and `fix.candidates` the legal set: every
  declared state for a transition target, the owning state's children
  for a compound `initial` (§3.3 restricts the initial configuration to
  descendants, so a wider list would offer illegal values).
- `validation/missing-element` — a `<history>` declares no default
  `<transition>`. This is a declaration rule, not a use rule: the child
  is required whether or not any transition names the pseudostate,
  because without it the pseudostate can never be entered. The legal
  default targets travel in `message` — SCE_ERROR_CONTRACT §3.1 has no
  add-child-element `fix` variant.

Both rules are enforced on every path that parses a document, and both
engines carry them: the Rust producers are
`sce-build/src/scxml_references.rs` and the `<history>` arm of
`parser.rs`, and the C++ Interpreter's counterparts are
`SemanticTransitionTargetUnknown` / `SemanticInitialStateUnknown` /
`SemanticHistoryDefaultMissing` thrown from
`SCXMLParser::validateModel`.

Why these are rejections rather than warnings: the code generators
lower a transition target to a `State` enum variant. An id that names
nothing lowers to a variant the generated enum never declares, so the
document would otherwise pass `check` with `status: ok`, pass
`generate`, and fail in the consumer's compiler.

---

### Design-time lints are opt-in (`--lint`)

Three validators below — graph reachability, event-set exhaustiveness,
and guard analysis — **reject legal SCXML**. Each flags a document the
W3C algorithms accept and an Interpreter runs; what they assert is
design intent, not validity. They are therefore off by default and
enabled with `sce-codegen check --lint` / `generate --lint`, which call
the same `sce_build::lint_statechart` the library entry points run
(`sce-build/tests/cli_lint_parity.rs` pins the two verdicts equal).

The W3C IRP corpus is why the default is off — these are conformance
documents that build and pass:

| Document | Shape the lint flags | Why the document is correct |
|---|---|---|
| `resources/278` | `s1` unreachable | `s1` exists only to host a `<datamodel>`; the test checks that `s0` can read a variable from outside its lexical scope |
| `resources/576` | `s0` unreachable | The test proves `<scxml initial>` is honoured, which requires the document-order-first state to stay unentered |
| `resources/355` | `s1` unreachable | The test distinguishes default entry by document order; entering `s1` would be the failure |

Turn the lints on for authored documents, where an orphan region or a
sibling missing an event handler is nearly always a mistake.

The rules a document must satisfy to be lowered **at all** — reference
resolution, a resolvable `initial`, at least one state, a loadable
top-level `<script>` — are not lints and always run, on every entry
point (see [state-reference
resolution](#statechart-state-reference-resolution)).

---

### Statechart graph reachability (NL→IR Mapping Roadmap Item 3 Phase A)

Every `<state>`, `<parallel>`, and `<final>` declared in an SCXML
document must be reachable from the document's initial configuration
through the W3C SCXML §3 entry semantics:

- the document `initial` attribute (or the default-first-child fallback
  when omitted)
- compound-state initial-cascade — entering `<state initial="X">`
  enters X (and recurses)
- parallel-all-children — entering `<parallel>` enters every
  non-history child region
- transition `target` edges
- history pseudostate default-target redirection (W3C SCXML §3.10)
- ancestor entry — entering a state enters every compound ancestor
  (§3.6), and an entered ancestor is a full member of the
  configuration: its own transitions are live (§3.13 selection climbs
  the ancestor chain) and, when it is a `<parallel>`, its every region
  is entered (§3.4). A walk that marked ancestors reached without
  following their edges reported `fail` states as orphans across the
  W3C IRP suite

After the parse completes, a BFS over those edges computes the
design-time reach set. A state outside the closure is dead code —
codegen would still emit per-state surface for it, but no execution
path ever enters it. Two rejection codes:

- `scxml/unreachable-state` — the orphan-state form, emitted when an
  unreachable `<state>` / `<parallel>` / `<final>` declares no
  `<transition>` children. The diagnostic carries only the state id;
  closest-match candidate lists are not surfaced because the orphan's
  id is typically correct — the topology is the bug.
- `scxml/dead-transition` — the per-transition form, emitted when an
  unreachable state contains at least one `<transition>`. The
  per-(source, target) granularity points the author at a concrete
  edge to delete or re-wire. Outranks the state-level form so each
  orphan subgraph reports its first transition rather than just the
  containing state.

Both rejection paths sit in the `scxml/*` family because reachability
is a Statechart-graph rule with no analog on the Forge-kind side
(Forge kinds carry no control-flow surface).

---

### Statechart event-set exhaustiveness (NL→IR Mapping Roadmap Item 3 Phase B)

A compound `<state>`'s sibling children are expected to agree on
event coverage when they share a vocabulary: if children A, B, and C
all handle the `cmd.*` event family, but only A and B declare a
transition for `cmd.stop` while C does not (and the parent has no
fallthrough), the gap in C is almost always an authoring mistake.
AI-generated SCXML produces this pattern frequently — the model
emits a coherent handler set for some siblings and forgets the
others.

The validator uses a narrow heuristic to keep false positives at
zero across the W3C IRP, conformance, and downstream-consumer corpora:

- The compound parent must be a non-`<parallel>`, non-`<final>`
  `<state>` (parallel regions are orthogonal by design and do not
  participate in this check).
- The siblings under consideration are direct child `<state>` /
  `<parallel>` nodes that have at least one `<transition>`
  (`<final>` and history pseudostates excluded — they have no
  transition surface to compare). At least two such siblings must
  exist for the check to fire.
- The siblings must share **common ground**: there must exist at
  least one event matched by every transition-carrying sibling
  (W3C SCXML §5.10 prefix-match semantics apply). The "sequential
  protocol stages with disjoint event vocabularies" pattern that
  prevails in the W3C IRP suite (e.g., one stage handles
  `childToParent` only, the next stage handles `pass`/`fail`/
  `timeout` only) has no common ground and is silently accepted.
- For each event `E` in the union of literal event tokens (no
  wildcards), if at least one sibling handles `E` and at least one
  does not, and the parent itself has no transition matching `E`,
  the validator emits `scxml/non-exhaustive-event-handling`.

Author escape hatch: `sce:unhandled="E1 E2"` on **the child that
leaves the events unhandled** — not on the compound parent. The
declaration exempts exactly the (child, event) pairs it names.

The attribute sits on the child because that is the grain at which
the author's claim is true: "`berserk` does not handle
`combo_timeout`", not "this compound is exempt". A parent-level
opt-out (`sce:exhaustive="false"`) existed in an earlier revision
and was withdrawn — it silenced every gap under the parent,
including gaps introduced after it was written, so a sibling added
later inherited an exemption nobody had judged. A document still
carrying `sce:exhaustive` rejects via `validation/invalid-attribute`
rather than being ignored, because an unrecognised `sce:` attribute
is accepted and ignored and the exemption would otherwise be lost
silently.

Token rules: whitespace-separated literal event names, at least
one, each named at most once, no wildcards. Wildcards are rejected
so the declaration is checked against the literal gap set under one
matching rule rather than two.

The declaration is checked in both directions, so it cannot decay
into unverified prose:

- A state declaring an event it actually handles (directly, by
  token-prefix, or via a wildcard transition) rejects with
  `scxml/contradictory-unhandled-declaration`.
- A state declaring an event that is not a gap under its parent —
  no sibling handles it, the parent absorbs it, the siblings share
  no common ground, or the state has no compound parent — rejects
  with `scxml/stale-unhandled-declaration`.

Repair guidance, in author preference order:

1. Add the missing `<transition event="E" ...>` to the non-handling
   sibling.
2. Add a parent-level `<transition event="E" ...>` so the event is
   absorbed by the compound state regardless of which child is
   active.
3. Declare `sce:unhandled="E"` on the non-handling child if the gap
   is genuinely intentional.

---

### Statechart guard analysis (NL→IR Mapping Roadmap Item 3 Phase C)

`<transition cond="...">` guards can be statically false (the
transition never fires) or be shadowed by an earlier unconditional
sibling (per W3C SCXML §5.10 transition selection, the first
matching transition in document order wins). Both patterns are
authoring mistakes that survive parse + reachability today.

The validator stops short of full SMT to keep the false-positive
surface at zero — it recognises only the structurally trivial
cases:

`scxml/always-false-guard` fires when the `cond` attribute matches
one of:

- The literal `false` (lowercase per W3C SCXML §B ECMAScript
  convention).
- The numeric literal `0`.
- A binary equality `N == M` where both sides parse as decimal
  numeric literals with differing values (`1==2`, `0==1`,
  `42==99`). Whitespace around `==` is tolerated.
- A binary inequality `N != M` where both sides parse as decimal
  numeric literals with equal values (`1!=1`, `0!=0`).

Language-prefixed `cond` values (`cpp:expr`, `kotlin:expr`,
`rust:expr`) remain opaque — the validator never inspects them.
Their semantics depend on the host language's expression
evaluator, which the parser cannot reason about statically without
risking false positives.

`scxml/shadowed-transition` fires when a state's `<transition>`
list contains an unconditional transition (empty `cond`, literal
`cond="true"`, or literal `cond="1"`) followed by a same-event
sibling. The shadowing transition matches every event the shadowed
one matches, so per W3C SCXML §5.10 it always wins and the later
one is dead. The validator requires literal equality of the
`event` attribute between the two transitions — token-prefix
superset cases (`event="foo"` shadowing `event="foo.bar"`) depend
on ancestor-priority rules the parser-stage walker cannot
disambiguate without running the full selection algorithm, so they
are deliberately not flagged.

Repair guidance:

1. Remove the dead transition.
2. Rewrite the guard to a satisfiable expression.
3. For shadowed transitions, reorder so the more specific transition
   precedes the unconditional one, or add a guard to the previously
   unconditional transition.

---

### Physical-quantity annotation (NL→IR Mapping Roadmap Item 4)

`<sce:field>` and `<data>` elements may carry an
`sce:quantity="<unit>"` attribute, optionally paired with
`sce:scale="<rational>"` and `sce:offset="<rational>"`. The triple
declares a linear `physical = raw * scale + offset` conversion in
the named opaque unit (SI base units `s` / `m` / `kg` / `A` / `K` /
`mol` / `cd` are recommended but not enforced — the unit string is
treated as an opaque equality key across operands).

The conversion is **codegen-effective** (not documentation-only):

* Codec fields carrying a quantity annotation emit a raw↔physical
  accessor pair (`<id>_phys()` / `set_<id>_phys()` per backend; or
  the language-idiomatic case-converted equivalent — `<id>Phys()`
  for Kotlin, `<Id>Phys()` for Go). The raw struct member retains
  its wire-level integer/float type; the accessor performs the
  conversion in IEEE-754 double precision.
* Transform `<data direction="in">` parameters and outputs may also
  declare a quantity. The generated `compute_*` function signature
  is unchanged (the function consumes raw, returns the body
  expression's typed result); a doc-comment block on the function
  surfaces the unit annotation for downstream readers.

`sce:scale` accepts decimal integers (`42`, `-17`), decimal
fractions (`0.5`, `-40.25`), and explicit `<num>/<denom>` ratios
(`1/100`). Scientific notation, hexadecimal, leading `+`, and zero
denominator are rejected at parse time with
`validation/invalid-attribute`. `sce:scale="0"` is also rejected —
a zero scale means the raw value never influences the physical
reading, which makes the annotation observably equivalent to
deleting both the scale and the unit.

`sce:scale` or `sce:offset` without `sce:quantity` is rejected
(the conversion factor needs a unit to anchor against). An empty
`sce:quantity=""` is rejected as a missing unit name.

The type system threads the quantity through expression inference:
binary arithmetic between two `Quantity`-typed operands carrying
**different** unit tags surfaces as
`validation/cross-kind-type-mismatch` via the typed
`ValidationError::QuantityUnitMismatch` payload (no new
`DiagnosticCode` slot — the validator reuses the cross-kind code
under the "type incompatibility" concept umbrella). Quantity
combined with concrete bare numeric strips the unit
(explicit-typed authorship opts out of unit checking at that
site); quantity combined with an untyped literal keeps the
annotation sticky.

ARXML COMPU-METHOD blocks map onto this layer directly: the
INTERNAL value is the raw, the PHYS value is the physical, and the
COMPU-NUMERATOR / COMPU-DENOMINATOR pair becomes
`sce:scale="<num>/<denom>"`. The widely-deployed automotive
temperature encoding `physical = raw * 0.5 - 40` Celsius for an
`int8` raw byte authors as:

```xml
<data id="raw_temp" sce:type="int8" sce:direction="in"
      sce:quantity="celsius" sce:scale="0.5" sce:offset="-40"/>
```

---

### EventSchema kind (NL→IR Mapping Roadmap Item C1 Path A)

A typed contract for the `_event.data` payload of a named SCXML
event — each schema names exactly one event and declares the typed
fields authors may read via `_event.data.<field>` in transition
`cond` attributes and write via `<send>/<param>` payloads.

**Surface forms** (DL-8' — both produce identical IR shapes):

- Top-level form (primary):
  `<scxml sce:kind="event-schema" sce:event-name="job.completed">…</scxml>`
- Inline form (sugar, deferred to F-ζ — out of scope for Path A
  α): `<sce:event-schema event-name="job.completed">…</sce:event-schema>`
  as a child of `<scxml>`. The top-level form is the only Path A
  α surface; inline lowering ships when an inline consumer
  surfaces.

**Field declaration** uses `<data id="..." sce:type="..."
sce:direction="in"/>` inside a `<datamodel>` wrapper. `sce:type`
may be any of the primitive `SceType` values (`uint8` / `uint16`
/ `uint32` / `uint64` / `int8` / `int16` / `int32` / `int64` /
`float32` / `float64` / `bool` / `string` / `bytes`) or
`enum:<enum-alias>` for an enum drawn from an imported
`sce:kind="enum"` document (Path A's 17th kind).

**Direction invariant** (DL-5'): `sce:direction` must be `in` —
the payload is the receiver's read-only view. `out` / `internal`
directions raise `validation/invalid-attribute` at parse time.

**Schemaless fallback** (DL-9'): events without an imported
EventSchema retain the dynamic `_event.data` baseline — no
diagnostic, identical W3C IRP behavior. W3C built-in event
namespaces are explicitly excluded:

- `validation/event-schema-on-builtin-event` — an EventSchema
  document declares `sce:event-name` against a W3C SCXML reserved
  event prefix (`error.*`, `done.invoke.*`, `done.state.*`). The
  platform raises these events with implementation-defined
  payload shape; an authored schema cannot meaningfully constrain
  them. Repair: rename the schema's `sce:event-name` to a
  non-reserved value or delete the schema document.

**Receive-side typecheck** (DL-5'): `_event.data.<field>`
expressions in transition `cond` attributes resolve against the
imported EventSchema for the transition's event. Field-not-found
surfaces through `validation/cross-kind-field-not-found` (existing
code reused per Item 4 precedent; carries the schema's declared
field surface as `Fix::ReplaceOneOf` candidates). Comparison
type-mismatch (e.g. `_event.data.elapsed_ms === 'forty-two'`
against a `uint32` field) surfaces through
`validation/cross-kind-type-mismatch` (also reused per Item 4
precedent).

A `bytes`-typed field compares against a printable-ASCII string
literal by value (`_event.data.raw === 'ack'`). Such a guard lowers
natively on all six backends — each to its own byte-equality
primitive over the same decoded constant (Rust `== b"ack"`, C++
`== std::vector<uint8_t>{…}`, Go `string(..)== "ack"`, Kotlin
`.contentEquals("ack".toByteArray())`, Python `== b"ack"`, C11
`_len == N && memcmp(.., "ack", N) == 0`) — so no script engine is
required. The C11 payload field is a no-alloc bounded buffer
(`uint8_t[CAP]; size_t _len`) whose `CAP` comes from `sce:max-size`
(default 256). Two `bytes`-specific rejections layer on top of the
shared receive-side checks (RFC `rfc-eventschema-bytes-guard.md` §3):

- `validation/bytes-comparison-not-equality` — an ordering operator
  (`<`, `>`, `<=`, `>=`) applied to a `bytes` field. Lexicographic
  ordering of an opaque payload byte-blob is not a meaningful author
  intent; only equality (`===` / `!==`) lowers to a well-defined,
  byte-identical comparison on every backend. A distinct
  operator-domain rule, so a dedicated wire code.
- `validation/cross-kind-type-mismatch` (reused per Item 4
  precedent) — the string literal carries a backslash escape or a
  non-ASCII byte. Such a literal has no unambiguous cross-backend
  byte constant, so it is rejected as a type-category mismatch; the
  printable-ASCII boundary is a validated, forward-compatible
  literal-syntax scope (the byte carrier widens later without
  touching the wire form).

**Send-side payload typecheck** (DL-4'): a `<send event="X">` or
`<raise event="X">` (in transition `actions`, `<onentry>`, or
`<onexit>` content, including nested `<if>` / `<foreach>` bodies)
whose event name `X` resolves to an imported EventSchema has its
`<param name="F" expr="..."/>` children validated against the
schema's declared field surface. Two rejections:

- `validation/event-payload-field-unknown` — `<param name="F">`
  carries a name `F` that is not declared on the schema. The
  schema's full field surface ships as a closed
  `Fix::ReplaceOneOf` candidate set, mirroring the receive-side
  `validation/cross-kind-field-not-found` shape so `did_you_mean`-
  style typo repair surfaces identically on both sides.
- `validation/cross-kind-type-mismatch` (reused per Item 4
  precedent) — `<param expr="...">` is a primitive literal whose
  syntactic type does not unify with the schema field's declared
  `sce_type`. Non-literal expressions (variable references,
  nested computations, function calls) defer to the existing
  typed-expression pipeline at codegen time.

**Mesh cross-machine validation** (DL-7'): when `<send target="#X">`
appears on a statechart deployed to machine A and addresses
machine B (the `#X` resolves to B's machine id via the
deploy.yaml topology), the sender and receiver EventSchemas for
the send's event name must agree on field shape. One rejection:

- `mesh/event-schema-mismatch` — either (a) both sides declare an
  EventSchema but their canonical structural hashes diverge
  (fields sorted by id, normalized JSON via schemars), (b) sender
  declares a schema while receiver does not, or (c) receiver
  declares a schema while sender does not. The `Display` form
  names the specific divergence reason so consumers do not have
  to substring-grep the prose. Repair is two-axis (realign the
  two schemas, or declare a schema on the side that is missing
  it) — author-domain choice, no closed candidate set.

**Per-backend payload codegen** (DL-6' continuation) ships the
per-backend payload struct surface (`struct <Schema>Payload` on
C++, `pub struct <Schema>Payload` on Rust, `data class
<Schema>Payload` on Kotlin, typed struct on Go, `@dataclass` on
Python, `typedef struct { … } <Schema>Payload_t` on C11) so the
cross-backend parity gate (§6.2.6) closes in lockstep.
Enum-typed fields reference the imported Enum kind's qualified
type per backend (`SCE::Generated::<E>::<E>` on C++, `<e>::<E>`
on Rust, wildcard-import bare `<E>` on Kotlin, `<e>.<E>` on Go,
`<e>.<E>` on Python, `<E>_t` on C11) — no variant re-emission;
the Enum document remains the single source of truth.

**Cross-doc Enum literal width narrowing** (DL-5'): integer
literals compared against (receive-side) or assigned to
(send-side) an enum-typed field are narrowed against the
imported enum's declared `underlying_type`. Decimal, hex (`0x`),
binary (`0b`), and octal (`0o`) literal forms all parse, with an
optional leading `-` — the carrier may be signed
(`int8`/`int16`/`int32`/`int64` alongside the unsigned four), so a
negative sentinel is an ordinary comparison. Values outside the
carrier's range in either direction raise
`validation/cross-kind-type-mismatch` (reused per Item 4
precedent — no new wire code). The narrowing fires only when the
statechart's own `<sce:import kind="enum" as="<alias>">` resolves
the enum alias; otherwise the narrowing silent-skips
(conservative-accept default — category check still requires the
literal to be an integer).

**Strict variant membership** (F-κ): after the width narrowing
layer accepts an integer literal against an enum-typed field, the
membership layer verifies the literal's value is one of the
imported enum's declared `<sce:variant value="…"/>` set. A
comparison like `_event.data.status === 7` against an enum
declaring `{ok=0, error=1, timeout=2}` fits the underlying carrier
but is not a declared variant, and raises
`validation/cross-kind-type-mismatch` (same reuse precedent — no
new wire code). The diagnostic enumerates the declared variant set
in declaration order so authors can pick the value they meant.
Receive-side and send-side branches share a single membership
helper so the diagnostic shape is identical on both sites; the
width-overflow diagnostic always wins when both conditions hold
(a literal that overflows the underlying carrier is the more
fundamental violation and surfaces first).

**Opt-out for open-set vocabularies**: enum documents that declare
only a partial set of values with the expectation that wire-side
values outside the declared set are legal (open-set status
vocabularies — e.g. UDS NRC, OEM-extensible response codes) opt
out of the membership check via `sce:strict-variants="false"` on
the enum's `<scxml>` root:

```xml
<scxml sce:kind="enum" name="UdsNrc"
       sce:underlying-type="uint8"
       sce:strict-variants="false">
  <datamodel>
    <data id="variants">
      <sce:variant name="generalReject" value="16"/>
      <!-- OEM extensions accepted at runtime; declared set is partial -->
    </data>
  </datamodel>
</scxml>
```

The opt-out is owned by the declaring vocabulary (not the
consuming statechart) so the decision lives with the schema-of-
record. Width narrowing still runs regardless of this opt-out —
overflow of the declared underlying carrier remains a wire-level
violation. Default is `true` (strict): an enum that does not set
the attribute is checked as closed-set, matching the typed-
vocabulary intent of `sce:kind="enum"`.

---

## Appendix — `DiagnosticCode` index (343 codes)

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
| `xml/preprocessor-not-run` | Xml |
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
| `validation/native-action-placement` | Validation |
| `validation/native-action-argument` | Validation |
| `validation/native-action-signature-conflict` | Validation |
| `validation/mesh-rpc-reserved-param` | Validation |
| `validation/mesh-rpc-missing-target` | Validation |
| `validation/mesh-rpc-duplicate-target` | Validation |
| `validation/removed-attribute` | Validation |
| `validation/bytes-max-size-violation` | Validation |
| `validation/duplicate-requirement-id` | Validation |
| `validation/unresolved-placeholder` | Validation |
| `validation/cross-kind-field-not-found` | Validation |
| `validation/cross-kind-type-mismatch` | Validation |
| `validation/cross-kind-circular-dependency` | Validation |
| `validation/enum-no-variants` | Validation |
| `validation/enum-variant-duplicate-name` | Validation |
| `validation/enum-variant-duplicate-value` | Validation |
| `validation/enum-variant-value-overflows-underlying` | Validation |
| `validation/enum-unsupported-underlying-type` | Validation |
| `validation/event-schema-on-builtin-event` | Validation |
| `validation/event-payload-field-unknown` | Validation |
| `validation/bytes-comparison-not-equality` | Validation |
| `mesh/event-schema-mismatch` | Validation |
| `algorithm/local-shadows-param` | Validation |
| `algorithm/lvalue-unsupported` | Validation |
| `algorithm/return-missing` | Validation |
| `algorithm/foreach-source-not-iterable` | Validation |
| `algorithm/call-target-unknown` | Validation |
| `algorithm/call-target-method-unknown` | Validation |
| `algorithm/bc-mutation-forbidden` | Validation |
| `algorithm/foreach-source-bc-with-bytes-item-type` | Validation |
| `algorithm/call-arg-count-mismatch` | Validation |
| `algorithm/append-target-not-buffer` | Validation |
| `algorithm/append-type-mismatch` | Validation |
| `algorithm/const-not-foldable` | Generate |
| `algorithm/const-fold-budget-exceeded` | Generate |
| `algorithm/const-yield-type-mismatch` | Generate |
| `codec/variant-arm-unreachable` | Validation |
| `codec/variant-duplicate-default-arm` | Validation |
| `codec/variant-arm-mid-mismatch` | Validation |
| `codec/variant-arm-inner-mid-undeclared` | Validation |
| `codec/variant-arm-body-caller-tag-unsupported` | Validation |
| `codec/variant-no-default-arm` | Validation |
| `codec/variant-default-overlay-arm-not-declared` | Validation |
| `codec/variant-dispatch-flag-not-resolved` | Validation |
| `codec/variant-dispatch-bit-width-mismatch` | Validation |
| `codec/variant-dispatch-arms-not-distinguishable-without-default` | Validation |
| `codec/variant-dispatch-flag-has-static-value` | Validation |
| `codec/variant-dispatch-carrier-after-embed` | Validation |
| `codec/flag-bind-input-not-declared` | Validation |
| `codec/flag-bind-source-not-resolved` | Validation |
| `codec/flag-bind-width-mismatch` | Validation |
| `codec/flag-input-unbound` | Validation |
| `codec/flag-bind-duplicate-input` | Validation |
| `codec/flag-bind-carrier-after-embed` | Validation |
| `codec/present-if-refs-later-field` | Validation |
| `codec/repeat-count-refs-later-field` | Validation |
| `algorithm/test-vector-unsupported-kind` | Validation |
| `codec/tlv-chain-depth-unspecified` | Validation |
| `codec/tlv-chain-truncate-under-entry-flag` | Validation |
| `codec/dma-alignment-unsatisfiable` | Validation |
| `codec/peek-byte-flag-layout-mismatch` | Validation |
| `link/framer-missing` | Validation |
| `link/link-class-unknown` | Validation |
| `link/backpressure-undeclared` | Validation |
| `link/class-unsupported-on-target` | Validation |
| `link/pool-slot-smaller-than-framer-max` | Validation |
| `link/pool-ref-not-declared` | Validation |
| `link/framer-ref-not-declared` | Validation |
| `mem/pool-section-conflict` | Validation |
| `mem/pool-too-large` | Validation |
| `mem/inter-pool-padding-not-emitted` | Validation |
| `mem/cache-line-alignment` | Validation |
| `mem/dcache-line-size-not-power-of-two` | Validation |
| `mem/alignment-not-power-of-two` | Validation |
| `mem/slot-size-not-alignment-multiple` | Validation |
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
| `scxml/unreachable-state` | Validation |
| `scxml/dead-transition` | Validation |
| `scxml/non-exhaustive-event-handling` | Validation |
| `scxml/contradictory-unhandled-declaration` | Validation |
| `scxml/stale-unhandled-declaration` | Validation |
| `scxml/always-false-guard` | Validation |
| `scxml/shadowed-transition` | Validation |
| `scxml/on-sample-invalid-parent` | Validation |
| `scxml/on-sample-link-duplicate-in-state` | Validation |
| `scxml/on-sample-event-name-conflict` | Validation |
| `scxml/on-sample-link-not-declared` | Validation |
| `scxml/on-sample-link-wrong-kind` | Validation |
| `scxml/unknown-session-role-kind` | Validation |
| `scxml/duplicate-session-role-declaration` | Validation |
| `link/deploy-role-listener-without-scxml-accept-side-role` | Validation |
| `scxml/accept-side-role-without-listener-link` | Validation |
| `link/role-listener-with-non-session-arming-trust-class` | Validation |
| `scxml/accept-side-states-without-role-declaration` | Validation |
| `reassembly/per-peer-quota-build-invariant-violated` | Validation |
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
| `mesh/deploy-invalid-dedup-window` | Mesh Deploy |
| `mesh/deploy-invalid-custom-tcp-socket` | Mesh Deploy |
| `mesh/deploy-invalid-dds-qos` | Mesh Deploy |
| `mesh/deploy-invalid-liveliness` | Mesh Deploy |
| `mesh/deploy-invalid-server-response-deadline` | Mesh Deploy |
| `mesh/deploy-invalid-outbound-buffer` | Mesh Deploy |
| `mesh/deploy-invalid-retry-policy` | Mesh Deploy |
| `mesh/deploy-invalid-auth-policy` | Mesh Deploy |
| `mesh/deploy-discovery-not-supported` | Mesh Deploy |
| `mesh/deploy-pool-not-supported-by-transport` | Mesh Deploy |
| `mesh/deploy-pool-missing-member-list` | Mesh Deploy |
| `mesh/deploy-pool-empty-member-list` | Mesh Deploy |
| `mesh/deploy-pool-binding-field-not-supported` | Mesh Deploy |
| `mesh/deploy-pool-dispatch-without-member` | Mesh Deploy |
| `mesh/deploy-pool-invalid-placeholder` | Mesh Deploy |
| `mesh/deploy-server-pool-not-supported` | Mesh Deploy |
| `mesh/deploy-cross-target-reply-not-supported` | Mesh Deploy |
| `mesh/deploy-invalid-reply-from` | Mesh Deploy |
| `mesh/deploy-unknown-binding-field` | Mesh Deploy |
| `mesh/deploy-stage-pool-not-declared` | Mesh Deploy |
| `mesh/deploy-stage-pool-wrong-kind` | Mesh Deploy |
| `mesh/deploy-stage-pool-transport-mismatch` | Mesh Deploy |
| `mesh/deploy-scxml-invoke-target-conflict` | Mesh Deploy |
| `mesh/deploy-partition-duplicate-name` | Mesh Deploy |
| `mesh/deploy-partition-multi-device` | Mesh Deploy |
| `mesh/deploy-partition-unit-duplicate` | Mesh Deploy |
| `mesh/deploy-partition-machine-not-listed` | Mesh Deploy |
| `mesh/deploy-partition-empty` | Mesh Deploy |
| `mesh/deploy-partition-name-not-identifier` | Mesh Deploy |
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
| `deploy/link-driver-class-mismatch` | Mesh Deploy |
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
| `codegen/mcu-class-kind-on-non-mcu-language` | Generate | Shell-only at PR-0; producer + matrix walker land with the algorithm kind in Phase A3. MCU-class kind authored against a non-MCU language target (SCE Protocol-Synthesis RFC §5.J.4) |
| `codegen/generic-kind-backend-emit-missing` | Generate | Shell-only at PR-0; producer lands with the matrix walker. Generic-class kind expected to emit on a backend per the parity matrix but the per-kind template is absent (SCE bug, SCE Protocol-Synthesis RFC §5.J.5) |
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
| `forge/source-hash-input-uncovered` | Cli | the §6.2.6 `source-hash` about to be embedded in generated output would not describe the input that produced it — the collected set is empty (the header would carry the empty-input digest) or, where the root was inferred from the input's own location, omits that input; an invocation-layout failure, not an authoring one (re-point `--input-root` at a directory containing the input) |
| `forge/source-hash-walk-unbounded` | Cli | the §6.2.6 source set could not be enumerated within the walk's descent ceiling — a directory symlink naming a sibling contributes under every name that reaches it, so nested levels of such links name a path count exponential in the depth; refused rather than truncated, since a digest folded over the prefix the walk reached describes a subset of the input and is unauditable in the same way the empty-input digest is. An invocation-layout failure, not an authoring one (re-point `--input-root` below the aliasing, or remove it) |
| `traceability/scxml-line-range-missing` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 0 IR provenance pre-emit guard: a node eligible for SCE-MAP marker emission reaches the codegen pre-emit walker with `source_location: None`. Codegen-internal invariant — authors never see this signal in practice; the fix lives in the parser site that produced the IR node. |
| `traceability/state-id-collision` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — symbol mangling collision. Two distinct IR nodes mangle to the same `<machine>__<state_path>__<artifact>` identifier (typically XInclude or `sce:template` composition importing a state fragment whose id collides with a top-level state). The repair (rename one of the two ids) is author-facing, but the wire payload also carries the two colliding `<file>:<line>` sites as candidates. |
| `traceability/symbol-name-exceeds-c-identifier-limit` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — mangled symbol exceeds C99 §5.2.4.1 external-identifier limit (31 chars). Default rendering is warn (sourcemap still emits); `platform.strict_c99_identifiers: true` in deploy.yaml escalates to hard-error. Repair is to shorten any of the contributing names (machine id, state id, artifact suffix) or relax the strict flag. |
| `traceability/sourcemap-source-hash-mismatch` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — sourcemap `source_hash` field drifted from the §6.2.6 header's `source-hash`. Codegen-invariant — every `sce_sourcemap.json`'s top-level `source_hash` MUST be byte-equal to the per-file header (spec lines 3321-3324). Not preventable by authoring SCXML; regenerate via `sce-codegen generate` to repair. |
| `traceability/sce-map-attribute-stripped` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 — Rust SCE-MAP `#[doc]` preservation heads-up (OQ-W16 b). The dual-emit fallback (`// SCE-MAP:` line comment, default since Atomic 0c) covers the strip, so this is a warn-only signal that the `#[doc]` form was not preserved by rustdoc under the named profile / target. |
| `traceability/meta-generated-source-line-marker-missing` | Generate | SCE Protocol-Synthesis RFC §5.O Atomic 1 follow-up — codegen-internal traceability invariant: every SCE-emitted file (one carrying a §6.2.6 drift header) must contain at least one `SCE-MAP:` marker line. Fires from `forge::sourcemap::validate_emitted_files_have_markers` walking `out_dir` after every successful `cmd_generate` / `cmd_generate_w3c`. ARCHITECTURE.md "Traceability Ownership Boundary" pins the scope: external meta-generator output (protoc, bindgen, cbindgen) carries no drift header and is silently out-of-scope. Not author-preventable — fix lives in the template that lost its `sce_map_marker` macro call. |
| `mcu/driver-header-not-found` | Validation | SCE Protocol-Synthesis RFC §5.2 Round F-α — a top-level `<sce:driver href="..."/>` reference on the SCXML root cannot be resolved against `deploy.yaml`'s `platform.driver_root` (or the SCXML file's parent directory as fallback). The referenced header is the author's contract with the C11 backend: `*_sm.c` `#include`s the resolved path, so absence breaks cross-TU symbol resolution before any C compiler can speak up. Repair is author-domain — fix the `href` value, add the missing file, or set `platform.driver_root` so the relative path resolves. |
| `mcu/section-attribute-on-non-mcu-target` | Generate | SCE Protocol-Synthesis RFC §5.2 Round F-α — `platform.c11_section_attribute` is set in `deploy.yaml` but the target codegen backend is not `c11`. The section attribute injects `__attribute__((section("...")))` syntax that only the C11 backend emits; non-MCU backends (cpp / rust / kotlin / go / python) have no equivalent contract and reject the field by design (Q-Round-F-D3 lock, mirrors Q-Call-7 non-MCU pattern). Repair is multi-axis — remove the section attribute, switch the backend to `c11`, or split deploy configurations per target. |
| `mcu/section-attribute-name-invalid` | Generate | SCE Protocol-Synthesis RFC §5.2 — `platform.c11_section_attribute.class` names a section the C11 emitter cannot place verbatim into a string literal. The name reaches two nested string contexts (a plain C string in `__attribute__((section("...")))`, and a string inside a string in `_Pragma("location=\"...\"")` on IAR), so a quote or backslash would terminate one of them. Accepted characters are letters, digits, `.`, `_`, `$` and `-`; the name must be non-empty. Repair is a rename in `deploy.yaml` and in the linker script that places the section. |

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
