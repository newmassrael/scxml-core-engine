# ADR 0002 — No statechart liveness diagnostics

- Status: Accepted
- Date: 2026-07-12
- Scope: SCXML validation pipeline (`sce-build/src/scxml_*`); `DiagnosticCode` surface
- Related: `docs/SCE_ACCEPTED_SUBSET.md` (Statechart graph reachability / guard analysis), `SCE_ERROR_CONTRACT.md` §5

## Context

SCE already checks a *safety* property on the statechart graph: every
declared `<state>` / `<parallel>` / `<final>` must be reachable from the
initial configuration (`scxml/unreachable-state`, `scxml/dead-transition`),
and dead or masked edges are rejected (`scxml/always-false-guard`,
`scxml/shadowed-transition`).

The open question was whether SCE should also assert a *liveness*
property — that a machine can actually reach its endings (`<final>`
states) and cannot get stuck in a non-terminal configuration. A consumer
integration raised it: could SCE statically guarantee "all declared
endings are reachable, no deadlock"?

An implementation was carried through to completion and reverted; this
ADR records why, so the rejected candidates are not rebuilt.

## Considered alternatives

### Option A — add no liveness diagnostic (chosen)

Rely on the existing reachability + guard-analysis rejections for the
intent-free structural part, and leave ending-intent and behavioural
reachability to the layers above SCE.

**Pros:**

- Sound: every diagnostic SCE already emits rejects a provable defect
  with zero false positives on valid documents.
- No new false-positive surface. Idiomatic SCXML is unaffected.
- Honours the layering: SCE proves structure; the declarative world-line
  layer owns declared endings; a concrete playthrough dry-run owns "does
  this run reach that ending."

**Cons:**

- SCE does not, by itself, answer "can every ending be reached without
  deadlock." That guarantee must come from the caller.

### Option B — `scxml/unreachable-final` (rejected: vacuous)

Flag a `<final>` reachable only through provably-false guards.

`scxml/always-false-guard` already **rejects** any document containing a
provably-false guard anywhere. So no accepted document contains one, the
guard-pruned reach set equals the structural reach set, and the
diagnostic can never fire. It would be dead code.

### Option C — `scxml/dead-end-state` (black-hole / trap) (rejected: unsound)

Flag a reachable, non-`<final>`, atomic leaf with no outgoing transition
on itself or any ancestor — a configuration the machine can enter but
never leave.

Implemented in full (analysis pass, 11-site `DiagnosticCode` integration,
pipeline wiring, tests), refined to require the leaf be *transitioned
into* and to skip leaves beneath a `<parallel>`. A scan of all 290
machines in the W3C static-generation tree found **zero** occurrences,
which looked safe.

The full `cargo test -p sce-build --features cli` suite then rejected ~23
hand-written fixtures. Inspecting one
(`sce-build/tests/fixtures/event_schema/negative_statechart_bytes_ordering.scxml`):

```xml
<state id="waiting">
  <transition event="signal.received" cond="..." target="done"/>
</state>
<state id="done"/>
```

`<state id="done"/>` is the **idiomatic terminal-state pattern** — a plain
state the machine transitions into and stops at. It is ubiquitous in
valid SCXML. The W3C corpus showed zero only because W3C conformance
tests deliberately use `<final>` (which raises `done.state.*` so a harness
can detect pass/fail); ordinary SCXML uses plain terminal states
constantly. A rejection here would reject a large class of valid,
idiomatic machines.

### Option D — universal termination ("every run reaches a `<final>`") (rejected: undecidable + intent-dependent)

- Undecidable in general once datamodel guards are in play (unbounded
  concrete state space).
- Intent-dependent: reactive controllers, servers, and game loops
  legitimately never terminate.
- Under a sound finite-control abstraction the "on all paths" direction is
  almost always unprovable, so the check would be vacuous.

## Decision

Adopt Option A: **SCE adds no new liveness diagnostic.** The intent-free,
decidable, sound structural liveness defects are already rejected by
`scxml/unreachable-state`, `scxml/dead-transition`,
`scxml/always-false-guard`, and `scxml/shadowed-transition`. Ending-intent
and behavioural reachability are intent-dependent or undecidable and are
owned by the layers above SCE.

## Consequences

**Short term**

- No change to the `DiagnosticCode` surface or the validation pipeline.
- Callers that need "all endings reachable / no deadlock" must obtain it
  from a concrete playthrough dry-run or a declarative reachability
  layer, not from SCE compilation.

**Long term**

- A zero-hit corpus scan is necessary but not sufficient evidence that a
  new rejection is sound: the W3C conformance corpus is unusually
  disciplined and can hide a false-positive class that ordinary authoring
  hits immediately. Validate a candidate rejection against the full test
  suite (including `--features cli` integration fixtures), not just a
  corpus scan.

## Revisiting

Reopen if:

1. A declared-terminal marker is introduced (an explicit "this non-final
   state is intentionally terminal" opt-in), which would let a narrowed
   dead-end rule distinguish intent from omission soundly.
2. A consumer supplies an ending-reachability contract (a declared set of
   expected terminal `<final>` states) that SCE could verify structurally
   against, turning an intent-dependent property into a checked contract.
