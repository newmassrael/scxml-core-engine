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

⚠ The widening now happens BEFORE the layout (`Renderer.measureStateWidths`),
which removes the defect but not the blind spot: the harness still supplies
no text metrics, so it measures a drawing in which that pass found nothing.

**Whether the drawn box is the reserved box.** `label-metrics.js` computes a
label's rectangle, the layout keeps that rectangle clear, and the renderer
draws into it. The harness reads the RESERVED rectangle at both ends — so if
the renderer ever again sizes a label to something else, every collision
count here stays at zero while labels pile up on screen.

That is not hypothetical either. `renderer.js` measured the rendered HTML with
`getBoundingClientRect()` and resized the container to match, and the CSS said
`width: fit-content`. The reserved rectangle and the drawn one were two
different things for as long as both existed; a reader reported every `check`
label overlapping while this file's numbers read zero. Both are gone, and
nothing here would notice their return.

**The background worker.** `csp-solver.js` runs its optimisation in a Web
Worker. The harness sets `Worker: undefined`, so the main-thread fallback runs
and the worker path — including its error handling — is never executed. A
defect lived there: the `onerror` handler called `terminate()` on a reference
it had already nulled, so a worker that failed to start threw inside the
handler meant to cope with it.

**The rendering itself.** Colours, fonts, arrowheads, dash patterns, the
annotation marks, whether a click works. None of it is geometry.

## ⚠ An aggregate cannot find everything

Collision counts, detached-edge counts, drawing extent — these are sums over
the whole picture, and a defect that does not change a sum is invisible to
every one of them.

Collapsing a compound left it at its EXPANDED size, 580x835 inside a parent
of 460x407: a white rectangle over the diagram. Nothing collided that was not
already colliding, no edge came adrift, the extent barely moved. Every column
read clean. What found it was asking the question directly — *is a collapsed
box the size a collapsed box is, and does it fit inside its parent* — which is
now asserted in `interaction.js`.

⚠ The fixture mattered as much as the check. The probe collapsed `outer` and
passed while the defect was live, because `outer` is small enough that keeping
its expanded size broke nothing. It collapses `drive` now: inside a
`<parallel>`, holding another compound, so a stale size escapes its own
parent. A property nothing in the corpus can violate is not being tested.

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

## The habit this file exists to break

Four times in the round that produced it, a measurement here was taken of
something the product does not draw:

| measured | what ships |
|---|---|
| ELK's intermediate result | the visualizer discarded it |
| line proximity, at a 24px threshold | a reader sees label boxes |
| `allLinks` | the renderer binds `getVisibleLinks`' copies |
| the reserved label box | the renderer drew a different one |

Each was a clean zero, and each was correct about the thing it measured.
⭐ Before trusting a number from this harness, check that what it reads is
what reaches the screen — the failure mode is never a wrong number, it is a
right number about the wrong rectangle.
