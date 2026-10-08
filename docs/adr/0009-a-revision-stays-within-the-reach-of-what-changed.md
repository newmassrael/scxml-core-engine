# ADR 0009 — A revision stays within the reach of what changed

- Status: Accepted for steps 4 and 5 (implemented and tested, see "What steps 4 and 5 measured")
- Date: 2026-10-08
- Scope: `tools/authoring` (`sce_author/revision.py`, the MCP tools `scxml_revision_check` and
  `scxml_revision_report`, and what `scxml_accepted_for` hands back when an acceptance lapsed).
  Steps 4 and 5 of the chain begun in ADR 0006
- Related: `docs/adr/0006-a-requirement-keeps-its-id-across-a-revision.md`,
  `docs/adr/0007-an-acceptance-keeps-what-each-requirement-rests-on.md`,
  `docs/adr/0008-an-acceptance-says-what-moved-since-it-was-taken.md`

## Context

The chain so far gives a revision two separate facts about every requirement:

- **what happened to its WORDS** (ADR 0006): carried (the same), changed, new or retired, from the
  lineage the requirement list is built against;
- **what happened to its EVIDENCE in the design** (ADR 0008): `unchanged`, `changed`, `new` or
  `dropped`, from the acceptance record against the design now.

Neither fact alone says whether a revision is sound. A requirement whose words did not change and
whose evidence did is a design that moved with nothing asking it to. A requirement whose words
changed and whose evidence did not is a design that may not have heard. Joined by requirement id,
the two facts are the whole of what an owner needs to look at again, and what they do NOT need to.

Today, when an acceptance lapses because the specification was revised, `scxml_accepted_for`
answers "write a draft", and the next draft is written afresh. The accepted design is still on disk
and is the right place to start.

### Where the join can live

The words are the authoring side's (the lineage is a Python artifact), and the evidence is the
product's, already reachable through `scxml_acceptance_delta`. The join takes two JSON objects and
returns one, and needs no product change: it lives in the authoring tool, as a pure function. The
product stays the only reader of the design and the record.

## Decision

**`scxml_revision_check` joins the two facts per requirement and says whether the revision stayed
within its reach. `scxml_revision_report` renders the same join as the page an owner reads, showing
only what to look at again.**

