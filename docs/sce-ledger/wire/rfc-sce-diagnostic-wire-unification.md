# RFC: AOT ↔ Interpreter Diagnostic Wire Contract Unification

**Update 2026-04-26 — W3 (XInclude family) landed.** 5 commits
FF-merged to main on `feat/sce-wire-w3`
(`9342257d`..`a14fb6ae`):

- `9342257d` extracts `computeFnv1aDiagnosticId` from
  `TemplateError.cpp`'s anonymous namespace to a free function in
  `Diagnostic.h` shared by both error families.
- `2bf2cbab` refits `XIncludeExpansionError` to implement
  `Diagnostic` with 7 typed leaves
  (`XIncludeMissingHref`/`NotFound`/`ReadError`/`Cycle`/`TooDeep`/
  `Malformed`/`Unsupported`) one-to-one with Rust `XIncludeError`
  variants and `xml/xinclude-*` `DiagnosticCode`s; rewrites the
  10 expander throw sites to use typed leaves; extends the
  `XIncludeErrorWire.{Missing,Empty}Href` consumer probes with
  `EXPECT_EQ` on `code()`.
- `74e9f72d` pins `XIncludeError` JSON wire against v1 schema —
  7 `ConformsToV1Schema` tests + `EveryCuratedXIncludeCodeIsExercised`
  count guard + `IdDiffersAcrossSubtypesWithSameMessage`.
- `acabf63d` adds dual-side drift tests
  `cpp_xinclude_subtypes_match_rust_diagnostic_codes` +
  `cpp_xinclude_subtype_code_returns_rust_wire_string`.
- `a14fb6ae` re-throws typed `XIncludeExpansionError` from
  `PugiXMLDocument::processXInclude` so the leaf reaches
  `SCXMLParser`'s typed catch arms; both `parseFile` and
  `parseContent` populate `addError` (Q4-B legacy) +
  `recordDiagnostic` (typed `getDiagnostics()`).

W3 design pins: **W3-B** (parallel surface on concrete class) over
**W3-A** (extend `IXIncludeProcessor` interface) — the interface
is `@deprecated` per its header doc and has only one concrete
impl, so forcing every future impl to carry both surfaces would
have no consumer signal. **One subtype** for missing+empty href
(Rust folds them). **No source-location stamping on XInclude
throw sites yet** — `setLocation` exists on the base for parity
but no expander throw site calls it; gated on a consumer asking
for typed coords vs the `(row, col)` already in the message text.
**Path A (re-throw typed)** over Path B (parser polls
`IXMLDocument` typed surface) — same interface-extension family
rejected at session start.

**Update 2026-04-26 — W2 + audit #1 boundary flatten landed.** 4
commits FF-merged to main on `feat/sce-wire-w2`
(`60b591dd`..`f9df4468`):

- `60b591dd` adds `Diagnostic::clone()` pure virtual + 8 leaf
  overrides; `SCXMLParser::getDiagnostics()` parallel typed surface;
  typed catch arm records cloned diagnostic. Closes §W1 audit #1.
- `fe374762` adds `Diagnostic::to_canonical_json_string()` (impl in
  new `sce/src/parsing/Diagnostic.cpp`).
- `198b4a89` adds `emit_json_diagnostics(vec, ostream)` batch NDJSON
  formatter mirroring Rust `--error-format=json` line shape.
- `f9df4468` pins the end-to-end pipeline:
  `parseContent` → `getDiagnostics` → `emit_json_diagnostics` →
  `nlohmann::json::parse`.

W2 design pins: Path A (clone() virtual + leaf overrides) over Path
B (throw `unique_ptr`); library API only — no CLI binary, no
`--error-format=json` flag on any C++ binary; C++ id stays
message-text-derived (cross-side id byte-equivalence remains W3+);
schema status stays `pre-release`; bindings unchanged (Q4-B / RFC §A
non-finding "language binding blast radius is zero").

**Update 2026-04-26 — W0 + W1 landed.** 5 commits FF-merged to main
on `feat/sce-wire-w0-w1` (`86dff500`..`cb0306ca`):

- W0 deletes `sce/src/main/main.cpp` (pins "no C++ codegen CLI" boundary).
- W1-1 adds `sce/include/parsing/Diagnostic.h` abstract base.
- W1-2 refits `TemplateError` to implement `Diagnostic`; FNV-1a id
  arithmetic mirrors `sce-build/src/forge/diagnostic.rs::Fnv1a64`.
- W1-3 pins `to_json()` schema-conformance for all 8 subtypes.
- W1-4 pins each C++ `code()` literal to its Rust wire string via
  `cpp_template_subtype_code_returns_rust_wire_string`.

