# ADR 0040 — A document is divided among specs, and every paragraph has a place

- Status: Proposed (the owner asked on 2026-10-19 for the best long-term design of importing a document
  that holds several specs, and said that no private-document handling is wanted, since spec owners may
  use an external AI); nothing here is built yet, and the order of building is under "How it is built"
- Date: 2026-10-19
- Scope: how one document becomes several specs, what record says where each paragraph of it went, what
  that record is held to and by whom, how an AI helps to draw it, how it is carried to a revised
  document, and what it leaves to the specs it feeds
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`,
  `docs/adr/0036-the-ai-loop-runs-jobs-of-any-kind-and-what-a-kind-needs-is-declared.md`,
  `docs/adr/0037-a-proposal-is-kept-in-the-spec-and-accepting-it-is-one-locked-step.md`,
  `docs/adr/0039-the-scope-of-an-import-is-the-owners-and-is-an-input-to-the-check.md`

## Context

ADR 0039 lets an owner choose the part of a document one import is of, and what is outside the part is
counted and not asked about. That is right for one spec. For a document that holds several, it leaves the
hole the owner was worried about: the parts that were not chosen are *out of scope*, and nothing says
whether they were meant for another spec, meant to be dropped, or forgotten. Three imports of one
document, made on three days, leave no record that the document was ever accounted for, and a paragraph
that went to none of them is indistinguishable from one that was never looked at. A requirements document
is exactly where that loss is expensive.

Three other things were found by trying the tools on real documents and by asking what a person does.

- **A mixed document is not refused, it is divided.** An owner who is told "this holds several specs" has
  learned nothing they did not know. What they need is help making the cut, and a record that the cut left
  nothing out. Refusing is a decision for the owner to make about a part (it is not a requirement), never
  for the system to make about the document.
- **The document will be revised.** The customer sends a second version, and the work that was done on the
  first (which part is which spec) is lost unless it can be carried. Carrying it is only safe if what
  cannot be carried is *asked*, not dropped.
- **The check is per block and per spec.** ADR 0026 holds a conversion to its original block by block, and
  ADR 0039 to the part of it that was chosen. Neither can say that the parts chosen for several specs
  together cover the document, because no record holds them together.

## Decision

**A document being divided is an intake, and an intake is a record beside the specs, not in one of them.**
It names, by hash, the original (ADR 0032) and the extraction read out of it, and it holds the owner's
ledger. It is kept in the specs folder's root (`.sce-intakes/<intake-id>/`), because it is about several
specs and a record inside one of them would make that spec the document's owner and leave the others
dependent on it. An intake is a fact and not soft state: like the specs it is a chain of saved ledgers,
each named by its own digest, with a head moved by a save that names its base (the compare-and-set every
head of the folder already uses). Removing a spec does not remove an intake that names it; the intake says
the destination is gone.

**A ledger is a list of allotments, and an allotment is a scope with a destination.** Its form is
`{"v": 1, "extraction": <revision>, "allotments": [{"to": ..., "reason": ..., "blocks": [<ids>]}]}`, where
`to` is a spec that exists, a spec to be made (with its title), or *left out*. The blocks are ids of the
extraction in its order, each at most once in an allotment, as in a scope. A block may be in the allotments
of several specs (a definition that two of them rely on), and may not be both left out and in a spec. A
`reason` is asked for a left-out allotment and may be empty: leaving a part out is an act, and an empty
reason is still that act. The screen works in the parts of the outline (ADR 0039) and the tools expand them
to blocks, so that no host computes a block range.

**The ledger is counted by SCE and never reports itself complete.** `check-ledger` takes the extraction and
a ledger and says, by id: which blocks with words are in no allotment (*unplaced*), which are both left out
and in a spec (*contradicting*), which are in more than one spec (*shared*), and which ids the extraction
does not have. The ledger is complete when none is unplaced and none contradicts. This is ADR 0026's rule
again, for the same reason: the one who made the allotments must not be the one who decides that nothing
was missed.

**Confirming an intake makes the proposals, and each is the ADR 0037 machinery unchanged.** Confirming is
refused unless the check says the ledger, at its current revision, is complete. It then makes, for each
spec destination, an import proposal whose scope is that allotment (ADR 0039), registered under a key made
of the intake, the destination and the ledger revision, so that pressing it again makes what is missing and
nothing twice. A spec to be made is made then, and named by the title the owner gave. Each spec keeps the
whole original and the extraction, which are named by hash and so are shared in content; the part of it
that is the spec's is its scope. The proposals are then reviewed, decided and accepted one spec at a time,
as ever. **Nothing here spans two specs in one step.** Each step takes one spec's lock, never two, so there
is no order to deadlock in; before any proposal is made every destination is asked whether it already has
an open one, and the confirm is refused naming it, but a race after that can leave an intake partly
proposed, and that is a state the screen shows (*2 of 3 proposed*) and pressing confirm again finishes.
A proposal made from an intake says so (`intake`, and the ledger revision), so that its review can say
which other specs the document was divided into.

**An AI draws a draft of the ledger and the owner owns it.** Dividing a document is a job of a kind
(ADR 0036), given the outline (each part's id, title, size and its first words) and answering with
allotments by part id. What it answers is a claim, read the way a conversion's answer is: an id the outline
does not have is an unreadable answer, a part it did not place is left *unplaced* and named by the check,
and nothing it says is placed on its authority. The draft is saved as a ledger marked as a model's, which
confirming is not refused for: the owner is shown all of it before they confirm, and a wrong allotment
costs a proposal that is reviewed paragraph by paragraph and written to no spec's text until accepted.
That is the reason a draft needs no gate of its own, and it is what would have to change if accepting ever
stopped being the last step. The draft is a suggestion to divide and is never a refusal of the document.

**A revised document carries the ledger by what is the same, and asks about the rest.** `carry-ledger`
takes the old extraction with its ledger and the new extraction, and writes a draft for the new one.
Paragraphs are matched in order by their words as the check reads them (`plain_of_block`), so that a
sentence that occurs twice is matched by its place. A matched paragraph is placed where its twin was; a new
or changed paragraph is *unplaced*, and one of the old ones that has no twin is listed as *gone*, with where
it had been placed, so that an owner sees which of their allocated paragraphs the customer removed. A
changed requirement is the very thing an owner must decide, so a near match is shown beside the paragraph as
a pointer and is never carried.

**The one-spec import of ADR 0039 is the ledger with one destination, and stays as it is.** Choosing the
parts for one spec, with the rest left aside, needs no intake and keeps working. The parts screen grows
into the ledger screen by gaining destinations: it starts with every part going to the spec being made, a
part is moved by choosing where it goes, and a second destination is what turns the import into an intake.
So the *unplaced* state is reached by revision and by an AI draft, which place nothing by themselves, and
not by the first manual cut, which moves what is placed. That is where silent loss was possible, and it is
where it is closed.

**The screen is drawn on a board first** (the workbench keeps its design boards in its own repository),
since it is the part of this that a person lives in, and it says four things the model above makes available: how many paragraphs are placed and where
the rest are, which parts are shared or left out and why, what blocks the confirm in words (as the review's
save does), and, once confirmed, how many specs were proposed and which are waiting.

## How it is built, in order

Each step lands with its tests and is useful before the next.

1. **The ledger in the tools.** The ledger's form and revision, `check-ledger`, and the expansion of parts
   to blocks; paired tests for each way a ledger is wrong and the nearest one that is not.
2. **The intake in the store.** The record, the ledger chain with its compare-and-set save, the confirm that
   makes the proposals under their keys, the `intake` field of a proposal, and the commands that say them
   (the command set's version moves, and the new forms are registered as `pre-release`).
3. **The workbench commands and the board**, then the screen built from the board.
4. **Carrying a revision**: `carry-ledger`, the read of two documents, and the screen that shows what was
   carried, what is asked and what is gone.
5. **The AI draft**, last, because it needs the AI loop's product wiring (ADR 0036) that is not built, and
   because what it is worth is a number to be measured on public documents before anyone relies on it.

## Alternatives rejected

- **The ledger inside the first spec.** It makes that spec the document's owner, breaks when it is removed,
  and gives the others a dependency on a spec that has nothing to do with them.
- **No ledger: the same import repeated for each spec, with a view that adds the scopes up.** The scopes
  live in different specs and nothing holds their union, so a paragraph in none of them cannot be told from
  one not yet imported. The accounting is the feature, and it needs one place.
- **Refusing a document that holds several specs.** It rejects the owner's input instead of helping with it,
  and there is nothing in a document that SCE can check to say it holds one.
- **An AI that decides what is left out.** Whether a part is a requirement is the owner's. The AI places, the
  owner may move, and only the owner leaves a part out.
- **Carrying a changed paragraph by a similarity score.** A score that is high for a requirement that moved
  from "five seconds" to "fifty seconds" is the failure, and the owner is the only one who can say it is not.

## Consequences

- A document is accounted for as a whole: confirming says that every paragraph with words went to a spec or
  was left out on purpose, and a revised document cannot lose a paragraph without being asked.
- Each spec holds the whole original and the extraction, shared by hash, and a spec's proposal says which
  part is its own.
- A spec's review can show that its proposal is a part of a divided document and which other specs the rest
  went to.
- An intake outlives a spec it names and says so; a confirm that stopped partway is visible and finished by
  pressing it again.
- The ledger, the check's report of it, and the carried draft are `pre-release` surfaces, to be registered in
  `SCE_WIRE_CONTRACTS.md` when they are built.

## Not decided here

- Changing a ledger after it was confirmed. The direction is that the owner changes it and confirms again,
  that a destination whose allotment did not change keeps its proposal, that one whose did gets a new
  proposal in place of the open one, and that a destination left with nothing has its open proposal
  rejected. What happens to decisions already made on a proposal that is replaced is for step 2 to settle.
- Carrying the owner's decisions of a proposal (keep, correct, drop, add) to the proposal of a revised
  document. A correction can be carried when the paragraph it corrects is the same, but the conversion's
  ids are not the extraction's and the rule has not been written.
- The AI draft's recipe, its unit of work for a document with hundreds of parts, and how it is measured.
- Whether two documents may be divided into one intake (a specification and its amendment), or an intake is
  always of one.
- Choosing by something other than a part of the outline (a typed page range, a search), which ADR 0039
  also left open. A ledger is of block ids, so any of them can make an allotment.
