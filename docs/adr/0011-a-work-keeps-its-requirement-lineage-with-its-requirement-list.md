# ADR 0011 — A work keeps its requirement lineage with its requirement list

- Status: Proposed (design only; nothing below is implemented, and two decisions are the
  owner's: see "Open decisions")
- Date: 2026-10-08
- Scope: `app-core` (the requirement list's record, `save_requirements`, `read_requirements`,
  `save_request_candidate`, bundles), the `sce-work` command layer, the authoring MCP's
  `works_*` tools, and a pure `between` / `extends` in `tools/authoring/sce_author/requirement_lineage.py`.
  Step 6 of the chain begun in ADR 0006
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`,
  `docs/adr/0008-an-acceptance-says-what-moved-since-it-was-taken.md`,
  `docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`,
  `app-core/src/store.rs`, `app-core/src/requirements.rs`, `app-core/src/acceptance.rs`

## Context

The lineage (ADR 0006) is what makes an id mean the same requirement from one revision of a
specification to the next: it remembers which id was issued for which requirement, so that an
id is issued once and never again. It is a file the CALLER keeps and hands back
(`lineage_text`) to the next build.

A work in the workbench is the application's own unit: its text, its model, the owner's
answers, its requirement list and its acceptances, five chains in one folder that ONE
implementation writes (`app-core/src/store.rs`). The folder has no place for the lineage. A
client that revises a work reads `manifest_text` and `sidecar_text` from `works_read`, and
those are all it can give back: `previous_manifest` and `previous_sidecar`, the path ADR 0006
calls adoption.

Adoption is the weakest guarantee the chain has, and the workbench is where revision happens.
Measured on 2026-10-08 with three revisions of a three-sentence specification, the second
dropping the last sentence and the third adding a new one:

| What the next build was given back | ids of revision 3 |
|---|---|
| the lineage of the previous build | `R1`, `R2`, **`R4`** |
| the previous manifest and sidecar only (what a work can hand back today) | `R1`, `R2`, **`R3`** |

`R3` was the sentence dropped in revision 2. A design accepted for revision 1 cites `R3` for
that sentence; read against revision 3, the same id now names the new requirement and nothing
says so. This is the failure ADR 0006 exists to prevent, reached by the path owners will use.
Adoption also loses the previous specification's digest and so the sentence delta, and a
client that gives back nothing at all is told "this is the first list as far as this call can
tell" and restarts at `R1`.

### What the folder already offers

- A requirement list is ONE record in the requirements chain: `{format, v, manifest, sidecar}`,
  a save is a compare-and-swap on the revision the writer read, taken under the folder's lock,
  and every revision is kept and can be read by name (`read_requirements(id, revision)`).
- A work is read as one state (`read_work_snapshot`), so the text, the model, the list and the
  acceptance a screen shows belong together.
- An acceptance names the revisions it was taken from (`Basis.requirements`).
- A request for a model is answered through a candidate that carries the list as a stored text,
  and `complete_request` publishes model and list together as a bundle; a work that keeps
  bundles refuses `save_requirements` with `bundled-work`. The request flow is the workbench's
  main path, and any place the lineage lives has to be on it.

## Decision

**The lineage is part of the requirement list's record: the same chain, the same revision, the
same compare-and-swap.**

1. **One record, not a sixth chain and not a file beside the folder.**
   - A sixth chain would be saved in a second step, so a list and its lineage could be saved
     apart and disagree: a list whose ids the lineage never issued, or a lineage that issued
     ids no list uses. They are one fact, "which ids this list uses and which were ever
     issued", and one save has to make it.
   - A file beside the folder is a second place two processes write, which `store.rs` names as
     the way a folder and its copy come to disagree.
   - In the record, the candidate and the bundle carry it without a line of their own: both
     hold the list as a stored text. And an acceptance names the requirements revision it was
     taken from, so the lineage of an acceptance is the lineage in that revision's record,
     exactly, not reconstructed.
2. **Format.** The record gains an optional `lineage`: the text exactly as
   `scxml_requirement_set` returned it, byte for byte, as the manifest and the sidecar are. The
   core checks its shape and not its meaning: a JSON object whose `lineage` member is
   `sce-requirement-lineage`, so a swapped file is refused and what a lineage says remains the
   authoring package's to define. A record with no lineage is written as `v: 1`, byte for byte
   as before, so every digest already kept and every acceptance pinning one is undisturbed. A
   record with a lineage is `v: 2`. A build that predates this refuses a `v: 2` record loudly
   (`deny_unknown_fields`, and a version it does not know) and does not drop the lineage
   silently. The command layer's version moves, as it has for each change of an answer.
3. **A save that would lose the lineage is refused.** Under the folder's lock, a save without a
   lineage onto a head that has one is refused (`lineage-dropped`, naming what to give). There
   is no override: starting over is a new work. A client that forgot the field is told, and
   the ids do not restart.
4. **A save has to extend the lineage it follows.** `extends(previous, next)` in
   `requirement_lineage.py` refuses a lineage of another `doc_id`, a lower `next`, an id of the
   previous that is missing or whose quotes are not a prefix, a retired id that is live again,
   and a revision list that is not a continuation. The authoring tool runs it after reading the
   head and before saving; the compare-and-swap on `base` guarantees the head it checked is the
   head that is replaced. The core does not run it (see "What this does not claim").
5. **The tools carry it.** `works_read` returns `requirements.lineage_text`;
   `works_save_requirements` takes `lineage_text` (and the candidate command takes `lineage`);
   the instructions say to build with `lineage` and `previous_sidecar` from `works_read`.
6. **The words delta of a work is derived from two states of its own chain, not carried.**
   `between(older, newer)` is a pure function of two lineages: an id live in `older` and live in
   `newer` with the same latest quote is `carried`, with another is `changed`, and live in
   `older` and retired in `newer` is `retired`; an id live in `newer` and absent from `older` is
   `new`; one issued and retired in between is not listed (no design could cite it). The two
   states are the requirements revision an acceptance names and the head. Consequences:
   - ADR 0009's refusal of a delta that is not the record's own cannot arise for a work, because
     both sides are the work's;
   - any two revisions can be compared, so the limit ADR 0009 states (a delta of a later step is
     refused and steps are not composed) does not bind a work;
   - the lineage of ONE state plus a revision number would not do: a specification that did not
     change can still be re-quoted, and two quotes of one id then carry the same `rev`.
7. **Works that already have a list.** The first build against a work whose head has no lineage
   passes its manifest and sidecar as the previous list (adoption), and the lineage it makes is
   saved with the list. Ids are kept; the previous specification's digest and the sentence delta
   of that one step are unknown, as ADR 0006 says. From the next revision on the guarantee is
   the full one.

## Open decisions (the owner's)

- **D1. Where the join lives once two surfaces need it.** ADR 0009 put the join of words and
  evidence in Python because it needed no product change, and the workbench's report needs the
  same join while `app-core` is Rust. A second table written in Rust would be two definitions
  of what a revision finding is, and they would drift. The choices: (a) keep one in Python and
  have the application reach it through the authoring package, (b) port it to `app-core`, (c)
  move it into the product as one command (`revision-join`, with its schema registered in
  `SCE_WIRE_CONTRACTS.md`) that the Python tools and `sce-work` both call. **Recommendation: (c).**
  It changes where ADR 0009's table lives and adds a wire surface, so it is not done by this
  ADR. Stages 1 and 2 below do not depend on it.
- **D2. A way to start the lineage over.** Recommendation: none. A specification written again
  from nothing is a new work, and an id that may be issued twice is the defect.

## What this does not claim

- **The lineage is as good as its similarity.** ADR 0006's limits stand: a wrong succession is
  listed `changed`, never `same`, and a very short identifier may be retired and issued again.
- **The core does not understand a lineage.** A direct `sce-work save_requirements` can store
  any lineage object that has the right marker. Only `lineage-dropped` is structural enough
  for the core to enforce; `extends` is the authoring package's, because it owns what a lineage
  means, and two implementations of that would be two definitions.
- **Nothing new leaves the folder.** The lineage holds hashes and no word of the specification;
  the sidecar, which holds sentences, is already kept there.
- **An acceptance still pins the manifest's bytes** and not the lineage; the product never
  reads it, and staging for the product (`spec/requirements.manifest.json`) is unchanged.
- **Not retroactive.** A list saved before this has no lineage until the next build against it
  (decision 7).
- **Two clients revising one work** are serialised by the compare-and-swap on `base`: the second
  save is refused as a `conflict`, reads again and builds again, and the lineage `next` cannot
  be issued twice.

## Stages, and what each is done when

Criteria set before code. Each stage ends with a push; none needs the next.

1. **The record and the commands (`app-core`).** Done when: a list saved with a lineage reads
   back byte for byte, and one saved without is the same bytes (the same digest) as today; a save
   without a lineage onto a head that has one is refused `lineage-dropped`; a candidate carries
   the lineage and `complete_request` publishes it with the list; a core that predates the field
   refuses a `v: 2` record and does not read it as a list without one; the command layer's
   version has moved; the breaks (the field not written, not read, not carried by the candidate,
   the drop not refused) are each caught.
2. **The tools (`tools/authoring`).** Done when: through the real `sce-work`, three revisions of a
   work that hands the client back only what `works_read` gave keep their ids with no
   `lineage_text` passed by hand, and the measurement above gives `R4`, not `R3`; a save whose
   lineage does not extend the head's is refused with the reason; `extends` has a refusal for each
   rule of decision 4 and a break that removes it.
3. **The derived delta and a check by work.** `between`, and a `works_revision_check` that joins
   it with the product's evidence over the staged work (`app-core/src/acceptance.rs` already
   stages a work for the product, `Snapshot::stage`). Done when: the check of a work agrees with
   `scxml_revision_check` given the same two lineages, and a delta of a later step needs no
   composition. The join's home is D1.
4. **The workbench report.** The screen shows the revision report of a work whose acceptance
   lapsed because its text was revised. Depends on D1 and on the screen; not specified here.
