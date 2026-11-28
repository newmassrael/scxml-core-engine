# ADR 0160 — A landing is judged by every gate that can turn main red, made fast by warm, content-addressed execution, and its budget is the judgment's makespan

- Status: Accepted (2026-11-28), under the owner's standing rule: the most long-term correct, textbook structure,
  cost not a criterion, recommended and decided.
- Amends: ADR 0120 D1 as amended 2026-11-25 (a landing judges the set that fits `PUSH_BUDGET_S`), by saying what
  "fits" means and which way a gate that does not fit is to be brought back.
- Related: ADR 0145 (gates in images), bx RFC 0024 (the landing queue), RFC 0035 (the run's view of the
  machine), RFC 0084 (a verdict kept by the trace of what it read), RFC 0088 (the batch's transfer).

## Context

On 2026-11-27 and 28 the landing queue held items for eight and nine hours. Measured from the integrator's own
records (`bxland-view`):

- Each judgment ran 1,600 to 3,650 seconds and most ended "not measured". The gates read three to ninety times the
  `cost_s` the registry declared (nl-ir-closure 1.3s declared, 117s judged; forge-c11 17s, 453s), because the
  landing runs each unit in a copy of the tree on a build machine and only one machine hands that copy its build
  cache; the declared figures were warm ones from a developer's tree.
- Outside the queue, force-main moved main every 15 to 30 minutes, so the queue judged items again from the start
  before any judgment ended.
- A batch spent 130 to 300 seconds beside its unit sending history (bx fixed it in 27ff9907).

The budget then summed declared costs: 188 seconds on paper against judgments of an hour.

## Decision

**D1. The merge queue keeps main green.** What a landing judges is every gate whose verdict can turn main red on a
platform the build machines run (the "not rocket science" rule: test the merge result before it lands). `ci_only`
is the state of a gate whose landing cost does not fit the budget yet, and the way back is to lower that cost
(D3 to D5), never to raise `PUSH_BUDGET_S`, which is the owner's number. A platform no build machine has (the
Windows lane) stays with hosted CI.

**D2. The budget bounds the judgment's length, which is a makespan.** bx runs a judgment's units at once on the
build machines that take this repository's units, so the judgment ends with its last unit. For m machines and
unit costs c the makespan is at least max(max c, sum c / m), and list scheduling reaches it within one unit
(Graham 1966). `judgment_seconds` in gate_registry.py is that bound plus a batch's measured overhead, with m =
`LANDING_MACHINES` (5) and the overhead `BATCH_OVERHEAD_S` (35s); `budget_is_not_exceeded` and `budget_breach`
both hold the ceiling against it. On today's costs it brings `clippy`, `ecma262-lowered-kotlin` and `w3c-go`
back into the landing (815s of gates, 295s judged); `forge-c11` and `forge-go` stay with CI until D4 lowers them.

**D3. A cost is measured where it is spent.** A landing gate's `cost_s` is the median of its last ten landing runs
in the integrator's `timings.tsv`, and it is read again whenever the landing seat changes (D4, D5).

**D4. The landing seat is warm and the same everywhere.** Every build machine gives a unit's copy the tree's own
path, so the build cache, which records paths, serves the copy (bx RFC 0035, the step that converges the rest of
the machines to it); an image is pulled by its digest from a registry every executor can read; the objects a copy
borrows are visible to the containers it starts. A cold seat turns a warm 17s gate into 453s, and no budget
arithmetic repairs that.

**D5. A verdict is reused by what it read.** A unit whose inputs, by the trace of its last run, have not changed is
not run again (bx RFC 0084 stage 3, after its shadow weeks): a main that moved under an item costs only the units
the move reached.

**D6. One write path.** main moves only through the queue (`bx --land`); force-main was turned off on 2026-11-28
once a judgment took 221s. A second writer is what made the queue starve, whatever its speed.

**D7. Hosted CI has the capacity to audit every push.** Its pool (20 jobs, shared across the account) left a job
waiting a median 13.5 minutes for a 2.6-minute run. An organisation on GitHub Enterprise with larger runners (500
standard concurrent jobs, 1,000 larger) is the remedy; self-hosted runners are not, for a public repository whose
pull requests would run on them.

## Who does what

- In this repository (done 2026-11-28): D2, D3, D6, and `embed-manifest-stamp` out of the image it did not need.
- bx (bx-94): D4's convergence of the copy's view and cache to every machine, the copy's objects to containers, a
  pack sent only as what the machine lacks; D5's stage 3.
- The owner: GHCR packages `sce-gate-host` and `sce-gate-swift` public (D4); the GitHub plan (D7).

## Alternatives considered

- **Keep the sum.** It bounds one machine's serial run, which no judgment is; it excluded gates that fit the
  judgment and said nothing about the hour the judgment really took.
- **Raise the budget.** Keeps main green and the landing slow; the cost was a cold seat, which D4 removes.
- **Judge nothing before landing, audit after.** Main goes red between the landing and the audit, and every
  session builds on that red; the queue exists to prevent it.
