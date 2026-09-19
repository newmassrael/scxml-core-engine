// Does the drawing survive a reader who will not sit still?
//
// `interaction.js` performs ONE drag and ONE collapse, in a fixed order, on
// one document. Every defect it found was real, and it still only ever
// looked at three drawings. A reader drags a state, collapses a container,
// drags something inside what is left, expands it again, and scatters half
// the diagram before deciding they liked it better before — and each of
// those leaves geometry the next one starts from.
//
// This runs those operations in a SEEDED random order and checks the
// invariants after every single one.
//
// ⚠ Seeded, not random. A failure that cannot be replayed is a rumour: the
// seed is printed on every run and `SCE_STRESS_SEED=<n>` replays exactly
// that sequence, down to which state was dragged where.
//
// ⚠⚠ What this does NOT cover, said out loud because the gap is invisible
// from the inside: zoom and focus. Both read the real DOM —
// `focusOnTransition` looks a label up with `document.querySelector`,
// `getContainerDimensions` reads `clientWidth` — and under this harness's
// stubs they return nothing and carry on, which looks exactly like working.
// They are covered by `stress-browser.js`, which needs a browser.

const fs = require('fs');
const path = require('path');
const {
    REPO, makeSandbox, loadEngine, structureOf, layoutDocument, LABEL_TO_LINE_LIMIT,
} = require('./harness');

const OPS = Number(process.env.SCE_STRESS_OPS || 40);
const SEED = Number(process.env.SCE_STRESS_SEED || (Date.now() % 2147483647));

