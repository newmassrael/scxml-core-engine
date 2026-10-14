# ADR 0025 — An import is a proposal pinned to the revision it was made from

- Status: Accepted as the direction (the owner decided it on 2026-10-14); not yet implemented
- Date: 2026-10-14
- Scope: how a document brought in from outside (a word-processor file or a PDF) becomes the text of a
  spec: the original, its conversion, the review, and what happens when the spec moves meanwhile
- Related: `docs/adr/0011-a-work-keeps-its-requirement-lineage-with-its-requirement-list.md`,
  `docs/adr/0016-the-rules-of-a-generation-request-are-one-document.md`,
  `docs/adr/0024-a-work-is-called-a-spec.md`, `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`

## Context

A specification owner usually receives a specification as a document, not as Markdown. The owner
decided that the imported document is the original, that an AI client converts it to Markdown, that
the owner reviews the conversion beside the original before anything is accepted, and that PDF is
supported as well as word-processor files. The authoring tools refuse a PDF today
(`tools/authoring/sce_author/ingest.py` answers "convert it first"); this record reverses that.

The review takes time, and the server may have several people on one spec. The question is what the
import does while it waits: whether it holds the spec (a lock), or whether it is a proposal that the
spec may move away from.

## Decision

**An import is a proposal, and a proposal is pinned.** It names the original (by hash), the
conversion (by hash) and the revisions of the spec it was made against (the text head at least). It
changes nothing until it is accepted.

**There is no lock.** Accepting is the same compare-and-set the store already uses for every head:
the text head moves from the revision the proposal was made against to the new revision, or the
accept is refused as `moved`, naming both revisions and saying that nothing was written. A refusal is
not a loss: the proposal stays, and the owner is shown what the spec has become.

**A proposal that moved is rebased, not redone.** When the original is unchanged and only the spec's
text moved, SCE offers the conversion again against the new text, carrying every decision the owner
already made on blocks that did not change; only blocks the move touched are asked about again. When
the original itself is replaced, the earlier proposal is `superseded` and kept.

**Who else is looking is shown, never enforced.** A reviewer holds a lease on a proposal, with a
heartbeat and an expiry, using the same lease machinery as a generation request. The lease is read by
other people as a line of text ("Min-jun is reviewing this, last seen 4 minutes ago"). It disables
nothing, and its expiry changes nothing but that line.

**The life of a proposal is one document.** Its states (`pending`, `accepted`, `superseded`,
`rejected`) and the events between them are written once, in `import_life.scxml`, and the Rust and
Python code are generated from it, as `request_life.scxml` already does for generation requests (ADR 0016).
The refusal sentences are the product's words, identical in both languages.

**Decisions are data.** Each decision the owner makes on a block (keep, correct, drop, add from the
original) is appended to a log beside the proposal, never applied by editing the conversion in place.

## Alternatives rejected

- A hard lock on the spec while a proposal is pending. With a server, one person who closes a window
  blocks everyone, and releasing it by force needs rules that grow without end.
- A timed lock. The time a person is away cannot be predicted, so any value is wrong for somebody, and
  expiry races with the very accept it protects.
- Applying the conversion to the spec at once and reviewing afterwards. The record would then say the
  spec says something nobody has checked.

## Consequences

- The store needs no new concept: an original is a blob, a conversion is a blob, a proposal is a
  small record with a state, a decision log is an append-only log, a lease is a lease.
- Review has an end: a proposal is accepted, superseded or rejected, and each is a fact that is kept.
- The screen shows `moved` as a first-class state with a path forward, not as an error.
- The authoring tools gain a PDF path, so `ingest.py` stops refusing `.pdf`; what a reader could not
  carry is still reported, never discarded, as `ingest.py` already requires of every reader.
