# ADR 0017 — A rank wider than the page is set as several ranks

- Status: Accepted and implemented (the owner asked for it on 2026-10-11, after the measurement below)
- Date: 2026-10-11
- Scope: `sce-build/src/diagram` (`layout.rs`, `fit.rs`), the `cli/diagram-does-not-fit` row of
  `docs/SCE_ACCEPTED_SUBSET.md`
- Related: `docs/SCE_ACCEPTED_SUBSET.md` (the refusal), `SCE_ERROR_CONTRACT.md` (the code is unchanged)

## Context

`sce-codegen diagram` draws one figure per container: the container's direct children as boxes, ranked by the
arrows between them, one row per rank, with the table of the transitions under the drawing. A figure is never
shrunk below the minimum type size (7 pt): one that does not fit the page is refused as
`cli/diagram-does-not-fit`, with the size it needs.

A `<parallel>` of many regions is one rank: its regions share no arrow, so every one of them is rank 0 and the
row holds them all. A statechart of seven regions (one an AI writer produced from a specification) needed 1345 pt of width
against the 1106 pt an A3 landscape page gives, and was refused, while the figure was 157 pt tall on a page with 757
pt of height to spare. The refusal told the author to split the container, which is a change to the document made
to suit the drawing.

## Measurement

Before the change, 2,816 SCXML documents (1,085 committed, and 1,731 distinct documents an AI writer produced from
specifications) were drawn on A4 portrait and on A3 landscape:

| | A4 portrait | A3 landscape |
|---|---|---|
| drawn | 2,124 | 2,280 |
| refused as not fitting | 544 | 388 |
| of which too wide only | 194 | 37 |
| of which too tall only | 117 | 331 |

After the change, with the same two pages and the same documents:

| | A4 portrait | A3 landscape |
|---|---|---|
| drawn | 2,158 (34 more) | 2,298 (18 more) |
| refused as not fitting | 510 | 370 |
| drawn before and drawn now, figures identical to the byte | 2,124 of 2,124 | 2,280 of 2,280 |
| drawn before and not now | 0 | 0 |

The change is safe by the second-to-last row: no figure that fitted before is drawn differently. It is small by the
first two: most of what is refused is too TALL, not too wide (331 of 388 on A3), which this change does not address.

## Decision

A rank wider than the width a row may take is set as several consecutive ranks. The ranks are cut in the order the
figure draws its boxes: a rank is filled until the next box would pass the limit, and the next rank is started.

Boxes of one rank share no arrow (an arrow between two boxes puts them in different ranks), so cutting a rank
redraws nothing: an arrow that now crosses one of the new ranks gets a slot in it like in any rank it crosses. The
layout is otherwise as it was: the same ordering sweeps, the same coordinates, the same routing.

The limit is the page's printable width. What it does not count (labels that reach beyond the boxes, the table under
the figure) can still make a figure wider than the page; the limit is then lowered by what passed it and the figure
laid out again, at most six times. A figure that still does not fit is refused as before.

A rank that fits is not cut, so a figure that fitted is laid out exactly as before: the layout takes the limit as an
option and a test holds `None` and a limit larger than the figure to the same result.

## Consequences

- The refusal for width becomes rare and is for a figure whose single box is wider than the page.
- The order of the regions in the figure is the order the figure draws them in (the states it shows as themselves,
  then the folded ones), which is not always the order the document writes them in.
- Rows are separated by the same gap as any two ranks (four lines), which is generous for rows that are only a
  wrapped rank; a tighter gap for the rows of one rank is a later refinement.

## What this does not decide

- A figure too TALL for the page. Of the 370 figures still refused on A3 landscape, most are refused for height: the
  drawing, and the table of the figure's transitions under it, are one sheet. Setting a figure on several sheets
  (the table continuing on the next one, and a drawing too tall for a sheet cut between ranks, with the arrows that
  cross the cut marked on both) is the next step and needs its own decision.
