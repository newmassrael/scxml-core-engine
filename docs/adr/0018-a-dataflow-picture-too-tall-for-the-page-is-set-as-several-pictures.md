# ADR 0018 — A dataflow picture too tall for the page is set as several pictures

- Status: Accepted and implemented (the owner asked for it on 2026-10-11, after the measurement in ADR 0017)
- Date: 2026-10-11
- Scope: `sce-build/src/diagram/kinds/flow.rs`, the `cli/diagram-does-not-fit` row of
  `docs/SCE_ACCEPTED_SUBSET.md`
- Related: `docs/adr/0017-a-rank-wider-than-the-page-is-set-as-several-ranks.md`

## Context

The picture of a transform, condition, filter or validator is three columns of boxes: the inputs, the computation
(an expression, the settings of a filter, a rule of a validator), and the outputs, with an arrow where an expression
reads an input and where a computation gives an output. A transform of 40 inputs and 40 outputs is one column of 40
boxes, taller than any page, and was refused (`'<name>: dataflow' needs 523 x 1355 pt ... page gives 510 x 757 pt`).
Of the figures `sce-codegen diagram` still refused after ADR 0017, this picture was the larger part: 393 of 507 on A4
portrait and 287 of 367 on A3 landscape, all of them too tall and not too wide on A3.

A refusal writes nothing, so a document whose picture did not fit also lost its field tables, which are the reading
of the document.

## Decision

A picture that fits is drawn as it was, one picture named `dataflow`. A picture refused for HEIGHT alone (it is
narrower than the page) is set as several pictures, named `dataflow`, `dataflow-2`, ... as a table continued on
further sheets is, and titled `<name>: dataflow (n/total)`.

The parts are cut from units in this order: each computation with the inputs it reads and the output it gives, as
the document writes the computations; then each input no computation reads; then each output no computation gives.
A picture is filled with units, in order, while it fits the page, and the next picture is started when the next
unit would not. An input read by computations in two pictures is drawn in both. Nothing the one picture showed is
left out, and an input nobody reads is still drawn with no arrow, which is the thing worth seeing in it.

A picture refused for width, or a single unit that does not fit alone, is refused as before.

## Consequences

- A document with many inputs and outputs is drawn, and its field tables are written with it.
- A reader follows the parts by their numbers; an input is on the pictures of the computations that read it, not
  at its place in the declaration.
- The cost of filling a picture is one drawing per unit (a few dozen) for a document that was refused; a document
  that fits is drawn once.
