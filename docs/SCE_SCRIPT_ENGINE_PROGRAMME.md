# SCE script-engine programme

ADR 0003 decided that SCE compiles one portable document into each backend's
own language, and that a runtime script engine is the **fallback** for what
cannot be decided at build time. That decision implies work: every construct
that reaches for an engine today is a candidate for being decided at build
time instead.

This document is the **order** that work is taken in, and the order was not
chosen. It is read off `docs/SCE_SCRIPT_ENGINE_CENSUS.md`, which counts why
each tracked document needs an engine. Nothing here is a preference.

## The denominator, stated first

Every share below is taken over **475 judged documents**, not the 736 the
sweep walks. The difference is documents the census cannot judge -- 187 forge
documents (stateless by construction), 30 templates and non-statecharts, and
44 deliberate negative fixtures -- and the census records all of them.

**233 of those 475 need an engine: 49%.** Moving that number down is what
this programme is for. The remaining 51% already compile without one, which
is what makes "the engine is a fallback" a description of the tree rather
than an aspiration.

## The order

| § | Cause kind | Records | Share of 870 |
|---|---|---|---|
| §1 | `datamodel-variable-init` | 236 | 27% |
| §2 | `transition-guard` | 210 | 24% |
| §3 | `assign-action` | 206 | 24% |
| — | the remaining 15 kinds | 218 | 25% |

The first three carry **75% of all 870 records** and are the same thing seen
three times: the ECMAScript expression core, evaluated in three positions.
A programme that paid off the long tail first would spend its effort where
three quarters of the population is not.

## §1 `datamodel-variable-init` -- 236 records

`<data>` initialisers. The largest single cause, and the one furthest from
runtime: an initialiser's value is needed once, at the point the data model
is created, and a great many of them are literals or expressions over
literals.

**Closed when**: every `<data expr>` whose value is decidable at build time
is decided there, and the remainder is documented as genuinely dynamic rather
than merely unhandled. ⚠ The condition is deliberately not "the count reaches
zero" -- an initialiser reading `_event` or a host value cannot be folded,
and a target nobody can reach is a target nobody measures against.

## §2 `transition-guard` -- 210 records

`<transition cond>`. Four classes already need no engine today and are the
proof that this section is tractable: a `cpp:`/`kt:` native guard, a pure
`In()` predicate, a constant-folded expression, and an EventSchema-typed
`_event.data` guard. `check_expression_needs` in `sce-build/src/parser.rs` is
the single classifier that decides which.

**Closed when**: the classifier admits every guard shape the accepted subset
allows, and each refusal names the construct it could not decide rather than
falling through to the engine silently.

⚠ The native-prefix escape hatch is **not** the plan for this section. The
census carries its use in its own column (1 document today) precisely so that
an escape hatch cannot quietly become the path.

## §3 `assign-action` -- 206 records

`<assign expr>`. The same expression core as §1 and §2, in the position where
a value is written back. It sits third not because it is smaller -- it is
within four records of §2 -- but because its lowering depends on the type
information §1 establishes for the destination.

**Closed when**: an `<assign>` whose source expression and destination type
are both decidable at build time lowers natively, and the rest is named.

## What is deliberately not a section

`elseif-condition` and `unresolved-external-script` are real cause kinds with
**zero instances** in this corpus. No section is written for either, because
a section written against no evidence cannot be measured, and the census
would say nothing about whether it had been finished.

⚠ If either gains a population the census gate reddens, and *that* is the
signal to write the section -- not this document being edited to anticipate
one.

## The tail -- 218 records across 15 kinds

`child-invoke-needs-script-engine` 46, `log-expr` 45, `send-param-expr` 32,
`send-dynamic-attr` 30, `foreach-action` 15, `static-invoke-namelist` 9,
`donedata-param` 9, `donedata-content` 9, `inline-script-action` 7,
`send-namelist` 5, `if-condition` 3, `global-script` 3, `mesh-rpc-srcexpr` 2,
`hybrid-invoke` 2, `cancel-expr` 1.

Several of these are the expression core again in another position
(`send-param-expr`, `log-expr`, `if-condition`), so §1-§3 will move them
without being aimed at them. That is a prediction this document makes and the
census can check: if the tail does not fall while the head does, the shared
core was not as shared as this paragraph assumes.

## What this document does not yet do

**It is not gated.** `sce-build/tests/script_engine_census.rs` ratchets the
counts -- a rise fails until the ledger is edited in the same commit -- but
nothing asserts that a count *falls* on any schedule.

That absence is deliberate and is a decision left open rather than an
oversight. A programme ledger arguably should redden when work stalls, which
is the failure mode this repository knows well: a list that only ever grows
is one nobody reads. But a ratchet on progress needs a schedule, and a gate
that goes red because work is merely slower than a number somebody chose is a
gate that gets bypassed. Choosing that number is a decision with an owner,
and it has not been made.

Until it is, the honest description is: the census measures the population,
this document orders the work, and neither one asserts a rate.
