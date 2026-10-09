# ADR 0013 — A revision is held to its reach by a loop its caller runs

- Status: Accepted (`revision_gate.hold`, `judge`, `command_reviser` and the `revise-gate` command
  implemented and tested on judgments made by `revision.join` and on the committed design with the
  product's generator). What a live client does under it is under "What was not measured"
- Date: 2026-10-09
- Scope: `tools/authoring/sce_author/revision_gate.py`, the `revise-gate` command of
  `sce_author/__main__.py`, and `scxml_revision_check` / `scxml_revision_report`, which now read
  the judgment through `revision_gate.judge`. No change to the judgment (`revision.join`,
  `sce-revision`), to the shared cases that hold the two to each other, or to any wire surface
- Related: `docs/adr/0009-a-revision-stays-within-the-reach-of-what-changed.md`

## Context

ADR 0009 gives a revision a verdict: `within-reach` when the design's changes are accounted for by
the specification's, `outside-reach` when a requirement whose words did not change moved, or is
still cited though the specification dropped it. Asking a model to "change only what the delta says
moved" is how a revision is meant to stay there. It is a request, and nothing holds the model to it:
the model that revises runs in the owner's client, and nothing in the product refuses a design for
a verdict. The verdict reaches the owner after the fact, as a report.

What can be held is the result. A revision that is outside its reach is found after it is made, and
the places it moved at are exactly what is needed to ask for it to be put back. A caller that
revises with a model can therefore judge, hand the violations back, and judge again, and call the
revision within reach only when the judgment of the design as it stands says so.

Where the loop can live was the question. It cannot be a tool the model calls: the model decides
whether to call it, which is the failure. It cannot be in the product's publishing of a work
without judging a draft before it is published, which `read_revision_report` does not do (it
reads what the work holds), and that is a change to the command layer of its own. It can be a
function and a command that the caller runs around its client, which needs no change to either.

## Decision

**`revision_gate.hold(judging, revise, rounds=3)` judges the design, and while the verdict is
`outside-reach` hands the violations to `revise` and judges again, up to `rounds` times
(at most 10). It ends in one of five ways, and only one of them is a pass.**

1. **`within-reach`**: the judgment made after the last revision saw evidence (`summary.seen` above
   0) and found no violation.
2. **`outside-reach`**: the rounds are used up and the last judgment still has violations.
3. **`stalled`**: a revision left the violations exactly as they were, places included, so the same
   request would be answered the same way. A revision that put back two of three places is not
   a stall.
4. **`not-judged`**: a judgment that said nothing. `summary.seen` 0 means no requirement was compared
   (no node cites one, or the kind of document has nowhere to), and its verdict, which reads
   `within-reach` because there is no violation to find, says nothing about the design; or the
   product refused to judge (a design it cannot read, a record taken under an older rule).
5. **`reviser-failed`**: the reviser could not do its round (`ReviserError`).

The request handed to the reviser names each violation with the places it moved at and what to do
about it (put back what the design had; or remove the id of a requirement the specification dropped
from every `sce:req` that cites it), then the page `scxml_revision_report` renders, so that what the
reviser reads and what the owner is shown cannot differ. It says that the reviser's own report of
being finished is not read.

`revise-gate` is the same loop with a command as the reviser: any client runner that edits the
design it is told of (`SCE_REVISION_DESIGN`) from a request in a file (`SCE_REVISION_REQUEST`). The
command is run without a shell. Its exit status is the gate: 0 only for `within-reach`, 1 for
`outside-reach` and `stalled`, 2 for `not-judged` and `reviser-failed`. The design is edited in place,
so a caller gives a copy; the accepted design is never touched.

`revision_gate.judge` is the one place the words delta, the acceptance record and the product's
evidence are joined into a judgment; the tools read it through that, so the loop and the report
cannot disagree about what was judged.

**The baseline is held, and what a timeout stops is the reviser's whole group.** Two ways a reviser
could pass a design it had not put right were found in a review of the first version
(2026-10-09, each reproduced before it was fixed):

- *The record is the baseline, and a reviser that can write to it chooses it.* A reviser told where
  the acceptance record lies replaced it with the record of the design it had left broken, and the
  gate answered `within-reach`. The record is now read once (`keep_baseline`), every judgment reads
  a private read-only copy of those bytes, and `guarded` fails a round after which the record on
  disk (or its absence) is not the bytes that were hashed (`reviser-failed`, in the words of what
  moved). Reading the copy also ends a smaller defect: one judgment read the record twice, so a swap
  between the two reads joined the evidence of one record with the manifest of another.
- *A timeout that stops the command and not what it started.* `subprocess.run` kills the direct
  child, and a client runner starts programs. One of them wrote to the design after the gate had said
  the round was over. The reviser is now started in a session of its own (`process.run`,
  `own_group`) and, on a timeout, the whole group is killed while its leader is still uncollected.

## What this does not claim

- **It is not an enforcement inside the client.** A model that edits more than it was told to within
  one round is found after the round and asked to put it back. Restraining the edit itself would
  take an editing tool that refuses writes outside the elements the changed requirements cite, and
  is not built.
- **`within-reach` is not "right".** It says the design's changes are accounted for by the words', on
  the product's closure of what a requirement depends on (ADR 0007, ADR 0009). A requirement that
  moves only where a changed neighbour also stands is a `look`, not a violation, and the gate does
  not hold it: the owner still reads it.
- **It does not judge a draft before a work publishes it.** The judgment of a work reads what the
  work holds, so a client that finishes a generation is judged after, not before. Judging the draft
  would be a command of the application's core and is not part of this.
- **The rounds cost what a call to a model costs.** The bound exists so the tenth is not paid for.
- **It is not a sandbox for the reviser.** The reviser runs with the caller's own rights. The record
  it was told of is guarded, and the copy the judgment reads is read-only, but a reviser that went
  looking for that copy in a temporary folder and changed its mode could still move the baseline: no
  gate run with the reviser's own privileges can stop that, and none is claimed. The design the
  reviser is told to edit is the one thing it may change.
- **A program the reviser leaves running after it EXITS is not stopped.** Only a timeout ends a
  group, because only then is the leader still uncollected and its number still its own; after it
  has been collected the number belongs to nobody, and a signal to it could reach a stranger
  (`process.Session.end` says the same). A design edited by such a program after the gate's last
  judgment is not the design that was judged.

## What was measured

On the committed design and the product's own generator, with an edit that renames a transition a
requirement cites while every requirement's words are carried (a violation):

1. **Met.** A reviser that puts the design back is within reach after one revision, and is told the
   requirement, `moved-without-reason`, and the product's own places.
2. **Met.** A reviser that only says it is done leaves the design outside its reach and the gate
   `stalled`: the report is not read.
3. **Met.** A design the product cannot read is `not-judged`, with the product's sentence.
4. **Met.** The command line exits 0 for a reviser that puts the design back, 1 for one that
   changes nothing and 2 for one that exits non-zero; `--out` carries how it ended.
5. **Met.** The judgment `judge` returns is the one `scxml_revision_check` answers.
6. **Met.** A reviser that replaces the acceptance record with the record of the design it left broken
   fails its round, through the guard and through the command line (exit 2), and the judgment of the
   private copy is unmoved by a replaced source. A record the reviser removed is a record that moved.
7. **Met.** A program the reviser started is gone after a timeout and writes nothing to the design;
   a command that finishes is read as before.

Each of the fixes in 6 and 7 was put to the tests by breaking it (the session left to the caller's,
the group kill reduced to a kill of the command, the guard left out of the command line, the
moved-record check made blind, the private copy left writable) and each was caught.

On judgments made by `revision.join`: every way of ending above is reached on purpose (already within
reach, one revision, several revisions with fewer places each time, nothing changed, the rounds used
up, no rounds, a verdict that saw nothing, a refused judgment before and after a revision, a failing
reviser, a number of rounds that is not one), and the request names a retired requirement's
citations.

## What was not measured

A live client under the gate. The loop was run with revisers that are functions and commands, never
with a model asked to revise and told what it broke. What a model does with the request (puts back
what it was told to, puts back more, or edits somewhere else) is the next measurement, and the one
that says whether the bound of three rounds is the right one.
