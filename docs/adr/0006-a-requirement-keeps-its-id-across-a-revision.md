# ADR 0006 — A requirement keeps its id across a revision of its specification

- Status: Accepted for step 1 (implemented and measured, see "What step 1 measured"); steps 2 to 5
  are named, not decided
- Date: 2026-10-08
- Scope: `tools/authoring` (`sce_author/requirement_lineage.py`, `requirement_set.py` and the MCP
  tool `scxml_requirement_set`); later steps reach `sce-build` (`acceptance_record.rs`,
  `acceptance_report.rs`)
- Related: `SCE_WIRE_CONTRACTS.md` (acceptance-record row), `sce-build/src/requirement_manifest.rs`,
  `sce-build/src/acceptance_record.rs`, `tools/authoring/README.md` ("When the same
  specification is drafted more than once"), `tools/authoring/eval/revision_identity.py`

## Context

An owner's specification changes. Today the product handles that by lapsing the acceptance
and asking for a new draft: the acceptance pins the specification, the decision record, the
profile and the scenario set by sha256 (`Lapse::Source`), and the server tells a client that
a lapsed acceptance means "a draft is written" (`scxml_accepted_for`) or, for a work, that the
model is `behind` until it is saved again. Nothing in that path starts from the design the
owner accepted, and nothing says which requirements the change touched.

The second half of that is not a missing feature, it is a missing premise. To say which
requirements a revision touched, a requirement's id has to mean the same requirement in both
revisions. It does not:

1. **Ids are reading order.** `requirement_set.build` numbers the quoted requirements `R1..Rn`
   by where their words sit, and the sentences `S1..Sm` the same way. A sentence inserted
   anywhere but the end renumbers everything after it.
2. **`rev` never moves.** `scxml_requirement_set` calls `build` with `doc_id` and no `rev`,
   so every list it makes says `rev: "1"`. The product's staleness note
   (`requirement_manifest::classify_citations`) fires when a document cites `doc_id@rev` and the
   manifest carries another `rev`; two revisions of one specification that both say `1` can
   never trip it.
3. **The failure is silent.** An id the new list no longer contains is reported `dangling`.
   An id the new list contains and that now names a different requirement is reported
   `implemented`, against a sentence the design element was never written for.

### Measured

`tools/authoring/eval/revision_identity.py` applies an owner's edit to a specification and to
its requirement list by rule, so what each requirement became is known by construction and no
model is in the figure. Three specifications, 16 edits, 127 requirement ids, the scheme as it
was before this ADR (the second list built with no lineage):

| Edit | Ids | Hold | Silent wrong | Lost (loud) |
|---|---|---|---|---|
| sentence inserted after the first | 11 + 6 + 5 | 3 | 19 | 0 |
| sentence deleted | 11 + 6 | 7 | 8 | 2 |
| two sentences swap places | 11 | 9 | 2 | 0 |
| a sentence replaced by another | 11 + 6 + 5 | 3 x (n-1) | 3 | 0 |
| sentence appended / a number reworded / text reflowed (control) | 11 .. 5 | all | 0 | 0 |

32 of the 127 ids name a different requirement in the second revision and nothing flags it;
2 more are lost. Every one of the 32 is an edit an owner makes in the ordinary course. The
control (`reflow`, no word changes) holds every id, so the measurement can read stability. (The
first version of this table, taken before the replacement edits were added, said 29 of 105.)

### Two constraints the code already states

- **No requirement sentence is committed.** The manifest holds coordinates only and the
  sentences live in a sidecar that is never committed (`requirement_manifest.rs`, "Why the
  manifest holds no requirement text"): a specification is often someone else's document. Any
  new committed artifact in this chain carries hashes, never words.
- **An acceptance lapses by bytes, on purpose.** `acceptance_record.rs`: the byte pin "lapses
  more often than the behaviour moves, and that is the direction to be wrong in: a lapse costs
  a person a second look, and a false hold costs the acceptance its meaning." A finer pin that
  keeps an acceptance standing through an edit is therefore not a goal of this work. What can
  be finer is the report of what moved.

## Decision

**A requirement's id is issued once and follows the requirement through revisions. Identity
and sameness are two facts, kept apart.**

1. **A lineage beside the specification.** `requirements.lineage.json`, committed, no words:
   `doc_id`; one row per revision (`rev`, sha256 of the normalised specification, and the
   sha256 of each of its sentences, which is what lets two revisions be compared sentence by
   sentence without either text); one row per id ever issued (`id`, the revision it first
   appeared in, the revision it was retired in or null, the sha256 of its normalised quote in
   each revision it lived); and `next`, the counter ids are issued from. A retired id is never
   issued again. Its shape is `tools/authoring/schema/requirement-lineage.v1.schema.json`.
2. **The first revision is today's list.** Ids in reading order `R1..Rn`, so a specification
   built once produces byte-identical output. A list made before lineages existed is adopted
   from its manifest and sidecar (`previous_manifest`, `previous_sidecar`): its ids are kept
   without renumbering and its `rev` is the lineage's last. The digest of the text it was
   built from was never kept, so the next revision is always a new one and no sentence delta
   is given for it.
3. **A later revision carries ids by rule, in this order, and says which rule:**
   a. a quote whose normalised sha256 equals a live id's latest quote keeps that id
      (`carried`, and the requirement is **same**);
   b. among the quotes left, a new quote that is the unambiguous near-match of one live id
      keeps that id (`carried-with-edit`, and the requirement is **changed**: its words are
      not its predecessor's). Unambiguous means one counterpart above the similarity
      threshold and a margin to the next; the threshold is a starting value that the harness
      decides, not a number to trust before it has measured false pairings. ⚠ The words of
      the id's last quote are not in the lineage, so this needs them: the caller gives the
      previous sidecar (`previous_sidecar`), each text is used only if its sha256 is the one
      the lineage last saw for that id, and without them 3b does nothing and the requirement
      is issued afresh, which is loud and never wrong. (The first draft of this ADR missed
      that a lineage of hashes cannot compare wordings.)
   c. a client that read both revisions may state a continuation it is sure of
      (`continues: {quote -> id}`). The tool validates it (the id is live and claimed once)
      and records its source as the client's; it never invents one;
   d. every other new quote is issued a new id (`new`); every live id nothing continues is
      retired (`retired`).
4. **"Same" is claimed only on equal hashes.** Carrying an id through an edit asserts
   succession, not equality. A wrong succession costs a requirement reported `changed` that
   was really replaced, which sends a person to read it; it never reports a different
   requirement as unchanged. That is why the near-match in 3b needs no owner gate and why the
   quantity that must be zero is "an id names different words and is reported same".
5. **`rev` follows the specification.** A new revision exists exactly when the normalised
   text's sha256 differs from the last row's; its `rev` is the next integer. The product's
   staleness note then fires for a design cited against `spec@1` when the manifest is `spec@2`.
6. **The tool stays pure.** `scxml_requirement_set` takes the previous lineage (`lineage` or
   `lineage_text`), the previous sidecar and `continues`, returns the new lineage beside
   `manifest_text` and `sidecar_text`, and returns a delta in the vocabulary above plus the
   sentence-level change (added / removed, by position and digest). It writes nothing, as it
   writes nothing today. ⚠ Not built in this step: keeping the lineage with a work in the
   application. `works_save_requirements` stores a manifest and a sidecar under a
   compare-and-swap, and whether `sce-work`'s folder format takes a third file is the
   application's to say. Until it does, a revision of a work is built by adoption: `works_read`
   returns the saved manifest and sidecar, and they are enough to keep the ids (not to tell
   an unchanged text from a changed one).
7. **Sections stay revision-local.** `S<n>` remains a positional label inside one revision,
   the way a page number is. Identity across revisions is the requirement's; for sentences it
   is their hash.
8. **No product wire surface changes in this step.** The manifest's ids are strings and `rev`
   is a string already. The lineage is an authoring-side artifact; its schema goes beside the
   others in `tools/authoring/schema`, which is not listed in `SCE_WIRE_CONTRACTS.md`.

## Alternatives considered

- **Content-hash ids** (an id is a hash of the quote). Measured with the same harness by
  replacing the id function: silent wrong falls from 32 to 0, and every requirement whose words
  an edit changes loses its id (all 4 reworded requirements of the corpus, the same 118 holds,
  4 lost and 5 retired as a lineage given no words). Stable under insertion and reordering,
  blind to succession, and an unreadable `sce:req="Q3fa91c07"` in a design. The hash stays, as
  the key the lineage matches on; it is not the id.
- **Ids written by the owner in the prose** (`Ids::Native`). The best identity there is, and
  already supported by the manifest: when the source names its own requirements nothing in this
  ADR applies. It asks the owner to number their sentences, which a prose specification does
  not do, so it is an option and not the plan.
- **Asking the model to map old requirements onto new.** Puts a non-deterministic step in the
  path of identity. The model may state a continuation (3c) and the tool decides whether it is
  admissible; it does not decide alone.

## What this unlocks, and what it does not

Each step needs the one before it; none changes how an acceptance lapses.

| Step | Adds | Needs |
|---|---|---|
| 1 | this ADR: ids that mean the same requirement in both revisions | |
| 2 | the acceptance record also keeps, as hashes, the list, the element each requirement maps to and each element's canonical form, and points to the record it succeeded | 1 |
| 3 | the delta of two revisions (added / removed / changed / unchanged requirements) and the design elements it reaches, by the product's own reachability and guard analysis | 1, 2 |
| 4 | a revision starts from the accepted design; a check refuses a result that changes an element outside the reached set, or breaks a scenario of an unchanged requirement | 3 |
| 5 | the report an owner reads shows only the delta and says which requirements carry over, on the evidence of equal hashes | 2, 3 |

⚠ Steps 2 to 5 are named here so that step 1 is not designed without them; they are not
decided. In particular the earlier idea of finer pins that keep an acceptance standing
(a profile pinned only by the rules the design applied, say) conflicts with the constraint
above and is dropped: the acceptance still lapses, and step 5 makes the second look short.

Not goals: making two drafts equal (the model is outside the product, and `compare` already says
where drafts part); choosing for the owner what a new question means; changing what lapses an
acceptance.

## Consequences

- **A lineage is one more file an owner keeps.** Without it the tool can only start a list over,
  which is the old behaviour and the old silent re-pointing. A call that gets no lineage cannot
  know whether the specification was built before, so the answer says it is the first list as
  far as it can tell and what to give if it is not.
- **Similarity is a judgement made by a formula.** Its failure is the safe one by the fourth
  point, but a wrong succession pollutes an id's history. Measured below, on a corpus too small
  to tune a threshold against: `SIMILARITY` 0.8 and `MARGIN` 0.1 are starting values.
- **The baseline moves.** The reading-order scheme is now what a call with no lineage does, so
  `test_the_revision_measurement_reads_what_became_of_each_id` still reads it as the baseline;
  a first list is byte-identical to what it was.

## What step 1 measured

`revision_identity.py` over the same 16 edits and 127 ids, three schemes. The lineage schemes
build the second list against the first list's lineage; "words given" also passes the first
list's sidecar.

| Scheme | Hold | Silent wrong | Miscarried (flagged changed) | Lost (loud) | Retired (right) |
|---|---|---|---|---|---|
| reading-order ids (no lineage) | 93 | **32** | 0 | 2 | 0 |
| lineage, hashes only | 118 | **0** | 0 | 4 | 5 |
| lineage, words given | 122 | **0** | 1 | 0 | 4 |

Against the criteria this ADR set:

1. **Met.** Silent wrong is 0 on every edit under both lineage schemes; the `reflow` control holds
   every id under all three.
2. **Met.** Insert, append, delete and swap hold every id of a requirement that is still there.
3. **Met with the words.** All 4 reworded requirements keep their ids and are listed `changed`.
   Without the previous sidecar they lose them (4 lost), loudly.
4. **Reported, not a pass mark.** Of 3 requirements the owner replaced, 2 (unrelated sentences)
   were not carried and 1 was: "the door is closed" became "the door is locked", 8 of 9 words
   alike, so the near match took it as the same requirement reworded and listed it `changed`. That
   is the price of carrying an id through a rewording: wording cannot tell an edit from a
   replacement, and what keeps it safe is that "same" needs equal words. One case in three is
   not a rate.
5. **Met.** `rev` is 2 for every edit that changes a word and stays 1 for the reflow.

Tests of the tool hold the rest: the first list is the list it always was, an id of a closed
requirement is never issued again, an ambiguous near match carries nothing, a continuation that
does not fit is refused, the lineage holds no word of the specification, and a lineage that is
another specification's, malformed, or would issue an id twice is refused. Twelve ways of
breaking the lineage's rules were put to those tests and all twelve were caught (one was not,
at first: a continuation claiming an id twice, which the tests did not exercise and now do).
