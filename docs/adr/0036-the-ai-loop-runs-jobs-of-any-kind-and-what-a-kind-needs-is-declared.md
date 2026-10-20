# ADR 0036 — The AI loop runs jobs of any kind, and what a kind needs is declared

- Status: Proposed (the owner asked on 2026-10-18 for the conversion work to be designed generally and to
  be extensible, and for the workbench to be the first user of the AI loop); not yet implemented
- Date: 2026-10-18
- Scope: how the loop that takes a job, runs it with an AI client or a program, checks what came back and
  publishes it is made to serve more than one kind of work; what an extractor, a reader and a kind of job
  each are; and in what order they are built
- Related: `docs/adr/0016-the-rules-of-a-generation-request-are-one-document.md`,
  `docs/adr/0025-an-import-is-a-proposal-pinned-to-the-revision-it-was-made-from.md`,
  `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0027-the-spec-store-is-a-contract-with-a-folder-and-a-database-behind-it.md`,
  `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`,
  `docs/adr/0035-a-reader-is-declared-as-data-and-what-a-reader-says-is-checked.md`

## Context

The loop that exists today does one thing. A request names a spec, a runner claims it with a lease, a
generator writes a model and a requirement list from the spec's text, the product checks the draft and
refuses it with words the generator is told, and a draft that passes is published. Everything in it is
written for that one thing: the job carries a spec's text and answers, the generator trait returns a
model and a list, the instructions and the answer's schema are about statecharts, and the tools a client
may use are nine tools of the authoring server scoped to one spec.

Converting an original (ADRs 0025, 0026, 0029, 0032, 0035) is the second kind of work, and it is unlike
the first in every one of those places: its input is a document's blocks or a page's picture, its output
is blocks that cite their sources, its check is the citation check of ADR 0026, its unit of work is a
page and not a spec, the client needs no tools but may need to be shown a picture, and what leaves the
computer is the original's content. A third and fourth are already named (the owner asking a question
about a design, ADR 0033; a review of a shared list, ADR 0034). If each is added by copying the loop and
editing it, the loop's hard-won parts (leases, time, cancellation, the journal, the sandbox of a client,
the reasons a job waits) exist once per kind and drift.

Three lessons of the loop as it stands apply directly. A client's CLI changes under the application
(Codex 0.159 to 0.163 changed what a sandbox lists), so nothing about a client is decided by a list of
versions; it is asked, and what it cannot do is declared. A refusal is an answer and not a failure, so a
check's verdict is read before a run's error flag. And a stand-in that does not behave as the real thing
hides defects (the refusal count was wrong until a real run), so each client has a live test.

## Decision

**Three seams, each open by data and closed by a contract.** What is variable is a format, a reader and a
kind of job. Each has one contract and a registry that is filled by declarations, and nothing outside its
seam learns what is on the other side of another.

1. **An extractor reads a format** (ADR 0026). It turns one member of an original into an extraction:
   blocks, the list of what it could not carry, and the members it derived (renditions, ADR 0032).
2. **A reader reads pixels** (ADR 0035). It turns a picture into blocks. It is a manifest, and what runs
   it is a runtime.
3. **A kind of job is a piece of work the loop runs.** Generating a model is one kind and converting
   blocks is another.

The loop knows kinds and runtimes. A kind knows nothing of which client runs it, and a runtime (Claude
Code, Codex, a model server, a program) knows nothing of which kinds it is asked to run.

**An extractor is a contract and a registry.**

- The contract: bytes of one member, the limits of the jail, and a way to say what it is doing, in; an
  extraction out, as the schema of the extraction fixes it (a `pre-release` surface, listed in
  `SCE_WIRE_CONTRACTS.md` when it exists). An extractor never calls a model and never opens a path.
- An extractor declares, as data: the formats it reads (a suffix and a sniff of the first bytes, because
  a suffix is a claim), the ways of reading it gives (`text-layer`, and for a mixed document the unit it
  chooses by, a page), what it needs to run (nothing, or a named pack), and the limits it asks of the
  jail.