1. **The join, per requirement id, over the union of both sides.**

   | words | evidence | kind | severity |
   |---|---|---|---|
   | carried | unchanged | `carries-over` | ok |
   | carried | none (no node cites it, before or now) | `uncited` | uncovered |
   | carried | changed or dropped | `moved-without-reason` | **violation** |
   | carried | changed, only where a changed or new requirement also stands | `moved-with-a-changed-neighbour` | look |
   | carried | new | `newly-cited` | look |
   | changed | changed | `revised` | ok |
   | changed | unchanged | `words-changed-design-same` | look |
   | changed | dropped or none | `changed-but-uncited` | look |
   | new | new or changed | `implemented-new` | ok |
   | new | otherwise | `unimplemented-new` | look |
   | retired | none or dropped | `retired-cleanly` | ok |
   | retired | unchanged, changed or new | `retired-still-cited` | **violation** |
   | not listed | any | `unlisted` | look |

   Rows that claim no requirement and are new to the design (`unclaimed.added`) are one more
   `look`: behaviour no sentence asked for.

   ⚠ `uncited` is not `carries-over`. The first version of this table folded a carried requirement
   that no node cites into `carries-over`, and the page then said the design had been checked
   against it. A review found it the same day (2026-10-08): there is no evidence on either side, so
   nothing was compared. `uncovered` is a severity of its own: it never changes the verdict, it is
   counted apart (`summary.uncovered`), listed apart, and left out of the line of requirements that
   carry over. `summary.seen` is the number of requirements the check saw evidence for. For a kind
   of document with nowhere to cite a requirement (the product's `NoAnnotationSite`) every carried
   requirement is `uncited` and `seen` is 0, and the page opens by saying the check compared
   nothing, so a verdict of `within-reach` cannot be read as a finding about that design.
   ⚠ `moved-with-a-changed-neighbour` is not `moved-without-reason`. A node is cited by several
   requirements, and the product's closure of each includes the node's rows, so asking ONE
   requirement to change moves the evidence of every requirement that cites the same node. The
   first version called all of those violations, and a real trial (2026-10-08: a pop-up state
   cited by R7 and R8, the sound of R8 changed, R7 read as a violation) made a correct revision
   in place come out `outside-reach`. The rule, in `revision._explained_by_neighbours`:
   a carried requirement whose evidence `changed` is a look when
   - EVERY place it moved is a place that a requirement whose words `changed` or are `new` also
     moved (`moved`) or newly stands on (`at`); and
   - no more recorded rows are `gone` than places moved (a row that disappeared cannot be
     located, so more rows gone than places gained is a loss no neighbour accounts for).

   Anything else stays a violation: a place no changed requirement stands on, a carried
   neighbour (two requirements whose words did not change have no reason to move together), a
   retired or dropped neighbour (it has no place to stand on now), and a requirement whose
   citations all vanished (`dropped`). The row names the neighbours (`shared_with`) and the page
   prints them. It is a look and not an ok because the shared place is exactly where the
   neighbour's edit could have broken the requirement nobody asked to change: the owner is sent
   to that place with the neighbour's name, which is a shorter look than a violation with no
   cause.
2. **The verdict.** `outside-reach` when any finding is a violation, else `within-reach`. A
   violation is a design that moved where the specification did not ask it to, or kept a
   requirement the specification dropped. A `look` is a place a second look should go; it is never
   a refusal, because a design may already satisfy words that changed.
3. **What the verdict is not.** `within-reach` is not "the revision is right". It says the changes
   in the design are accounted for by changes in the words, on the product's closure of what a
   requirement depends on (ADR 0007). It does not run the scenarios; a scenario of an unchanged
   requirement that now fails is the next layer, and `scxml_scenarios` already plays them.
4. **The report is the same join as a page.** `carries-over` requirements fold into one line, a
   count and their ids, because words and evidence both read the same and there is nothing to
   re-read. Everything else is listed with its words status, its evidence status and the places in
   the design that moved. The page says in its own header that `carries-over` means the product's
   closure reads the same, not that the requirement is met, and that the acceptance still lapses
   by its bytes.
5. **Sentences only when given.** The page prints a requirement's sentence only when the caller
   hands over the sidecar (the file that is not committed). The page then says on its face that it
   carries someone else's sentences, as the acceptance report does. Without it the page holds ids
   and places and no words.
6. **The inputs.** The words are the `delta` that `scxml_requirement_set` returned when the revised
   list was built (passed back as it came). The evidence is computed inside the tool from the
   acceptance record, the root and, for another draft, `design`, through the product. Local
   servers only, like `scxml_acceptance_delta`.
7. **Starting from the accepted design.** When `scxml_accepted_for` finds the acceptance lapsed,
   and every lapse is of what the design was AUTHORED from (the specification, the decision record,
   the profile, the examples, the manifest) and none of the design's own files, the accepted design
   is still on disk exactly as accepted. The answer then carries `revise_from` (its document, its
   text and its page) and a `next` that says to revise it, build the requirement list against the
   lineage, change what the `delta` says moved, and run `scxml_revision_check`. When a design file
   moved, the accepted bytes are gone and no base is offered: the answer says why.
8. **No product change.** Nothing here reads the design or the record except through commands the
   product already has, and no wire surface is added.

## What this does not claim

- **A second look is still the owner's.** Nothing is accepted by these tools.
- **The closure is the product's.** A dependency it does not follow can move without moving a
  requirement's evidence (ADR 0007), and the report inherits that.
- **A shared place can hide a real fault.** The neighbour rule reads places, not meaning. If a
  carried requirement and a changed one stand on the same node and the edit for the changed one
  broke the carried one, the check says `look`, not `violation`: it is in the owner's list, with
  the neighbour's name and the place, and only the scenarios (not run here) or the owner can say
  whether it still holds. Places are matched exactly: a child of a node the neighbour stands on is
  not explained by it, which errs towards the violation.
- **Matching is by id, so it is only as good as the lineage.** A requirement the lineage carried
  wrongly (ADR 0006: a replacement taken for a rewording) reads as `revised` and is listed, which
  is the safe direction.

## What steps 4 and 5 measured

Criteria set before the code. The join is held by
`tests/test_a_revision_joins_the_words_and_the_evidence_of_each_requirement.py` (no product needed)
and the whole path by `tests/test_a_revised_design_is_checked_against_what_the_specification_changed.py`,
which runs the product's generator on the committed ISO design the other acceptance tests use:

1. **Met.** Each row of the table is produced by the join: every (words, evidence) pair the join can
   meet is one of twenty cases, and a test fails if a pair has no case. (One of the twenty was
   wrong when first written: see `uncited` above.)
2. **Met.** A violation makes the verdict `outside-reach` and nothing else does: looks and new rows
   that claim nothing leave it `within-reach`.
3. **Met.** A `delta` or an evidence object of the wrong shape is refused, and so is a requirement
   named twice; through the tool it is an argument error the client can read.
4. **Met.** The page folds what carries over into one line, lists the rest with its places, prints
   a sentence only when handed a sidecar (and then says it is a local artefact), and says what
   `carries over` does not mean.
5. **Met, against the real product.** On the committed design an unchanged design carries every
   requirement over; renaming the event of one cited transition moves exactly the requirements
   `acceptance-delta` reports, calls them violations when the words are carried and `revised` when
   the words changed, and the places are the product's.
6. **Met.** `scxml_accepted_for` hands the accepted design back to revise when only what the design
   was authored from moved; when a design file moved it says the accepted bytes are gone and offers
   none; an unchanged specification is still answered as before.
7. **Met.** A record from before evidence is refused through the tool rather than read as "every
   requirement is new".
8. **Met, against the real product.** On the committed design, editing the one transition that
   `3.DoIP-124` and `3.DoIP-081` both cite moves both; with `3.DoIP-124` `changed` and
   `3.DoIP-081` carried, the second is `moved-with-a-changed-neighbour` (a look naming
   `3.DoIP-124`) and the verdict `within-reach`; with both carried the same edit is two
   violations. The unit cases hold the limits of the rule: a place the neighbour does not stand
   on, more rows gone than places moved, a carried, retired or dropped neighbour, and a requirement
   whose citations all vanished each stay a violation.

Twenty-four ways of breaking the join (the verdict ignoring violations, each violation row turned
into an `ok` or a `look`, new unclaimed rows not counted, the page listing what carries over as
findings, the two shape checks and the two duplicate checks switched off, a sentence quoted though
none was given, a carried neighbour explaining a move, rows gone not bounded by places moved, one
shared place being enough, an explained move counted as `ok`, the neighbour not named) were put to
those tests and each was caught. With the product's binary built, the whole authoring suite passes
(1517 tests, 51 skipped for a missing optional tool).

Not done: the scenarios of an unchanged requirement are not replayed by this check. `scxml_scenarios`
plays them and a revision should run it; folding that into the verdict needs the scenario set
pinned to the requirement list, which is the next layer and not this one.
