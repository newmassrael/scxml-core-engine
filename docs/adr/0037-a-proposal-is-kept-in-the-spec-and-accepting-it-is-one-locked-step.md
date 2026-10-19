# ADR 0037 — A proposal is kept in the spec, and accepting it is one locked step

- Status: Accepted as the direction (the owner chose the import path first on 2026-10-19, after the
  critique of the application against its boards); implemented in the steps listed under "How it is
  built"
- Date: 2026-10-19
- Scope: what an import proposal is as a record of the specs folder, the rules of its life, what its
  decisions are and how they are kept, what accepting does and what it is refused for, how a proposal
  that moved is carried forward, and what the store does and does not take on trust
- Related: `docs/adr/0016-the-rules-of-a-generation-request-are-one-document.md`,
  `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`,
  `docs/adr/0036-the-ai-loop-runs-jobs-of-any-kind-and-what-a-kind-needs-is-declared.md`

## Context

ADR 0025 decides that an import is a proposal pinned to the revision it was made from, that there is no
lock while someone reviews, that accepting is the compare-and-set every head already uses, and that the
life of a proposal is one document from which the code is generated. ADR 0026 decides the conversion is
checked against its original block by block. What is built is the original and its extraction in the
store (ADR 0032), the conversion form and SCE's check of it, and a converter by rule. Nothing yet keeps
a proposal, so nothing can be reviewed, decided or accepted.

Reading the code for what that takes showed four things the earlier ADRs leave open.

1. **Where the check runs.** The extraction, the conversion and the check are written in the authoring
   tools (Python), because the readers that need a pack are. The store is Rust and never runs Python.
   The check cannot move into the store without writing it twice, and a store that recomputes it would
   need the authoring tools to be installed wherever a spec is kept.
2. **What the owner decides.** ADR 0025 names four decisions on a block (keep, correct, drop, add from
   the original) and says they are data and are never applied by editing the conversion. It does not say
   who turns the conversion and the decisions into the text that becomes the spec, and that text is a
   function of both.
3. **How accepting stays one step.** The text head is moved under the spec's lock by a save that names
   its base. Accepting has to check the proposal's life, check what is still undecided, move the head
   and end the proposal as one step, or two people accepting at once could both pass.
4. **What "carried forward" carries.** ADR 0025 says a proposal that moved is rebased, with the owner's
   decisions kept on blocks that did not change. A conversion is made of the original, and the original
   did not move when the spec did, so every block of it is unchanged; what moved is what accepting would
   overwrite.

## Decision

**A proposal is a record in the spec's folder.** It belongs to one spec and names, by hash, the original
it is made of (ADR 0032), the extraction it was read into, and the conversion made of that extraction;
it records who converted (a rule of the tools or a model, by the name it reports), the revision of the
text head it was made against (`against`, none for a spec that has no text yet), the checks SCE made of
the conversion, when it was made, and its state. The conversion and the check report are kept as blobs
under their own hashes, like every other text of the folder, and the record names them.

**What SCE checks is run by the host and kept by the store; the store never recomputes it.** The authoring
tools make the extraction, the conversion and the check (`check-conversion --json`), and the host runs
them. The check report names the extraction and the conversion it is of by revision, and the store
refuses a report that names others. The store reads one thing of it: the list of what a person is asked
about (`asks`: the reason and the id of each block, in the order they are asked). A conversion cannot
report itself complete (ADR 0026), and the report is not the conversion's: it is made by SCE's own tool
over the conversion and the extraction, and a host that hands the store a report made by anything else
has made the same kind of mistake as a host that saves a text it did not read. That is the trust the
specs folder already places in its callers (`save_source` keeps the text it is given), and it is said
here so that nobody takes the store for a second check.

**A proposal has one life, and it is a document.** `sce-specs/machines/import_life/import_life.scxml`
holds it, as `request_life.scxml` holds a request's, and the Rust is generated from it. A proposal is
`pending` until it ends one of three ways: `accepted`, `rejected` (the owner declines it) or `superseded`
(another proposal replaced it, or the original was replaced). Events: `accept`, `decide` (a decision may
be taken while the proposal is open, and is refused once it ended: that is a rule of its life, so the
document says it), `reject` and `supersede`. The machine reads what a rule reads and nothing else: whether the spec's text
head is still the one the proposal was made against (`headMatches`, the host's answer, because it is a
comparison of two digests no rule reads into) and how many of the questions asked are still undecided
(`undecided`). An event the proposal cannot take leaves it where it was and says why in `verdict`:
`moved` (the head is not `against`), `undecided` (some question has no decision), `ended`, or `nothing`
(there was nothing to do: superseding what ended, rejecting what was rejected). The same `accept` said
again by the same proposal is the same acceptance, and says `ok`.