- The registry picks by format and by what is available, page by page for a document whose pages differ.
  A format nobody reads and a pack that is not installed are ordinary states with a typed reason
  (`format-unread`, `pack-missing`), shown, and never an error: the person is told which pack would read
  it. The readers that exist in `ingest.py` are the first extractors and move behind the contract
  without changing what they say.
- Adding a format is adding an extractor and its declaration. No screen, no tool and no job kind changes.

**A job kind is a declaration and four small parts.** A kind is registered with:

- a name and a version;
- the schema of its input and of its output, so that an adapter can ask a client for a structured answer
  without knowing the kind;
- its recipe, the versioned document that tells a client what the work is (ADR 0029), which the kind
  owns and a runtime only carries;
- the tools a client may use for it (possibly none) and the attachments it hands over, both scoped to the
  job and not to the whole folder;
- what it requires of a runtime (`tools`, `vision`, `structured-output`, a longest input) and what it
  sends out (`text`, `pictures`, nothing), so that the loop can refuse a pairing before it starts and say
  why;
- the unit of work, and how units are made from an input and put back together;
- its check, which is the product's own function over the output and decides what is refused, with
  words, and what is accepted (for conversion, ADR 0026's citation check; for generation, the core's
  check of the model).

The parts the loop provides, once, for every kind: the claim and the lease with its heartbeat, the time
the job is given and its deadline, cancellation, the number of jobs one connection runs at once, the
journal of turns and checks, the retry in which a check's refusal is told to the client, the typed reason
a job waits, and publication through the store's compare-and-set.

**A runtime is a contract too, and it is the other half of the Generator that exists.** The generators of
today (Claude Code, Codex, a model server) each fuse two things: how a client is run and what a spec is.
They are split. A runtime takes a task (the instructions, the schema of the answer, the tools it may use,
the attachments, the deadline, a way to say what it is doing) and returns a structured answer or a typed
failure. It declares what it can do (`tools`, `vision`, `structured-output`, the longest input, whether
it can be cancelled). The runtimes keep everything that has been learned of running a client safely: the
features of Codex that are switched off and the sandbox that is asked of it, the stop of a run that
reports a command, the one place a login is kept, the verdict read before the error flag. None of that is
in a kind.

A kind is paired with a runtime by their declarations. A kind that needs `vision` and a connection whose
runtime does not declare it is a job that waits with the typed reason that says so (ADR 0029), and a job
is never given to a runtime nobody chose.

**A runtime that is a program is the same loop.** An extractor runs in a jail as a program and not as an
AI client, and an OCR program on the person's machine is a reader that is a program (ADR 0035). They are
runtimes of the same contract with no model in them: they claim a job, run it, and publish what the kind
checks. So one queue, one lease, one journal and one set of reasons serve work that needs no model and
work that does, and a job that needs no model is not a separate subsystem to keep in step.

**A picture is handed over as an attachment, not as a tool.** A reading job gives a client one picture
and asks for blocks. The attachment travels the way each client takes one natively (a content block of an
image to Claude Code, an attached file to Codex, an image part of a message to a server that speaks the
chat protocol), and the kind's tool list is empty, so the client reads and answers and can do nothing
else. The picture that is handed over is the rendition of ADR 0032 and is recorded as what the client
saw. How each client takes an attachment is held by a live test against the real client before the
runtime declares `vision`; a runtime that has not been tried declares nothing.

**A document is split into units of work and put together by the product.** A kind says what its unit is.
Conversion's unit is a page, because a whole document in one turn runs past a client's time and context,
and a retry or a failure should cost one page. Each unit is a job; the product puts the blocks back in
order and records which unit each came from. A unit that failed is retried and shown by itself, and the
others are kept. How many units run at once is the connection's `places`, as it is for any job.

**What leaves the computer is declared by the kind and decided by the person.** A kind declares the
classes of data it sends (text of the spec, text of the original, pictures of the original). The
workspace's policy names which classes may go to which kinds of runtime, and before a job is made the
sheet says in one line how much goes and where. A job whose class a policy forbids waits with a typed
reason. A job that sends nothing out (a program on the machine) is never held by that policy.

