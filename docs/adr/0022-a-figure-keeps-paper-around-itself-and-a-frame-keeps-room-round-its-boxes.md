# ADR 0022 — A figure keeps paper around itself, and a frame keeps room round its boxes

- Status: Accepted and implemented (the owner pointed at both on 2026-10-11)
- Date: 2026-10-11
- Scope: `sce-build/src/diagram/fit.rs` (`Page::with_figure_margin`), `sce-build/src/diagram/svg.rs` (`with_margin`),
  `sce-build/src/diagram/layout.rs` (`frame_inset`), `sce-build/src/bin/sce_codegen.rs` (`diagram --margin`)
- Related: `docs/adr/0019-a-loop-to-itself-is-drawn-large-enough-and-inside-its-box-and-frame.md`

## Context

Two defects were found on figures the owner was putting in a report:

1. A figure is an SVG whose canvas is exactly the figure: the outline of the picture is the edge of the file. Set on
   a slide or a page, the boxes touch whatever is next to them, and the reader of the report had to add padding in the
   page's own stylesheet, once per figure kind. The page's 15 mm margin is counted by the fit, but it is not written
   into the file, so it was not a thing a caller could ask for.
2. Inside a figure of a state that holds other states, the boxes of the children stood flush against the left, right
   and bottom of the frame drawn round them, so the frame's outline ran along the edge of the boxes.

## Decision

- `sce-codegen diagram` takes `--margin <pt>` (default 0). Every figure of every kind, and the checklist and the
  field-table pages, is written on a sheet of paper that many points wider and taller on each side, with the figure
  itself in the middle.
- The margin is spent from the page's printable area, in `Page::with_figure_margin`, before the figure is fitted: a
  figure that fits at the type size the page allows still fits once the margin is written. A margin that leaves no
  printable area, or is negative or not finite, is a usage error, and a figure that no longer fits is refused with
  `cli/diagram-does-not-fit` as it was, never shrunk below `--min-pt`.
- The margin is written by `svg::with_margin`, which reads the figure's header and rewrites it: the same figure, in a
  group translated by the margin, on a paper rectangle. A text that is not a figure of this renderer is refused.
- A margin of zero writes no byte differently from before.
- The boxes inside a frame keep one and a half lines of the figure's body type from the frame's left, right and bottom
  outline. The figure is wider and taller by that room; the document figure, which has no frame, is unchanged.

## Consequences

- A figure of a state that holds other states is drawn differently, whatever the margin: a committed figure or a
  comparison with `--assert-unchanged` for such a document is regenerated, not edited. A figure of a machine with no
  frame is the same to the byte at `--margin 0`.
- A caller that pads figures itself, in a stylesheet, should ask for the margin instead and drop its own padding, or
  the figure has both.
- The frame's room is not an option. It is the room the outline needs to be read as a frame, and how much paper
  surrounds the whole figure is the option.
