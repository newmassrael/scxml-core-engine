# ADR 0012 — The ids are started over by retiring them, not by resetting them

- Status: Accepted for the authoring tool (`fresh`, implemented and tested); the owner's way to ask
  for it from the application is not done (see "What this does not do")
- Date: 2026-10-09
- Scope: `tools/authoring/sce_author/requirement_lineage.py` (`advance`), `requirement_set.py`
  (`build`), the `scxml_requirement_set` tool. No change to the lineage's format, its schema, the
  judgment in `sce-revision` or the store
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md` (D2)

## Context

ADR 0011 left D2 open: a way to start a work's lineage over. Its recommendation was none, because
an id that may be issued twice is the defect the lineage exists to prevent.

That is right about resetting and incomplete about the need. A lineage can be wrong without being
unusable: a succession nobody should have believed was made (`continues`, or a near match that was
not one), and from then on an id names a requirement the owner never meant it to. The work then has
no way out. A new work loses the acceptances and the history of the old one.

## Decision

Starting the ids over is a REVISION that retires every id and issues new ones, and the numbering
goes on. It does not reset to R1.

`scxml_requirement_set` takes `fresh: true` together with the lineage (or the previous manifest and
sidecar of a list that predates lineages). Then:

- nothing is carried: not by equal words, not by a near match, and `continues` is refused with it;
- every id the lineage had live is retired, in this revision;
- every requirement is issued a new id from `next`, so the first new id is the one after the
  highest the lineage has ever issued;
- it is a revision of its own even when the text is the same: the old ids are retired IN a
  revision and the new ones issued in it, and one row on which an id is both issued and retired
  would say nothing about when either happened;
- the delta says it as it says any revision: nothing carried, every old id retired, every new id
  new, and a note says the numbering went on and that a design citing the old ids has to be
  written again.

## Why not the alternatives

- **Reset to R1.** It reuses numbers. `R3` in the acceptance the owner made would name one
  requirement, and `R3` in the list the work has now another, with nothing to say so. The
  acceptance records and the revision report both rest on an id meaning one requirement for good.
- **A closing row in the lineage.** It was the design first written for this decision. Reading
  `advance` and `extends` showed it is not needed: a revision in which no quote carries a live id
  already retires them all and numbers on from `next`, and the store's `extends` already accepts
  exactly that (a live id retired, new ids from `next`, the revisions only added to). What was
  missing was a way to ASK for it when the words would otherwise carry ids. Adding a row kind
  would have changed the format, the schema, the judgment in two languages and the shared cases
  to say what the existing rows already say.
- **Nothing.** A wrongly built lineage would stay.

## Consequences

- The design that cited the old ids is outside the new list: its citations are dangling until it
  is written again to cite the new ones. That is what the revision report and the measure say, and
  it is the right thing to say: the ids moved.
- The acceptance the owner made keeps pinning the list it was taken of. After a fresh revision the
  report says every requirement of the accepted list was retired and every one of the list now is
  new, because that is what happened to the ids; it does not claim the words carried.
- Nothing about the stored form changes: a lineage made with `fresh` is an ordinary lineage that
  continues the one it was made from, and `app-core` judges and keeps it as it judges and keeps any.

## What this does not do

- **The owner cannot ask for it from the application.** A list is made by an authoring client,
  so the owner asks the client. Carrying a "fresh ids" request through the application's
  generation (the request, the task text, the form and with them the Codex execution contract,
  which needs a verification with the real client) is a stage of its own.
- **A lineage the store cannot read.** `refuse_a_lineage_not_kept` judges a new lineage on its own
  when the one the work holds is unreadable to this build (a newer version, or damage), so that
  such a work can be saved at all. A lineage given then can number from R1 and reissue an id an
  earlier list of the work carried. Refusing it by comparing with the ids of the kept list
  revisions was considered and not done: a client cannot choose where the numbering starts
  (`next` comes from the lineage it builds against, and there is none it can read), so the work
  would be stranded where it is merely unsafe. The honest repair is a lineage that carries its
  own floor, which is a change of the format and is not made here. Measured: the only way found
  to be in this state is a lineage this build's `check` rejects.
- **How often a lineage is built wrongly is not known.** This exists so that the work has a way out
  when it is, not because it was seen often.

## What was measured

`tools/authoring/tests/test_a_requirement_keeps_its_id_when_its_specification_is_revised.py`,
class `EveryIdCanBeIssuedAfreshWhenTheOwnerAsks` and two tests of the tool (13 cases): the ids and
`next` after it, the delta, the revision row and the retired and first revisions of each id, what
`between` says of it (every old id retired, every new one new, nothing carried), that what it
makes is accepted by `extends` (the store's own judgment), that a later ordinary revision
keeps the new ids and never the old, that it validates against the lineage schema, that it is
refused with `continues` and with a first list, and that a list that predates lineages can be
renumbered too. The 158 tests of the neighbouring files still pass.
