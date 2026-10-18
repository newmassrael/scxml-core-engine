# ADR 0035 — A reader is declared as data, and what a reader says is checked

- Status: Accepted as the direction (decided on 2026-10-18, on the owner's instruction to decide after
  further investigation); not yet implemented
- Date: 2026-10-18
- Scope: which part of reading an original needs a model, how a reader (a model or a program that turns an
  original into blocks) is added, replaced and chosen, what the product asks of any reader and does with
  what it says, where positions come from, and where reading runs
- Related: `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`

## Context

ADRs 0026 and 0029 say that an original becomes blocks, that a provider the person chooses converts
them, and that the product checks every conversion and does not rank models. They do not say how a model
that reads pixels is added to the product, which models are worth offering, or what to do when a model
is wrong in a way it does not report. The owner asked that the model be easy to change, and asked for
this to be decided from measurement, not from the benchmark tables published for the models.

A trial was run before deciding (2026-10-18, on the owner's machine: a laptop GPU with 8 GiB, and the
CPU of the same machine). The sample is a synthetic three-page Korean requirements document with
nested items, a table with row and column merges, a second table, one drawing, a running header and
page numbers. It exists as a `.docx`, as a PDF with a text layer, and as two scans of that PDF: a mild
one (150 dpi, a slight tilt, grain) and a severe one (110 dpi, a larger tilt, perspective, uneven
light, a shadow, low contrast, JPEG damage). Two lines of the scans are blurred until nobody can read
them, and two characters are printed as look-alikes (a letter O for a zero). The truth is the source of
the sample; 52 units are readable (sentences, table cells, drawing words) and the blurred lines are
judged apart from them. It is one small synthetic document, cleaner than a real scan, and every number
below is from it and from no other.

What the trial measured, of the 52 readable units reproduced exactly:

| Reader | Mild scan | Severe scan | Gives boxes | Says what it could not read |
|---|---|---|---|---|
| Claude, read by an agent that had not seen the truth (3 trials each) | 52, 51, 50 | 52, 52, 52 | no | yes: both blurred lines, in all 6 trials |
| PaddleOCR-VL 1.6 (Apache-2.0), GPU | 45 | 40 | no | no |
| MinerU2.5-Pro (weights Apache-2.0), GPU | 44 | 36 | yes, normalised | no (wrote nonsense for a blurred line) |
| dots.ocr (MIT), GPU | 43 | 39 | yes, about a line off | no |
| GLM-OCR (MIT weights, Apache-2.0 code), GPU | 27 | 22 | no | no |
| Classic line OCR, Korean (PP-OCR family), CPU | 27 | 29 | yes, per line | a score per line |

Claude was not flawless either: in two of the three mild trials one table cell had the active form of
a verb (to close) where the page has its passive form, one syllable away, and the reader's own notes
said it was transcribed exactly. No text was invented, and the two lines blurred on purpose were marked
unreadable every time. The misses of the
other readers are of the same kind and more frequent: a character or a syllable replaced by a look-alike
one (a state name written as a different word one syllable away), which changes what a requirement says
and is not marked. Native sources were read without any model: Docling (MIT) reproduced the text layer
PDF and the `.docx` with every table cell and the merged cells repeated, and gave a page and a box for
each block of the PDF; the current reader of this repository reads the same `.docx`'s text but leaves a
vertically merged cell empty and a horizontally merged row one cell short, so its columns shift.

Speed, on one page: on the GPU, 6 seconds (PaddleOCR-VL), 23 to 29 (dots.ocr), 42 to 126 (MinerU,
through `transformers`); on the CPU alone, 381 seconds (PaddleOCR-VL), 755 (GLM-OCR, with the machine
shared), and no answer after 25 minutes (dots.ocr). A classic line OCR took about 30 seconds a page on
the CPU.

What a reader's own token probabilities say of its mistakes (dots.ocr): on the mild scan, 4 of 5 wrong
characters were below 0.6 and 7 right tokens were too, of 509; on the severe scan, 5 of 9 and 37. The
look-alike characters were read with probability 1. A classic OCR's line score below 0.9 caught 5 of 9
and 6 of 10 wrong lines, and warned on 3 of 34 and 10 of 29 right ones.

Inputs made to hurt a reader were run in a jail without network, with a read-only disk but its own
folder, 4 GiB of memory and 150 seconds. A 2 MB file whose document part expands to 600 MB, and a PDF
whose one page is 14400 by 14400 points, were stopped by the jail's limits for Docling, and the
current reader of this repository did not finish the first within 150 seconds. A document that names an
external entity and one with an entity expansion were refused by both, and nothing of the disk was read
into the text; a file of random bytes and a truncated PDF were refused by Docling with an error.

## Decision

**Reading is split by what the original carries, not by model.** A native original carries its text
(Markdown, HTML, plain text, `.docx`, a PDF with a text layer) and is read by an extractor, with no
model. An image-only original (a scan, a page that is a picture) needs a reader that reads pixels. A
PDF with a text layer for some pages and none for others is read page by page by what each page has.

**A reader is data.** A reader is described by a manifest and the product has no model name in its
code. A manifest names the kind of runtime (`client`: an AI client the person connected, through the
authoring tools; `endpoint`: a server that speaks the common chat interface, on the person's machine or
elsewhere; `key`: the host calls a model with a key, ADR 0031), the files with their size and hash, the
runtime arguments, the instruction it is given, the kind of output it gives (`plain`, `markdown`, or the
name of a structured format an adapter reads), what it declares it can do (`ocr`, `vision`, `boxes`,
`tables`), and the licence of what is distributed. Replacing a model is replacing a manifest. An adapter
reads a format of output, not a model: any model that writes a format the product reads is a manifest and
no code, and a format nobody reads yet is one adapter. The trial's driver was written this way and ran
three different models without a change to the driver.

