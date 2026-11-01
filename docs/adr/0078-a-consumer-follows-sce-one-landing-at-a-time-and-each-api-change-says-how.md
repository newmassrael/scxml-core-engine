# ADR 0078 — A consumer follows SCE one landing at a time, and each API change says how

- Status: Accepted (2026-11-01). The owner ordered the work started on the recommendation of that day, under the rule
  of 2026-10-26 and 2026-10-27: the most long-term correct structure, textbook in its foundations, cost ignored.
- Date: 2026-11-01
- Related:
  - ADR 0065 (each refinement step changes the generated API and the declarations a document may make);
  - ADR 0077 (a decode fix a consumer asked for, which it could not take without the steps before it);
  - `docs/SCE_CODEC_CONSUMER_MIGRATION.md` (the guide this ADR makes a rule);
  - the rule that no consumer is named in this repository (consumers are called by their role).
- Scope: the migration guide and the gate that holds it, the check-before-write contract of `sce-codegen check`, and
  what this repository expects of a consumer. The consumer's own automation is in the consumer's repository.

## Context

A downstream consumer pins SCE at a commit and regenerates its codecs from it. On 2026-11-01 it asked for the decode
fix of ADR 0077. Its pin was the landing of ADR 0065 R3; between that pin and the fix, five landings had changed the
generated codec API:

- R2: lists became `Bounded`;
- R4a: a flag input the value says left `encode`'s arguments;
- R4b and R4c: gated fields became groups;
- R7: an entry-flag chain's first entry became required.

Each was announced to the consumer at its landing. The consumer chose to move once after R4, and then did not move.

Moving to the fix therefore meant moving across all five:

- 14 declarations had become untrue, and the build refused them one at a time;
- the consumer's lowest two crates reported 192 errors of five unrelated kinds, in files other work was editing;
- the consumer's generation tool had written 13 files before the first refusal stopped it, leaving a tree that was
  half one pin and half the other.

The consumer then asked for a branch carrying the fix on its old pin. That is a second SCE line that no one
maintains. Tests passed on it would say nothing about `main`, and the five steps of debt would remain and keep
growing.

Two facts caused this, and neither was the decode defect:

1. Nothing told the consumer, at a landing, what that landing changed in its code. The facts were in the ADRs,
   mixed with the design.
2. Nothing moved the consumer while the distance was one landing. Integration debt grows with distance, and the cost
   of crossing it grows faster than the distance, because the errors of different changes arrive mixed.

## The textbook structure

Large codebases that consume each other by pinned commit solve this the same way:

- **A dependency autoroller.** Automation in the consumer moves the pin forward one upstream change at a time,
  builds, and either lands the move or stops at the change that breaks and reports it.
- **Migration notes.** The producer writes, with each change, the note a consumer needs to follow it. This is the
  changelog of semantic versioning, written per change instead of per release, since there are no releases.

Moving one change at a time keeps each failure attributable to one change, which is what bisection buys a debugger.
The notes make each failure fixable from one place.

## Decision

### 1. Each landing that changes what a consumer compiles against says how to follow it

`docs/SCE_CODEC_CONSUMER_MIGRATION.md` holds one section for each landing that changes:

- the generated codec API of any backend;
- the codec runtime's public API;
- what a decode accepts or refuses;
- or which declarations a document may make.

Each section names the landing's commits, what stops compiling or behaves differently, and how to write it now. The
section is written in the same commit as the change. The consumer is named by its role, never by its name.

### 2. A gate holds the rule

A commit that changes:

- a public item of a generated codec golden (`tests/forge/expected/**`);
- a public item of a codec runtime (`backends/*/forge-runtime`) or of a mesh library (`backends/*/mesh`);
- or a reject vector of the codec oracle;

must also change the migration guide. Otherwise it is refused, with the paths that made it consumer-visible. A
change whose trigger is not visible to a consumer states that in a commit message with a bullet,
`- No consumer-visible change: <why>`. The gate reads the bullet; it is the author's claim, and the reviewer checks it.

*Amended while landing.* The gate judges the push, not each commit (`tools/git-hooks/consumer-guide-gate.sh`,
sourced by `pre-push`). A landing here is a `feat` commit and the `chore` commit that regenerates its goldens, and
the section belongs to the `feat`. Judged commit by commit, the `chore` would be refused for a section its landing
already has. What counts as consumer-visible (`tools/git-hooks/consumer_visible_change.py`):

- a public declaration REMOVED or rewritten, since an addition breaks no consumer;
- in a golden that already existed, since a golden added or removed whole is a fixture of SCE's own tests;
- in a codec runtime's or a mesh library's source outside its tests, the mesh library being the client trait and
  conformance kit a consumer implements and runs;
- or a removed `"error"` line of an oracle reject.

The predicates read one line at a time and are a tripwire, not a parser; the waiver bullet covers what they
mistake.

### 3. Generation validates before it writes

- `sce-codegen check` reaches the verdict `generate` would and writes nothing. This is the documented way to know
  that a pin move will generate before writing a file.
- SCE adds a library entry point that checks a set of documents and returns every diagnostic of every document. A
  consumer's generation tool can then refuse a pin move whole, naming every refusal at once instead of one per run.

### 4. What this repository expects of a consumer

A consumer moves its pin one landing at a time, in order, and treats the first landing that does not build as the
one to fix. Running that is the consumer's: an autoroller in its own repository, on its own machines or CI. This
repository does not list consumers, so it cannot run their builds. A consumer that stops at a landing tells SCE which
section did not describe what it met, and that section is corrected.

SCE does not keep a branch for a consumer's old pin. A fix a consumer needs is taken by moving to it.

## Alternatives

- **A backport branch for the consumer's pin.** Refused for the reasons in the Context: an unmaintained second line,
  tests that say nothing about `main`, and debt that grows.
- **A producer-side canary that builds each consumer on every SCE commit** (the crater model). This is the strongest
  form, but it puts a list of consumers and access to their private repositories inside a public repository's
  automation. It is refused for that reason. The autoroller gives the same early signal from the consumer's side.
- **Deprecation windows, the old API kept beside the new for some landings.** For generated code this means
  generating both shapes of every changed codec. It doubles the surface that the refinement steps exist to shrink,
  and leaves a consumer's move unforced until the window closes. Refused.

## Order of work

1. The migration guide, covering every landing from ADR 0065 step 1 to ADR 0077 step 1, with this ADR.
2. The gate of §2.
3. The library entry point of §3.
4. The consumer is told the rule and the guide. Its autoroller is its own work.