// Deterministic PRNG (mulberry32): the same seed gives the same sequence on
// any machine and any Node version, which `Math.random` does not.
function rng(seed) {
    let a = seed >>> 0;
    return () => {
        a = (a + 0x6D2B79F5) >>> 0;
        let t = Math.imul(a ^ (a >>> 15), 1 | a);
        t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
        return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
}

let failures = 0;
const fail = (why) => { console.error('FAIL: ' + why); failures++; };

// ------------------------------------------------------------- invariants

/**
 * The properties that hold no matter WHERE the reader put things.
 *
 * ⚠ Crowding is deliberately not among them. A reader who drops two states
 * on top of each other has crowded their own diagram, and a check that
 * called that a defect would be demanding the drawing undo the gesture —
 * which `interaction.js` already forbids for good reason. Crowding is
 * counted and reported; only these are asserted.
 */
function invariants(v, label) {
    const links = v.getVisibleLinks(v.allLinks, v.getVisibleNodes())
        .filter((l) => l && l.linkType === 'transition');

    let undrawable = 0;
    // ⚠ Named, not counted. Every wrong turn in this area began with a
    // number that said something was broken and nothing that said which.
    const detachedEdges = [];
    let unstaged = 0;
    let worstOffLine = 0;
    let worstOffLineEdge = null;

    for (const link of links) {
        let d;
        try { d = v.getLinkPath(link); } catch (e) { undrawable++; continue; }
        if (!d || /NaN|undefined/.test(d)) { undrawable++; continue; }

        const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
        const pts = [];
        for (let i = 0; i + 1 < nums.length; i += 2) pts.push({ x: nums[i], y: nums[i + 1] });
        if (pts.length < 2) continue;

        const src = v.nodes.find((n) => n.id === (link.visualSource || link.source));
        const tgt = v.nodes.find((n) => n.id === (link.visualTarget || link.target));
        for (const [pt, node, end] of [[pts[0], src, 'source'], [pts[pts.length - 1], tgt, 'target']]) {
            if (!node || !Number.isFinite(node.x)) continue;
            const slack = 6;
            const offX = Math.abs(pt.x - node.x) - node.width / 2;
            const offY = Math.abs(pt.y - node.y) - node.height / 2;
            if (offX > slack || offY > slack) {
                // ⚠ Everything needed to tell "the route was wrong" from
                // "the boxes moved under it", because those look identical
                // from a count and this round spent five wrong explanations
                // on the difference. `elkFrame` carries which layout the
                // route came from and how far it missed WHEN IT WAS CHOSEN;
                // the rest is the state now.
                detachedEdges.push(`${link.source}->${link.target} (${end} ${node.id}:`
                    + ` off by ${Math.round(Math.max(offX, offY))}px,`
                    + ` route ${link.elkSections ? 'ELK' : 'fallback'})`
                    + ` drawn at (${Math.round(pt.x)},${Math.round(pt.y)})`
                    + ` box (${Math.round(node.x)},${Math.round(node.y)})`
                    + ` ${Math.round(node.width)}x${Math.round(node.height)}`
                    + ` when chosen ${JSON.stringify(link.elkFrame || null)}`
                    + ` layout now ${v.layoutManager._layoutSerial}`);
                break;
            }
        }

        const box = v.layoutManager.labelBoxForLink(link);
        if (!box) continue;
        if (!link.labelAnchor && !v.getCustomLabelPosition(link)) { unstaged++; continue; }

        const pos = v.pathCalculator.getTransitionLabelPosition(link);
        if (!pos || !Number.isFinite(pos.x)) { unstaged++; continue; }
        const r = {
            x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
            x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
        };
        // To the SEGMENTS, not the vertices: the nearest corner of a long
        // straight run can be hundreds of pixels from a label sitting right
        // beside the middle of it.
        let best = Infinity;
        for (let i = 1; i < pts.length; i++) {
            const a = pts[i - 1]; const b = pts[i];
            const len = Math.hypot(b.x - a.x, b.y - a.y);
            const steps = Math.max(1, Math.ceil(len / 6));
            for (let s = 0; s <= steps; s++) {
                const q = { x: a.x + (b.x - a.x) * s / steps, y: a.y + (b.y - a.y) * s / steps };
                const dx = Math.max(r.x1 - q.x, 0, q.x - r.x2);
                const dy = Math.max(r.y1 - q.y, 0, q.y - r.y2);
                best = Math.min(best, Math.hypot(dx, dy));
            }
        }
        if (best > worstOffLine) { worstOffLine = best; worstOffLineEdge = `${link.source}->${link.target}`; }
    }

    const unplaced = v.getVisibleNodes().filter((n) => !Number.isFinite(n.x) || !Number.isFinite(n.y)).length;

    if (undrawable) fail(`${label}: ${undrawable} arrow(s) could not be drawn at all`);
    if (detachedEdges.length) {
        fail(`${label}: ${detachedEdges.length} arrow(s) do not reach the states they join`
            + `\n        ${detachedEdges.join('\n        ')}`);
    }
    if (unstaged) fail(`${label}: ${unstaged} label(s) carry no placement from the stage`);
    if (unplaced) fail(`${label}: ${unplaced} visible state(s) have no position`);
    if (worstOffLine > LABEL_TO_LINE_LIMIT) {
        fail(`${label}: a label sits ${Math.round(worstOffLine)}px from the line it names`
            + ` (${worstOffLineEdge}, limit ${LABEL_TO_LINE_LIMIT})`);
    }
    return { links: links.length, worstOffLine: Math.round(worstOffLine) };
}

// -------------------------------------------------------------- gestures

function leafNodes(v) {
    return v.getVisibleNodes().filter((n) => Number.isFinite(n.x)
        && !(n.children && n.children.length && !n.collapsed));
}

function compounds(v) {
    return v.getVisibleNodes().filter((n) =>
        (n.type === 'compound' || n.type === 'parallel') && n.children && n.children.length);
}

/** One drag, driven the way the drag handler drives it. */
function drag(v, node, dx, dy) {
    node.isDragging = true;
    node.x += dx;
    node.y += dy;
    v.updateLinks(true);          // live frames
    node.isDragging = false;
    v.updateLinks(false);         // the settle the drag-end handler runs
}

/**
 * Every leaf thrown somewhere else at once — the reader who rearranges the
 * whole diagram by hand.
 *
 * ⚠ The hardest case this file has, and not because it is untidy: a scatter
 * puts states in positions no layout would choose, so edges take long
 * routes that cross everything, and the label stage has to find room where
 * there is none. It is the case where "least-bad" placement actually
 * happens, and the invariants above still have to hold.
 */
function scatter(v, rand) {
    const leaves = leafNodes(v);
    if (!leaves.length) return 0;
    const xs = leaves.map((n) => n.x);
    const ys = leaves.map((n) => n.y);
    const spread = Math.max(600, Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys));
    const ox = Math.min(...xs);
    const oy = Math.min(...ys);
    for (const n of leaves) {
        n.isDragging = true;
        n.x = ox + rand() * spread;
        n.y = oy + rand() * spread;
    }
    v.updateLinks(true);
    for (const n of leaves) n.isDragging = false;
    v.updateLinks(false);
    return leaves.length;
}

