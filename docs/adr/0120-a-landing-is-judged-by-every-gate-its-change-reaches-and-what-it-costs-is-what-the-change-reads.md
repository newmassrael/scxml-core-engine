# ADR 0120 — A landing is judged by every gate its change reaches, and what it costs is what the change reads

- Status: Accepted (2026-11-23, by the owner), as the direction; amended 2026-11-25 by the owner. D1 is replaced and
  D6's culprit finding and revert are withdrawn (see "Amendment, 2026-11-25" at the end); D5 is the after-landing tier
  of every `ci_only` gate. D2 to D4 stand and are built in the order of "Landing" below, each in a landing of its own.
- Date: 2026-11-23
- Written under the owner's standing rule (the most long-term correct structure, textbook in its foundations, cost
  ignored). The owner had asked whether every gate is needed and how to make a landing shorter.
- Related: bx RFC 0020 (a check's verdict is made once in the fleet), bx RFC 0024 (the landing queue), bx RFC 0026 (an
  isolated unit's copy is a repository of its inputs), bx debt 119 (parallel judging of independent items, requested
  2026-11-23). Registry: `tools/git-hooks/gate_registry.py`.

## Context

Measured on 2026-11-23:

- **Two ways of judging.** `main` judges a push with the gates that fit `PUSH_BUDGET_S = 300` and leaves the rest,
  marked `ci_only`, to hosted CI. The median landing round under that regime is 897 s (`bx --land-queue --stats`:
  rho 0.30, 1.08 items a push). Item 395 has the judgment of a `main` update select every gate its change reaches,
  `ci_only` included. It judges itself that way, and it had been judging at the head of the queue for more than
  3 h, with five items behind it. One of those was a one-file compiler fix.
- **Hosted CI does not finish what it is handed.** The C++ engine job reaches its 140 min limit, C11 W3C was cut at
  its 25 min limit four runs in a row, and a Rust shard failed because its runner lost contact. On those lanes,
  `ci_only` means a gate no machine finishes.
- **The verdict cache exists and seldom hits a landing.** Each of the 57 gates on the 395 branch states whether its
  pass may be kept (`cache`), and `[checks.prepush]` sets `cache = true`. Every gate's `inputs` is `None`, the
  whole tracked tree, so any rebase is a new key for every unit. `main` moves between any two landings, so a
  landing almost never meets a kept pass. The registry gives the reason: the isolated copy held no `.git`, and the
  runner asks git for the root. `narrowed-units-need-a-git-free-root` refuses a narrowed list "until that changes".
  It has changed: bx RFC 0026 (implemented, bx `80cdd0de`) makes the copy a one-commit repository of the inputs and
  links the tree's build caches into it.
- **Which gates a change reaches is written by hand.** A gate's triggers are path patterns in the registry. A missing
  pattern lets a change through unjudged. An over-wide one runs a gate the change cannot affect. Both have happened.
  Item 356 was returned because four workflows did not name `backends/rust/forge-runtime` after `main` made it a
  build input. An unclassified document under `docs/` used to select every gate.
- **The unit of work is a whole lane.** The heaviest units build and run a whole tree: `cpp-suite-wasm` 3328 s,
  `workspace-tests` 1363 s, `mutation-rounds` 877 s.

## The textbook structure

- **Not Rocket Science Rule** (Graydon Hoare, 2014; bors): `main` only ever moves to a tree that passed the required
  set, judged on the exact result of the merge.
