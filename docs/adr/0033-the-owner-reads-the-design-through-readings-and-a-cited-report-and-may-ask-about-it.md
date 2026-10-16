# ADR 0033 — The owner reads the design through readings and a cited report, and may ask about it

- Status: Proposed (the owner asked for it on 2026-10-16: the specification owner cannot read pseudocode,
  needs the SCXML shown in several ways, an AI report with pictures, and a chat about the report); the
  details below are a proposal for the owner to confirm; not implemented
- Date: 2026-10-16
- Scope: how a specification owner who does not read code understands a design, what an AI report is and
  how it is held to the design, how a picture in a report is made safe, and a conversation about the
  report
- Related: `docs/adr/0026-a-conversion-is-held-to-the-original-block-by-block.md`,
  `docs/adr/0029-a-conversion-is-made-by-a-provider-that-claims-a-job.md`,
  `docs/adr/0032-an-original-is-a-bundle-and-a-picture-is-named-by-its-hash.md`

## Context

What the application shows of a design today is SCE's pseudocode page, read against the text. The owner of a
specification is the person the application is for, and does not read it. SCE already draws the same
document other ways (`sce/tools/authoring`, `views`; `sce-codegen diagram`):

- a table per container: the state you are in, the signal that arrives, where it goes, what entering does,
  the condition; a rule for a region that only watches;
- a timeline of what happens, step by step, in each scenario of the owner's scenario set, played on the real
  engine; a scenario that waits on a decision is not drawn, and the command says so;
- a simulator: a button for every signal a person can send, and for time passing, built from the engine's own
  behaviour and offered only after random walks on engine and table agree on every step, else refused;
- pictures as SVG: the document, the inside of a state, its fields, a checklist of requirements; a figure
  that does not fit is refused (`cli/diagram-does-not-fit`).

The words in these pages are the document's own names, an optional labels file the owner of the words writes
(a label never replaces a name, and the name stays as its tooltip), and the scenarios; SCE does not invent
plain words for a state, because that would put its own reading into the owner's picture. SCE has no
Mermaid. There is no AI report and no conversation.

An AI can explain a design well and can be wrong in the same voice. ADR 0026 holds a conversion to its
original by citation, and an AI client already may not write the text, the answers or an acceptance.

## Decision

**The pseudocode is one reading among several, the last one.** The design is shown, in this order, as a
table, as the scenarios played step by step, as a simulator to press, as a picture, in words, and, for a
person who wants it, as SCE's pseudocode. The first four are SCE's own pages and are not re-drawn by the
application. The application chooses between them and puts them in the owner's words; it does not render a
design itself.

**The owner's words are a record, and the AI may propose them.** A label (and a one-line note of what being
in a state means) for each state, signal, output, lane and data item is kept as the labels file SCE already
reads, beside the model, and is the owner's. The AI may propose labels. A proposal is shown as the AI's, in
the AI's colour, and is used only after the owner confirms it; an unconfirmed label never reaches a page the
owner reads as though it were theirs. The name stays as the label's tooltip, as SCE does.

**What cannot be drawn is said, not left out.** A scenario that waits on a decision, a simulator SCE
refused (too many situations, too deep, too slow, or the table and the engine disagreed), a figure that does
not fit, and a page SCE did not write are each a state of the screen with SCE's own sentence, as the model
that was not drawn is today.

**The report is a document the AI writes and SCE holds to the design.** A report is written by a provider
that claims a job (ADR 0029) from the bundle: the text, the model, SCE's own pages, the answers. It is kept
as a record pinned to the revisions it was written for, like a model, and is shown as older when any of
them has moved. A new report is a new record and the earlier ones stay.

- Every statement that is about the design cites what it is about: a requirement, a line of the pseudocode,
  a scenario, a sentence of the text. SCE checks the citation as it does a conversion's: the thing cited
  exists, a quoted sentence is in the text, a state named is in the model. Each statement is then shown as
  **checked**, **not checked** (SCE could not decide), or **the AI's opinion** (it cites nothing). A report
  cannot call itself verified, and the counts are SCE's.
- The report is in the AI's voice and says so, with the provider, model and recipe version recorded with
  it (ADR 0029). It never writes in SCE's voice: what SCE found is quoted from SCE's pages, not restated.
- Its pictures are SVG or Mermaid, written by the provider and made safe by the product (next paragraph).
  A picture cites what it draws, as a statement does, and its source can be shown.

**A picture the provider wrote is untrusted input.** The product never inserts it into the page as markup.
SVG is cleaned against an allowlist (no script, no `foreignObject`, no external reference, no embedded
image, no event attribute, no animation that loads anything), refused when it cannot be cleaned, and shown
as an image, which never runs script (as SCE's pictures already are). Mermaid is a source language, not a
picture: the provider writes the source, and the product renders it with a renderer it bundles, in a mode
that allows no script and no HTML in labels, in a sandbox with no network, and shows the result as the same
kind of image. A diagram that does not render is shown as its source and a sentence, never as a blank. The
product bundles the renderer; it does not take one from the network.

**A conversation about the report is a provider job with read-only tools.** The owner may ask the AI about a
report, a section or a picture. The answer is written by a provider (ADR 0029, a provider declares whether it
can converse) which reads the bundle through read tools only.

- It cannot write the text, an answer, a label or an acceptance, and has no tool that does; the application
  and the authoring tools already hold to this. What it can offer is a **draft**: an answer to a question,
  a sentence to add to the text, a label. A draft is shown as a draft and becomes real only when the owner
  presses the button that saves it, in the screen that saves it today.
- An answer cites as a report does and carries the same three marks. A reply that cites nothing says that it
  is an opinion.
- A conversation is an append-only log beside the report, pinned to the revisions it was about. The
  provider, model and recipe version are recorded with each reply. When the spec has moved, the
  conversation says what it was about and may be continued about the present state as a new thread.
- What is sent is said before it is sent: the question, the part of the bundle read, and to which provider.
  A pressed section, line, state or requirement is attached to the question as context and shown as such.
  Pictures follow ADR 0032: not sent unless the owner asked.
- Which providers may be conversed with is the organisation's choice as it is for conversion (ADR 0029),
  and a provider that cannot converse is shown as one, with the reason.

## Consequences

- The application needs no renderer of its own for a design: SCE writes the table, the timeline, the
  simulator and the pictures, and the application chooses and frames them. It does bundle a Mermaid renderer
  and an SVG cleaner for the report, which are the only places it draws something a provider wrote.
- The labels file moves from an optional input of a command to a record the owner edits, kept in the store
  with the model (ADR 0027) and pinned like the other parts.
- A report and a conversation are two more kinds of record, both blobs, a log and a pin; the store
  contract needs nothing new. The conformance suite gets a case that a cleaned picture is never the
  provider's bytes.
- The citation check is SCE's and is a new check: cited requirement, line, scenario and quoted sentence.
  It reads what ADR 0026's check already reads, and needs a schema and a version, left to the first
  implementation.
- The report and the conversation are a second thing a provider does, so the provider's declared ways
  gain `report` and `converse`, and the privacy sentence of every screen that sends something names it.

## Not decided here

- The recipe that makes a report, and which sections every report has.
- Which Mermaid diagram kinds are allowed, and the renderer.
- How long a conversation may run before it is begun again from its summary.
- Whether the owner may export a report, and in which format; a report with pictures is a bundle (ADR 0032).
- Whether a report may be shared with a person who cannot open the spec.