// ------------------------------------------------------------------- run

(async () => {
    const docs = fs.readFileSync(path.join(__dirname, 'fixtures.txt'), 'utf8')
        .split('\n').map((l) => l.trim()).filter((l) => l && !l.startsWith('#'));
    const only = process.argv[2];
    const targets = only ? docs.filter((d) => d.includes(only)) : docs;

    console.log(`stress: seed ${SEED}, ${OPS} operation(s) per document,`
        + ` ${targets.length} document(s)`);
    console.log('        replay with SCE_STRESS_SEED=' + SEED);

    const { Module, elk } = await loadEngine();

    // ⚠ A stream PER DOCUMENT, keyed by its name.
    //
    // One shared stream was the first shape and it made the replay promise
    // false: narrowing a run to the failing document changed how much of
    // the stream the earlier documents had consumed, so the same seed
    // produced a different sequence and the failure moved from step 7 to
    // step 3. A seed that only replays when you run everything is not a
    // seed, it is a coincidence.
    const streamFor = (doc) => {
        let h = 2166136261;
        for (let i = 0; i < doc.length; i++) {
            h = Math.imul(h ^ doc.charCodeAt(i), 16777619);
        }
        return rng((SEED ^ h) >>> 0);
    };

    for (const doc of targets) {
        const rand = streamFor(doc);
        let structure;
        try { structure = structureOf(Module, doc); } catch (e) {
            fail(`${doc}: the engine could not read it — ${e.message}`);
            continue;
        }
        let v;
        try { v = await layoutDocument(makeSandbox(elk), elk, structure); } catch (e) {
            fail(`${doc}: layout threw — ${e.message}`);
            continue;
        }

        const name = doc.split('/').pop();
        invariants(v, `${name} @ layout`);

        const tally = { drag: 0, scatter: 0, collapse: 0, expand: 0 };
        let worst = 0;
        for (let step = 1; step <= OPS; step++) {
            const roll = rand();
            let what = 'drag';
            try {
                if (roll < 0.55) {
                    const leaves = leafNodes(v);
                    if (!leaves.length) continue;
                    const node = leaves[Math.floor(rand() * leaves.length)];
                    const before = new Map(v.nodes.filter((n) => Number.isFinite(n.x))
                        .map((n) => [n.id, [n.x, n.y]]));
                    drag(v, node, (rand() - 0.5) * 400, (rand() - 0.5) * 300);
                    tally.drag++;
                    // ⭐ Only a DRAG carries this: a collapse re-runs the
                    // layout by design, so states move and must.
                    let moved = 0;
                    for (const n of v.nodes) {
                        const was = before.get(n.id);
                        if (!was || !Number.isFinite(n.x) || n.id === node.id) continue;
                        if (Math.hypot(n.x - was[0], n.y - was[1]) > 1) moved++;
                    }
                    if (moved) {
                        fail(`${name} @ step ${step}: dragging ${node.id} moved`
                            + ` ${moved} state(s) the reader did not touch`);
                    }
                } else if (roll < 0.7) {
                    what = 'scatter';
                    if (!scatter(v, rand)) continue;
                    tally.scatter++;
                } else {
                    const boxes = compounds(v);
                    if (!boxes.length) continue;
                    const box = boxes[Math.floor(rand() * boxes.length)];
                    what = box.collapsed ? 'expand' : 'collapse';
                    await v.toggleCompoundState(box.id);
                    tally[what]++;
                }
            } catch (e) {
                fail(`${name} @ step ${step} (${what}): threw — ${e.message}`);
                break;
            }
            const m = invariants(v, `${name} @ step ${step} after ${what}`);
            worst = Math.max(worst, m.worstOffLine);
        }

        console.log(`  ${name.padEnd(46)} ${tally.drag} drag, ${tally.scatter} scatter,`
            + ` ${tally.collapse} collapse, ${tally.expand} expand`
            + ` — labels within ${worst}px of their line`);
    }

    if (failures) {
        console.error(`\n${failures} failure(s). Replay: SCE_STRESS_SEED=${SEED}`);
        process.exit(1);
    }
    console.log('OK');
})().catch((e) => { console.error('stress probe threw: ' + (e && e.stack || e)); process.exit(1); });
