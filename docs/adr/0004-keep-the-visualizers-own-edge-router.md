# ADR 0004 — Edge routing after a gesture: keep SCE's own router, do not vendor libavoid

- Status: Accepted
- Date: 2026-09-17
- Scope: `web/visualizer` — edge routing and transition-label placement
- Related: `LICENSE-THIRD-PARTY.md` (elkjs entry, EPL-2.0 §4), `LICENSE-EXCEPTION.md`,
  `web/visualizer/LAYOUT_MEASUREMENT.md`

## Context

ELK lays the visualizer out and routes its edges. When a reader drags a
state, ELK's routes for the edges touching it describe an arrangement that
no longer exists, so they are dropped and an orthogonal fallback router
draws those edges instead.

That leaves two producers of edge geometry, and every defect this area has
had came from the same shape: something changes the geometry after the
layout, and the layout's decisions are left behind.

The obvious repair is to re-run ELK after each gesture. It does not work,
and this is a measurement rather than a preference: ELK's interactive mode
treats given coordinates as hints for ORDER, not as fixed positions, and
moved 18 untouched states by up to 2327px even with every node pinned. A
reader who drops one box and watches the diagram rearrange has lost the
thread of what they are looking at.

So the question is whether to adopt a router that routes edges *without*
moving nodes.

## Considered alternatives

### Option A — vendor `libavoid-js` (rejected: licence)

libavoid (Adaptagrams, Monash) is the reference implementation of
incremental orthogonal connector routing with fixed node positions, and
`libavoid-js` is a WebAssembly port that runs in a browser. The ELK project
reached the same conclusion and shipped `org.eclipse.elk.alg.libavoid` in
2022 — but as a native executable driven over stdio, which does not reach
the browser.

**Pros:** the right algorithm, incremental by design, and the fallback
router would stop existing rather than being improved.

**Cons, and the decisive one:**

- libavoid is **LGPL-2.1**, and SCE does not hold its copyright.
- SCE is offered as `LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception
  OR LicenseRef-SCE-Commercial`. The linking exception is what the
  commercial offer is *for*: a customer links SCE into a closed product
  without LGPL §6 obligations.
- **That exception cannot be extended to libavoid.** A commercial customer
  shipping the visualizer would carry §6 for that library regardless of
  what they paid for. The commercial licence would stop covering the whole
  product.
- `web/visualizer/vendor/` already carries one non-owned copyleft
  dependency (elkjs, EPL-2.0), and `LICENSE-THIRD-PARTY.md` records its
  §4 commercial-distribution indemnity as an **open item, not a settled
  one**. Adding a second and stronger obligation before the first is
  discharged is the wrong order.
- Measured by its own port: 8-11x slower than the C++ original.

The asymmetry that settles it: **elkjs is unavoidable** — it is the
layout, and there is no comparable replacement — while **libavoid is
avoidable**, because the fallback router works. Accepting a copyleft
dependency one cannot avoid is a different decision from accepting one
one can.

### Option B — write SCE's own fixed-node orthogonal router (deferred)

An A*/Manhattan router over an orthogonal visibility graph, of the kind
Excalidraw published (MIT, with a two-part engineering write-up and the
implementing pull requests public).

**Pros:** wholly owned, so it is covered by the commercial licence, carries
no new third-party obligation, and can be bundled however the visualizer
likes.

**Cons:** real work on a genuinely hard problem — the GD'09 paper exists
because it is one.

Not rejected. Deferred, because after the label-placement stage landed
there is no defect left for it to fix (see below).

### Option C — keep the fallback, and fix what is actually wrong (chosen)

The visible complaints — bundled lines, labels stacked on top of each other
after a drag — were not caused by having a fallback. They were caused by:

1. ELK's routes being **discarded**: cross-hierarchy routes arrive in an
   ancestor's coordinate frame and were rejected as out of place, and
   edges nested under a parent in the ELK graph were never collected at
   all. 31 of 100 edges reached the renderer with no ELK route, and 8 more
   had one that was thrown away.
2. Label placement having **two implementations** that could not agree —
   ELK's on load, a per-label path-midpoint rule after a gesture — so five
   transitions leaving one state put five labels in one spot.

Both are now fixed, and label placement is a single stage that stores a
place *on the line* rather than a coordinate.

## Decision

Adopt Option C. **The visualizer keeps its own orthogonal fallback router.
`libavoid-js` is not vendored.**

The fallback is no longer a second-class path competing with ELK: ELK
routes every edge it is asked to route and the drawing uses all of them,
so the fallback is reached only during a gesture and for self-loops
(W3C SCXML 5.9.2), which have a dedicated shape anyway.

## Consequences

**Short term**

- No new third-party dependency, no new licence obligation, and
  `LICENSE-THIRD-PARTY.md` is unchanged.
- Edge routing during a gesture remains SCE's own, and its quality is
  whatever the orthogonal router achieves. Measured over the ten fixtures
  in `web/visualizer/measure/fixtures.txt`: lines drawn on top of each
  other 0, label-on-label 0, label-on-edge 0, label-on-state 0, both
  before and after a drag.

**Long term**

- The stage boundary this leaves — placement (ELK) / routing / label
  placement — is what makes Option B a replacement of one stage rather
  than a rewrite. Label placement already asks only for "the drawn path",
  so a different router can be dropped in without touching it.
- elkjs EPL-2.0 §4 remains an open item and this ADR does not discharge
  it. It is named here because the reasoning above leans on it.

## Revisiting

Reopen if:

1. The gesture-time routing becomes a measured problem — the layout gate's
   `STACKED` / `lbl-edge` columns move off zero on a document a reader
   cares about, and the fallback cannot be made to fix it.
2. The commercial licence question is settled in a way that makes an
   additional LGPL-2.1 component acceptable — for example a customer
   agreement that already accounts for §6 on bundled third-party code.
3. A permissively licensed fixed-node orthogonal router appears that is
   maintained and browser-ready, which removes the licence objection
   without the cost of Option B.
