# ADR 0019 — The arrow of a state to itself is drawn large enough, inside its box and its frame

- Status: Accepted and implemented (the owner pointed at the defects on 2026-10-11)
- Date: 2026-10-11
- Scope: `sce-build/src/diagram/route.rs` (`loop_path`, `loop_depth`), `sce-build/src/diagram/layout.rs`
- Related: `docs/adr/0017-a-rank-wider-than-the-page-is-set-as-several-ranks.md`

## Context

A transition from a state to itself is drawn as a loop under the box. Three defects were found on the figure of a
five-state machine the owner was reading (a state with 15 transitions, three of them to itself):

1. The loop was one line wide and a third of the gap deep: eleven points across and ten down at the minimum type
   size, with an arrow head of five points on one side. It read as a head with no line.
2. A loop is anchored on the box's bottom edge at the port the router gave the arrow. For a box narrower than the
   loop, the left side of the loop stood outside the box, and the head on it pointed at empty space.
3. A loop under a box of the LAST rank hangs below everything the figure had measured: the frame was drawn round
   the boxes only, so the loop and its number crossed the frame's border.

## Decision

- The loop is wider (1.6 lines each side of its port) and deeper (0.55 of the gap between ranks).
- Both ends of the loop are kept inside the box's width: the loop is narrowed when the box is narrower than it, and
  moved along the edge when its port is too near an end.
- When a box of the last rank has such an arrow, the body of the figure is taller by the depth of the loop and one
  line for its number, so the loop is inside the frame. A figure with no loop under its last rank is as tall as
  before.

## Measurement

The 2,816 documents of ADR 0017 (1,085 committed, 1,731 an AI writer produced) were drawn before and after, on A4
portrait and A3 landscape:

| | A4 portrait | A3 landscape |
|---|---|---|
| drawn in both, figures identical to the byte | 2,080 | 2,228 |
| drawn in both, figures changed | 44 | 52 |
| of the changed, in a document with an arrow of a state to itself | 44 | 52 |
| drawn before and not now | 0 | 0 |

Every change is a figure of a document with an arrow to itself, which is the figure this decision means to change;
none of the documents without one is drawn differently. (The documents drawn now and not before, 124 on A4 and 294
on A3, are ADR 0017's and ADR 0018's.)

## Consequences

- A figure committed or compared with `--assert-unchanged` that has such an arrow is drawn differently: it is
  regenerated, not edited.
- The numbers of two arrows that leave one narrow box at its bottom edge can still stand close together: the ports
  are spread along the edge by the router, and a narrow box has little edge. That is not decided here.
