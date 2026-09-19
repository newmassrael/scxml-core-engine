# SCE script-engine programme

ADR 0003 decided that SCE compiles one portable document into each backend's
own language, and that a runtime script engine is the **fallback** for what
cannot be decided at build time. That decision implies work: every construct
that reaches for an engine today is a candidate for being decided at build
time instead.

This document is the **order** that work is taken in, and the order was not
chosen. It is measured. Nothing here is a preference.

## The denominator, stated first

Shares below are taken over **475 judged documents**, not the 736 the census
sweep walks. The difference is documents the census cannot judge -- 187 forge
documents (stateless by construction), 30 templates and non-statecharts, and
44 deliberate negative fixtures -- and `docs/SCE_SCRIPT_ENGINE_CENSUS.md`
records all of them.

**233 of those 475 need an engine; 238 already do not.** Moving the 233 down
is what this programme is for.

## The order, and the metric that sets it

⚠ **The first version of this document ordered the sections by RECORD count,
and that was the wrong metric.** Measured 2026-09-17, per document rather
than per record:

| Cause kind | Records | Documents containing it | Documents where it is the ONLY cause |
|---|---|---|---|
| `transition-guard` | 210 | 135 | **20** |
| `datamodel-variable-init` | 246 | 126 | **5** |
| `assign-action` | 206 | 93 | **8** |

A record is not a document. `datamodel-variable-init` has the most records
and frees the fewest documents, because **the causes are entangled**: the
commonest non-empty combination is all three at once (32 documents), then
`datamodel + guard` (25), then `guard` alone (20), then `assign + guard`
(10), then `assign + datamodel` (9). A document is freed from the engine only
when *every* cause it carries is gone.

So the payoff is cumulative, and the order decides what arrives first:

| Order | After §1 | After §2 | After §3 |
|---|---|---|---|
| **guard → datamodel → assign** | **20** | **50** | **109** |
| guard → assign → datamodel | 20 | 38 | 109 |
| datamodel → guard → assign *(the old order)* | 5 | 50 | 109 |

All three finished frees **109 of the 233** engine-needing documents (47%).
No single section frees much alone; the old ordering simply delivered its
smallest instalment first.

## §1 `transition-guard` -- 210 records, 135 documents, 20 freed alone

`<transition cond>`. First on the measurement, and it also has the only
worked example: a `cpp:`/`kt:` native guard, a pure `In()` predicate, a
constant-folded expression, and an EventSchema-typed `_event.data` guard all
already need no engine. `check_expression_needs` in `sce-build/src/parser.rs`
is the single classifier that decides which.

**Closed when**: the classifier admits every guard shape the accepted subset
allows, and each refusal names the construct it could not decide rather than
falling through to the engine silently.

⚠ The native-prefix escape hatch is **not** the plan for this section. The
census carries its use in its own column (1 document today) precisely so that
an escape hatch cannot quietly become the path.

## §2 `datamodel-variable-init` -- 246 records, 126 documents, 5 freed alone

`<data>` initialisers. The largest cause by records and the smallest by
documents freed alone -- but second in order, because pairing it with §1
reaches 50 documents where pairing §1 with §3 reaches only 38.

**Measured 2026-09-17**: of 223 top-level `<data expr>` in judged documents,
**196 are pure literals** -- 153 integer, 33 string, 10 keyword -- and every
one of them forces an engine. The generated C++ for `<data id="unitId"
expr="1"/>` is `scriptEngine.evaluateExpression(...)` followed by
`setVariable`, and the read accessor is
`DataModelReadHelper::readInt(*scriptEngine_, ...)`. An integer literal is
carried in a Lua session.

The template decides this at `scriptengine_helpers.jinja2`: `{% if var.expr
%}` comes first, so any initialiser goes to the engine regardless of type.
`Variable::var_type` (int/string/bool/runtime) already exists but only picks
the accessor's return type -- it does not choose storage.

**Closed when**: a `<data>` whose initialiser is a literal of known type, in
a document that needs no engine for any other reason, is stored natively.
⚠ The condition names the *document*, not the variable: a variable can only
leave the engine session if no surviving expression could reference it.

## §3 `assign-action` -- 206 records, 93 documents, 8 freed alone

`<assign expr>`. The same expression core, in the position where a value is
written back, and last because its lowering depends on the type information
§2 establishes for the destination.

**Closed when**: an `<assign>` whose source expression and destination type
are both decidable at build time lowers natively, and the rest is named.

## A defect in the instrument, found and fixed 2026-09-17

**State-scoped `<data>` initialisers produced no cause at all.** `analyze()`
in `sce-build/src/script_engine_analyzer.rs` passed only `model.variables`
(top level) to `collect_datamodel_causes`, and nothing in that file read
`state.datamodel` -- while `classify_variables` in `analyzer.rs` does walk
it, and codegen initialises it into the engine session.

Found on `integration_resources/send_param_payload/send_param_payload.scxml`:
`<data id="tag" expr="'kept'"/>` inside `<state id="typedPhase">` produced
none of that document's 16 causes.

Fixed by collecting each state's datamodel alongside the document-level one.
`datamodel-variable-init` rose **236 → 246**: 9 documents carry such an
initialiser and one of them carries two.

⚠ **What this did NOT find, stated because the difference matters.** The
defect under-reported, which is the direction that can produce a false green
-- a document whose only engine need is a state-scoped initialiser would have
read as needing none. **No such document exists in this corpus**:
`engine-documents` stayed at 233 across the fix, so every affected document
already needed an engine for another reason. The danger was latent, not
realised, and saying otherwise would overclaim.

The correction did move the programme's own numbers: `transition-guard` alone
fell from 25 documents to 20, because five guard-only documents turned out to
carry a state-scoped initialiser too. The ordering is unchanged and its
margin widened.

⚠ **Still unexplained, recorded rather than rounded away**: top-level
initialisers in judged documents count 240 (223 `expr` + 4 `src` + 13
`content`) against the census's 236 before the fix. Four are unaccounted for.
Duplicate ids, multiple `<datamodel>` blocks, blank `expr` and `id`-less
`<data>` were each checked and are all zero.

## What is deliberately not a section

`elseif-condition` and `unresolved-external-script` are real cause kinds with
**zero instances** in this corpus. No section is written for either, because
a section written against no evidence cannot be measured.

⚠ If either gains a population the census gate reddens, and *that* is the
signal to write the section.

## The tail -- 218 records across 15 kinds

`child-invoke-needs-script-engine` 46, `log-expr` 45, `send-param-expr` 32,
`send-dynamic-attr` 30, `foreach-action` 15, `static-invoke-namelist` 9,
`donedata-param` 9, `donedata-content` 9, `inline-script-action` 7,
`send-namelist` 5, `if-condition` 3, `global-script` 3, `mesh-rpc-srcexpr` 2,
`hybrid-invoke` 2, `cancel-expr` 1.

The tail is not merely small: it is what stops the other 124 engine-needing
documents from being freed by §1-§3. `child-invoke` alone is the sole cause
of 15 documents, `send-dynamic-attr` of 10, `log-expr` of 8.

## What this document does not yet do

**It is not gated.** `sce-build/tests/script_engine_census.rs` ratchets the
counts -- a rise fails until the ledger is edited in the same commit, which
is exactly what happened when the fix above raised one -- but nothing asserts
that a count *falls* on any schedule.

That absence is deliberate. A ratchet on progress needs a schedule, and a
gate that reddens because work is merely slower than a number somebody chose
is a gate that gets bypassed. Choosing that number is a decision with an
owner, and it has not been made.

Until it is: the census measures the population, this document orders the
work, and neither one asserts a rate.
