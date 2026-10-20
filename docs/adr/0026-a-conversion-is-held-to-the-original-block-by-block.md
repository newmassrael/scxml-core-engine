# ADR 0026 — A conversion is held to the original block by block

- Status: Accepted as the direction (the owner decided it on 2026-10-14); not yet implemented
- Date: 2026-10-14
- Scope: the form between an original document and its Markdown conversion: positioned blocks, the
  mapping a conversion makes onto them, and the counts SCE derives from that mapping
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `tools/authoring/sce_author/ingest.py`

## Context

Two things must work for both a text PDF and a scanned PDF as well as for a word-processor file: a
person must be able to compare the conversion with the original, and SCE must be able to say how much
of the original the conversion accounts for without taking the AI client's word for it. A table that
flattens wrongly produces a document that reads as though the specification never said anything
(`ingest.py` records this as its founding hazard), and OCR text can be confidently wrong.

If each format had its own comparison, every new format would need its own screen and its own check.

## Decision

**One intermediate form: positioned blocks.** An extractor reads one format and writes an
extraction: a list of blocks, stored by hash like any other blob. A block has an id (stable within the
extraction), a kind (heading, paragraph, list item, table, figure, attachment), its text (a table keeps its cell
grid), where it came from, how it was read, and a confidence.

- Where: for a word-processor file, the part and the ordinal of the paragraph or table; for a PDF, the
  page and the box on it.
- How read: `text-layer` (the document carries its own text, confidence exact), `ocr` or `vision` (the
  text was recognised from an image, confidence is the recogniser's, and is never reported as exact).
- A figure is a block that points at a picture by its hash, written `[picture: NAME]` in the text as the
  authoring tools already do; what a bundle, a name and a rendition are is in ADR 0032. An attachment is
  an object embedded in the document, marked `[enclosed object: NAME (KIND)]`.

Only an extractor knows a format. Everything after the extraction (conversion, comparison, review,
coverage) reads blocks and nothing else, so a new format is one extractor and no new screen.

**A conversion cites its sources.** An AI client converts blocks to Markdown and records, for each
Markdown block, the original blocks it was made from. A Markdown block that cites nothing is *not in
the original*. An original block that no Markdown block cites is *missing*.

**SCE checks the citation; it does not trust it.** SCE compares the normalised text of each Markdown
block with the text of the blocks it cites. A block whose text the citation does not support is
reported as *check*, whatever the client said. The counts the owner sees (matched, not in original,
missing) are SCE's counts. A conversion cannot report itself complete.

**Low confidence has to be decided.** A block read by OCR or vision, or one SCE could not confirm, is
placed in the review queue first. The proposal cannot be accepted until each such block has a
recorded decision. A text PDF has no such blocks by construction and is reviewed on the exceptions
only.

**What a reader could not carry is reported.** As `ingest.py` already requires, every extractor
returns the blocks and a list of what it could not carry (a picture it could not place, a table it
could not recover, a page with no text and no image). The list is shown, not logged.

**What could not be carried is a code and its facts, and the sentence is written in `notes.py`.** An entry
says what happened as a code and the facts that go with it (`pdf-pictures-unplaced`, a count, a limit and
the pages), and never only as a sentence: an extraction is named by its hash, so it cannot hold the sentences
of the language its reader happened to use. The sentences are written once per language beside the code, in
`notes.py` (`en`, `ko`), and a code with a sentence in one language and not the other does not load; this is
the rule `open_matters.rs` holds for what a design leaves open, and for the same reason: a screen that
translated them would be a second author of the sentences. The extraction keeps the English sentence beside
the code, for a program that knows no code and for a reader whose language has no sentence for a code this
version does not know; a stored sentence is never the source of a translation. `render-notes --lexicon LANG`
says a list of entries in a language, and a host that shows them asks it and writes none. An entry with no
code (an older tool's) is said as it was stored.

## Consequences

- The comparison screen, the exception queue and the coverage meter work for every format.
- A scanned PDF is supported with its weakness stated: its blocks are flagged until a person has
  looked, and the flag is a count, not a feeling.
- The extraction is part of the record. Converting the same original again with another model gives a
  second conversion over the same blocks, and both are kept.
- The extraction format needs a schema file and a version; that is left to the first implementation and
  recorded in `SCE_WIRE_CONTRACTS.md` as a `pre-release` surface when it exists.
- Not decided here: which OCR or vision engine an extractor uses, and the confidence threshold that
  puts a block in the queue.
