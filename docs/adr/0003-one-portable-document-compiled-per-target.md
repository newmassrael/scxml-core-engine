# ADR 0003 — One portable document, compiled per target

- Status: Accepted
- Date: 2026-09-16
- Scope: expression lowering across the six backends; the `datamodel`
  attribute; `NeedsScriptEngineCause` and the `script_engine_causes`
  manifest field
- Related: `ARCHITECTURE.md` (Key Principles; Scripting Engine
  Architecture), `docs/SCE_ACCEPTED_SUBSET.md` (the `datamodel` table),
  `sce-build/src/script_engine_analyzer.rs`

## Context

SCE generates ahead of time for six backends. What it cannot lower ahead
of time it routes to a runtime script engine, and it already says so per
document: the generate manifest carries `needs_script_engine` together
with `script_engine_causes`, one record per cause with a source location.
Twenty cause kinds exist today, from `TransitionGuard` to
`DonedataContent`.

Two pressures met in the same week.

**A consumer could not move its pin.** A downstream harness holds 432
documents that pair `cond="cpp:…"` with `datamodel="null"` and return
their verdict through inline `<donedata><content>` text. Since
`189202b71e` made the `datamodel` attribute select a language rather than
decorate, W3C SCXML B.1.2 is enforced: under the null data model the
boolean expression language is `In(id)` and nothing else, so the
conditions are refused. Declaring `ecmascript` instead admits the
conditions — they stay native, and no cause names them — but the inline
donedata text then forces a script engine, because the ECMAScript data
model appendix gives that text a reading that is decided by evaluation.
The consumer is caught between a value that forbids its conditions and a
value that pulls in an engine it does not otherwise need.

**The tree already leans the other way in two places.** `SendParamExpr`
records that static `<param>` literals are *folded at build time*, and
`TransitionGuard` records that native `cpp:` / `kt:` conditions are
emitted inline and raise no cause at all. `is_native_script` states the
principle in words for `<script><cpp>…</cpp></script>`: such an element
"names no data model expression at all."

So the question was not whether to add another escape hatch. It was what
an AOT engine is for.

## Decision

**One portable document, compiled per target.**

1. The document is the single source. A statechart is authored once and
   SCE compiles it into each backend's own language.
2. A runtime script engine is a **fallback for what cannot be decided at
   build time**, not a stage of the pipeline. Every cause that *can* be
   decided statically is to be moved into the compiler.
3. Native prefixes (`cpp:`, `kt:`) remain an **escape hatch**, not a
   path. They are admitted where the surrounding rule is about data model
   languages — they are not one — and their use is **counted**, because a
   document that uses them is no longer portable.
4. The twenty cause kinds are the work list. Their per-cause population
   over the corpus is measured first, and the measurement is ratcheted so
   the number cannot grow unremarked.

## Considered alternatives

### Option A — one portable document, compiled per target (chosen)

Keeps a statechart one document while removing the engine from the common
path. Costs the most implementation, which is the point of taking it
deliberately rather than drifting into it.

### Option B — per-backend dialects: widen `cpp:` into `go:`, `py:`, `rs:`, `c:`

Rejected. With six backends this is N x M: the same machine is authored
once per target and the generator stops being a generator. The pressure
that raised this ADR is already the first frame of that future — those
432 documents are C++-only and cannot reach the Kotlin backend at all.
An escape hatch that becomes the front door ends the parity obligation
Key Principles 7 states.

### Option C — change the ECMAScript inline-content reading so it stays literal

Rejected as stated, because it changes a W3C reading. The defect it aims
at is real, but the correct form of the fix is different: when a
`<content>` body is **static**, which reading applies is decidable at
build time, so SCE can decide it in the compiler and emit a constant
without touching the semantics. Only `expr` — a value that exists at run
time — needs the runtime decision. This lands under Option A as one of
the twenty causes, not as a separate rule.

## Option D — define a platform `datamodel` value for native expressions