- **Selection from the build graph, results by content** (Google TAP; Bazel's action and test caches): a test runs
  when something in the closure it reads changed. Its result is kept under the hash of that closure. "Run everything"
  then costs what the change touched.
- **Hermeticity is checked, not assumed**: a read outside the declared closure makes the result unknown, never a pass.
  bx RFC 0020 §5.2 does this with `strace`.
- **A merge queue judges independent changes in parallel and speculatively, and bisects a failed batch** (Zuul;
  Ananthanarayanan et al., "Keeping master green at scale", EuroSys 2019).
- **Two tiers.** Deterministic, hermetic checks run before merge. Nondeterministic or exploratory ones (fuzzing,
  mutation analysis, performance, hardware the fleet lacks) run after merge, with automatic culprit finding and
  rollback.

## Decision

### D1. The landing judgment is the required set

A landing is judged by every gate its change reaches, `ci_only` included (item 395). Making the judgment shorter by
moving gates out of it is rejected. A gate leaves the judgment only for the after-landing tier (D6), and only for that
tier's reasons. `PUSH_BUDGET_S` keeps bounding a push of a branch that is not `main`, a developer's early signal,
which judges nothing.

### D2. A gate's inputs are derived from the build, and they decide whether it runs

Each gate's `inputs` is the closure of the tracked files its build and run read. It is derived from the build systems
that know it: cargo's dependency information for Rust units, CMake/Ninja dependency files for C and C++, and the
generator's own import closure for generated trees. It is checked against what the run actually read, through bx's
trace (RFC 0020 §5.2): a read outside the closure ends "could not measure", which never lands. A derivation that misses
a file therefore stalls a landing. It cannot pass one.

Path triggers are retired. The plan names every gate on every `main` update, and the kept-pass lookup over each
gate's narrowed key decides what runs. A gate whose closure the change did not touch is a kept pass and costs a lookup.
`narrowed-units-need-a-git-free-root` is retired with the first narrowed gate.

A gate that reads history (`commits = HEAD` or `CHANGES`) cannot run in an isolated copy (RFC 0026 §4.4). Such a
gate is split into a part that reads the tree, narrowed, and a part that reads the commits, which is kept by the
commit ids in its argv as it is today.

### D3. The unit is a shard of test targets, not a lane

Each gate whose unit is a whole tree is split into shards of test targets, each with its own derived closure. Two
assertions hold the split: the shards are disjoint, and their union is the registered set. A change to one engine's
code then rebuilds and reruns that engine's shards. The rest are kept passes.

### D4. The queue judges independent items in parallel

Owned by bx (debt 119, RFC on branch `rfc-land-independent-items`). From this repository it needs only what D2 and
D3 already give: each item's reached units and their closures, from which bx can decide independence.

### D5. Hosted CI is an after-landing audit of the same definitions

A workflow runs a gate through `scripts/gate`, as the fleet does. It is sized to fit its runner, by the shards of
D3. Its red result opens a repair item. It does not decide a landing.

### D6. The after-landing tier holds what is not deterministic or not available

`mutation-rounds` (already `audited_after_landing`), fuzzing, performance, and lanes on platforms the fleet does not
have (`windows-msvc`). Each tier entry names why it cannot be judged before landing. A red result there starts culprit
finding over the landed range, and the culprit's revert is submitted to the queue as an item.

## Landing

1. Item 395 lands (D1).
2. Retire the guard and narrow the cheapest tree-reading gates first, with `inputs` learned from a traced run and
   checked by the trace on every run (D2). Each narrowed gate is one landing, with its measured hit rate over the next
   landings recorded in the registry.
3. Derive the closures from the build systems in place of learned lists, beginning with the Rust workspace (D2).
4. Shard `workspace-tests`, `cpp-suite`, `cpp-suite-wasm` and the W3C lanes (D3), and fit the hosted workflows to
   the shards (D5).
5. Retire path triggers once every gate on a `main` update has a derived closure (D2).
6. Adopt bx's parallel judging when debt 119 lands (D4).

## Consequences

- A small change lands in about the time its own closure takes to judge, and `main` stays green on every gate that
  can be judged before landing.
- The first landing after each narrowing pays a full run per gate. Hits come after that.
- Residue: a tool that `bx.toml` names only by `needs` or `packages` is whatever version a machine carries, and its
  version is not part of the key. bx's one-in-sixteen resample (RFC 0020 §9) is the instrument that finds a verdict
  that moved with it.
- Residue: a build cache linked into the copy is not in the key (RFC 0026 §3.4). A build tool whose cached output
  changes a verdict is found by the resample, which runs without the links.

## Amendment, 2026-11-25: before landing, only what is fast and deterministic

The owner's decision of 2026-11-25, in their words as translated: "before landing, only fast, deterministic tests that
finish in a few minutes; no automatic revert; a red found after landing is repaid by a follow-up commit."

Measured: with D1 in force, a judgment of every gate a change reaches took about 1.5 h on the fleet, and the queue held
every item behind the one it judged for that long.

- **D1 is replaced.** The landing judgment is the set any push selects: every gate the change reaches less the
  `ci_only` ones, the set `PUSH_BUDGET_S` (300 s) bounds and `budget_is_not_exceeded` holds to that bound. Its units
  run with `--landing`, so a gate that would skip for want of a tool ends 3, could not run, and is never counted as a
  pass (`kept_only_from_a_landing`, SCE_GATE_LANDING). The registry has one selection for every push (`select`,
  `drop_ci_only` in `tools/git-hooks/gate_registry.py`); `--landing` changes how the units run and has the plan name
  the gates it left, never which gates it selects. `PREPUSH_FULL=1` on an update to main (`--all --landing`) stays the
  deliberate full run on the fleet, less the gates it cannot place or bound (`platform`, `audited_after_landing`).
- **D5 becomes the after-landing tier for every `ci_only` gate.** Hosted CI runs them on main after the landing. The
  next round reads that run first, and a red there is repaid by a follow-up commit that goes through the queue like any
  other item. `--ci-owed` names, for each change, the gates that audit judges and the workflow that owns each.
- **D6's culprit finding and revert are withdrawn.** Nothing is reverted automatically. A red after landing is
  repaired, not rolled back. The gates D6 named (mutation analysis, fuzzing, performance, platforms the fleet lacks) are
  in the tier of D5 with every other `ci_only` gate.
- **D2 to D4 stand**: inputs derived from the build and kept passes over narrowed keys (D2), shards of test targets
  (D3), and the queue judging independent items in parallel (D4). The instrument for D2 is bx RFC 0084, adopted on bx
  main at `bf32b9b3`: a verdict is kept by the verifying trace of what its run read, with early cutoff. Each of them now
  shortens the before-landing set and lets more gates leave `ci_only` within the budget, rather than shortening a
  judgment of every reached gate.

What this trades, stated: a `ci_only` gate's red can land on main and stay there until the next round repays it. The
first round after a landing reads hosted CI's run of main before it starts other work, so a red is not left unread.