**Two design points diverged from rev2:** (a) **No `Severity` enum** —
neither Rust `Diagnostic` nor v1 schema carries severity; rev2's §Q6
reasoning ("byte-diff parity requires severity") was a false premise,
verified by grep. (b) **Boundary flatten removal (rev2 §W1 #5)
landed in W2 (2026-04-26) — see header note above.** C++ id derives
from message text (single key fragment) so it is schema-valid but
does not byte-match Rust's structured-key id; cross-side id
correlation remains W3+ scope.

Full landing notes in `memory/wire_rfc_w0_w1_landed.md`.

**Status:** Draft (Milestone-roadmap landing contract). First pass
plus preflight audit revisions — design questions pinned, honest
scope boundaries set, and W1 deliverable updated against concrete
audit findings. W1 lands when this RFC's questions are resolved
with user signoff; downstream milestones reopen as each milestone's
consumer signal arrives.

**Preflight audit (2026-04-23, pre-W1).** Two-pass audit of the
C++ codebase surfaced **seven findings** total; see §A below for
the raw evidence. The revisions are inline in the sections they
affect. First-pass findings #1-#3 forced W1 scope expansion and
W5a prerequisite; second-pass findings #4-#7 added W0, rewrote
W2, and expanded W5 further.

- **#1 boundary flatten** — `SCXMLParser::parseFile` catches
  Phase B's typed `TemplateError` as `const std::exception&` and
  `addError(ex.what())`, erasing the type. → W1 now includes a
  **typed-aware catch chain**.

- **#2 no pugi offset tracking** — semantic call sites have no
  source position. → W5 prerequisite **W5a** (pugi offset capture)
  added.

- **#3 no JSON schema validator lib** — only `nlohmann/json`
  linked. → W1 conformance method changed to
  **canonical-JSON byte-diff against Rust output**.

- **#4 sub-parser bool-chain** (W5 impact) — ActionParser /
  StateNodeParser / DataModelParser / TransitionParser /
  GuardParser / InvokeParser / DoneDataParser all return `bool`;
  error detail leaks to `SCE_LOG_ERROR` logs but never into
  structured errors. Parent only learns "sub-parse failed",
  addError records a generic parent-level message. → W5
  prerequisite **W5b** (sub-parser interface redesign,
  `bool parse()` → diagnostic-emitting shape across 7 classes)
  added.

- **#5 abstract interface surface** (W3/W4 impact) —
  `IXMLParser::getLastError() const -> std::string` and
  `IXIncludeProcessor` (in `sce/include/model/`) are pure-virtual
  string-based contracts. → W3/W4 scope sketches now document
  the **extend-interface vs parallel-method tradeoff** per
  Q4-B permanent-coexistence discipline.

- **#6 C++ codegen duplicate** (W0 deletion) —
  `sce/src/main/main.cpp` (170 lines, header self-identifies as
  "SCXML-to-C++ Code Generator") is a **placeholder dumper**,
  not real codegen (line 142: `// TODO: Generate state machine
  logic based on SCXML`; line 143: `// This is a placeholder
  implementation`). Not registered in CMake build graph (grep
  returns 0 matches for build targets referencing `src/main/main`).
  → New **W0 milestone** deletes this dead-code CLI unambiguously.
  Architecture boundary confirmed per session discussion:
  **CLI surface is Rust `sce-codegen` alone; C++ side exposes
  Interpreter diagnostics via library API only (no C++ CLI
  binary)**.

- **#7 severity levels absent** (W3-W5 impact) — C++ has
  `addError(string)` only; no warning / info distinction. Rust
  `Diagnostic` has severity. → New **§1 Q6** decides day-one
  severity policy on `Diagnostic` base class so W3-W5 do not
  each have to relitigate.

**Scope:** Reduce the divergence between two error-reporting wire
surfaces today:

- **AOT (`sce-build` Rust side):** Typed `ForgeError` + named
  `DiagnosticCode` enum + JSON NDJSON envelope emitted via
  `--error-format=json`, schema at
  `schemas/sce-diagnostic.v1.schema.json`. Structured and
  machine-consumable.
- **Interpreter (C++ runtime side):** `SCXMLParser::addError(const
  std::string& message)` plain-string accumulation + `SCE_LOG_*`
  logging. Unstructured bytes; consumers that want
  machine-readable errors re-parse strings or re-run on the AOT
  path.

Template errors already agree 1:1 at the semantic level: eight
`SCE::parsing::Template<Variant>` subtypes map to eight
`xml/template-*` DiagnosticCode names (pinned by
`cpp_template_subtypes_match_rust_diagnostic_codes`, landed Phase B
M4). This RFC extends that *error-class* agreement to the *wire
surface*: structured code + location + fix hints on the C++ side,
optionally emitted as JSON matching the Rust schema.

**Driver:** Direct user request on 2026-04-23 — user-signal trigger
fired. The original Phase B RFC §2 named this explicitly as "a
separate future RFC", and Phase B M5's "scope markers not built"
list named it again. This RFC is that separate document.

**Scale honesty.** This is NOT a Phase B-sized effort. The scope
touches every parsing diagnostic path (not just templates), every
validator in `SCXMLParser` / `SCXMLValidator` / semantic validators,
and potentially every embedder that reads C++ error bytes. A
complete unification may span 5+ milestones and weeks of session
time. This RFC deliberately pins only W1 as a commit-series
contract and leaves W2-W5 as *scope sketches* — each downstream
milestone re-opens for its own design questions when its consumer
signal arrives. Landing W1 without a named W2 consumer is the
`feedback_built_but_unconsumed.md` anti-pattern; the RFC carries
the consumer-naming discipline forward.

**What this RFC does NOT cover:** Phase C (`PositionMap` C++ port),
which sits in a separate document `claudedocs/rfc-sce-template-phase-c.md`
drafted in parallel. Phase C provides `TemplateError::location()`;
this RFC decides how that location is *serialised* on the output
wire. The two RFCs share a natural downstream dependency but were
kept separate per the original Phase C prompt's explicit
"blast-radius" warning: mixing wire-format design into Phase C
would have forced premature decisions on producer-side surfaces
that have no consumer signal yet.

---

## §1 Design questions pinned this session

Each question here forces a W1-W5 deliverable (§3). The RFC is not
speculative — every decision has a named consumer, and milestones
without a named consumer remain `pending` sketches.

### Q1: Scope boundary — producer-side retype, consumer-side adapter, or both?

**Options:**

- **A. Producer-side retype (big-bang).** Walk every
  `addError(string)` call site in `sce/src/parsing/` and
  `sce/src/validation/` and promote each to a typed
  `SCE::parsing::Diagnostic` throw or structured append. ~100+
  call sites surveyed (`grep -rn addError sce/src/ | wc -l`),
  many of which re-use the same message template with slot
  substitution. Largest possible scope.

- **B. Consumer-side adapter (minimal).** Leave `addError(string)`
  untouched; add a new `SCXMLParser::addDiagnostic(Diagnostic)`
  sibling API. New-style call sites use the typed API; old-style
  call sites stay as strings. JSON wire formatter reads the typed
  side and best-effort-matches the string side via pattern
  matching. Smallest immediate change; longest tail of string call
  sites that never migrate.

- **C. Hybrid (typed where Phase B typed, string elsewhere).**
  Template errors already have typed subtypes — promote the
  Template<Variant> subtypes to implement a common
  `Diagnostic` interface, and extend to one additional error
  class family per milestone as consumer signal arrives.
  `addError(string)` stays as the catch-all fallback.

**Chosen:** **C. Hybrid, milestone-gated.** Each milestone promotes
one error-class family from string to typed when a downstream
consumer asks for it. No big-bang. `feedback_built_but_unconsumed.md`
is the enforcement discipline: a class family gets typed *because*
a consumer asks, not *in anticipation* of one asking.

**Why not A:** 100+ call sites with no current consumer signal for
most of them is the `feedback_built_but_unconsumed.md` anti-pattern
at maximum scale. Even if we had infinite session time, typing
every call site produces infrastructure that downstream parsers /
tools may never read. Rejected.

**Why not B:** Pattern-matching strings on the consumer side to
recover DiagnosticCode is fragile (string edits in producer-side
code silently break consumer-side parsers) and violates the
separation the Rust `ForgeError → Diagnostic` pipeline already got
right. Rejected.

**Concrete W-milestone contract:** W1 promotes the already-typed
`TemplateError` family (Phase B's 8 subtypes) to emit a
`Diagnostic` on the wire. Subsequent milestones (W2 onward) reopen
this RFC when the *next* consumer asks.

### Q2: Wire format — match Rust `sce-diagnostic.v1` schema exactly, or new C++-specific envelope?

**Options:**

- **A. Match Rust schema exactly.** C++ side's JSON envelope
  conforms byte-for-byte (modulo whitespace) to
  `schemas/sce-diagnostic.v1.schema.json`. Same consumers parse both
  outputs with one codepath.

- **B. C++-specific envelope.** Design a new schema tailored to
  Interpreter runtime errors (which can include runtime state
  context the AOT path lacks). Divergence from Rust side
  permanent.

- **C. Superset / subset.** C++ emits a superset of Rust (adds
  runtime fields) OR subset (drops AOT-only fields). Consumer
  reads based on schema version.

**Chosen:** **A. Match Rust schema exactly.** Unification is a
misnomer if the two sides emit different envelopes. Any
downstream consumer should be able to parse both sides' output
with one parser.

**Consequences:**
- C++ side gains an `emit_json_diagnostic(Diagnostic)` formatter
  that outputs NDJSON records conforming to `v1` schema.
- Schema file's `x-sce-schema-status` stays `pre-release`; the C++
  implementation is a second independent conformer that the schema
  must accommodate. Any schema change requires dual-side
  validation (CLAUDE.md `SCHEMA_STATUS = "pre-release"` guardrail).
- If the C++ runtime needs a field the Rust AOT side doesn't
  (e.g. runtime state at error time), schema v2 is the path, not
  a C++-specific extension field. Single schema evolution.

**Why not B:** Defeats the RFC's premise. Rejected.

**Why not C:** Schema supersets are technically fine but create
versioning complexity (consumers MUST read schema version). The
Rust side already emits v1; until there's a concrete C++-only
field that can't fit v1, stay on v1 exactly.

### Q3: DiagnosticCode enum — monolithic C++ enum, or per-module typed enums?

**Rust shape.** `sce-build/src/forge/diagnostic.rs::DiagnosticCode`
is a monolithic `enum DiagnosticCode` with ~80 variants (xml,
semantic, forge, mesh families). Every diagnostic carries exactly
one code.

**Options:**

- **A. Monolithic C++ enum.** Port the Rust enum directly to
  `sce/include/parsing/DiagnosticCode.h`, including all 80+
  variants, all 11 touchpoints from CLAUDE.md's
  `diagnostic_code_edit_checklist.md`. Drift-test agreement
  pinned end-to-end.

- **B. Per-module code families.** `SCE::parsing::TemplateCode`,
  `SCE::parsing::XIncludeCode`, `SCE::parsing::XmlCode`, etc.
  Each family maps to the Rust prefix (e.g. `xml/template-*`
  mirrors `TemplateCode`). Monolithic enum not required; JSON
  wire carries the fully-qualified string `"xml/template-cycle"`.

- **C. `std::string_view` code identity, no enum.** Fully-qualified
  code strings as the canonical identity; enums are optional
  compile-time conveniences. Diagnostic carries a
  `std::string_view code` field directly.

**Chosen:** **B. Per-module code families, JSON wire carries the
string.** Monolithic enum requires synchronising 11 touchpoints
across two languages for every new variant; per-module splits the
drift surface so a template-family change doesn't force a mesh-family
rebuild. String-on-wire is already the Rust JSON behaviour
(`"code": "xml/template-cycle"`), so wire-level drift is caught by
schema conformance, not by enum-order agreement.

**Why not A:** Synchronisation burden across the 11 touchpoints
memory-listed in CLAUDE.md scales poorly. Every new Rust variant
would force a simultaneous C++ header edit, and a skipped edit is
a silent wire-drift bug. Per-module splits localise that burden.

**Why not C:** Loses compile-time exhaustiveness checks. Code like
`switch (code_family) { case Cycle: ...; case NotFound: ...; }`
needs the enum; `string_view` switch cascades are verbose and
error-prone. Reject for readability.

### Q4: Migration path — deprecation vs coexistence

**Options:**

- **A. Deprecate `addError(string)`.** Mark as `[[deprecated]]`,
  flag each call site migrated-or-not. Each milestone closes more
  deprecation warnings. Final milestone deletes the API.

- **B. Permanent coexistence.** Both APIs exist forever. Typed
  API for new code; string API for legacy / low-priority paths.
  No forced migration.

- **C. Rename + wrap.** Rename `addError(string)` to
  `addError_legacy(string)`; new `addDiagnostic(Diagnostic)` is
  the new entry. `addError_legacy` wraps as
  `Diagnostic::make_from_legacy_string(...)` internally so JSON
  wire covers both call styles (best-effort code inference for
  legacy strings).

**Chosen:** **B. Permanent coexistence, milestone-by-milestone
migration under consumer pressure.** No `[[deprecated]]` pressure.
The anti-migration argument: `addError` is fine for today's
consumers. A forced migration produces churn across ~100 call
sites with no functional benefit unless a consumer asks for the
JSON wire on those specific code paths.

**Why not A:** Deprecation pressure without a consumer for the
typed form is the `feedback_built_but_unconsumed.md` anti-pattern
at industrial scale. Rejected.

**Why not C:** Wrapping legacy strings with heuristic code
inference silently produces wrong codes for unexpected message
shapes. Wire correctness requires producer-side intent, not
consumer-side guessing. Rejected.

**What "permanent coexistence" means in practice:**
- W1-W5 each promote one error family at a time.
- Unmigrated families continue using `addError(string)` forever
  (unless their consumers ask).
- The JSON wire emitted by `--error-format=json` (if we add that
  flag to the C++ side) covers only typed diagnostics; string
  accumulation appears in a parallel legacy block on the wire,
  or is omitted when JSON mode is active (consumer choice, pinned
  in W1).

### Q5: Schema conformance — `pre-release` status and the `schema_file_declares_status` guardrail

**Anchor.** CLAUDE.md: `SCHEMA_STATUS = "pre-release"`, and the
schema file's `x-sce-schema-status` must match. While pre-release,
non-additive changes are allowed. Flipping to `"stable"` requires
updating both the const and the schema file in one commit
(guarded by `schema_file_declares_status` drift test).

**Phase-C-era status:** Schema stays `pre-release` throughout this
RFC's milestones. W1 adds a new conformer (C++ side); that is
additive to the schema's consumer set, not a schema-shape change.
No schema edit required by W1.

**When schema flips to `stable`:** When both AOT and Interpreter
conformers are shipped AND the `x-sce-schema-status` field is
flipped to `"stable"` AND the `SCHEMA_STATUS` Rust const is
flipped in the same commit, the schema becomes a back-compat
contract. Pre-stable, schema shape can break either side. Once
stable, any schema change requires a v2 bump. This RFC does NOT
flip to stable; that flip is its own consumer-gated decision,
separate from the unification work itself.

### Q6: Severity policy — `Diagnostic::severity()` day-one or deferred?

**Audit finding #7.** Rust `Diagnostic` carries a severity
field (error / warning / advice); C++ has only `addError(string)`
with no severity distinction. Every W3-W5 migration would have
to relitigate what severity each specific call site emits unless
the `Diagnostic` base class pins the shape up-front.

**Options:**

- **A. Day-one on `Diagnostic` base, all-W1-errors-are-`Error`.**
  The `severity()` method lands in W1 on the base class.
  Template family in W1 hard-codes `severity() = Severity::Error`
  (all 8 subtypes are hard failures today, none warn). Later
  families (W3-W5) override where they have warn-semantics.

- **B. Deferred — no severity field until a consumer asks.**
  W1 base class has no `severity()`. W3/W4/W5 each introduce it
  if needed at their milestone.

- **C. Present but optional — `std::optional<Severity>
  severity() const { return std::nullopt; }` default.**
  Consumers that don't care (wire JSON fills `"severity": null`
  or omits the field) aren't affected; producers that do care
  override.

**Chosen:** **A. Day-one on base, all-W1-errors-are-Error.**

**Why:**
- **Byte-diff parity requires it.** Rust emits `"severity":"error"`
  for every template-family diagnostic (verified in Rust test
  fixtures). If C++ omits the field or emits `null`, the
  canonical-JSON byte-diff test (W1 #6) fails on a trivial
  format mismatch, not a content mismatch. Either both sides
  emit the field or neither does; Rust already emits, so C++
  must match.
- **No revisit cost.** Locking severity in W1 means W3/W4/W5
  inherit a settled policy — each new subtype declares its
  severity at construction time, and that's it. Per-milestone
  reopening of "should this have severity?" is a recurring tax
  we pay if we choose B or C.
- **Enum shape trivial.** `enum class Severity { Error, Warning,
  Advice };` mirroring Rust. Per-module drift test pins the
  three variants against Rust source.

**Why not B:** Recurring relitigation at every W milestone; plus
W1 byte-diff test would red on missing `"severity"` field,
forcing us to retrofit in W1 anyway. Rejected for being the
longer path.

**Why not C:** `std::optional<Severity>` on the wire produces
`null` vs Rust's `"error"` — byte-diff fails. We could special-case
the JSON formatter to always emit `"error"` when severity is
nullopt, but then the field is a lie. Rejected for wire
dishonesty.

**Implementation shape (W1):**
```cpp
// sce/include/parsing/Diagnostic.h
enum class Severity { Error, Warning, Advice };

class Diagnostic {
public:
    virtual Severity severity() const noexcept = 0;
    virtual std::string_view code() const noexcept = 0;
    // ...
};
```

Every `TemplateError` subtype returns `Severity::Error` in W1.
W5 semantic family may override to `Severity::Warning` for cases
where parse continues (e.g. unreferenced state declared).

---

## §2 Out of scope (this RFC does not add)

- **Phase C PositionMap port.** Covered in
  `claudedocs/rfc-sce-template-phase-c.md`. This RFC consumes
  `TemplateError::location()` once Phase C P2 populates it, but
  does not specify the coordinate-remap mechanism itself.

- **Big-bang migration of all `addError(string)` call sites.** Q1-C
  hybrid approach — each migration requires consumer pressure.

- **Schema v2 evolution.** Q5 — schema stays `v1 pre-release`
  throughout this RFC. Any v2 bump is its own RFC.

- **Runtime error reporting (post-parse).** `addError` today covers
  parse-time diagnostics only. Runtime state errors (invoke
  failures, event-queue overflows, send-action failures) have
  their own logging paths. This RFC does NOT extend wire
  unification to runtime errors — parse-time only, matching the
  Rust `sce-build` scope boundary.

- **Non-parsing validators.** `sce/src/runtime/` + `sce/src/codegen/`
  have their own error surfaces; those are runtime / build-tool
  concerns, not parser-wire concerns.

- **Retrofitting pattern-matching on legacy string messages** to
  back-fill DiagnosticCode for unmigrated call sites. Q4-C
  rejected; W-milestones migrate explicitly when a consumer asks.

- **Replacing `SCE_LOG_*`.** Logging is orthogonal to error wire.
  Logs stay as human-readable bytes.

- **XInclude-family error typing.** XInclude errors are today
  string-based; typing them requires its own milestone under its
  own consumer signal. Not named as a milestone in this RFC;
  reopens when a consumer asks.

---

## §3 Milestone roadmap

Unlike Phase B / Phase C which named all milestones as commit-series
contracts at RFC time, this RFC pins **only W1** as a commit-series
contract. W2-W5 are *scope sketches* — each milestone re-opens
design questions when its consumer signal arrives. This discipline
keeps the RFC honest about what has named consumers versus what is
speculative.

**Architecture boundary (pinned this session).** The CLI surface
is **Rust `sce-codegen` alone** (already carries
`--error-format=json`, verified 32 tests green). The C++ side
exposes Interpreter diagnostics via **library API only** (no CLI
binary on the C++ side, no `--error-format=json` flag on any C++
binary). This boundary is enforced by W0 (dead-code C++ CLI
deletion) and by W2's library-API-only scope.

Rationale: SCE serves two architecturally distinct surfaces —
(a) a **build-time toolchain** that invokes SCE as a subprocess
and consumes structured diagnostics from stdout/stderr, and
(b) a **runtime embedder** that links against `libsce_core` and
consumes typed diagnostics as C++ objects. Rust `sce-codegen`
serves surface (a); the C++ Interpreter library serves surface
(b). Two CLIs would be duplicate surface. Who ultimately reads
the output on either surface (CI pipelines, code generators,
IDE plugins, embedded application telemetry, human developers,
automated tooling) is outside SCE's concern — SCE provides the
surfaces; consumers choose.

| M | Subject | Status | Trigger condition |
|---|---|---|---|
| W0 | **Delete `sce/src/main/main.cpp` + related CMake/docs.** Audit #6: file is a 170-line placeholder ("TODO: Generate state machine logic"), not real codegen; not registered in CMake build graph (verified). Architecture boundary: no C++ codegen CLI. | commit-series contract (this RFC) | **Prerequisite for W1** — clean slate before the library-API-shaped wire lands |
| W1 | Define `SCE::parsing::Diagnostic` base (incl. `severity()` per Q6) + `emit_json_diagnostic` formatter; refit `TemplateError` subtypes to `Diagnostic`; remove SCXMLParser boundary flatten (audit #1); canonical-JSON parity test vs Rust (audit #3) | commit-series contract (this RFC) | user-signal (2026-04-23); W0 landed |
| W2 | **Library API only** — `SCXMLParser::getDiagnostics() const` (W1 already adds this), plus new free-function helpers `emit_json_diagnostics(vec, ostream)` (batch NDJSON) and `Diagnostic::to_json() const -> nlohmann::ordered_json` (per-diagnostic). **No CLI flag** per architecture boundary. | scope sketch | Named embedder consumer of typed C++ diagnostics (e.g. Android app error overlay, QNX embedded runtime telemetry) |
| W3 | Promote XInclude error family — `XIncludeError` subtypes mirroring the 6 Rust `xml/xinclude-*` DiagnosticCodes; resolve audit #5 `IXIncludeProcessor` interface tradeoff | scope sketch | XInclude consumer needs JSON wire |
| W4 | Promote core `SCXMLParser::parseFile` + `parseContent` top-level errors (file not found, XML parse failure, root-element mismatch); resolve audit #5 `IXMLParser` interface tradeoff | scope sketch | Downstream tooling needs JSON on non-template parse errors |
| W5 | Promote SCXMLParser semantic-validation surface to typed `SemanticError` family. NEW wire codes: 1 (`scxml/top-level-script-unloaded`); 3 sites fold into `validation/invalid-reference` + `validation/empty-collection` per W4 D4 reuse. `analyzer::can_generate_static` DynamicFeatures split. Stage E dead-code cleanup. | LANDED 2026-04-26 (B1 `d6375231`, B2 `523b0281`, C `dab280fa`, E `73afbc08`) | Trigger fired |
| W5a | (deferred prerequisite for *expanded* W5 covering sub-parsers + source coordinates) — pugi offset capture + `IXMLElement::sourcePos()` abstraction (audit #2). **Not a W5 prerequisite under narrow scope** — the 4 wire-able sites do not surface `location()` in test consumers. | scope sketch, decoupled from current W5 | Semantic consumer asks for source coordinates on any semantic diagnostic |
| W5b | (deferred prerequisite for *expanded* W5 covering sub-parser detail) — sub-parser interface redesign (audit #4). **Not a W5 prerequisite under narrow scope** — site #4 "Failed to parse a root state" deferred to expanded W5 to avoid hollow generic-message wire code. | scope sketch, decoupled from current W5 | Semantic consumer asks for detailed sub-parse errors beyond parent-level surface |

### W0 contract (this RFC; commit-series contract; prerequisite for W1)

Deliverables:

1. **Delete `sce/src/main/main.cpp`.** 170-line file, 0 real
   consumers (not in CMake build graph — verified by
   `grep -rn "src/main/main\|scxml-codegen\|sce_main" --include="CMakeLists.txt" --include="*.cmake"` → 0 matches).
   File's own header self-identifies as "SCXML-to-C++ Code
   Generator" but lines 142-143 state `// TODO: Generate state
   machine logic based on SCXML` + `// This is a placeholder
   implementation`. Real codegen lives in Rust `sce-codegen`;
   this file has never been real codegen.
2. **Delete related CMake / docs references** if any surface
   during the deletion commit's test run. Expected to be none
   given the build-graph absence, but verify via
   `cmake --build build_release -j8` post-deletion.
3. **Delete `sce/src/main/` directory** if it becomes empty
   after step 1 (the file stands alone there based on
   `ls sce/src/main/`).
4. **Update any docs / READMEs** mentioning the C++ codegen
   CLI. Verified none in-repo at RFC time via
   `grep -rn "scxml-codegen\|SCXML Code Generator" docs/ README*`
   — expected to be zero.

Explicitly **NOT** delivered in W0:
- Any changes to `SCXMLParser.cpp` or other parser code.
- Any changes to Rust `sce-codegen` (the authoritative codegen
  CLI — untouched).
- Any JSON wire code (that's W1).

**Standing consumer for W0:** None needed — W0 is pure deletion
of dead code with architectural intent (pin the "no C++ codegen
CLI" boundary). The standing consumer for the boundary *as a
whole* is the single-CLI discipline: one authoritative codegen
CLI (Rust `sce-codegen`) rather than two parallel CLIs with
overlapping purpose. W0's correctness is verified by
post-deletion build-green (`cmake --build` succeeds) and test
suite green (no test depended on the deleted file).

**Load-bearing verification:** Temporarily `git checkout`
one of the C++ tests that might have happened to link against
the deleted main.cpp's TU; confirm the test still builds (should,
because CMake never linked it). If any test does break, that's a
hidden CMake reference the audit missed — fix the CMake, re-run.

### W1 contract (this RFC; commit-series contract)

Deliverables:

1. **This RFC** (`claudedocs/rfc-sce-diagnostic-wire-unification.md`)
   — drafted this session, lives on disk only (`claudedocs/` is
   gitignored).
2. **`sce/include/parsing/Diagnostic.h`** — new header declaring:
   - `SCE::parsing::Severity` enum (Q6-A):
     `enum class Severity { Error, Warning, Advice };`
   - `SCE::parsing::Diagnostic` abstract base class with:
     - `virtual Severity severity() const noexcept = 0` (Q6
       day-one; all W1 subtypes return `Error`)
     - `virtual std::string_view code() const noexcept = 0` —
       fully-qualified diagnostic code (e.g. `"xml/template-cycle"`)
     - `virtual std::optional<SourcePos> location() const noexcept = 0` —
       populated by producer when available (Phase C's `SourcePos`)
     - `virtual std::string message() const = 0` — human-readable
     - `virtual std::vector<Fix> fixes() const = 0` — structured
       repair hints, mirroring Rust `Fix` enum in
       `sce-build/src/forge/diagnostic.rs`
   - `SCE::parsing::Fix` variant type mirroring Rust
     (AddAttribute, ReplaceElement, RemoveElement, etc. — the
     minimal set required to express template-family fixes).
   - **Drift test (Rust side):** `cpp_severity_enum_matches_rust`
     reads `Diagnostic.h` via `include_str!` and asserts the
     three variant names (`Error`, `Warning`, `Advice`) appear
     in declaration order. Parallel to existing
     `cpp_template_subtypes_match_rust_diagnostic_codes`.
3. **`TemplateError` refitted to implement `Diagnostic`.** Each of
   the 8 subtypes provides concrete `code()` / `message()` /
   `fixes()`. `location()` returns the Phase-C-populated
   `SourcePos` (or `nullopt` if Phase C has not shipped yet in
   the same codebase state — *soft dependency*, see §4 risks).
4. **`sce/src/parsing/DiagnosticJsonFormatter.cpp`** — single
   free function `emit_json_diagnostic(const Diagnostic&,
   std::ostream&)` producing one NDJSON record shape-equivalent
   to `sce-codegen --error-format=json` output on the same
   logical diagnostic. Uses `nlohmann::ordered_json` (already
   linked into `sce_core` via `CMakeLists.txt:85-87`, precedent:
   `sce/include/mesh/CommunicationError.h` serialisation).
5. **SCXMLParser boundary flatten removal** (audit finding #1).
   `SCXMLParser::parseFile` and `parseContent` gain a typed catch
   chain ahead of the existing `std::exception&` fallback:
   ```cpp
   } catch (const SCE::parsing::TemplateError &te) {
       // Preserve typed identity — emit JSON if JSON mode is
       // active on this parser instance; addError(te.what())
       // continues to run so string consumers are unaffected.
       recordDiagnostic(te);
       addError(te.what());
       return nullptr;
   } catch (const std::exception &ex) {
       addError("Exception while parsing file: " +
                std::string(ex.what()));
       return nullptr;
   }
   ```
   `recordDiagnostic` is a new private method on `SCXMLParser`
   that stores the `Diagnostic` alongside the string-errors
   vector; a sibling `getDiagnostics() const ->
   const std::vector<std::unique_ptr<Diagnostic>>&` exposes the
   typed side to consumers that want it. **`addError(string)` is
   NOT removed** (Q4-B permanent coexistence); the typed and
   string surfaces exist in parallel. This item discharges the
   audit finding #1 debt and becomes the single-W1 path for
   consumers (W1 standing consumer — the parity test — reads
   from `getDiagnostics`).
6. **Parity test — canonical-JSON byte-diff against Rust**
   (audit finding #3 resolution). C++ GTest driving the shared
   fixture family:
   - Fixture set: at minimum one fixture per `TemplateError`
     subtype that Phase B parity harness already exercises
     (`cycle_detected`, `not_found`, `malformed`,
     `missing_attribute`; W1 may skip `read_error` if it
     reproduces the Phase B M4 CI-determinism issue).
   - **Rust side:** `sce-codegen expand <fixture>
     --error-format=json` emits NDJSON on stderr; test captures
     the record.
   - **C++ side:** Load the same fixture through
     `SCXMLParser::parseFile`, catch the typed diagnostic, format
     via `emit_json_diagnostic` to a string.
   - **Canonicalisation step (both sides):** parse each JSON
     record via `nlohmann::json`, re-serialise with
     `dump(-1, ' ', false)` (no indent, no whitespace, keys
     alphabetically sorted via underlying `std::map`). This
     removes non-semantic ordering / spacing differences as a
     failure mode; the test reds only on genuine field content
     divergence.
   - **Diff:** `ASSERT_EQ(canonicalised_rust,
     canonicalised_cpp)`. Byte-identical after canonicalisation.
   - **Load-bearing:** temporarily corrupt the C++ formatter's
     `code` output (e.g. emit `"xml/template-cycleXXX"` instead
     of `"xml/template-cycle"`); the diff reds with a pointed
     string inequality. Swap-and-fail verification required in
     the landing commit message.
7. **Rust-side drift test** mirroring
   `cpp_template_subtypes_match_rust_diagnostic_codes` precedent:
   reads the C++ formatter source via `include_str!`, confirms
   every `TemplateError` subtype has a formatter branch emitting
   a `code` string that appears in the Rust DiagnosticCode
   registry. This is the producer-side drift guard; #6 is the
   consumer-side parity guard. Dual-gate.

Explicitly **NOT** delivered in W1:
- Any new `--error-format=json` CLI flag on the C++ side (W2).
- Any migration of non-Template error families (W3-W5).
- Any removal of `SCXMLParser::addError(string)` or the
  `getErrorMessages()` vector (Q4-B permanent coexistence;
  `recordDiagnostic` adds a parallel typed surface, does not
  replace the string surface).
- Any consumer reading the formatted JSON externally (W2 names
  the first; W1's parity test is an internal standing consumer).
- Any change to the Rust `--error-format=json` output (Rust side
  stays source of truth; C++ conforms to Rust, not the reverse).
- Any JSON schema validator library addition (audit finding #3
  makes byte-diff the conformance method; no validator library
  required).

**Standing consumer for W1:** The canonical-JSON byte-diff
parity test is the standing consumer. Every `TemplateError`
subtype flows through both `sce-codegen expand --error-format=json`
(Rust) and `emit_json_diagnostic` (C++); the test asserts
byte-equivalence of the two outputs after canonicalisation. If W2
never lands (no external consumer materialises), W1's test
continues to pin **Rust/C++ agreement on every fixture** —
producer-side correctness is anchored in the Rust ground truth
(32 passing tests), not in a sidecar assertion that humans maintain.
Per `feedback_built_but_unconsumed.md`, the test must genuinely
bite when the formatter regresses; the load-bearing swap-and-fail
described in item #6 satisfies this.

### W2 scope sketch — library API only (no CLI)

**Scope boundary:** Architecture decision this session pins W2
to **library API only**. No `--error-format=json` flag on any
C++ binary; the CLI surface is Rust `sce-codegen` alone (which
already has `--error-format=json`). W2's consumers are runtime
embedders (Android, QNX, dynamic SCXML loaders, etc.) that
already link against `libsce_core` and route diagnostics into
their own application telemetry / logging / error surfaces.

**Deliverables when W2 reopens:**

1. **`sce/include/parsing/Diagnostic.h` extension** — add
   member `nlohmann::ordered_json Diagnostic::to_json() const;`
   that returns a per-diagnostic JSON object matching the
   `sce-diagnostic.v1.schema.json` record shape. Default
   implementation on base class assembles the struct from
   `code()` / `severity()` / `location()` / `message()` /
   `fixes()`; subtypes can override if needed but the default
   covers all W1 cases.
2. **`sce/src/parsing/DiagnosticBatchFormatter.cpp`** — new
   free function
   `void emit_json_diagnostics(const std::vector<std::unique_ptr<Diagnostic>>&, std::ostream&);`
   that iterates and emits NDJSON (one JSON record per line,
   matching Rust `--error-format=json` convention). Internal
   consumer of each `Diagnostic::to_json()`.
3. **Canonicalisation method on `Diagnostic`** —
   `std::string Diagnostic::to_canonical_json_string() const;`
   uses `nlohmann::json::dump(-1, ' ', false)` to produce the
   same canonical form the W1 parity test uses. Consumer
   utility — same bytes that byte-diff-parity pins.
4. **Binding passthrough (optional, per-consumer):** if an
   embedder asks (Python / Kotlin / Go), a binding exposes
   `getDiagnosticsJson() const -> std::string` returning the
   batch NDJSON output. Not forced in W2 proper; each binding
   opt-in.

**Explicitly NOT in W2:**
- Any CLI binary or flag on C++ side. `sce/src/main/main.cpp`
  is deleted by W0 and stays deleted.
- Replacement of `addError(string)` or `getErrorMessages()`
  (Q4-B permanent coexistence).
- Any schema version change. Stays at `v1 pre-release`.

**Expected reopening design questions (smaller now):**
- Should `emit_json_diagnostics` accept a filter (severity >=
  Warning)? — likely yes for Android app use-cases where advice
  is too noisy.
- Should `Diagnostic::to_json()` memoise? — probably no,
  diagnostics are rare by construction.

**Consumer signal needed:** A named embedder (Android-app team,
QNX-runtime team, specific product) that wants typed C++
diagnostics surfaced as JSON for their own telemetry. Until
that signal fires, W2 stays a sketch.

### W3 scope sketch

Mirror W1's promotion pattern for `XInclude` error family. 6 Rust
DiagnosticCodes: `xml/xinclude-missing-href`, `xml/xinclude-not-found`,
`xml/xinclude-read-error`, `xml/xinclude-too-deep`,
`xml/xinclude-cycle`, `xml/xinclude-malformed`. Today the C++
side's `processXInclude` uses `addError(string)` for all failures;
W3 would introduce `XIncludeError` base + 6 subtypes matching the
template family's shape.

**Audit #5 — `IXIncludeProcessor` interface tradeoff (decide at
W3 reopening):** The abstract interface at
`sce/include/model/IXIncludeProcessor.h` declares
`const std::vector<std::string>& getErrorMessages() const` as a
pure-virtual. W3 has two paths:

- **W3-A: Extend the interface.** Add pure virtual
  `const std::vector<std::unique_ptr<Diagnostic>>& getDiagnostics() const = 0`
  alongside the existing string method. All implementations
  must implement both. Existing string surface stays (Q4-B).
  Cleanest for consumers; forces every future `IXIncludeProcessor`
  implementation to carry both.

- **W3-B: Parallel method on concrete class only.** Add
  `getDiagnostics()` on `XIncludeProcessor` (concrete), not on
  the interface. Consumers that want typed errors cast to the
  concrete type; interface-level consumers still get strings.
  Smaller API footprint; less clean consumption pattern.

W3 RFC reopening must pin A vs B before coding. Default leaning
is **W3-A** (matches Q4-B parallel-surface discipline at every
level), but W3-B is defensible if no alternative IXIncludeProcessor
implementation is ever expected (survey: 1 implementation today).

Trigger: consumer asks. Until then, unmigrated.

### W4 LANDED 2026-04-26 (α-strict, D1-C typed-throw)

**Closeout commits on `feat/sce-wire-w4` (FF-merged):**
- `ef0c3379` feat: Add W4 ParseError typed leaves + 2 Rust producers (α-strict) — Stage B2 foundation
- `fa38c354` feat: Refit parser to typed-throw idiom (W4 Stage C D1-C) — Stage C
- `f1dcb5f8` test: Pin W4 ParseError wire contract via drift tests (Stage D) — Stage D

**Α-strict outcome (vs starter prompt's 5-NEW-codes plan):**
- 2 NEW wire codes (`xml/file-not-found`, `xml/wrong-root-element`) — both have full Rust producers in `sce-build/src/parser.rs`
- 5 typed C++ leaves (NullDocument dropped under D1-C; 3 reuse `xml/parse` because the Rust error model has no producer for those scenarios)
- D1-C (PugiXMLParser typed-throw) chosen over starter's D1-A (interface extension) — eliminates nullptr-return-and-poll C-era pattern, no parallel-surface burden on future XML backends, consistent with W3 Path A precedent
- D4 reverse-default (drop typeid leak) chosen over starter's "carry typeid" — `typeid().name()` is implementation-defined per `[lib.type.info]`, would emit different strings on libstdc++/libc++/MSVC

---

### W4 RFC (legacy section header, retained below for the design record)

W4 promotes the **top-level parser-entry** error family on
`SCE::SCXMLParser::parseFile` / `parseContent` /
`parseAbstractDocument` to the typed `Diagnostic` surface. After
W4 lands, every top-level parse failure surfaces both as a legacy
`addError(string)` (Q4-B coexistence) **and** as a typed
`Diagnostic` on `SCXMLParser::getDiagnostics()` consumed via the
sister `cpp_parse_subtype_*` Rust drift tests. This completes the
Template (W1) + XInclude (W3) + parser-entry (W4) producer triad.

**W4 trigger fired (codified, not consumer-claimed).** Per the W3
closure memo, W4's trigger was *"tooling asks for JSON on non-
template parse errors (file-not-found, no-root-element)."* The
trigger here is codified-as-test in D8 below: the
`ParseErrorConsumer.TypedCodeDistinguishesFailureClassWhereStringParsingIsFragile`
+ `TypedCodeStableUnderMessageTextEdit` test pair IS the consumer
signal made load-bearing. The dispatch lambda the test installs
(retry-strategy by `code()` class) cannot be replaced by
`startsWith("File not found:")` once message-text-mutation is
introduced — that asymmetry is what avoids
`feedback_built_but_unconsumed.md`.

#### Call-site inventory (5 typed leaves over 7 throw sites)

**α-strict scope (Stage A 3rd-pass, after Rust-producer
feasibility survey).** The original starter inventory listed 6
typed leaves with 5 NEW Rust wire codes, but a producer-survey
revealed the Rust error model (Result-based, `roxmltree`
always-has-root, no exceptions) cannot produce 3 of the 5
proposed wire codes. Per `feedback_built_but_unconsumed.md`,
adding Rust enum variants without producers is the very
anti-pattern this milestone exists to avoid. α-strict drops
those 3 wire codes; the corresponding C++ leaves either
(a) collapse onto reused `xml/parse` for in-process typed
dispatch only, or (b) drop entirely if unreachable under D1-C.

Additionally, **`ParseNullDocument` is unreachable under D1-C**
(PugiXMLParser throws on failure instead of returning nullptr,
so callers never see a null document). Drop the leaf entirely.

`sce/src/parsing/SCXMLParser.cpp` carries **7 top-level call
sites** (8 minus the now-dead null-doc check) that surface as
`addError(string)` with no typed counterpart. The 7 sites
collapse to **5 typed leaves**:

| Line | Today's `addError` text | Typed leaf | Wire `code()` | Rust producer? |
|------|-------------------------|------------|---------------|----------------|
| 60   | `"File not found: <path>"` | `ParseFileNotFound` | `xml/file-not-found` (NEW) | ✅ `parser.rs:parse_file` ErrorKind::NotFound branch |
| 77   | `"Failed to parse XML file: <pugi_err>"` | `ParseXmlFailed` | `xml/parse` (existing reuse) | ✅ `parser.rs:parse_impl` `XmlError::Parse` |
| 136  | `"Exception while parsing file: <ex>"` | `ParseException` | `xml/parse` (reuse) | ❌ Rust uses Result, no exception model |
| 153  | `"Failed to parse XML content: <pugi_err>"` | `ParseXmlFailed` (reused) | `xml/parse` (reuse) | (same as 77) |
| 203  | `"Exception while parsing content: <ex>"` | `ParseException` (reused) | `xml/parse` (reuse) | (same as 136) |
| ~~210~~ | ~~`"Null document"`~~ | ~~`ParseNullDocument`~~ | DROPPED | unreachable under D1-C |
| 217  | `"No root element found"` | `ParseNoRootElement` | `xml/parse` (reuse) | ❌ roxmltree rejects root-less input at parse time |
| 223  | `"Root element is not 'scxml', found: <name>"` | `ParseWrongRootElement` | `xml/wrong-root-element` (NEW) | ✅ `parser.rs:parse_impl` after `doc.root_element()` |

**Net α-strict surface:**
- 5 typed C++ leaves (1 dropped from starter)
- **2 NEW** Rust `xml/*` wire codes: `xml/file-not-found`, `xml/wrong-root-element`
- **3 reused** `xml/parse` (for `ParseXmlFailed`, `ParseException`, `ParseNoRootElement` — wire-share, in-process typed dispatch via `dynamic_cast`)
- 2 NEW Rust `XmlError` variants: `FileNotFound`, `WrongRootElement` (both with full producer wiring)
- `ALL_DIAGNOSTIC_CODES.len()`: 150 → **152** (not 155)

The `Template` / `XInclude` catch arms at lines 110-134 / 178-201
are already typed via W1+W3 — W4 does NOT touch those. Below the
parser-entry boundary (lines 354+: `parseScxmlNode`, sub-parser
chain, validation traversal) stays on `addError(string)` per
Q4-B permanent coexistence — that surface is W5 scope, gated on
W5a (pugi offset capture) + W5b (sub-parser interface redesign)
landing first.

**On the 3 reused `xml/parse` leaves**: `ParseXmlFailed`,
`ParseException`, `ParseNoRootElement` all share the wire code
`xml/parse`. Wire-level consumers cannot distinguish them; only
in-process C++ consumers can dispatch via `dynamic_cast`. This
is acceptable because (a) D8's consumer-fragility test
dispatches between `xml/file-not-found` and
`xml/wrong-root-element` — the 2 codes that DO have distinct
wire codes; (b) the 3 reused-code leaves still benefit from C++
typing (better stack traces, dynamic_cast dispatch in C++
tooling). The 5-leaf inventory is the textbook clean Path α
result.

#### Decisions to pin (Stage A blocks on user OK)

##### D1. PugiXMLParser failure protocol — **D1-C (typed-throw) over D1-A (extend IXMLParser interface) over D1-B (concrete-class only)**

**Stage A 2nd-pass evaluation flipped this from D1-A to D1-C as
default.** The original starter prompt named D1-A as the
conservative path; critical re-evaluation against project
invariants (`feedback_pre_release_no_compat.md`,
`feedback_yagni_vs_engineering_avoidance.md`,
`feedback_silently_broken_hooks.md`, C++ Core Guidelines E.2)
shows D1-C is more textbook on long-term grounds.

Three options:

- **D1-C (default):** `PugiXMLParser::parseFile` / `parseContent`
  throw `SCE::parsing::ParseError` subtypes (`ParseFileNotFound`
  / `ParseXmlFailed` / `ParseException`) on failure. The
  `nullptr`-return path is removed; the typed exception bubbles
  to `SCXMLParser`'s typed catch arm. `IXMLParser::getLastError()`
  is marked `@deprecated`, returns empty string, and is removed
  in a future cleanup. **Mirrors the W3 Path A re-throw idiom**
  (`PugiXMLDocument::processXInclude` re-throws typed XInclude
  errors instead of returning a result struct with a nullable
  error message). Same pattern, same surface, same cost basis.

- **D1-A (alternative):** Extend `IXMLParser` interface with
  `virtual std::unique_ptr<Diagnostic> getLastDiagnostic() const = 0;`.
  Every future XML backend implements both `getLastError()` AND
  `getLastDiagnostic()`. Forces parallel-surface discipline at
  the interface level.

- **D1-B (alternative):** Typed surface on concrete
  `PugiXMLParser` class only. Consumers `dynamic_cast` the
  interface pointer if they want typed errors.

**Why D1-C is default:**

1. **Eliminates a C-era pattern.** nullptr-return-and-poll is
   the C idiom for error reporting; throw is the C++ idiom (C++
   Core Guidelines E.2: "Throw an exception to signal that a
   function can't perform its assigned task"). Removing the
   poll contract reduces the typed-surface API to one path,
   not two.

2. **API-stability rejection (the original D1-C
   counter-argument) doesn't apply pre-1.0.** The user's
   `feedback_pre_release_no_compat.md` is explicit:
   *"Until SCE 1.0: no version bumps, no back-compat shims."*
   Out-of-repo `IXMLParser` consumers (if any exist) carry the
   migration cost; that is the documented pre-1.0 contract.

3. **D1-A's hidden cost.** `IXMLParser::getLastDiagnostic()`
   becomes a parallel surface every future backend must
   implement. A backend that forgets to populate it surfaces
   as silently-broken (`feedback_silently_broken_hooks.md`).
   D1-C makes the typed surface the ONLY surface — implementers
   cannot forget it because the throw path is the only failure
   path.

4. **YAGNI consistency.** `IXMLParser`'s header comment claims
   "multi-backend support" but the comment immediately
   contradicts itself with "Implementation: PugiXMLParser
   (unified for all platforms)." Today: 1 backend. No second
   backend in flight. D1-A bills future implementers for
   non-existent backends per
   `feedback_yagni_vs_engineering_avoidance.md`. D1-C charges
   nothing to non-existent backends.

5. **W3 precedent argues for D1-C, not D1-A.**
   `XIncludeExpansionError` flattening was W3 Path A
   (re-throw); the analog at the parser-entry layer is D1-C,
   not interface extension.

**Reverse-default — D1-A.** Defensible IF the multi-backend XML
roadmap materializes (e.g. a Bazel-only XML parser appears with
a different error model). Default lean rejects it on:
- multi-backend is class-doc aspiration, not committed work;
- D1-A's parallel-surface burden falls on every future implementer;
- W5a will need `IXMLElement::sourcePos()` interface extension —
  but that's a single getter, not a paired typed-error accessor.

**Reverse-default — D1-B.** Worse than D1-C; loses interface
abstraction without gaining the throw-flatten benefit. Forces
SCXMLParser to `dynamic_cast<PugiXMLParser*>(xmlParser.get())`,
which is brittle (asserts implementation knowledge at the
interface call site). Rejected.

**Concrete D1-C wire pattern in `PugiXMLParser`:**

```cpp
// before:
if (!std::filesystem::exists(filename)) {
    lastError_ = "File not found: " + filename;
    return nullptr;
}

// after:
if (!std::filesystem::exists(filename)) {
    throw SCE::parsing::ParseFileNotFound(filename);
}
```

`SCXMLParser` catches typed:

```cpp
try {
    auto doc = xmlParser->parseFile(filename);
    // doc is non-null on successful return; the `if (!doc)` check
    // and the xmlParser->getLastError() poll are both removed.
    ...
} catch (const SCE::parsing::ParseError& pe) {
    addError(pe.message());
    recordDiagnostic(pe.clone());
    return nullptr;
} catch (const SCE::parsing::TemplateError& tpl)        { /* unchanged */ }
catch (const SCE::parsing::XIncludeExpansionError& xie) { /* unchanged */ }
catch (const std::exception& ex) {
    // Wrap as ParseException for typed surface.
    SCE::parsing::ParseException pe(
        std::string("Exception while parsing: ") + ex.what());
    addError(pe.message());
    recordDiagnostic(pe.clone());
    return nullptr;
}
```

**`IXMLParser::getLastError()` lifecycle under D1-C:**

- W4 marks it `@deprecated` in the header comment, body returns
  empty string in `PugiXMLParser`. Out-of-repo callers see empty
  but no compile break.
- Future cleanup milestone (gated on grep showing zero in-repo
  call sites and a deprecation grace period) removes the method
  from the interface.
- Pre-1.0: removal does not require version bump or back-compat
  shim per `feedback_pre_release_no_compat.md`.

**D1 is the central W4 decision and Stage A blocks on user
OK** for the D1-A → D1-C flip.

##### D2. Rust-side wire codes — **2 new `xml/*` variants** (α-strict, Rust-producer-feasible only)

**α-strict adjustment** (Stage A 3rd-pass): the original
starter listed 5 new wire codes; survey showed only 2 have
plausible Rust producers. Adding the other 3 as Rust enum
variants would be `feedback_built_but_unconsumed.md` —
producer-less dead variants. α-strict adds only the 2 that
ship with full Rust producer wiring.

Add 2 new `DiagnosticCode` variants in
`sce-build/src/forge/diagnostic.rs`, source-ordered after
existing `XmlSchemaValidation`:

- `xml/file-not-found` — `XmlFileNotFound`
- `xml/wrong-root-element` — `XmlWrongRootElement`

Plus 2 matching `XmlError` variants in
`sce-build/src/forge/error.rs::XmlError`:

- `XmlError::FileNotFound { path: String }` — produced by
  `parser.rs:parse_file` `read_to_string` `ErrorKind::NotFound`
  branch (existing `ForgeError::Io` arm splits)
- `XmlError::WrongRootElement { found: String }` — produced by
  `parser.rs:parse_impl` after `doc.root_element()`, when
  `root.tag_name().name() != "scxml"`

The 11-place sync per `diagnostic_code_edit_checklist.md`
brings `ALL_DIAGNOSTIC_CODES.len()` from `150 → 152` (not 155).

**Reverse-default — fold into existing `XmlError::Parse(String)`
with discriminator field.** Rejected: the Rust+C++ wire codes
must be 1:1 (the drift test
`cpp_*_subtype_code_returns_rust_wire_string` byte-asserts each
subtype's `code()` against a Rust literal); folding loses that
grain and reverts the W1 design choice that "code is the
wire-stable handle for cross-side correlation".

**Rust producer wiring details:**

```rust
// parser.rs::SCXMLParser::parse_file (line ~219):
let content = std::fs::read_to_string(scxml_path).map_err(|e| {
    if e.kind() == std::io::ErrorKind::NotFound {
        Located::new(
            ForgeError::Xml(XmlError::FileNotFound {
                path: scxml_path.to_string(),
            }),
            scxml_path, None, None,
        )
    } else {
        Located::new(
            ForgeError::Io { path: ..., source: e },
            scxml_path, None, None,
        )
    }
})?;

// parser.rs::SCXMLParser::parse_impl (line ~319, after
// `let root = doc.root_element();`):
if root.tag_name().name() != "scxml" {
    return Err(Located::new(
        ForgeError::Xml(XmlError::WrongRootElement {
            found: root.tag_name().name().to_string(),
        }),
        diag_label,
        Some(root.range().start as u32),  // approximate position
        None,
    ));
}
```

**On `parse_impl` callers**: only `SCXMLParser::parse_string`
and `parse_file` invoke `parse_impl`. Forge-pipeline documents
(`<sce:codec>`, etc.) are routed via `classify_document` BEFORE
`compile_model`, so they never reach `parse_impl`. The
WrongRootElement check is safe.

##### D3. Throw-site rewrite shape — **typed exception throw, parser catches and surfaces** (W3 Path A precedent)

Each of the 8 sites converts from
`addError("Failed to ..."); return ...` to throwing the typed
leaf:

```cpp
// before:
addError("File not found: " + filename);
return nullptr;

// after:
throw SCE::parsing::ParseFileNotFound(
    "File not found: " + filename);
```

`parseFile` and `parseContent` already wrap their bodies in
`try { ... } catch (TemplateError&) { ... } catch (XIncludeExpansionError&) { ... } catch (std::exception&) { ... }`.
W4 inserts a typed catch arm for `ParseError&` ahead of the
generic `std::exception&` fallback:

```cpp
try {
    // existing body, with typed throws on the 4 in-method sites
} catch (const SCE::parsing::ParseError& pe) {
    addError(pe.message());                  // Q4-B legacy surface
    recordDiagnostic(pe.clone());            // W2 typed surface
    return nullptr;
} catch (const SCE::parsing::TemplateError& tpl) {
    /* W1+W2, unchanged */
} catch (const SCE::parsing::XIncludeExpansionError& xie) {
    /* W3, unchanged */
} catch (const std::exception& ex) {
    // catches non-typed std::exceptions; the line-136/203 sites
    // ALSO move INSIDE the try as typed `ParseException`, so
    // this arm catches only truly-unexpected throws (logic
    // errors, bad_alloc, etc.) — kept as a defensive fallback.
}
```

`parseAbstractDocument`'s 3 sites (lines 210/217/223) also throw
typed; the `parseFile` / `parseContent` catch arms above pick up
the throw because they wrap the call to `parseAbstractDocument`.

**Reverse-default — keep `addError` direct calls, add
`recordDiagnostic` parallel call beside each.** Rejected: 8
sites × 2 calls each = 16 places to keep in sync; refactor
force-multiplier. The throw idiom keeps the typed surface
load-bearing — a missed `recordDiagnostic` would surface as a
typed-surface gap caught by the boundary test, vs a silent
legacy-only path.

##### D4. `ParseException` typed shape — **drop `typeid(ex).name()`** (Stage A 2nd-pass flip)

**Stage A 2nd-pass flipped this from "carry typeid" to "drop
typeid".** The original starter prompt named carrying
`typeid(ex).name()` as the default lean (debug-tier metadata).
Critical re-evaluation showed `typeid().name()` returns
implementation-defined strings:

- libstdc++: `St13runtime_error` (Itanium-mangled)
- MSVC: `class std::runtime_error` (full prose)
- libc++: `NSt3__113runtime_errorE` (Itanium with std-version)

Putting these into a JSON `detail` field crossing to consumers
(LSP, CI parser, agent dispatch) is a portability violation —
the same logical exception serializes to three different wire
strings depending on the compiler that built `sce_runtime`.

**Default — drop type-name.** `ParseException` carries only
`what()` text. Constructor takes
`(filename, ex.what())`; `code()` returns `xml/exception`.

**Reverse-default — demangle via `__cxa_demangle`.** Adds GCC/
Clang-only dependency (libcxxabi); MSVC builds need a different
path. Rejected: demangle adds platform-specific code for a
debug-tier surface with no codified consumer.

**Future extension** (out of W4 scope): a hardcoded set of
known exception types could be enumerated via
`dynamic_cast<const std::bad_alloc*>(&ex)` chain, surfaced as
distinct typed leaves (e.g. `ParseOutOfMemory`). Promote only
when a consumer asks.

##### D5. Source-location stamping — **omit on W4 throw sites** (parity with W3 `XIncludeError`)

W3 RFC's pinned design point: "No source-location stamping on
XInclude throw sites yet. `setLocation` exists on the base for
parity with `TemplateError` but no expander throw site calls it.
Stamping is its own milestone gated on a consumer asking for
typed coords." W4 follows the same posture:

- `ParseFileNotFound` has no in-document position (the file
  itself doesn't exist).
- `ParseXmlFailed` already embeds pugi's `(row, col)` in the
  message string via `result.description()`.
- `ParseException` is by-construction unexpected and has no
  typed coords.
- `ParseNullDocument` / `ParseNoRootElement` / `ParseWrongRootElement`
  fire after pugi parse succeeded but before sub-parser walk —
  the document root has byte offset 0 by definition.

W5a will add `IXMLElement::sourcePos()` and W5 will then stamp
semantic diagnostics — neither blocks W4.

**Reverse-default — stamp where possible (e.g. `ParseXmlFailed`
has `pugi_err.offset()`).** Acceptable extension; gated on a
consumer asking. Adding it later is forward-additive (calling
`setLocation` does not break clients that ignore it).

##### D6. `ParseError` base class shape — **multi-base `std::runtime_error + Diagnostic`** (parity with W3 `XIncludeExpansionError`)

```cpp
class ParseError : public std::runtime_error,
                   public SCE::parsing::Diagnostic {
public:
    using std::runtime_error::runtime_error;

    void setLocation(SourcePos pos);

    // Diagnostic interface
    std::string_view code() const noexcept override = 0;
    const std::optional<SourcePos> &
        location() const noexcept override;
    nlohmann::ordered_json to_json() const override;
    // clone() pure-virtual on each leaf

private:
    std::optional<SourcePos> location_;
};
```

6 typed leaves implement `code()` + `clone()` only (the rest is
base-class boilerplate). Mirrors `XIncludeExpansionError` exactly
— file in `sce/include/parsing/ParseError.h`, impl in new
`sce/src/parsing/ParseError.cpp` registered in `sce_runtime`.

**Reverse-default — single-base from `Diagnostic` only, drop
`std::runtime_error`.** Rejected: `parseFile` / `parseContent`
callers expect `std::exception&` semantics from any throw they
don't know about; dropping `runtime_error` breaks the existing
`catch (std::exception&)` fallback's reachability and forces
every caller to add a typed catch. Multi-base is the pattern W3
set; no reason to diverge.

##### D7. Stage constant — **file-local `kParseStage = "xml"`** (matches Rust `Stage::Xml`)

`stage()` returns the constant `"xml"`. Mirror of
`TemplateError.cpp::kTemplateStage` and
`XIncludeError.cpp::kXIncludeStage` — both pinned to `"xml"`
because all `xml/*` codes share `Stage::Xml::as_str() == "xml"`
in the Rust authority (`sce-build/src/forge/diagnostic.rs:244`).
Stays file-local because (per W3 closure memo) "the Rust prefix→
stage table is not 1:1, so a shared stage helper would carry
per-prefix logic that has to grow with each future W milestone".

**Reverse-default — extract to shared `Diagnostic.cpp` table.**
Rejected per the W3 closure rationale; revisit only at the
fourth stage-constant consumer (W5).

**Note:** the W4 starter prompt's draft proposed
`kXmlStage = "xml-parse"` — that is inconsistent with the Rust
authority (no `xml-parse` Stage variant exists; all `xml/*`
codes route through `Stage::Xml` → `"xml"`). The RFC pins
`"xml"` and the starter's draft is corrected here.

##### D8. Drift-test count + curation — **5 schema-conformance + 1 curated-count + 1 id-distinguishes + 2 boundary-surface + 2 consumer-fragility tests** (α-strict)

Mirror W3's `XIncludeErrorWire` test fixture for the
schema/curated/boundary tests (see
`tests/parsing/Diagnostic_test.cpp:351-422`), AND add the
**load-bearing consumer-fragility tests** that codify the actual
behavior W4 unlocks. Without the consumer-fragility pair, this
whole milestone is `feedback_built_but_unconsumed.md` — typed
surface exists but no caller distinguishes it from string
parsing.

```cpp
// ── 5 schema conformance tests (one per typed leaf, α-strict) ──
TEST(ParseErrorWire, FileNotFoundConformsToV1Schema)         { ... }  // xml/file-not-found
TEST(ParseErrorWire, ParseXmlFailedConformsToV1Schema)       { ... }  // xml/parse (reused)
TEST(ParseErrorWire, ParseExceptionConformsToV1Schema)       { ... }  // xml/parse (reused)
TEST(ParseErrorWire, NoRootElementConformsToV1Schema)        { ... }  // xml/parse (reused)
TEST(ParseErrorWire, WrongRootElementConformsToV1Schema)     { ... }  // xml/wrong-root-element

// ── Curated count + id-distinguishes ──────────────────────────
TEST(ParseErrorWire, EveryNewCuratedParseCodeIsExercised)    { ... }  // count == 2 NEW wire codes
TEST(ParseErrorWire, IdDiffersAcrossSubtypesWithSameMessage) { ... }  // FNV-1a id varies across all 5 leaves

// ── Boundary surface (parser produces typed) ──────────────────
TEST(SCXMLParserBoundary, ParseFileSurfacesTypedFileNotFoundDiagnostic)     { ... }
TEST(SCXMLParserBoundary, ParseContentSurfacesTypedNoRootElementDiagnostic) { ... }

// ── Consumer-fragility tests (LOAD-BEARING — W4 trigger) ──────
TEST(ParseErrorConsumer,
     TypedCodeDistinguishesFailureClassWhereStringParsingIsFragile) { ... }
TEST(ParseErrorConsumer,
     TypedCodeStableUnderMessageTextEdit)                           { ... }
```

**`TypedCodeDistinguishesFailureClassWhereStringParsingIsFragile`
shape** (the load-bearing consumer test):

```cpp
// Two distinct parse failures — file-not-found (path retry
// strategy) vs malformed root (syntax suggestion strategy).
// A real consumer (LSP / CI report / build tool) needs to
// dispatch on the failure CLASS, not on the message text. This
// test proves typed code() makes that dispatch reliable; the
// parallel string-parsing path would have to startsWith("File
// not found:") and would silently break if message text changed.

SCE::SCXMLParser p1(std::make_shared<NodeFactory>());
p1.parseFile("/nonexistent/path.scxml");

SCE::SCXMLParser p2(std::make_shared<NodeFactory>());
p2.parseContent("<not-scxml/>");  // wrong root

ASSERT_EQ(p1.getDiagnostics().size(), 1u);
ASSERT_EQ(p2.getDiagnostics().size(), 1u);

// THE consumer pattern — typed dispatch:
auto retry_strategy = [](const Diagnostic& d) -> std::string {
    if (d.code() == "xml/file-not-found")     return "PATH_RETRY";
    if (d.code() == "xml/wrong-root-element") return "SYNTAX_FIX";
    return "GENERIC";
};
EXPECT_EQ(retry_strategy(*p1.getDiagnostics()[0]), "PATH_RETRY");
EXPECT_EQ(retry_strategy(*p2.getDiagnostics()[0]), "SYNTAX_FIX");
```

**`TypedCodeStableUnderMessageTextEdit` shape**: codifies that
`code()` IS the wire-stable handle. Construct two
`ParseFileNotFound` instances with intentionally divergent
message-text pretexts (one as if from a future edit). Assert
`code()` is byte-identical across both, while `message()`
diverges. Bites if a future PR changes wire codes by editing
message text.

**Plus Rust drift tests** (sister to W3's pair, in a NEW
`sce-build/src/parser.rs` module since no parser-entry Rust
module exists today — or alternatively, add to the existing
`sce-build/src/forge/diagnostic.rs::tests` if the file-discovery
cost of a new `parser.rs` is unjustified):

- `cpp_parse_subtypes_match_rust_diagnostic_codes` —
  `include_str!("../../sce/include/parsing/ParseError.h")` then
  byte-assert 6 subtype names + 6 `xml/*` codes.
- `cpp_parse_subtype_code_returns_rust_wire_string` — locate
  each class block, assert `code()` body returns the Rust wire
  literal (the 5 NEW + 1 reused).

**Reverse-default — drop the ParseErrorConsumer pair** (treat
W4 as pure surface-add, no consumer test). Rejected: that's
exactly the `feedback_built_but_unconsumed.md` anti-pattern.
The consumer-fragility tests ARE the W4 trigger condition
codified in code — without them, W4 is built-but-unconsumed and
a future audit would correctly flag it for deletion. With them,
the dispatch lambda IS the consumer; a string-parsing
alternative would visibly trail in the diff (more code, fragile
to message-text changes), making the typed surface's value
load-bearing.

**Anti-pattern to actively avoid in Stage D drift authoring:**
do NOT write the consumer test against literal
`EXPECT_EQ(diag.code(), "xml/file-not-found")` only — that's
self-referential (schema-existence assertion). The test must
dispatch on `code()` to a behavior-distinct branch (the
`retry_strategy` lambda above). The dispatch IS the consumer;
the surface IS what makes it possible. **If during Stage D the
consumer test ends up trivially passing on a string-parsing
baseline** (i.e. the dispatch could be done with
`startsWith("File not found:")` equally well), STOP — that
means the consumer-signal codification failed and W4 reverts to
built-but-unconsumed. Strengthen the test (e.g. add a
parameterized message-text mutation that the typed path absorbs
but the string path cannot) before continuing.

#### Stages

| Stage | Scope | Verification |
|-------|-------|--------------|
| A | This RFC — user OK gate on D1 (W4-A vs W4-B) and D2-D8 collectively | n/a (writeup only; `claudedocs/` is gitignored, no commit) |
| B1 | **No B1.** `computeFnv1aDiagnosticId` already extracted to `Diagnostic.cpp` in W3; W4 reuses directly. | — |
| B2 | **Foundation (α-strict).** Rust: 2 new `XmlError` variants (`FileNotFound`, `WrongRootElement`) + 2 `DiagnosticCode` variants (`XmlFileNotFound`, `XmlWrongRootElement`) + 11-place sync (count `150→152`). 2 Rust producer wire-ups in `parser.rs` (`parse_file` `ErrorKind::NotFound` branch, `parse_impl` root-tag check). 2 golden entries. Schema enum + acceptance doc rows. C++: `ParseError.h` base + 5 leaves (NullDocument dropped) + `ParseError.cpp` registered in `sce_runtime`. | `cargo test -p sce-build --features cli` green; cmake build clean |
| C | **D3 throw-site rewrite + D1-C typed-throw refit + boundary catch arm.** `PugiXMLParser::parseFile` / `parseContent` rewritten to throw `ParseFileNotFound` / `ParseXmlFailed` on internal failures (no more `lastError_` + `nullptr`-return path). 3 `parseAbstractDocument` sites throw `ParseNullDocument` / `ParseNoRootElement` / `ParseWrongRootElement`. `SCXMLParser::parseFile` / `parseContent` gain `catch (ParseError&)` arm ahead of `std::exception&`; the legacy `if (!doc \|\| !doc->isValid()) { addError(...); return nullptr; }` branches are removed (typed-throw replaces both). `IXMLParser::getLastError()` marked `@deprecated`, body returns empty string. | `cargo test` green; cmake green; existing `Diagnostic_test.cpp` baseline preserved |
| D | **Drift tests.** 6 schema-conformance + 1 curated-count + 1 id-distinguishes + 2 boundary-surface + 2 consumer-fragility in `Diagnostic_test.cpp::ParseErrorWire` / `ParseErrorConsumer` / `SCXMLParserBoundary`. Rust `cpp_parse_subtypes_match_rust_diagnostic_codes` + `cpp_parse_subtype_code_returns_rust_wire_string`. | `ctest -R "ParseErrorWire\|SCXMLParserBoundary\|ParseErrorConsumer"` 100%; full ctest no regression |
| E | **RFC closeout + memory.** Flip §W4 status `open → LANDED` with commit SHAs. Extend `wire_rfc_w0_w1_landed.md` with W4 closure section (1-screen budget). MEMORY.md one-liner update. | `cargo test --features cli` green; acceptance-doc count `150→155`; full ctest stable |

Stages B2-E land on one feature branch (`feat/sce-wire-w4`),
one commit per stage following COMMIT_FORMAT.md.

#### Stage F (future, out of W4 scope) — actualize a real consumer

After W4 lands, `SCXMLParser::getDiagnostics()` will be the
typed surface for Template + XInclude + Parser-entry families
combined. Today, **no production code calls `getDiagnostics()`**
— the only consumers are test fixtures
(`SCXMLParserBoundary.*`). The W3 closure pattern of
codified-test-as-consumer is acknowledged as the same risk:
without a real consumer, the entire W2-W4 typed-surface chain
remains `feedback_built_but_unconsumed.md`-adjacent.

**Stage F (future milestone, separate RFC):** wire one real
consumer in `tools/` or `bindings/` that calls
`getDiagnostics()` and emits structured output (e.g.
NDJSON via `emit_json_diagnostics`). Candidate consumers:

- A new `tools/sce-validate-cli/` C++ binary that takes an
  SCXML file, parses it, and prints typed diagnostics.
- Extend `tools/txml_converter/` to surface typed errors on
  conversion failure.
- A binding-side passthrough (Python/Kotlin) that exposes
  `get_diagnostics() -> list[dict]` to the embedder.

**This is NOT a W4 blocker.** W4 closes with codified consumer
parity to W3. Stage F is named here so future audits know the
gap is tracked, not forgotten. The trigger for Stage F is the
first production consumer asking for structured diagnostics
(LSP integration, CI tooling, agent dispatch loop).

#### Out of scope (do NOT bundle into W4)

- **W5a pugi offset capture** — separate prerequisite milestone.
- **W5 semantic family** — separate RFC, separate session.
- **CLI `--error-format=json` flag on any C++ binary** — W0
  architecture boundary still holds; W4 is library API only.
- **Language binding passthrough** (Python/Kotlin/Go).
- **Cross-side id byte-equivalence** — gated on a consumer
  asking; C++ id stays message-text-derived per W1+W3.
- **Schema status flip to `stable`** — stays `pre-release`.
- **Modifying `IXMLDocument`** — rejected D1-C analog at the
  document level.
- **`<sce:import>` `ImportError::FileNotFound`** — already typed
  in Rust at `forge::error.rs:401`; not a W4 concern.

Trigger fired (codified in D8 consumer-fragility tests).
**Stage A awaits user OK on D1 before B2 begins.**

### W4.5 LANDED 2026-04-26 (debt repayment, polling surface removed)

**Closeout commit on `feat/sce-wire-w4.5` (FF-merged):**
- `e62a6f6e` refactor: Remove IXMLDocument result-polling surface (W4.5)

**Follow-up cleanup on `refactor/datamodel-isvalid-redundant` (FF-merged):**
- `513ff9ea` refactor: Drop redundant IXMLDocument::isValid polling in DataModelItem — splits the D4 note option "may ride in the same Stage B commit or split out as a follow-up" into a separate commit; 6 monotonically-true `isValid()` guards removed (under W4 D1-C the class invariant `xmlContent_ != null ⇒ wrapper is valid` holds without runtime check); 1 file, +16 / -16

**Outcome (vs starter prompt's site inventory):**
- 0 NEW wire codes (D2 reuses `xml/parse`, D3 reuses `xml/xinclude-malformed`)
- 0 NEW Rust producers (Rust pipeline is single-pass typed throw via `expand_preprocessors` — no analog to the C++ document-level polling shape)
- Net **-30 lines** across 9 files in the main commit (+141 / -171) — pure deletion, no compensating expansion
- D4 firm KEEP for `isValid()` after Stage B grep verified 7 in-scope `IXMLDocument::isValid()` callers in `DataModelItem.cpp`; `getErrorMessage()` deleted (4 callers all cleaned in same commit). Follow-up commit drops the redundant calls themselves while keeping the interface method available for any future caller that genuinely needs it.
- Stage B re-grep caught 3 sites the starter inventory missed: `XIncludeProcessor.cpp:24` (deprecated stub `.ok` polling), `TemplateExpander_test.cpp:432-435 + 511-535`, `phase_b_parity_test.cpp:324-325 + 546-547` — site-inventory completeness is a verify-before-ship lesson, not a design surprise

---

### W4.5 RFC (legacy section header, retained below for the design record)

**Status**: LANDED 2026-04-26 (commit `e62a6f6e`). Authoritative
starter prompt at `claudedocs/w4-5-starter-prompt.md` (drafted at
W4 closure, retained for the design record).

**W4.5 trigger** (codified, not consumer-claimed): the user named
the polling pattern as remaining debt at the end of the W4 cleanup
commit (`fd20f712`) and requested a starter prompt for follow-up
repayment. The trigger is `feedback_pre_release_no_compat.md`
policy applied consistently — W4 D1-C eliminated the parser-entry
polling, but the document-level polling (the same shape, one layer
down) survived because it was outside W4 scope. W4.5 finishes the
job. Pure debt-repayment milestone — **0 NEW wire codes**, **0 NEW
Rust producers**, **0 schema changes**.

#### Surface to delete

`IXMLDocument` carries two methods with a `Result { ok, positions }
+ errorMessage_` polling shape:

```cpp
struct XIncludeResult { bool ok = false; PositionMap positions; };
struct SceTemplateResult { bool ok = false; PositionMap positions; };

class IXMLDocument {
    virtual XIncludeResult processXInclude() = 0;
    virtual SceTemplateResult processSceTemplate(const PositionMap &upstream) = 0;
    virtual std::string getErrorMessage() const = 0;
    virtual bool isValid() const = 0;  // grep audit pending in Stage B
    // ...
};
```

`PugiXMLDocument` impl carries an `errorMessage_` member set by 4
`return result(ok=false)` branches plus the `getErrorMessage()`
accessor. `SCXMLParser::parseFile` / `parseContent` poll
`templateResult.ok` then `addError(... + getErrorMessage())` on
failure (XInclude `.ok` is silently ignored — the W3 typed throw
bypasses the polling layer entirely, so the polling path is dead
on that side).

#### Site inventory (verified 2026-04-26 against HEAD `fd20f712`)

`PugiXMLParser.cpp::PugiXMLDocument::processXInclude` (lines 221-322):

| Line | Failure path | Today | W4.5 target |
|------|--------------|-------|-------------|
| 232-235 | `if (!doc_)` null doc | `errorMessage_ = "Document is null"; return result` | **DROP** — unreachable under W4 D1-C (PugiXMLParser never produces a wrapped null doc) |
| 280-285 | reparse failure of expanded text | `errorMessage_ = "Failed to reparse..."; return result` | `throw SCE::parsing::ParseXmlFailed` (D2: reuse `xml/parse`) |
| 305-315 | `catch XIncludeExpansionError` | already typed (W3 Path A) | unchanged |
| 316-321 | `catch std::exception` fallback | `errorMessage_ = "..."; return result` | `throw SCE::parsing::XIncludeMalformed` (D3: reuse `xml/xinclude-malformed`) |

`PugiXMLParser.cpp::PugiXMLDocument::processSceTemplate` (lines 324-411):

| Line | Failure path | Today | W4.5 target |
|------|--------------|-------|-------------|
| 345-348 | `if (!doc_)` null doc | `errorMessage_ = "Document is null"; return result` | **DROP** — unreachable under W4 D1-C |
| 402-406 | reparse failure of expanded text | `errorMessage_ = "Failed to reparse expanded template..."; return result` | `throw SCE::parsing::ParseXmlFailed` (D2 reuse) |
| (no `catch` arm) | typed `TemplateError` from `expandString` | already typed (W1) | unchanged — propagates through the body |

`SCXMLParser.cpp` polling callsites:

| Line | Function | Today | W4.5 target |
|------|----------|-------|-------------|
| 83-100 | `parseFile` | `xincludeResult.ok` ignored; `if (!templateResult.ok) addError(... + getErrorMessage())` | `documentPositions_ = doc->processSceTemplate(doc->processXInclude())` — typed throws bubble to existing W1/W3/W4 catch arms |
| 173-185 | `parseContent` | identical pattern | identical refit |

`IXMLDocument::getErrorMessage()` outside-scope callsites (verified
2026-04-26 full-repo grep — must be cleaned up in Stage B alongside
the interface deletion):

| File | Line | Today | W4.5 cleanup |
|------|------|-------|--------------|
| `sce/src/model/DataModelItem.cpp` | 170-179 | `if (xmlContent_ && xmlContent_->isValid()) { content_=""; } else { LOG(... + xmlContent_->getErrorMessage()); xmlContent_.reset(); content_=content; }` | The `else` branch is **dead under W4 D1-C** — `parser->parseContent(content)` at line 168 throws on failure, never returns a non-valid wrapped doc. Drop the `else` branch; the `try/catch (std::exception)` at line 165/180 already handles all parse-failure paths via the existing log + `xmlContent_.reset(); content_=content;` recovery. (`isValid()` itself is also redundant under typed-throw, but dropping the redundant call is W4.5-adjacent dead-code cleanup that may ride in the same Stage B commit or split out as a follow-up — Stage A note, not Stage A decision.) |
| `tests/parsing/XIncludeExpander_test.cpp` | 278-279 | `const SCE::XIncludeResult result = doc.processXInclude(); ASSERT_TRUE(result.ok) << ... << doc.getErrorMessage();` | Refit to direct return + typed-throw assertion: `auto positions = doc.processXInclude();` (any failure throws — test would fail on the gtest typed-throw assertion). The `XIncludeResult` type ceases to exist post-W4.5 anyway, so this site is a hard compile-time forcing function. |

`LuaDOMBinding.cpp:77` and `DOMBinding.cpp:173` carry
`document->getErrorMessage()` calls as well, but `document` there
is `std::make_shared<XMLDocument>(...)` — a separate class declared
in `sce/include/scripting/XMLDOMWrapper.h:64` with its own
`isValid()` / `getErrorMessage()` pair, NOT a subclass of
`IXMLDocument`. Out of W4.5 scope.

**Net W4.5 surface:**
- 3 `errorMessage_ + return` → typed `throw` rewrites (#2 #4 #6 above)
- 2 `if (!doc_)` dead branches dropped (#1 #5 above)
- 2 SCXMLParser callsite simplifications (`parseFile`, `parseContent`)
- 1 DataModelItem dead-else-branch drop (line 170-179)
- 1 XIncludeExpander test refit (line 278-279)
- `IXMLDocument::getErrorMessage()` deleted from interface (D4)
- `PugiXMLDocument::errorMessage_` member deleted
- `XIncludeResult` + `SceTemplateResult` structs deleted (D5)
- `IXMLDocument::isValid()` — **KEEP** (7 in-scope callers verified, see D4 below)
- 0 NEW wire codes (`xml/parse`, `xml/xinclude-malformed` reused)
- 0 NEW Rust producers (Rust `expand_preprocessors` is single-pass; no
  reparse step exists — adding a wire code for a producer-less leaf
  would be a textbook `feedback_built_but_unconsumed.md` violation)

#### Decisions to lock (Stage A user OK gate)

##### D1. Return-type shape — **return `PositionMap` directly**

The `XIncludeResult` and `SceTemplateResult` structs exist solely
to carry the `ok` flag. With typed-throw, `ok=false` is unreachable;
the structs become single-field carriers and the `.positions`
accessor is pure ceremony. Direct return is the textbook
simplification.

```cpp
// before:
auto result = doc->processXInclude();
auto positions = result.positions;
// after:
PositionMap positions = doc->processXInclude();
```

**Reverse-default — keep wrapper, always set `ok=true`.** Rejected:
single-field structs are textbook YAGNI violations
(`feedback_planned_not_yagni.md`); future statistics/warnings
fields can be reintroduced if and when a consumer asks.

##### D2. Reparse failure typed leaf — **reuse `ParseXmlFailed` (`xml/parse`)**

After XInclude/Template expansion produces text, that text is
reparsed by pugi. The failure is semantically a parse failure of
the spliced text — same family as the original parser-entry parse
failure. Reuse keeps the wire code count flat.

```cpp
throw SCE::parsing::ParseXmlFailed(
    "Failed to reparse expanded XInclude: " +
    std::string(parseResult.description()));
```

**Reverse-default — introduce NEW `xml/template-reparse-failed` /
`xml/xinclude-reparse-failed` codes.** Rejected: would force NEW
Rust `XmlError` variants + producers + 11-place sync per
`diagnostic_code_edit_checklist.md`, but Rust has no equivalent
reparse step (`expand_preprocessors` is single-pass), so the Rust
variants would be dead — exactly the α-strict failure mode W4
already taught us about. Wire-level dispatch on "reparse failed"
vs "initial parse failed" has no consumer signal.

##### D3. `std::exception` fallback typed leaf — **reuse `XIncludeMalformed` (`xml/xinclude-malformed`)**

The non-typed catch arm in `processXInclude` (line 316-321) catches
anything `expandStringX` might throw that's not already an
`XIncludeExpansionError`. Today's audit shows expandStringX's
producer surface (`XIncludeExpander.cpp:117, 308`) only throws
`XIncludeMalformed` (XIncludeExpansionError-derived), so the arm is
defensive against future drift (e.g. `std::bad_alloc` propagating
through the expander). The semantically-closest typed leaf is
`XIncludeMalformed` — the catch-all for "the XInclude expander
failed and the failure didn't fit any of the named leaves".

**Reverse-default — introduce new `XIncludeException` leaf
(`xml/xinclude-exception`).** Rejected: same as D2 — would force
NEW Rust variant for which Rust has no producer (Rust uses
`Result`, not exceptions). The W4 D4 finding ("typeid leaks are
wire-portability violations") applies — generic exception wrapping
should not get its own wire code without a structured payload.

##### D4. `getErrorMessage()` removal — **delete from interface, no deprecation grace period**

Pre-1.0 no-back-compat policy
(`feedback_pre_release_no_compat.md`) — same precedent as the W4
cleanup commit (`fd20f712`) that removed
`IXMLParser::getLastError()` outright.

Full-repo grep (2026-04-26, with `*.h *.hpp *.cpp *.cc *.c *.py
*.kt *.go *.rs *.java *.ts *.js` filters) found
`IXMLDocument::getErrorMessage()` callers at exactly 4 sites: the
2 SCXMLParser polling callsites (W4.5 deletes them), 1
DataModelItem dead-else-branch (W4.5 cleans up — see site
inventory table above), and 1 XIncludeExpander test (W4.5 refits
to typed-throw assertion). After Stage B, 0 callers remain →
interface method deletes cleanly.

`isValid()` is **KEEP** (firm). The same full-repo grep found 7
in-scope `IXMLDocument::isValid()` callers, all in
`sce/src/model/DataModelItem.cpp` (lines 83, 113, 170, 189, 200,
204 + the `tempDoc->isValid()` at line 83), all valid-doc gating
patterns guarding subsequent `getRootElement()` / DOM mutation.
The 8 other `isValid()` matches in the grep output dispatch to
distinct types (`ExecutionContextImpl`, `XMLDOMWrapper`,
`MockExecutionContext`, `ITestMetadataParser`) — none are
`IXMLDocument`-receivers. Starter prompt's KEEP decision verified.

(Note: under W4 D1-C the DataModelItem `isValid()` calls themselves
are also redundant — typed-throw guarantees a valid wrapped doc —
but dropping the redundant calls is dead-code cleanup adjacent to
W4.5, not part of the polling-surface deletion. The interface
method stays available for any future caller that genuinely needs
it.)

**Reverse-default — `[[deprecated]]` grace period.** Rejected on
the same grounds W4 cleanup rejected the same shim: pre-1.0 means
no back-compat shims; out-of-repo callers carry the migration cost;
the deprecation attribute is exactly the hack pre-1.0 policy
forbids.

##### D5. Wrapper struct deletion — **delete `XIncludeResult` and `SceTemplateResult` entirely**

With D1, the structs are unused. Delete the type definitions from
`IXMLDocument.h`. Any out-of-repo caller that referenced
`XIncludeResult.positions` migrates to the direct return.

**Reverse-default — keep as `[[deprecated]] using` aliases of
`PositionMap`.** Rejected; same pre-1.0 policy as D4.

#### Stages

| Stage | Scope | Verification |
|-------|-------|--------------|
| A | This RFC § — user OK gate on D1 (direct PositionMap return) and D2-D5 collectively | n/a (writeup only; `claudedocs/` is gitignored, no commit) |
| B | **Refit + cleanup in one commit.** `IXMLDocument` interface signature change (`processXInclude` return type, `processSceTemplate` return type, `getErrorMessage` deletion, struct deletions; `isValid` KEEP firm per D4). `PugiXMLDocument` impl: 3 typed-throw rewrites + 2 dead-branch drops + `errorMessage_` member deletion. `SCXMLParser` callsite cleanup: 2 sites (parseFile + parseContent). Outside-scope cleanup forced by interface change: `DataModelItem.cpp:170-179` dead-else-branch drop + `XIncludeExpander_test.cpp:278-279` test refit to typed-throw assertion. Site inventory verified by full-repo grep (2026-04-26); no further consumer survey expected, but Stage B re-runs the same grep before commit to catch any concurrent additions. | `cmake --build build_release` clean; `ctest -R "diagnostic\|parsing\|template\|xinclude"` 100%; full ctest baseline preserved |
| C | **Drift tests (skip if D2/D3 take defaults — none expected).** | n/a if D2/D3 default; otherwise 11-place sync + drift test pair if reverse-default selected |
| D | **RFC closeout + memory.** Flip §W4.5 status `open → LANDED` with commit SHAs. Extend `wire_rfc_w0_w1_landed.md` with W4.5 closure section (1-screen budget rule). MEMORY.md one-liner update or fold into existing W0+...+W4 pointer. | `cargo test --features cli` green; full ctest stable |

Stages B-D land on one feature branch (`feat/sce-wire-w4.5`),
1-2 commits per stage following COMMIT_FORMAT.md. Stage B is one
commit (the refit + cleanup ride together because the interface
change forces all callsites to update atomically — splitting them
would leave the codebase non-compiling).

#### Anti-pattern to actively avoid in Stage B

Do NOT introduce intermediate `[[deprecated]]` shims for
`getErrorMessage()` / `isValid()` / `XIncludeResult` /
`SceTemplateResult` even briefly — the W4 cleanup commit
(`fd20f712`) explicitly demonstrated that pre-1.0 means
delete-not-deprecate. If the migration cost feels high in any
callsite, the answer is "find the missing typed catch arm in
SCXMLParser", not "soften the deletion with a shim".

If Stage B's out-of-repo consumer survey reveals a binding
(Python/Kotlin/Go) that polls `getErrorMessage()`, **STOP** — that
breaks the `wire_rfc_w0_w1_landed.md` pin "binding passthrough is
consumer-gated and opt-in" assumption (bindings consume errors via
`ReadySCXMLEngine::lastFactoryError()`, not via
`IXMLDocument::getErrorMessage()`). If the survey contradicts this
pin, escalate to user before deleting.

#### Why W4.5 avoids `feedback_built_but_unconsumed.md`

Unlike W4's typed leaves (which face the codified-not-actualized
risk per Stage F future work), W4.5 has **zero net producer
additions** — it's pure deletion of the legacy polling surface.
The "consumer signal" question is moot because no new surface is
added. The pre-1.0 cleanup discipline IS the consumer.

### W5 LANDED 2026-04-26 (semantic family typed-throw, test-as-consumer + dead-code cleanup)

**Status**: LANDED 2026-04-26 on `feat/sce-wire-w5` (4 commits: B1 `d6375231`, B2 `523b0281`, C `dab280fa`, E `73afbc08`). User OK'd D1-D7 verbatim including (d) reuse-existing-codes + analyzer.rs DynamicFeatures split + Stage E dead-code cleanup. Authoritative starter at `claudedocs/w5-starter-prompt.md`.

**Outcome (vs starter prompt's site inventory)**:
- Pre-flight #1 surfaced **10 sites** in `SCXMLParser.cpp` (vs starter's "~50+" estimate inherited from the original §W5 sketch). After dead-code + W5b-deferred filter: **4 wire-able**, **4 dead-post-W4**, **1 deferred** (line 411 awaits W5b sub-parser detail).
- Pre-flight #2 confirmed zero pre-existing `semantic/*` or `scxml/*` Rust producers; existing forge `validation/*` + `analyzer::can_generate_static` provided the prior-art surface for D2 fold + D3 mis-classification refit.
- **Net wire change: +1 NEW code** (`scxml/top-level-script-unloaded`, W3C SCXML §5.8). 3 of 4 wire-able C++ sites fold onto existing `validation/*` codes (`validation/invalid-reference` × 2, `validation/empty-collection`). ALL_DIAGNOSTIC_CODES count went 152 → 153.
- Pre-W5 mis-classification of `validation/dynamic-features` for "initial state names undeclared" + "document rejected by W3C SCXML 5.8" was corrected by D3 refit at the analyzer-source level — Rust analyzer pipeline now emits the correct semantic codes for these failures, matching what C++ throws.
- 14 GoogleTest (`SemanticErrorWire` × 8 + `SemanticErrorConsumer` × 6) + 5 SCXMLParserBoundary + 2 Rust drift tests + 4 analyzer.rs unit + 9 scxml_semantic.rs unit + 2 integration test refits = **36 NEW tests** pinning the typed surface from day one.

**Pinned design points** (audited & landed):
- D1 typed-throw mirroring W4 D1-C ✓ (`SemanticError` base + 4 leaves)
- D2 reuse `validation/*` per W4 D4 fold + 1 NEW `scxml/top-level-script-unloaded` ✓
- D3 `analyzer::can_generate_static` refit (split DynamicFeatures into stage-correct codes) ✓
- D4 `forge::error::ValidationError` preserved as forge-scoped (parallel `ScxmlSemanticError` enum at `sce-build/src/scxml_semantic.rs`) ✓
- D5 catch arm Q4-B coexistence (parseFile + parseContent both wire `addError + recordDiagnostic`) ✓
- D6 test-as-consumer with W4 `*Boundary` + `*Consumer` template applied verbatim ✓
- D7 11-place schema sync executed for the 1 NEW code ✓

---

### W5 RFC (legacy section header, retained below for the design record)

**Status**: LANDED 2026-04-26 (commits `d6375231` / `523b0281` / `dab280fa` / `73afbc08`). Authoritative starter prompt at `claudedocs/w5-starter-prompt.md`, retained for the design record.

#### Trigger fired (codified, not consumer-claimed)

- W4.5 closure (commits `e62a6f6e` + `513ff9ea`, 2026-04-26) cleared the document-level polling debt; `parseScxmlNode` + `validateModel` semantic checks are now the sole remaining `addError(string)` producer in the parsing pipeline.
- User's explicit decision (2026-04-26): **test fixtures count as consumers** for fragility-pinning purposes, applying the W4 closure precedent (`*Boundary` + `*Consumer` test fixtures were the only post-W4 consumers of `getDiagnostics()`) symmetrically to W5.
- Stage F (production consumer wiring) stays a separate future milestone — out of W5 scope.

#### Pre-flight outcomes (2026-04-26 against HEAD `513ff9ea`)

**Pre-flight #1 — C++ semantic-validation site inventory**: starter prompt's "~50+" estimate inherited from a stale section of the original §W5 sketch. Actual: **10 sites, all in `sce/src/parsing/SCXMLParser.cpp`**. No `SCXMLValidator` class exists; `HistoryValidator` and `ForeachValidator` are runtime, out of scope. After dead-code + W5b-deferred filter: **4 wire-able sites**.

**Pre-flight #2 — Rust semantic-producer audit**: zero pre-existing `semantic/*` or `scxml/*` wire codes. But `forge::error::ValidationError` already covers the **conceptual** semantic-stage failures (`InvalidReference`, `EmptyCollection`, `MissingElement`, `DuplicateId`), each routed to a stable `validation/*` wire code via `validation_fields()`. Additionally, `analyzer::can_generate_static` already detects "initial state names undeclared state" but routes it to `ValidationError::DynamicFeatures` — a mis-classification W5 corrects (D3).

**Pre-flight outcome — option (d) reuse-existing-codes**: 3 of 4 wire-able C++ sites map to existing `validation/*` wire codes (W4 D4 fold precedent — concept identity over namespace duplication). Only 1 NEW wire code — `scxml/top-level-script-unloaded` for the W3C SCXML §5.8 top-level script rejection (no forge analog).

#### Site inventory table (10 sites, narrow-scope classification)

| # | File:Line | Failure shape | Classification | W5 action |
|---|-----------|---------------|----------------|-----------|
| 1 | SCXMLParser.cpp:270 | "Null scxml node or model" | Dead post-W4 (parseAbstractDocument typed-throws on null root before parseScxmlNode is reached) | **Stage E remove** |
| 2 | SCXMLParser.cpp:371 | "Top-level script element #N cannot be loaded ... document rejected per W3C SCXML 5.8" | Wire-able semantic | **NEW** `scxml/top-level-script-unloaded` |
| 3 | SCXMLParser.cpp:393 | "No state nodes found in SCXML document" | Wire-able semantic | **REUSE** `validation/empty-collection` |
| 4 | SCXMLParser.cpp:411 | "Failed to parse a root state" | W5b prereq — sub-parser detail not surfaced; generic parent message would be hollow per `feedback_silently_broken_hooks.md` | Out of scope, deferred to expanded W5 |
| 5 | SCXMLParser.cpp:531 | "Null model in validation" | Dead post-W4 (validateModel only called from success path of parseScxmlNode) | **Stage E remove** |
| 6 | SCXMLParser.cpp:541 | "Model has no root state" | Dead post-W4 (line 393 fires first if no states parsed) | **Stage E remove** |
| 7 | SCXMLParser.cpp:550 | "Initial state '<id>' not found" | Wire-able semantic | **REUSE** `validation/invalid-reference` |
| 8 | SCXMLParser.cpp:570 | "State '<X>' has parent '<Y>' but is not in parent's children list" | Internal model-construction invariant; defensive code without invariant assertion = silently broken (no Rust producer can emit the cross-side equivalent because Rust model construction can't violate parent/children pairing) | **Stage E remove** |
| 9 | SCXMLParser.cpp:581 | "Transition in state '<X>' references non-existent target state '<Y>'" | Wire-able semantic | **REUSE** `validation/invalid-reference` |
| 10 | SCXMLParser.cpp:596 | "State '<X>' references non-existent initial state '<Y>'" (compound state initial) | Wire-able semantic; same shape as #7 | **REUSE** `validation/invalid-reference` (fold with #7 — one C++ leaf `SemanticInitialStateUnknown` discriminates root vs compound state via payload field) |

**Net W5 typed-leaf surface: 4 wire-able sites** (#2, #3, #7+#10 fold, #9). **NEW wire codes: 1** (#2 only). **Dead-code removals: 4** (Stage E separate commit; #1, #5, #6, #8). **Deferred: 1** (#4, awaits W5b).

#### Decisions to pin (each with reverse-default; user OK gate before Stage B1)

##### D1. SemanticError throw shape — typed-throw mirroring W4 D1-C

```cpp
// sce/include/parsing/SemanticError.h (NEW)
class SemanticError : public std::runtime_error, public Diagnostic {
public:
    explicit SemanticError(std::string msg) : std::runtime_error(std::move(msg)) {}
    virtual std::unique_ptr<Diagnostic> clone() const = 0;
};

class SemanticInitialStateUnknown : public SemanticError {
    enum class Scope { DocumentRoot, CompoundState };
    // payload: state_id, scope, available_state_ids (for fix candidates)
};
class SemanticTransitionTargetUnknown : public SemanticError { /* state_id, target, available */ };
class SemanticNoStates : public SemanticError { /* no payload — document-level */ };
class SemanticTopLevelScriptUnloaded : public SemanticError { /* index, src (sanitized) */ };
```

Each subtype's `code()` returns the wire-code path string assigned by D2.

**Reverse-default — Result-based return (`std::expected<Model, SemanticError>`)**. Rejected: SCE C++ pipeline standardised on typed-throw at W4. Mixing return shapes reintroduces polling discipline W4 + W4.5 just deleted.

##### D2. Wire code namespace — REUSE existing `validation/*` per W4 D4 fold + 1 NEW for W3C SCXML §5.8 top-level script

| C++ leaf | Wire code | Reuse / NEW | Rust producer |
|----------|-----------|-------------|---------------|
| `SemanticInitialStateUnknown` (covers #7 + #10) | `validation/invalid-reference` | REUSE | `ScxmlSemanticError::InitialStateUnknown` (NEW Rust enum, REUSED wire code) |
| `SemanticTransitionTargetUnknown` (#9) | `validation/invalid-reference` | REUSE | `ScxmlSemanticError::TransitionTargetUnknown` (NEW Rust enum, REUSED wire code) |
| `SemanticNoStates` (#3) | `validation/empty-collection` | REUSE | `ScxmlSemanticError::NoStates` (NEW Rust enum, REUSED wire code) |
| `SemanticTopLevelScriptUnloaded` (#2) | `scxml/top-level-script-unloaded` | **NEW** | `ScxmlSemanticError::TopLevelScriptUnloaded` (NEW Rust enum, NEW wire code) |

`forge::error::ValidationError` is **NOT** modified — it stays forge-scoped per its file-level doc ("violates forge domain rules"). A parallel `ScxmlSemanticError` enum lives in `sce-build/src/scxml_semantic.rs` (NEW file), with its own `scxml_semantic_fields()` mapping that emits the SAME `DiagnosticCode` values via separate field-mapping but unified wire output. Stage = `Stage::Validation` (REUSED — SCXML semantic-validation IS post-parse semantic validation, same analytical stage as forge `validation/*`; if a future production consumer needs separate-stage dispatch, that's a separate decision, not a W5 prerequisite).

**Reverse-default — NEW `scxml/*` namespace for all 4 codes**. Rejected: violates W4 D4 fold precedent — `validation/invalid-reference` and "SCXML transition target unknown" are conceptually identical ("name X did not resolve to declared symbol Y"); inventing a parallel code creates duplicate wire identity for one concept and invites future drift.

**Reverse-reverse-default — generalize `forge::error::ValidationError` to admit non-forge document kinds (option b)**. Rejected per scope discipline (`feedback_yagni_vs_engineering_avoidance.md`). Touching 20+ existing variants and forge-namespace boundaries for a 4-site W5 is engineering avoidance dressed as elegance. If forge/SCXML enum unification ever becomes a real ask, that's a separate architectural RFC, not bundled here.

##### D3. analyzer.rs prior-art refit — split `ValidationDynamicFeatures` mis-classification

`analyzer::can_generate_static` currently returns 3 reasons all routed to `ValidationError::DynamicFeatures` (`validation/dynamic-features`):

| Reason | Current code | Correct classification | W5 action |
|--------|-------------|------------------------|-----------|
| 1 | "document rejected by W3C SCXML 5.8" | Hard semantic violation (top-level script failed; Interpreter would also reject) | **Refit** to `ScxmlSemanticError::TopLevelScriptUnloaded` (`scxml/top-level-script-unloaded`) |
| 2 | "no initial state (runtime default resolution required)" | Codegen limitation, NOT semantic error (runtime CAN resolve via §3.3 default) | **Keep** as `validation/dynamic-features` (correctly classified) |
| 3 | "initial state attribute names a state that is not declared" | Hard semantic violation (Interpreter would also reject) | **Refit** to `ScxmlSemanticError::InitialStateUnknown` (`validation/invalid-reference`) |

This refit changes the wire output of `sce-codegen` for documents that previously emitted `validation/dynamic-features` for reasons #1 + #3. Schema status `pre-release` allows this per `feedback_pre_release_no_compat.md` (no version bumps, no back-compat shims until SCE 1.0). The refit IS load-bearing for α-strict cross-side parity: without it, C++ would emit `validation/invalid-reference` and Rust would emit `validation/dynamic-features` for the same conceptual failure — exactly the drift the α-strict + drift-test pair exists to prevent.

**Reverse-default — leave analyzer.rs alone, accept C++/Rust wire-code drift on these failures**. Rejected per `feedback_correctness_before_features.md` and `feedback_no_versioning.md` — α-strict invariant violated would compound over future stages.

##### D4. forge::error::ValidationError architectural scope — preserve as forge-scoped, do NOT generalize

Option (b) of design exploration — generalize `ValidationError::InvalidReference` to drop `kind: ForgeKind` (replace with `Option<DocumentKind>`) — was explicitly rejected. Reasons:

- `forge::error::ValidationError` is **deliberately** forge-scoped per its file-level doc comment
- Refactoring would touch 20+ existing variants + downstream call sites (parser.rs, generator.rs, codegen test fixtures, drift tests)
- `spec_anchor()` mapping is forge-keyed (`SCE Forge §3.2`, etc.) — generalization would force per-variant per-doc-kind anchor lookup
- Wire-code REUSE (D2) achieves the cross-document-type concept identity goal without enum-level surgery

W5 introduces `ScxmlSemanticError` as a parallel enum at `sce-build/src/scxml_semantic.rs`. Both enums map to the same `DiagnosticCode` values via separate field-mapping functions — wire-format unified at the catalog layer, enum architecture preserved at the source layer.

**Reverse-default — generalize ValidationError now (engineering-avoidance avoidance)**. Rejected per scope discipline. If forge/SCXML enum unification ever becomes a real ask (driven by a third document type joining the validation surface, or by a consumer that needs typed dispatch across doc kinds), that's a separate architectural RFC.

##### D5. SCXMLParser callsite migration pattern — addError → throw typed leaf, with Q4-B coexistence

```cpp
// before (current pattern at SCXMLParser.cpp:550)
if (!model->findStateById(initialStateId)) {
    addError("Initial state '" + initialStateId + "' not found");
    isValid = false;
}

// after (W5 pattern, mirror of W4 D1-C)
if (!model->findStateById(initialStateId)) {
    throw SCE::parsing::SemanticInitialStateUnknown(
        /*scope=*/SemanticInitialStateUnknown::Scope::DocumentRoot,
        /*state_id=*/initialStateId,
        /*available=*/model->collectStateIds());
}
```

`SCXMLParser::parseFile` + `parseContent` catch arms (already wired for ParseError + TemplateError + XIncludeError per W4) gain:

```cpp
catch (const SCE::parsing::SemanticError &se) {
    addError(se.what());          // Q4-B legacy string surface
    recordDiagnostic(se.clone()); // Typed getDiagnostics() surface
    return nullptr;
}
```

The catch arm's `addError` populates `getErrorMessages()` for backward-compatible CLI tools / harnesses that grep the string list (`feedback_built_but_unconsumed.md` consideration: existing string consumers must not regress).

**Reverse-default — leave addError, throw additionally (dual-write)**. Rejected: dual-write at the throw site is exactly what W4.5 just deleted. Single-source typed-throw + catch-arm fanout is W4 textbook.

##### D6. Test-as-consumer pattern — W4 `*Boundary` + `*Consumer` template, applied verbatim

For each NEW C++ leaf, Stage C adds:

1. **`SCXMLParserBoundary.Semantic{Subtype}`** (4 tests in `tests/parsing/Diagnostic_test.cpp`) — pins parser's catch arm forwards the typed leaf to `getDiagnostics()` AND populates legacy `getErrorMessages()` (Q4-B coexistence).
2. **`SemanticErrorConsumer.TypedCodeDistinguishesFailureClass{Subtype}`** (4 tests) — pins that a hypothetical consumer dispatching on `code()` distinguishes this subtype from any other semantic subtype using only the wire code. Mirrors W4 `ParseErrorConsumer.TypedCodeDistinguishesFailureClass*`.
3. **Rust drift test `cpp_scxml_semantic_{name}`** (4 tests in `sce-build/src/scxml_semantic.rs::tests`) — pins Rust producer's wire code matches what C++ surfaces when same input shape is rejected.

For REUSE codes (`validation/invalid-reference`, `validation/empty-collection`), the Boundary + Consumer tests pin that the wire code emitted matches existing `validation/*` semantics — confirming the fold is honest (a consumer dispatching on `validation/invalid-reference` gets both forge AND SCXML failures uniformly; this is the W4 D4 fold success criterion applied symmetrically).

**Reverse-default — production consumer in `tools/sce-validate-cli/` first**. Rejected per user's explicit 2026-04-26 decision: test-as-consumer suffices for fragility-pinning at producer-introduction layer; Stage F (production consumer) stays separate, gated on real LSP/CI/agent ask.

##### D7. Schema + acceptance doc 11-place sync — applied to the 1 NEW code only

Only `scxml/top-level-script-unloaded` triggers `diagnostic_code_edit_checklist.md` 11-place sync:

1. `DiagnosticCode::ScxmlTopLevelScriptUnloaded` enum variant
2. `ALL_DIAGNOSTIC_CODES` list entry
3. `as_str()` match arm → `"scxml/top-level-script-unloaded"`
4. `spec_anchor()` match arm → `Some("W3C SCXML 5.8")`
5. `non_overlap_class()` match arm
6. Exhaustive match drift test arms (parser test + diagnostic test)
7. `len()` count assertion (152 → 153)
8. Payload builder in `scxml_semantic_fields()`
9. Golden snapshot if affected
10. JSON schema enum — `schemas/sce-diagnostic.v1.schema.json`
11. Acceptance doc appendix — `docs/SCE_ACCEPTED_SUBSET.md`

REUSED codes (`validation/invalid-reference`, `validation/empty-collection`) are schema-stable — already in JSON schema enum + acceptance doc appendix from prior milestones; no schema edits needed.

**Reverse-default — partial sync, file later**. Rejected: 11-place sync is a load-bearing drift guard; skipping any place produces silent acceptance / non-overlap drift the existing test suite catches at landing time, not at design time.

#### Stages

| Stage | Scope | Verification |
|-------|-------|--------------|
| A | This RFC §W5 (replaces line-1755 sketch with full body); pre-flight #1 + #2 results documented inline; D1-D7 user OK gate; site inventory table | n/a (`claudedocs/` is gitignored, no commit) |
| B1 | **Rust producers + REUSE.** New `sce-build/src/scxml_semantic.rs` (`ScxmlSemanticError` enum, 4 variants + `scxml_semantic_fields()`). Add `DiagnosticCode::ScxmlTopLevelScriptUnloaded` (1 NEW variant; 11-place sync). D3 refit: `analyzer::can_generate_static` reasons #1 + #3 emit `ScxmlSemanticError` in place of `ValidationError::DynamicFeatures`. Each variant pinned by Rust unit test. ALL_DIAGNOSTIC_CODES count goes 152 → 153. | `cargo test -p sce-build --features cli --lib` 100% |
| B2 | **C++ typed leaves.** New `sce/include/parsing/SemanticError.h` (base + 4 subtypes mirroring B1) + `sce/src/parsing/SemanticError.cpp` (`to_json()`, `clone()`, `code()`). SCXMLParserBoundary tests for each leaf (Q4-B coexistence + getDiagnostics surface). | C++ build clean; `ctest -R "Semantic\|Diagnostic"` 100% |
| C | **SCXMLParser refit.** Each of 4 wire-able sites rewrites `addError + return false` → `throw SemanticError-leaf`. Add SemanticError catch arm on parseFile/parseContent (paralleling W4 ParseError arm). Add `cpp_scxml_semantic_*` drift tests. SemanticErrorConsumer tests pin typed dispatch. | `cargo test` + `ctest` full baseline 100% |
| D | **RFC closeout + memory + FF-merge.** Flip §W5 status `OPEN → LANDED` with commit SHAs. Extend `wire_rfc_w0_w1_landed.md` with W5 closure section (1-screen budget, mirroring W4 / W4.5 closure structure). MEMORY.md one-liner update. | `cargo test --features cli` green; full ctest stable; ALL_DIAGNOSTIC_CODES count assertion 152→153 passes |
| E | **Dead-code cleanup (W4.5 follow-up precedent, separate commit on same branch).** Remove 4 dead-post-W4 sites (#1, #5, #6, #8). Commit message attribution: "revealed dead by W5 inventory, NOT by W5 typed-throw" (honest classification — these were W4-dead, just unnoticed until W5 site inventory). | C++ build clean; ctest 100%; site count after delete confirms 4 removals |

Stages B1 → B2 → C → D → E land on `feat/sce-wire-w5`. 1 commit per stage. Estimated effort: **1-2 sessions, 4-6 hours each** — narrower than starter's 2-3 sessions estimate due to (d)-fold reducing NEW code count from N to 1.

#### Validation pipeline (per stage)

```bash
cargo build --release -p sce-build --features cli
cargo test -p sce-build --features cli --lib

SCE_TEMPLATE_DIR=${PWD}/tools/codegen/templates cmake --build build_release -j8

# Targeted W5 surface
ctest --test-dir build_release -R "Semantic|Diagnostic|SCXMLParserBoundary" --output-on-failure

# Full regression (re-run flaky timer benchmark on miss — known unrelated flake)
ctest --test-dir build_release -j 4
```

#### Branch + tree state at session start

- HEAD verified: `513ff9ea` (or later) — clean working tree
- Branch: `feat/sce-wire-w5`
- Stage A drafts the RFC body alone — `claudedocs/` is gitignored, no commit. Stages B1 → C → D → E commit on same branch then FF-merge.

#### Out of scope (do NOT bundle)

- **Stage F (production consumer in `tools/` or `bindings/`)** — separate milestone, gated on real LSP/CI/agent ask.
- **W5a (pugi offset capture)** — was a prerequisite for the *expanded* W5 (covering all sub-parsers + source-coordinate wire field). Under narrow scope (4 wire-able sites, no `location()` field used by tests), W5a is **not** a W5 prerequisite. Re-opens if/when expanded W5 (W6+) ever surfaces.
- **W5b (sub-parser interface redesign)** — was a prerequisite for the *expanded* W5 (sub-parser detail surface). Site #4 (line 411 "Failed to parse a root state") falls in this scope and stays out of W5. Re-opens if/when expanded W5 surfaces.
- **`forge::error::ValidationError` generalization (option b)** — separate architectural RFC if forge/SCXML enum unification ever becomes a real ask.
- **AOT-side `parser.rs` SCXML semantic checks beyond the analyzer.rs refit** — `parser.rs` already validates structural shape via existing `ValidationError`. Whether to refit those to `ScxmlSemanticError` is pre-empted by D3 (D3 only refits `analyzer::can_generate_static` reasons #1 + #3). Wider parser.rs refit is a separate decision.
- **Datamodel coercion errors / runtime-execution errors** — separate milestone (W6+ if ever).
- **Schema status flip to `stable`** — stays `pre-release` through W5.
- **Cross-side id byte-equivalence** — gated on consumer ask (W4 precedent).

#### Anti-patterns to actively avoid

1. **NEW C++ leaf without Rust producer.** `feedback_built_but_unconsumed.md`. The 1 NEW wire code (`scxml/top-level-script-unloaded`) MUST land with its Rust producer in B1 before C++ leaf in B2. The 3 REUSED codes already have Rust producers (existing `validation/*` infrastructure plus B1's NEW `ScxmlSemanticError` variants emitting them) — α-strict satisfied by reuse plus 1 NEW variant.
2. **Test consumer as `EXPECT_THROW` without dispatch.** D6 requires consumer test casts to typed leaf and dispatches on `code()`. Hollow "something throws" tests fail Stage C review.
3. **Q4-B legacy surface broken.** Existing `getErrorMessages()` consumers must still see semantic failures populated as strings. Catch arm responsibility — verify with existing parser test suite no string-surface regression.
4. **Stage A skipped under "I already know the inventory" pressure.** Pre-flight #1 surfaced 4 dead-post-W4 sites starter prompt's "~50+" estimate didn't anticipate — verify-before-ship lesson applied.
5. **NEW wire code count > NEW Rust producer count.** 1 NEW code = 1 NEW Rust producer (`ScxmlTopLevelScriptUnloaded`). 3 REUSED codes have NEW Rust producers (`InitialStateUnknown`, `TransitionTargetUnknown`, `NoStates`) emitting EXISTING wire codes — α-strict satisfied at the producer × code matrix.
6. **Bundling Stage E (dead-code removal) into Stage C (typed-throw refit) commit.** Stage E is its own commit per W4.5 follow-up precedent (`513ff9ea` cleanup post-`e62a6f6e` main W4.5 commit). Commit-time attribution distinguishes "W5 typed-throw" from "W4-dead cleanup noticed during W5 inventory".
7. **Adding `Stage::ScxmlSemantic` for stage-level dispatch without consumer ask.** The current narrow scope reuses `Stage::Validation` for all 4 sites. If a future consumer needs separate-stage routing, that's a separate decision — pre-emptive new stage variant violates `feedback_built_but_unconsumed.md`.

#### Pre-flight check (before starting Stage B1)

**W5 trigger** (codified):
- W4.5 closure cleared the document-level polling debt; semantic stage is the sole remaining `addError(string)` producer in the parsing pipeline.
- User's explicit 2026-04-26 decision: test-as-consumer suffices for typed-surface introduction.

**Why W5 is NOT a `feedback_built_but_unconsumed.md` violation**:
- Test fixtures count as consumers (W4 closure precedent applied).
- α-strict filter (D2): NEW wire codes constrained to 1 by W4 D4 fold reuse — 3 of 4 wire-able sites map to existing `validation/*` codes.
- The 1 NEW code (`scxml/top-level-script-unloaded`) has Rust producer (analyzer.rs refit + scxml_semantic.rs) AND C++ leaf (SemanticError.h) AND test consumer (SemanticErrorConsumer + Rust drift test) — fully closed loop at landing.

**Until user OK on D1-D7**, semantic family stays on `addError(string)` and the W5 branch is not created.

---

## §4 Risks and mitigations

| Risk | Mitigation |
|---|---|
| W1 ships without a JSON consumer; infrastructure is dead code | Canonical-JSON parity test is the standing consumer — Rust output is the ground truth (32 passing tests), C++ must match byte-for-byte after canonicalisation. Per `feedback_built_but_unconsumed.md`, the test must bite; load-bearing swap-and-fail verification required in the landing commit message. If W2 never materialises, W1 still pins Rust/C++ agreement on every shared fixture. |
| `Diagnostic` base class adds virtual-dispatch cost to every `TemplateError` throw | Measured, not speculated. Template-error throws are not hot-path (parse-time, not runtime-event-dispatch). Profile post-W1 if consumer concern arises. |
| Schema evolution forces dual-commit coordination across both sides | `x-sce-schema-status = "pre-release"` through this RFC. Additive changes only. `schema_file_declares_status` drift test pins the flip. Canonical-JSON parity test additionally catches any C++-side serialisation lag behind Rust schema changes. |
| `emit_json_diagnostic` diverges from Rust's serializer silently | **Dual-gate:** Rust-side drift test (item #7) asserts every TemplateError subtype has a formatter branch; canonical-JSON parity test (item #6) asserts the branches' actual output agrees with Rust byte-for-byte. Producer-side presence + consumer-side content both pinned. |
| Phase C slips; W1 ships with `location() == nullopt` on every Diagnostic | Honest soft-dependency. Rust side's JSON schema allows `location` to be absent; Rust output on non-Phase-C-aware fixtures also emits `location: null` (verified in `sce-build/tests/error_format_json.rs`). Canonical-JSON byte-diff passes on `null == null`. When Phase C P2 ships, Rust and C++ both start populating `location` and the byte-diff continues to hold. No W1 code change when Phase C lands. |
| Q4-B permanent coexistence leaves `addError(string)` as a permanent fallback, eroding the "unification" promise | Honest. The unification is class-by-class, driven by consumer demand. The RFC does not claim all parse errors become typed; it claims *Template* errors do in W1, and the pattern is extensible as consumers arrive. `getDiagnostics()` + `addError(string)` parallel surfaces: typed consumers read the former, legacy consumers read the latter. |
| W1 passes W1 tests but produces malformed NDJSON under corner cases (empty message, control chars in template names, path with spaces) | Canonical-JSON parity test fixtures include corner cases when Rust side emits them. Because the test compares against Rust output, any corner-case-handling divergence manifests as a byte-diff mismatch. Rust side's corner-case handling (verified in `error_format_json.rs`) becomes the C++ spec. |
| Canonicalisation step hides a real semantic divergence | Canonicalisation only normalises **key order** and **whitespace**. Field names, values, and types are preserved bit-for-bit through the parse / re-serialise cycle. An actual content divergence (wrong code, missing field, wrong message, different fix kind) still surfaces as a byte-diff mismatch after canonicalisation. The only class of divergence suppressed is non-semantic ordering, which is acceptable per `feedback_green_tests_not_correct.md` — canonicalisation is spec-compliant normalisation, not bit-approximation. |
| W5a (pugi offset capture) has no standalone consumer — might never land, blocking W5 forever | W5a's standalone consumer is W5 itself. Until a semantic-error consumer appears, W5a is not justified. RFC is honest that W5 is a long-term aspiration, not a commitment; the chain "consumer → W5 → W5a + W5b" is the trigger shape. |
| W5b sub-parser interface redesign risks ABI breakage for embedders that subclass `ActionParser` / other sub-parsers (audit #4) | Survey: no in-repo embedder subclasses a sub-parser (grep `class.*: public .*Parser` in `bindings/`, `sce-python/`, `sce-kotlin-runtime/` → 0 matches). Out-of-repo embedders would be unknown — honest risk, tracked as "if W5b reopens RFC, enumerate out-of-repo sub-parser subclasses before committing to ABI shape." |
| Q6-A severity day-one requires every W1 subtype to declare `severity()` — churn if a subtype needs `Warning` later (e.g. "unused state" warning in W5) | Subtype can override `severity()` at any time; returning `Error` in W1 and switching to `Warning` later is a trivial source-local change. The load-bearing discipline is that the *base class* carries the field from day-one; subtype-level overrides are cheap. |
| Q6 drift test (`cpp_severity_enum_matches_rust`) missed if Rust adds a fourth variant | Test asserts exactly 3 variants; adding `Info` or `Hint` to either side causes a red diff on both sides. Pre-release status means additive schema changes are allowed per Q5; coordinating the enum extension across sides follows the same dual-commit pattern as adding a new DiagnosticCode. |
| W0 deletion silently breaks a consumer (embedder that linked against the deleted main.cpp somehow) | W0's load-bearing verification catches this: `cmake --build` post-deletion must stay green, and `ctest` must stay green. In-repo survey shows 0 CMake references — but any out-of-repo consumer linking against `sce/src/main/` directly would break. Pre-1.0 status (`feedback_pre_release_no_compat.md`) means no back-compat obligation; if out-of-repo consumers surface, they can restore the file themselves or migrate to calling `SCXMLParser::parseFile` directly. |
| Binding passthrough (`getDiagnosticsJson()`) on W2 risks Python / Kotlin / Go binding surface churn if each picks a different JSON shape | W2 contract pins the batch NDJSON format; bindings expose it as-is. Each binding is opt-in and does minimal work (passthrough of a `std::string`). No per-binding re-serialisation. |
| W2-W5 never reopen; RFC becomes permanent "W1-only" (with W0 cleanup) | Acceptable outcome. The RFC explicitly names W2-W5 as reopenable sketches, not commit-series contracts. Permanent-W1 is a coherent end-state if no other consumers arrive — Template family unification alone is meaningful progress. |
| Monolithic vs per-module enum decision (Q3-B) gets revisited when W5 lands (semantic family is the largest) | W5 reopens Q3 specifically. Q3 landing in W1 is binding only for W1's template family; W5's design questions re-examine enum strategy for the semantic family's scale. |

---

## §5 Commit shape this session

This session delivers the RFC only (one commit-series-sized
document on disk at
`claudedocs/rfc-sce-diagnostic-wire-unification.md`). Because
`claudedocs/` is fully gitignored (`.gitignore:115`), the RFC file
creates no git history. Memory updates likewise live outside the
repo. Therefore:

- **Zero commits expected this session.**
- No branch cut for W1 (branch `feat/sce-diagnostic-wire-w1` is
  created when W1 code lands in a follow-up session).
- Working tree stays at current HEAD `1940c51f`.

**W0 landing** (follow-up session, BEFORE W1) is expected as a
1-2 commit series on `feat/sce-diagnostic-wire-w0`:

1. `refactor: Delete dead-code C++ codegen placeholder
   (sce/src/main/main.cpp)` — 170-line file deletion + any
   empty-directory cleanup + docs grep verification. Commit
   message cites audit finding #6.
2. (optional) `chore: Remove sce/src/main subdirectory` — only
   if step 1's `ls sce/src/main` is empty after deletion and
   the directory itself should go.

**W1 landing** (follow-up session, depends on W0) is expected
as a 6-8 commit series on `feat/sce-diagnostic-wire-w1` (grew
from the original 4-6 estimate after the preflight audit
surfaced the boundary-flatten work):

1. `feat: Add SCE::parsing::Severity enum + Diagnostic base +
   Fix variant type` (Q6-A day-one severity)
2. `feat: Refit TemplateError subtypes to implement Diagnostic`
   — each of 8 subtypes returns `Severity::Error` in W1.
3. `feat: Add DiagnosticJsonFormatter (nlohmann::ordered_json NDJSON)`
4. `feat: Remove SCXMLParser boundary flatten for TemplateError
   (audit finding #1)` — typed catch chain ahead of std::exception
   fallback; `recordDiagnostic` + `getDiagnostics()` parallel
   surface alongside existing `addError(string)` /
   `getErrorMessages()`.
5. `test: Pin TemplateError JSON output via canonical-JSON byte-diff
   vs sce-codegen --error-format=json` — shared fixture family
   replay through both paths; `nlohmann::json::dump(-1)` on both
   sides before ASSERT_EQ.
6. `test: Add cpp_severity_enum_matches_rust + cpp_template_formatter_has_branch_per_subtype
   drift tests` — Rust-side mirrors of
   `cpp_template_subtypes_match_rust_diagnostic_codes`.
7. (optional) `chore: Regenerate embed/MANIFEST.json after <sha>` —
   Diagnostic.h + Fix.h affect embedding if they end up in the
   published public headers.
8. (optional) `docs: Note Diagnostic consumer pattern in
   SCE_ACCEPTED_SUBSET.md` — only if the subset doc needs a
   new §X for structured diagnostics. Likely deferred unless W2
   materialises.

**LOC estimate (revised):**
- **W0:** Prod: -170 (deletion only) / Test: 0 (regression
  coverage is the existing build-green + ctest-green).
- **W1 Prod:** ~430 (Severity.h ~15 + Diagnostic.h ~80 +
  Fix.h ~60 + DiagnosticJsonFormatter.cpp ~180 + SCXMLParser
  catch-chain ~55 + TemplateError refit ~40)
- **W1 Test:** ~380 (6 parity-test fixtures + canonicalisation
  helper + 2 drift tests + swap-and-fail coverage)
- Approx 2x the original estimate because boundary flatten
  removal + parity test (byte-diff) + day-one severity are more
  code than the originally-planned single-file schema validator
  would have been. Net still small relative to Phase C's P1
  (~1300 LOC).

---

## §6 References

- `claudedocs/rfc-sce-template-phase-b.md` — RFC §2 named this
  wire-contract work as "a separate future RFC"; this document
  discharges that commitment.
- `claudedocs/rfc-sce-template-phase-c.md` — Phase C (PositionMap
  C++ port); this RFC consumes Phase C's `SourcePos` as the
  `location()` return type on `Diagnostic`.
- `sce-build/src/forge/diagnostic.rs` — authoritative Rust
  `DiagnosticCode` + `ForgeError` + `Fix` enums; C++ side
  mirrors the template family in W1, extends per-milestone.
- `schemas/sce-diagnostic.v1.schema.json` — the schema C++
  `emit_json_diagnostic` must conform to.
- `docs/SCE_ERROR_CONTRACT.md` — referenced by `CLAUDE.md`
  guardrails; surveyed via `ls docs/` (not found at that exact
  path at RFC time; CLAUDE.md reference may point to a planned
  doc or a doc at a different path).
- `docs/adr/0001-error-format-flag-naming.md` — prior ADR
  context for `--error-format=json` flag naming.
- Memory `feedback_built_but_unconsumed.md` — W1 standing
  consumer discipline; W2-W5 consumer-gated.
- Memory `feedback_green_tests_not_correct.md` — load-bearing
  verification required at W1 landing.
- Memory `feedback_yagni_vs_engineering_avoidance.md` — W2-W5
  scope sketches are the "phased deferral" shape this memory
  sanctions for large-design-surface work.
- Memory `diagnostic_code_edit_checklist.md` — 11 touchpoints
  for monolithic enum evolution; Q3-B per-module choice avoids
  most of these, but the `code` string on the JSON wire must
  still appear in the Rust registry (W1 drift test pins).
- Memory `feedback_no_versioning.md` — schema stays at v1,
  pre-release, throughout this RFC. No v1 / v2 tunables.
- Memory `feedback_silently_broken_hooks.md` — cited by W5a
  prerequisite rationale: `location() == nullopt` on every
  semantic diagnostic would be a silently-broken wire.

---

## §A Preflight audit evidence (2026-04-23)

Five-area audit run before W1 landing, forcing the inline
revisions above. Raw greps and file-reads, not speculation.

### Finding #1 — SCXMLParser boundary flattens typed exceptions

`sce/src/parsing/SCXMLParser.cpp:80-95`:

```cpp
SCE_LOG_DEBUG("Processing sce:template");
doc->processSceTemplate();   // throws TemplateCycle, TemplateNotFound, etc.

return parseAbstractDocument(doc);
} catch (const std::exception &ex) {
    addError("Exception while parsing file: " + std::string(ex.what()));
    return nullptr;
}
```

Phase B M4's typed `TemplateError` subtypes exist (verified via
`grep class.*Template.*: public sce/include/parsing/TemplateError.h`
→ 8 subtypes) but are caught by the `std::exception&` overload
and immediately flattened through `ex.what()` + string
concatenation. No typed consumer exists on the SCE side today.

**Consequence:** The original W1 contract was "add JSON wire on
top of typed exceptions" — but the typed exceptions never reach
any W1-adjacent consumer because they're erased 3 lines earlier.
W1 must preserve typed identity at the catch site, not just add
downstream wire. Revision landed as deliverable item #5.

### Finding #2 — pugi `offset_debug` not used anywhere in sce/

`grep -rn offset_debug sce/include/ sce/src/` → 0 matches.
`grep -rn "TextPos\|SourcePos" sce/src/parsing/` → 0 matches.

The C++ parser handles `pugi::xml_parse_result` (load-time byte
offset on parse failures) at 3 call sites in `PugiXMLParser.cpp`
(lines 287, 917, 954), but does NOT walk parsed nodes with
`offset_debug()` to capture per-node (row, col). At semantic-
validation time, every sub-parser (StateNodeParser,
TransitionParser, DataModelParser, ActionParser, GuardParser,
InvokeParser, DoneDataParser) receives a pugi node pointer with
no bundled source coordinate.

**Consequence:** W5 semantic-family typing cannot fill
`Diagnostic::location()` without a preceding milestone that
captures offsets. Revision landed as W5a prerequisite scope
sketch.

### Finding #3 — No JSON schema validator library in the tree

`grep -rn "json-schema-validator\|json_validator\|JsonSchema\|validator.hpp" sce/ third_party/ CMakeLists.txt cmake/`
→ 0 matches.

`grep -rn nlohmann sce/`:
- `sce/CMakeLists.txt:85-87` — `nlohmann/json` header-only lib
  linked through third_party.
- `sce/include/mesh/CommunicationError.h:47` — `#include
  <nlohmann/json.hpp>` + `nlohmann::ordered_json`
  serialisation precedent.
- `sce/include/runtime/JsonUtils.h` — centralised JSON utils.

**Consequence:** C++ cannot load and validate JSON against
`sce-diagnostic.v1.schema.json` without adding a validator
dependency. Adding one is non-trivial (build-system change,
new third_party, potentially new license). Revision landed as
the byte-diff-vs-Rust conformance method — uses `nlohmann/json`
(already linked) for canonicalisation; Rust output is the
standing contract.

### Non-finding — Language binding blast radius is zero

`grep -rln "getErrorMessages\|parseFile\|parseContent\|SCXMLParser"
sce-python/src/ sce-kotlin-runtime/ sce-forge-runtime/` → zero
matches against the parser's vector-of-strings API.

All bindings consume errors via `ReadySCXMLEngine::lastFactoryError()`
(single thread-local `std::string`) — Python (`bindings.cpp:41,
51, 156, 267`) and Kotlin (JNI-side, no direct parser usage).

**Consequence:** W1 adding `getDiagnostics()` parallel surface
does NOT force any binding-layer change. Bindings can opt-in
later if their consumers want typed errors, but zero is forced.

### Non-finding — Exception hierarchy has one typed beachhead

`sce/include/parsing/TemplateError.h`:

```cpp
class TemplateError : public std::runtime_error { ... };
class TemplateReadError : public TemplateError { ... };
```

Other error families (XInclude, core parser, semantic) inherit
`std::runtime_error` or use `addError(string)` directly — no
common base beyond `std::runtime_error`.

**Consequence:** W1's `Diagnostic` base can sit alongside
`TemplateError` (Template family refit implements both
interfaces). W3/W4/W5 introduce `XIncludeError`,
`ParseError`, `SemanticError` base classes as each milestone
lands. No existing hierarchy forces a wider refactor.

### Finding #4 — Sub-parser bool-chain with log-only detail

Survey (`grep -c "addError\|SCE_LOG_ERROR\|return false"
sce/src/parsing/*.cpp`):
- GuardParser.cpp: 1
- ActionParser.cpp: 6
- DataModelParser.cpp: 3
- StateNodeParser.cpp: 1
- TransitionParser.cpp: 1

`grep "addError"` against the same files: **0 matches across
all sub-parsers**. Sub-parsers do not accumulate errors into the
parent parser's vector.

Concrete pattern (`ActionParser.cpp:45, 67, 89, 98`):
```cpp
return false;   // control-flow only; no error recorded
```

`ActionParser.cpp:179, 286`:
```cpp
SCE_LOG_ERROR("ActionParser: Failed to parse action node: '{}'",
              element->getName());
SCE_LOG_ERROR("ActionParser: {}", errorMsg);
```

→ **Detail goes to logging, not to any error container.** Parent
`SCXMLParser::parseScxmlNode` receives only `bool` and calls
`addError` with its own generic message. The sub-parser's
specific failure reason is discarded from the error path.

**Consequence:** W5 typed semantic errors need real detail
(what attribute was bad, what expression failed to compile,
etc.). The bool-chain discards that detail. W5b is a separate
pre-W5 milestone that reshapes sub-parser interfaces. Revision
landed as W5 "depends on W5a + W5b" in the milestone table.

### Finding #5 — Abstract interfaces are string-typed

`sce/include/parsing/IXMLParser.h`:
```cpp
class IXMLParser {
    virtual std::shared_ptr<IXMLDocument> parseFile(
        const std::string &filename) = 0;
    virtual std::shared_ptr<IXMLDocument> parseContent(
        const std::string &content) = 0;
    virtual std::string getLastError() const = 0;   // ← string-typed
};
```

`sce/include/model/IXIncludeProcessor.h`:
```cpp
class IXIncludeProcessor { ... };   // similar string-based shape
```

Concrete implementations (`PugiXMLParser`, `XIncludeProcessor`)
override the string methods.

**Consequence:** W3/W4 typed-error migration affects the
interface surface. Two tradeoffs documented in each milestone's
scope sketch (W3-A/B, W4-A/B). Default is extend-interface at
each W milestone reopening (add typed method alongside string
method per Q4-B); switchable to concrete-only if consumer
signal justifies.

### Finding #6 — C++ codegen duplicate (dead code)

`sce/src/main/main.cpp` (170 lines):
- Line 34: `SCE_LOG_INFO("SCXML-to-C++ Code Generator");`
- Line 104: `auto model = parser.parseFile(inputFile);` —
  actually parses the SCXML
- Lines 121-158: writes C++ boilerplate to `outFile`, ending at
  line 142: `// TODO: Generate state machine logic based on SCXML`
  and line 143: `// This is a placeholder implementation`

→ **Parses but does not generate code.** Writes a skeleton with
TODO markers. Real codegen is Rust `sce-codegen`
(`target/release/sce-codegen`, verified).

CMake references (`grep -rn "src/main/main\|scxml-codegen\|sce_main"
--include="CMakeLists.txt" --include="*.cmake"`): **0 matches**.
The file is not registered in any build graph.

Binary existence check (`find build_release -name scxml-codegen`):
**0 matches**. No binary is built from this file.

**Consequence:** Dead code that self-identifies as a codegen
tool but isn't one, not built, not consumed. W0 deletes it
unambiguously. Architecture boundary: the codegen CLI surface is
Rust `sce-codegen` alone; C++ exposes Interpreter via library
API only (no C++ CLI binary).

### Finding #7 — Severity absent from C++ error surface

C++ side: `SCXMLParser::addError(const std::string &message)` —
one method, one semantic ("error"). No `addWarning`,
no `addInfo`, no severity parameter.

Rust side (`sce-build/src/forge/diagnostic.rs`): `Diagnostic`
struct carries `severity` field (`Severity::Error`,
`Severity::Warning`, `Severity::Advice`). All template-family
diagnostics emit `"severity": "error"` in JSON wire.

**Consequence:** W1 canonical-JSON byte-diff test would fail
on a trivial format mismatch if C++ omits `"severity"`. Q6
pins day-one policy (Severity enum lands in W1, all
Template-family subtypes return `Error`). Future families (W5
especially) may introduce `Warning` where the semantic parser
continues past a non-fatal issue.
