# ADR 0039 — The scope of an import is the owner's, and is an input to the check

- Status: Accepted as the direction (the owner asked on 2026-10-19 that the part of a document that goes to
  a model be chosen, for a PDF, a word file and Markdown alike); implemented in the steps listed under
  "How it is built"
- Date: 2026-10-19
- Scope: how an owner says which part of a document an import is of, what the part is made of, where it
  is held, what it changes in the cut, the check, the proposal and the screen, and what it does not
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`,
  `docs/adr/0036-the-ai-loop-runs-jobs-of-any-kind-and-what-a-kind-needs-is-declared.md`,
  `docs/adr/0037-a-proposal-is-kept-in-the-spec-and-accepting-it-is-one-locked-step.md`

## Context

An import is of a whole document today: the reader reads every block, the converter converts every block,
and the check counts every block with words that no conversion block cites as *missing*. A document often
holds more than the spec it is brought in for (several functions in one file, an appendix, a cover, the
parts of a contract that are not requirements), and a model that is given all of it is given what it does
not need, and for a document the owner is bound not to share, what must not leave.

Importing a part by cutting the document first does not do. Cutting a PDF or a word file is the work of the
reader, and a cut that is made outside the reader is a second document with its own hash and no mark of
where it came from. And leaving the check as it is shows everything outside the part as a question the
owner has to decide before the proposal can be accepted.

The check cannot be given the part by the conversion. ADR 0026 is explicit that a conversion cannot report
itself complete: the form has no field in which its maker could say what counts. A scope the maker wrote
would let it make its own omissions disappear.

## Decision

**A scope is a document of the owner's, named by the blocks of an extraction.** It is a JSON object `{"v":
1, "extraction": <revision>, "blocks": [<ids>]}`: the extraction it is of, and the ids of the blocks that
are in it, each once, in the order the extraction has them. A scope that names a block the extraction does
not have, a block twice, another order, or no block at all is refused. It is kept by the digest of its
text, as an extraction is, and its form is a `pre-release` surface.

**The tools turn what the owner points at into a scope, so that no host knows how a document is cut.**
`outline` lists the sections of an extraction, each with an id (`s1`, `s2`, ...), a title, a level, how
many blocks and characters it holds, and the pages it spans. The sections are the headings of the
document, nested by level, with a first section for what comes before the first heading; for a document
with no heading they are its pages, and for one with neither it is one section. `make-scope` takes the ids
of sections and writes the scope: a section brings its own blocks and those of the sections under it. A
host and a screen show the outline and send ids, and never compute a block range.

**The scope is an input to everything that reads blocks, and is never a field of the conversion.**

- The cut (`prepare-conversion`) and the converter by rule (`convert`) take only the blocks in the scope.
  What is outside is never given to a model.
- The check (`check-conversion`) counts only the blocks in the scope. A block outside it is not *missing*;
  it is *out of scope*, counted and not asked about. A conversion block that cites a block outside the
  scope is *to check*, with the reason said, since a model that was not given it cannot have read it.
- The report names the scope it was made under, as it names the extraction and the conversion, so that a
  report made under one scope is not taken for the check of another.
- A scope that is not given is the whole extraction, and everything is as it was.

**The proposal holds the scope it was made under.** It is a part of the proposal's record, by digest, and
the text of it is kept beside the conversion and the report. The store refuses a proposal whose report names
another scope than the one it was given, and a scope that is not of the proposal's extraction, as it refuses
a report of another conversion. The same key given with another scope is another input and is refused as
`key-reused`. The questions the check asked are in the scope, and are kept with the proposal as before.

**An import is two steps for the person.** The host reads the document first (it keeps the original, reads
and keeps the extraction, and says the outline), and the owner chooses the parts; then it converts, checks
and proposes for them. Choosing is never required to be slow: every section is chosen to begin with, so a
document that is wanted whole is one press more.

**A scope is the owner's choice and is not the system's judgement of the document.** Whether a document
holds one spec or several is not something SCE can check, and nothing here claims to. What is built is the
way for the owner to say it, and for that to be held by the cut, the check and the record. Whether an AI is
asked to refuse a document it finds mixed is not decided here.

## How it is built, in order

Each step lands with its tests and is useful before the next.

1. **The scope in the tools.** The scope's form and its revision, `outline`, `make-scope`, and `--scope`
   for the cut, the converter by rule and the check; the report names the scope.
2. **The scope in the store.** The proposal's scope and the blob beside its conversion, the refusals above,
   `propose_import` taking one and `read_proposal_documents` giving it back.
3. **The host and the screen.** `read_document` and `import_selection` in the workbench, and the screen
   that shows the outline, the choice and what leaves the computer, before the review.

## Alternatives rejected

- Cutting the document before importing it. The cut is the reader's work, and a cut made outside it is a
  document with no mark of where it came from.
- A scope in the conversion's form. Its maker would decide what is missing.
- A scope as ranges of pages or blocks written by the screen. Every host would then know how each format is
  cut; the outline is the tools', as the extraction is.
- Leaving the check as it is and letting the owner decide every block outside the part. It is the work the
  scope exists to spare, and it makes a partial import look like a failed one.

## Consequences

- A document the owner is not free to share goes to a model only in the part the owner chose, and what was
  not chosen is not on the way anywhere.
- One original can be imported more than once, for different scopes, as different proposals.
- The surfaces this adds (the scope, the outline) are `pre-release`.

## Not decided here

- Whether an AI is told to refuse a document that holds several independent behaviours, and how a refusal
  is said to the owner.
- Choosing by something other than a section: a block, a range of pages typed by hand, a search. The scope
  is of block ids, so any of them can make one.
- A signal that a document may hold several specs (many top-level sections, a long text), shown as a
  suggestion to choose and never as a refusal.
