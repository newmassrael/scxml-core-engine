# Measuring the diagram's layout

The visualizer's geometry can be measured without a browser. The routing
pipeline — `path-calculator.js` and `optimizer/*` — contains zero references
to `d3`, `document` or `window`, and `visualizer-core.js` is the only file in
that path that touches them. Stubbing those two globals is enough to
construct the real `SCXMLVisualizer` under node, run the real ELK, and read
the path strings the renderer would put in `d`.

That matters because a rendered diagram is not bytes a test can diff, but the
layout behind it is: node boxes, bend points and label rectangles are data.

## ⚠ What this measurement CANNOT see

**Text measurement.** `renderer.js` measures real text with `getBBox()` and
widens a state whose content overflows its estimated box. `getBBox()` is the
browser's text engine. A stub returns whatever it is told, which makes it a
stub standing in for the very quantity under test — so the harness does not
simulate it, and every number it reports describes a drawing in which no
state was ever widened.

This is not hypothetical. A reader opened
`ancestor_entry_is_not_default_entry` and saw `by_default` overlapping
`chosen`, and `watch` overlapping `drive`. The harness reported zero
overlapping pairs for that document, and it was right about what it measured:
at estimated widths ELK leaves 80px and 70px between those pairs. The
overlap only exists once the real text is measured.

⚠⚠ So a zero from this harness on any size-dependent question means "not
measured", not "not present". The size-dependent questions are: state
overlap, label-on-state overlap, and anything derived from a compound's
bounds, since those follow from their children.

**The rendering itself.** Colours, fonts, arrowheads, dash patterns, the
annotation marks, whether a click works. None of it is geometry.

## ⚠ Two traps that cost real time

**elkjs does not fail when it cannot lay out.** A graph object built inside
a `vm` context cannot be laid out by an elkjs running on the host — and
`layout()` resolves with the input UNCHANGED, every `x` still `undefined`.
No throw, no rejection, no warning. Downstream that becomes
`node.x = undefined + 0 + 70` = `NaN`, and `path-calculator` has a documented
tolerance for missing coordinates, so it produces a full set of
plausible-looking paths around the origin. Cross the graph to the host with
`JSON.parse(JSON.stringify(graph))` before handing it over.

**A metric that filters out the failing case reports a clean sheet.** The
first version of the node-overlap check compared leaf nodes only
(`!(n.children && n.children.length)`), so a document nested nine deep
compared almost nothing; and it dropped nodes without coordinates instead of
reporting them. Neither filter was hiding anything in the end, but a third
one was: "undrawable" meant the path string contained `NaN`, which is a check
on arithmetic and not on the picture. A route of finite numbers that does not
touch the states it joins passed every column — and twelve of them were doing
exactly that.

## What to measure

| quantity | why it is the one that matters |
|---|---|
| label-on-label, label-on-edge, label-on-state | what a reader sees colliding |
| detached edges | a route must reach the boxes it claims to join |
| unplaced nodes | reported, never filtered out |
| node overlap | every positioned pair, ancestors excluded — not leaves only |
| drawing extent | the companion to every collision count |

⚠ The extent is not optional. Every collision count above has a trivial
optimum — push everything apart — and a change judged on collisions alone
takes that trade every time. Measured across seven spacings, the extent moves
under 1% here, so the trade is not being made; that is a finding, not a
reason to stop reporting it.

⚠⚠ And the thresholds are not validated. "Within 24px over a run of 25px"
was chosen because a label is about that tall. Whether a diagram scoring
better on these numbers reads better to a person has not been established,
and until it is, these numbers rank changes rather than judge them. A
line-proximity proxy stood in for label collisions for one round and pointed
the opposite way from the direct measurement.