**What a reader says is one normal form.** Every reader's output is reduced to blocks: `page`, `kind`,
`text`, `how_read` (`text-layer`, `ocr`, `vision`), a place, and the marks the reader made. The place
is the page and, when there is one, a box with a stated precision (`exact`, `approximate`, `none`).
This amends ADR 0026, which asks for a page and a box for every PDF block: the page is required and the
box is not, because a reader that is chosen for how well it reads (the trial's best reader of text gives
none) cannot also be required to say where. A native `.docx` block is placed by its part and ordinal as
that ADR says.

**Nothing a reader says is believed unchecked.** ADR 0026's check of every citation stands. Every block
a reader gives is in one of three states: *read*; *check*, when the reader marked it unsure, when its
token probability or line score is below a threshold the product shows, or when SCE's check of the
citation failed; *unreadable*, when the reader said so or when a layout that has boxes sees ink where the
reader wrote nothing. The probability and the score are hints that put a block into *check*; they are
never what makes one *read*, because the trial's look-alike characters had probability 1. A reader that
gives no boxes cannot show an omission: for it the count of what is missing is *unknown* and never zero,
and the review shows the original page beside the conversion as the design does. The person decides
every block that is not *read*, and the decisions are kept as ADR 0025 says.

**The product does not rank readers.** ADR 0029 stands: the person chooses, the product keeps the
conversion honest whatever model made it. The manifests the project ships are listed in alphabetical
order with what they declare and nothing about quality, and only readers whose licence allows
redistribution are shipped. A job for an image-only original with no reader that declares `vision`
waits with a typed reason that says so; it is not given to a small reader that nobody chose.

**A reader that runs on the person's machine is a download, never part of the installer.** The files of
a manifest are fetched on the person's request, checked against their hash, and kept outside the
installed program. Before the first job the product says in one line where pages go, and measures one page
on that machine and says how long a page takes: a page took minutes on a CPU in the trial and seconds on a
GPU, and the person is told which they have before a document of many pages is started.

**Reading runs in a jail.** An extractor or a reader that is a program on the machine runs in a process
with no network, a read-only disk except its own folder, a memory limit and a time limit, and the
product refuses an original whose containers expand past a stated size or whose page is larger than a
stated size before it is handed over. The current `.docx` reader of this repository gets the same limits:
it reads a whole document part into memory.

**Native extraction keeps merges.** A vertically merged cell repeats the text of the cell that starts it
and a horizontally merged cell repeats across its columns, as the table of the trial's Docling output
does, so that a decision table keeps its columns. The extractors that read a `.docx`, Markdown, HTML and
plain text stay in the product and are small; the reader of PDF tables (a layout and table model, about 1.5
GB with its Python environment in the trial) is an optional pack, downloaded as a reader is, and the
review says that a table of a PDF was not read when the pack is not there.

**A trial set is kept for the project, and is not in the product.** The sample, its truth and the scoring
that produced the table above are kept as a development tool, and a manifest the project ships has to
normalise on it and declare its boxes truthfully. This is a test of the project's own parts; the product
does not run it, rate with it or show its result (ADR 0029 rejected approving models inside the product).

## Alternatives rejected

- Putting one model in the installer. It would be 1 to 3 GiB, minutes a page for a person without a GPU,
  and a model that was wrong in the trial in ways it did not say; and the product would be tied to it.
- Choosing the best-ranked open model on a published benchmark and fixing it. The ranking did not predict
  the trial: a model third in a published ranking of document readers was the worst reader of Korean
  here by a wide margin (27 units against 43 to 45 on the mild scan), and a model outside the first three
  of that ranking (dots.ocr) read within two units of the one ranked second. A choice made once from a
  table would have rested on what the trial did not show, on a language and a kind of page that the
  table was not made from.
- One tool for everything (Docling alone). Its layout step gave no boxes for text on a scan without a
  reader, and with a classic OCR attached it read the scan badly. It is the right extractor for native
  originals and not a reader of images.
- Believing a reader's probabilities. They caught some of the wrong characters and warned on many right
  ones, and did not move at the look-alike characters at all.
- Requiring a box for every block. It would exclude the reader that read best.
- Borrowing the boxes of a cheap line detector for the words of a reader that gives none. It was tried
  once against a truth that was too coarse to judge it, so it is not established either way; it is left to
  the follow-up below and is not assumed to work.
- Checking a reader in the product by running the trial there. That is the benchmarking inside the product
  that ADR 0029 rejected.

## Consequences

- ADR 0026's requirement of a box is relaxed to a page, with a box that says how exact it is.
- The reader manifest, the adapters for `plain`, `markdown` and the structured outputs of the readers
  that give boxes, the jail, the size limits, the three states of a block and the review's use of them
  are new, and so is the repair of the `.docx` reader's merged cells.
- The Claude path for an image-only original needs the pages as pictures the authoring tools can hand
  over (ADR 0032's renditions) and a mark per page in the text of the original, so that the existing tool
  that hands over a marked picture serves a page. That tool refuses a picture larger than 4,000,000 bytes,
  so a page is reduced to what the tool takes and the review shows the reduced picture, as ADR 0032 says.
- The trial's open questions are follow-ups and not decisions: whether boxes can be taken from one reader
  for the words of another, with a truth that can judge it; how a reader that gives no boxes can be made to
  show an omission; and the same trial on the owner's own documents, scans included, before the first
  release, with nothing of those documents copied into a repository.
- Where a manifest's licence is not a plain permissive one (a project licence with added conditions on the
  size of the user, or a licence on the weights different from the licence on the code) the manifest is not
  shipped until the terms have been read in the files and not in a summary of them.