Deferred, not rejected. W3C SCXML 3.2 permits platform-defined values and
`docs/SCE_ACCEPTED_SUBSET.md` records that SCE defines none. A value such
as `sce:native` would give the `cpp:` extension a declared home instead of
letting it live under `null`. It is costlier for consumers (every
document's declaration changes) and it is worth revisiting **if the
escape-hatch column in the census grows** rather than shrinks.

## What is already built, measured 2026-09-16

Written down because the decision reads as "build a compiler" and that is
not the state of the tree. Half of it exists and is in use, and a reader
who starts from zero will re-derive the wrong cost.

**Four condition classes already need no engine, and the list is closed
rather than sampled** (`check_expression_needs`, the single classifier
every backend's guard arm is chosen by):

1. a `cpp:` / `kt:` condition — the author wrote target-language source;
2. a pure `In(...)` predicate — answered from the active configuration;
3. a condition the frontend folds at build time
   (`crate::ecmascript::constant_truthiness`);
4. **a typed `_event.data` guard under an imported EventSchema** —
   codegen emits it as a native typed-payload guard, and
   `generator.rs` holds that no backend can reach codegen with an
   un-lowerable one.

Class 4 is the existence proof this programme extends: a native lowering
of an ECMAScript guard, across all six backends, already shipping.

**The expression front half exists.** `sce-build/src/ecmascript/` carries
a parser, a scope resolver, and a builtin surface that is closed and
guarded: `INSTALLED_GLOBALS` (`Boolean`, `In`, `JSON`, `Math`, `Number`,
`Object`, `String`, `parseFloat`, `parseInt`, plus the five system
variables) and `LOWERED_METHODS` (fourteen names), whose membership is
pinned to the emitter's arms by
`ecmascript_builtin_vocabulary::every_lowered_method_has_an_emitter`. A
free identifier is not a builtin question at all — `resolve` answers it
from the document's own bindings.

**What is missing is the back half for five of six targets.** There is
exactly one emitter, `ecmascript/lua.rs`. So the pipeline today parses
and checks an expression and then lowers it to *Lua source handed to a
runtime engine* — translation is already happening; it lands in an
interpreter rather than in the target language. The programme is
therefore "add target emitters beside the Lua one, against a typed data
model", not "write a compiler".

**What blocks it is types, not syntax.** ECMAScript is dynamically typed;
`x > 1` lowers to C++ only when `x`'s type is known. `<data>`
declarations and EventSchema are where types exist today, which is
exactly why class 4 above is the class that already works. Where a type
is unknown the compiler must infer it, refuse, or fall back — and that
boundary is the subset's definition, a design choice rather than a
discovery.

**Two causes are statements, not expressions.** `GlobalScript` (3
records) and `InlineScriptAction` (7) are arbitrary ECMAScript statement
bodies. They are the honest candidates to stay engine-bound, and the
decision's "fallback for what cannot be decided at build time" is aimed
at them.

**And what the escape hatch actually delivers, stated so it is not
re-derived as worse than it is:** a `cpp:` guard *is* compiled, natively,
with no engine. It buys the engine-free half outright. What it spends is
the portable half — the same document reaches one backend, and
`first_unlowerable_native_cond` refuses the others by name rather than
mis-compiling them. The consumer that raised this ADR is already
generating and running that way; what it cannot do is take those
documents to a second backend.

## Consequences

- The census comes before the programme. Nobody knows the per-cause
  distribution today, so the first artefact is a corpus-wide count of
  `script_engine_causes` by kind, with a ratchet that reddens when a
  count grows. Writing programme sections before that would order the
  work by guess.
- `cond="cpp:…"` is admitted under `datamodel="null"`, for consistency
  with `<script><cpp>` which is already admitted there on the stated
  grounds that native code is not a data model expression. The admission
  is recorded in `docs/SCE_ACCEPTED_SUBSET.md` beside that one, and the
  census carries escape-hatch use in its own column.
- `ARCHITECTURE.md`'s Scripting Engine Architecture section describes the
  present state, in which the C++ backend emits the author's ECMAScript
  verbatim. That section now says it is the state this decision shrinks,
  so the two documents do not describe different futures.

## What would reverse this

- More than half of the twenty cause kinds prove **undecidable** at build
  time once measured — then the engine is not a fallback but a
  requirement, and the honest architecture says so.
- A backend cannot express a compiled form of the accepted expression
  subset, so "compiled per target" holds for five backends and not six.
  Parity is the obligation; a decision that quietly drops one backend is
  not this decision.