**The workbench uses nothing a second client could not.** The kinds, the registries, the store's job
records and the commands are SCE's contracts, and the workbench holds no private path to any of them: it
is the first client of SCE's AI loop, so what is hard to use from it is hard for everyone and is fixed in
SCE. The loop's code that is specific to a runtime stays in the workbench's core as it is now, and the
kinds, the extractors and the store's records are SCE's.

## How it is built, in order

Each step lands with its tests and is useful before the next; none is a placeholder for a later one.

1. **The extraction.** The schema of an extraction (blocks, the list of what was not carried, members), the
   extractor contract and registry, the readers of `ingest.py` behind the contract, and a PDF extractor
   for a document with a text layer whose dependency is a named, optional pack. A format or a pack that is
   missing says so with a typed reason. Needs no model and no store change beyond the manifest of ADR 0032.
2. **Extraction as a command.** `extract` in the authoring tools writes the extraction of one file and
   names it by its revision, so a host that has no store for originals yet can already show an original's
   blocks beside it. Not an AI tool: a client of a run is never offered it.
3. **The original and its extraction in the store.** ADR 0032's bundle (a manifest of members named by
   hash) and the extraction kept by hash, as blobs of the specs folder, with the commands that add and
   read them. This comes before the job record because a job's inputs are hashes of these, and a record
   that names what the store cannot hold is a record with nothing behind it (found on 2026-10-18, when
   the request of today was read for what it would take to generalise it).
4. **The conversion and its check, with no model.** The form of a conversion (Markdown blocks, each
   citing the blocks of an extraction it was made from), SCE's check of the citation (ADR 0026: what
   matched, what is not in the original, what the conversion left out), and the converter for what an
   extraction carries as text. This is on the way of every conversion and needs no job: an original that
   carries its text (Markdown, HTML, a word file, a PDF with a text layer) is converted by rule, and 286
   of the 289 PDFs over 50 kB on the owner's machine had a text layer. The check is the one a conversion
   by a model is held to later, so it is written once, here, and the converter by rule is its first
   client and its control: a conversion it makes must match every block.
5. **The proposal and its accept.** ADR 0025's proposal pinned to the revision it was made from, the
   accept that is the store's compare-and-set or is refused as `moved`, and the log of the owner's
   decisions on blocks; then the workbench's import screen over them.
6. **The job record.** What a request is today is tied to a spec's text: it is registered against the
   heads of the source and the answers, refused as `moved` when they move, one is open at most, and it
   publishes a model and a list. What is not tied to it (the life with its lease, attempts and fence,
   the places of a connection, the journal, the pin, the key that makes a repeated press one request) is
   already one document and one store. Reading the code for what generalising it takes (2026-10-18)
   showed two things: its inputs are the two heads, and a spec that is new has no text and an import
   must work for it, so the record has to stop requiring a text head for a kind that does not read one;
   and its life, its pin, its lease and its outcome are fields of the request that a second record would
   copy unless they become one core that the request and the job both carry (`#[serde(flatten)]`, so
   that a record written before reads as it did). So it is a refactor of the request store and not an
   addition to it, and it is done when a kind needs it, which is the page that is a picture, and not
   before: nothing earlier on this list is a job. A table of kinds in the specs crate then says, for
   each, what it is registered against, how many may be open (one per spec, or one per unit), whether
   the move of a head supersedes it, and what it must have made before it may publish; generating a
   model is the first kind and its existing tests hold it.
7. **The loop split.** The runtime contract and the kind contract in the workbench's core; Claude Code,
   Codex and the model server behind the runtime contract with their tests and live tests unchanged;
   generating a model as a kind.
8. **Pictures.** The attachment path for each runtime, tried live one at a time; the reader manifests of
   ADR 0035; conversion of a page that is a picture as a kind of job, with its unit and its check from
   step 4; the omission check from a layout and the reader's text.
9. **Policy and the sheet.** The data classes, the workspace's policy, the line that says where pages go.