**Decisions are a log, and the log is the proposal's.** Each decision names a block of the conversion or
of the original, says what the owner did (`keep`, `correct` with the words, `drop`, `add` the original's
block) and when. They are appended to `decisions/<proposal>.log`, one JSON line each, never edited and
never applied by changing the conversion. The latest decision on a block stands, and an earlier one is
kept as history. A block is decided when any decision names it, and a question is answered when the
block it asks about is decided. What may be said of a block depends on whose it is. A block of the
conversion is kept, corrected or dropped. A block of the original that the conversion left out is added,
or dropped, which says the owner leaves it out; there is nothing in it to keep or to correct. A decision
is refused when the proposal has ended, when it names a block that neither the conversion nor the
original has, when it keeps or corrects a block the conversion does not have, when it corrects with no
words, and when it adds a block the original does not have.

**Whoever makes the final text says what it was made from.** The text that becomes the spec is a function
of the conversion, the original's extraction and the decisions, and the assembly is written once, in the
authoring tools, with the rest of the reading. Accepting carries that text and the number of decisions it
was assembled from (`decisions`). The store refuses the accept as `decisions-moved` when the log has
since grown, so that a text assembled before somebody decided one more block is never saved as if it
answered the log. The store does not check that the text is the assembly of those decisions, as it does
not check that a saved source is what its owner meant: the owner saw the text before accepting it.

**Accepting is one step, under the spec's lock.** With the lock held, the store reads the proposal and the
log, computes `headMatches` and `undecided`, and says `accept` to the machine. When the machine takes it,
the text is saved from the base `against` in the same step (the save every head goes through, so a text
saved from another base is the same `conflict`), and the proposal ends as `accepted`, recording the
revision the save made. When the machine says `moved` the store answers the refusal ADR 0025 describes,
naming both revisions and saying that nothing was written; the proposal stays, and the owner is shown
what the spec has become. Nothing is locked while a person reviews: the lock is taken for the accept,
and not for the review.

**A proposal that moved is carried forward, not redone.** `rebase` makes a new proposal against the head
the spec has now, from the same original, extraction and conversion, with the decisions of the old one
copied in order, and supersedes the old one in the same step. It is refused for a proposal that has
ended, and for one whose head still matches (there is nothing to carry). The old proposal and its log are
kept.

**A spec has one open proposal at a time.** Registering another is refused (`active-proposal`) unless the
caller says it replaces the open one, which supersedes it: whether a review that is under way is thrown
away is the person's to say, as for a request. (It was first written as one open proposal for each
original. A spec that is brought in from a second document while the first is under review has two
proposals fighting for one text head, and the second to be accepted would be refused as `moved` by the
first, so the rule is the request's.)

**A document that carries its text needs no model, and the same record serves it.** The proposal made of
the converter by rule has no question to ask, so it is accepted in one step, and the owner is shown what
it changes in the spec and not how a conversion compares with an original (ADR 0025). The same life, the
same refusals and the same lock hold it.

## How it is built, in order

Each step lands with its tests and is useful before the next.

1. **The proposal and its life.** The machine and its generated Rust, the record, the proposal store with
   the decisions log, the commands (`propose_import`, `read_proposal`, `list_proposals`,
   `read_proposal_documents`, `decide_block`, `accept_proposal`, `reject_proposal`, `rebase_proposal`) and
   their tests, and the workbench's recorded replies.
2. **The assembly.** The authoring tools' `assemble`: the final text from the conversion, the extraction
   and the decisions, with the count of decisions it was made from, and the report that says what the
   text changes in the spec's current text.
3. **The host.** The workbench's way to run the tools and hand the store what they made: one function that
   extracts, converts, checks and proposes, and one that assembles and accepts.
4. **The screens.** Main (a document in), the import's states, and the review: the original and the
   conversion side by side, one decision to a flagged row, the count in the bar that falls as rows are
   decided, and accepting at zero.

## Alternatives rejected

- Checking the conversion in the store. It would need the authoring tools and the readers' packs wherever
  a spec is kept, and would write the check twice.
- Applying the decisions to the conversion as they are made. ADR 0025 refuses it: the record would then
  say the spec says something nobody has checked, and the owner's earlier word could not be read back.
- Locking the spec while a proposal is reviewed. ADR 0025 refuses it: one person who closes a window
  blocks everyone.
- Writing the proposal's life as code. The request's was moved into a document because the rules of such a
  life are the kind SCXML writes and the product holds its own application to them (ADR 0016), and a
  proposal's are more of that kind: it ends three ways, and a refusal says which rule it broke.
- Recomputing `undecided` from the conversion on every read. The questions are asked once, by the check,
  and kept with the proposal; a question that appears later is another check, which is another proposal.

## Consequences

- The workbench can keep what a person is reviewing across a restart and across people (ADR 0028): the
  proposal, its questions, and every decision, in the spec's folder.
- A refusal at accept is a first-class state with a way forward: `moved` is carried by `rebase`, and
  `undecided` names the blocks.
- The accepted text is the owner's act and is recorded with the proposal that made it, so the history of
  the text says where each revision came from (a proposal) and not only that it was saved.
- The surfaces this adds (the proposal's record, the report and the decision) are `pre-release`.

## Not decided here

- The words the review shows for each question's reason, and the threshold at which a recognised block is
  asked about (ADR 0026).
- How a proposal is made across a team server and who holds the review's lease (ADR 0025's reviewer lease
  is a later step and changes nothing above).
- Whether a decision on a block can be taken back by the same person, and how that is shown.
