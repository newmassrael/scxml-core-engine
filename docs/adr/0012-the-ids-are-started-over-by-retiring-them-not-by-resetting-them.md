# ADR 0012 — The ids are started over by retiring them, not by resetting them

- Status: Accepted (`fresh`, `next_at_least`, the store's `lineage-numbers-reused`, and the owner's
  ask through the application, `fresh_ids`, implemented and tested). What the live run of a real
  client did and did not measure is under "What was not measured"
- Date: 2026-10-09
- Scope: `tools/authoring/sce_author/requirement_lineage.py` (`advance`, `first`),
  `requirement_set.py` (`build`), the `scxml_requirement_set` tool, `app-core/src/store.rs`
  (`refuse_a_lineage_not_kept`: one refusal for a lineage the work holds and this build cannot
  read; `refuse_a_carried_id`: a list that carries an id the owner asked to retire), the request
  (`Request.fresh_ids`, `request_generation`, command set 23), the task text, and the screen's
  generation controls. No change to the lineage's format or schema, or to the judgment in
  `sce-revision`
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

## The owner's ask, through the application

The screen offers "Issue every requirement a new id" beside the generation button for a work that
already has a requirement list, and the tick is one press's: it is put away when the request is made
and it is only of the work it was ticked on. The request carries it (`Request.fresh_ids`, said back
as `fresh_ids`; `request_generation` takes it, and it is an input of the key, so a press that asked
is not the press that did not), the runner hands it to the generator in the job, and the task text
tells the client to give `fresh` to `scxml_requirement_set`.

The ask is enforced and not only passed on. A client may ignore it (a model that builds the list as
it always has), and publishing that list would say the ask was kept when it was not. Before a
candidate is published, `refuse_a_carried_id` compares the lineage the work holds with the one given
(`between`, the judgment the report uses) and refuses a list that carries an id, or carries one with
other words, as `fresh-ids-not-issued`, naming the ids. The request stays the executor's, which
writes the list again; the runner gives the refusal back to the client as it does every refusal. A
work with no lineage (a first list, or one this build cannot read) has no id to retire, and what
cannot be judged is not refused.

The task text says it only for a request that asked. What every run is told is what a Codex version
is verified against (`codex_support.json`), and `tests/codex_support.rs` held: the execution contract
did not change, so no new verification with the real client was owed. That is a measured fact of the
base task and the same sentence is not claimed for the added one: see below.

## What was not measured

- **A real client building a revision against a held lineage, with and without `fresh`.** The live
  run of a real Codex (2026-10-09) used a synthetic specification and a work that holds no lineage.
  Nothing here shows that a model reads the added sentence and gives `fresh`; what is held is that
  the core refuses the list when it does not, so a client that ignores the ask cannot publish.
- **How often a lineage is built wrongly.** This exists so that the work has a way out when it is,
  not because it was seen often.

## A lineage the store cannot read

`refuse_a_lineage_not_kept` judges a new lineage on its own when the one the work holds is unreadable
to this build (a newer version, or damage), so that such a work can be saved at all. A lineage given
then could number from R1 and reissue an id an earlier list of the work carried. This was first left
open: refusing it would strand a work, because a client cannot choose where the numbering starts
(`next` comes from the lineage it builds against, and there is none it can read).

It is closed in two parts that need each other:

- **The tool can be told where to start.** `scxml_requirement_set` takes `next_at_least`: new ids
  start from it when it is past where the lineage's own numbering would. It never moves an id that
  is carried, and a floor at or below `next` changes nothing. It goes with `fresh` and with a first
  list.
- **The store refuses a lineage that numbers from an id the work already carried**
  (`lineage-numbers-reused`), only when the held lineage cannot be read. The highest id is read from
  every list revision the work kept (they are the files of one folder, whichever chain or bundle
  kept them: the manifest's ids, and the lineage's ids and `next`). The refusal says the number
  (`detail.floor`, and the message names `next_at_least`), so the client rebuilds once and the work
  is not stranded.

Not done, and why: a work that never had a lineage at all is not held to the ids of its lists. With
no lineage held, nothing says which of those ids were retired or live, so the first lineage is judged
on its own (the weaker guarantee ADR 0011 names), and a test pins that boundary. A file that cannot
be read as a list is not counted: it is damage that a read names, not something this save refuses
for.

## What was measured

`tools/authoring/tests/test_a_requirement_keeps_its_id_when_its_specification_is_revised.py`,
class `EveryIdCanBeIssuedAfreshWhenTheOwnerAsks` and two tests of the tool (13 cases): the ids and
`next` after it, the delta, the revision row and the retired and first revisions of each id, what
`between` says of it (every old id retired, every new one new, nothing carried), that what it
makes is accepted by `extends` (the store's own judgment), that a later ordinary revision
keeps the new ids and never the old, that it validates against the lineage schema, that it is
refused with `continues` and with a first list, and that a list that predates lineages can be
renumbered too. The 158 tests of the neighbouring files still pass.