**A picture the owner chose is read before the job record (amended 2026-10-20).** The owner asked that a
picture in a word document can be handed to AI to read, and that this belongs to the product, not to a later
step. Of step 8 this needs only the kind and one runtime's attachment path, and not steps 6 and 7: a picture is
one unit, one answer and seconds of a client's time, so it is asked as one question while the owner waits, the
way a division is drawn (ADR 0040). The job record is still built when a kind needs a queue (a scanned document
of many pages), as step 6 says. In order:

- **The kind, with no model** (`reading.py`). The unit is one picture a figure block marks, taken out of the
  document as it is and named by its hash; what cannot be handed over is refused by a typed reason. The recipe
  gives the client the picture and its name and none of the document's words, so nothing it writes can be
  completed from the text beside it. What it read is added after the mark, which stays. Each block names the
  picture it was read from; the conversion keeps one reading per picture (what was handed over, by hash, which
  runtime, model and recipe, what it could not read); and the check asks about every such block as *read from
  a picture* and matches none, since nothing in the original can be compared with it.
- **The attachment path for Claude Code**, tried live first: an image content block on standard input with
  `--input-format stream-json`, no tools, and `--json-schema` for the answer's form, which a run on
  2026-10-20 answered in that form. The runtime declares that it reads pictures only once its live test
  passes, as above; the others declare nothing until theirs do.
- **The attachment path for Codex**, tried live the same day: `codex exec --image=FILE` with the picture
  a file in the run's own folder, the confinement of a draft (the sandbox that reads that folder and
  writes nothing, no network, no question, no web search, every unreviewed feature switched off), no
  server at all, and `--output-schema` for the answer's form. A reading is stopped for any item the
  client reports but its own words and reasoning, a call of the authoring server included, which a
  draft may make and a reading may not. A run answered the form for a picture file and for an image
  drawn from a PDF. Codex does not say which model answered, so a reading records the model the
  connection pinned and else the word `codex-default`. A model server declares nothing until its own
  live test passes.
- **The import reads what the owner marked**, in the import's own step, and the screen gives a figure a third
  state beside chosen and left out.
- **A picture in a PDF** (landed): an image object a page holds is a figure block named by its page and place
  (`page-0003-image-02.png`) and by the hash of the bytes the document stores for it, and what is handed over
  is a PNG drawn from its decoded samples (`pdf_pictures.py`). The picture is drawn from the document each time
  it is read, so the bundle keeps no rendition member for it yet; the hash a reading records is of the PNG
  that was handed over, and a rendition stored at import (ADR 0032) is the step that follows. A drawing made
  of strokes is not an image object and is not placed; the extraction says so.

## Alternatives rejected

- Copying the generation loop for conversion. It would give each kind its own lease, deadline, journal and
  waiting reasons, and a client's change would have to be absorbed in each.
- One generator trait with a branch per kind. The kinds would be inside the runtimes, so a new kind would
  mean opening every client, which is the coupling this removes.
- Giving a reading client a tool that fetches a page. A tool is a way to do something; a client that is
  only meant to read should have nothing to do. A tool scope of none is the strongest one.
- A separate subsystem for work that needs no model. It would need its own queue and its own account of
  why a job waits, and a person would meet two ways to ask the same question.
- Choosing the extractor for a PDF by suffix alone. A suffix is what a file is called; a sniff of its
  bytes and a page-by-page look at what each page carries is what it is.

## Consequences

- A fifth kind of work (a review of a shared list, an answer to a question about a design) is a
  declaration, a recipe, a check and a unit, and no change to the loop or to a client.
- A client that changes (a new release of Codex, a new way of attaching an image) is a change in one
  runtime and its live test, and no kind notices.
- Generation moves behind the contract before conversion is built on it, so the first user of the new
  seams is the work that already has the most tests; a regression shows there.
- The wire surfaces this adds (an extraction, a job record, a kind declaration) are `pre-release` in
  `SCE_WIRE_CONTRACTS.md` when they exist.

## Not decided here

- Which library the PDF text-layer pack uses and how a pack is installed and kept (a download of ADR 0035's
  kind is the direction).
- The schema's field names and its version.
- The workspace policy's wording and the threshold at which a block is queued for a person (ADR 0026).
- Whether a kind may call another kind (a review that asks a question), and how a cycle is refused.
