// How crowded is each document's drawing, measured over the real pipeline?
//
// Every part is the shipped one:
//   the structure   from the C++ engine compiled to WASM (`visualizer.js`),
//                   through `InteractiveTestRunner.getSCXMLStructure()` —
//                   the same call main.js makes. Transcribing a document
//                   into a structure by hand here would be a second parser.
//   the graph       from the real `SCXMLVisualizer`, so node building, link
//                   merging, label measurement, ELK and the optimizer are
//                   all the ones that ship.
//   the geometry    the path strings the renderer would put in `d`.
//
// Only d3 and window are stubbed, because `visualizer-core.js` is the one
// file in this path that touches them.

const fs = require('fs');
const vm = require('vm');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const REPO = path.resolve(__dirname, '../../..');

const fakeElement = {
    clientWidth: 1600,
    clientHeight: 1000,
    getBoundingClientRect: () => ({ x: 0, y: 0, width: 1600, height: 1000, top: 0, left: 0 }),
    getBBox: () => ({ x: 0, y: 0, width: 0, height: 0 }),
    appendChild() {}, removeChild() {}, setAttribute() {}, querySelector: () => null,
    querySelectorAll: () => [], addEventListener() {}, style: {},
    classList: { add() {}, remove() {}, contains: () => false },
};
const d3Chain = new Proxy(function () {}, {
    get: (_t, prop) => (prop === 'node' ? () => fakeElement : d3Chain),
    apply: () => d3Chain,
});

function makeSandbox(elkInstance) {
    const sandbox = {
        console: { log() {}, warn() {}, error() {}, debug() {}, info() {} },
        logger: { debug() {}, info() {}, warn() {}, error() {} },
        setTimeout: () => 0,
        clearTimeout() {},
        Worker: undefined,
        d3: d3Chain,
        window: { location: { search: '' }, addEventListener() {} },
        document: {
            addEventListener() {},
            querySelector: () => fakeElement,
            querySelectorAll: () => [],
            getElementById: () => fakeElement,
            createElement: () => fakeElement,
            createElementNS: () => fakeElement,
            body: fakeElement,
        },
        URLSearchParams: class { has() { return false; } get() { return null; } },
        // The CSP solver times itself to decide when to stop. Without this
        // it throws mid-optimisation, and the throw surfaces as "this
        // document could not be measured" — which reads as a defect in the
        // document rather than in the harness.
        performance: { now: () => Date.now() },
        // ⚠ The INSTANCE, built on the host. elkjs constructed inside the vm
        // cannot reach node's module system, and does not fail when it
        // cannot — `layout()` resolves with the graph unchanged.
        // ⚠ Wrapped so the graph becomes a HOST object at the boundary.
        // elkjs running on the host cannot lay out an object built inside
        // this vm context — and it does not fail: `layout()` resolves with
        // the input UNCHANGED, every `x` still undefined. Code inside the
        // context calls `computeLayout()` itself, so the round trip belongs
        // here rather than at each call site.
        ELK: function ELKFromHost() {
            return { layout: (g) => elkInstance.layout(JSON.parse(JSON.stringify(g))) };
        },
    };
    sandbox.globalThis = sandbox;
    vm.createContext(sandbox);
    for (const file of [
        'utils.js', 'edge-direction-utils.js', 'routing-state.js', 'label-metrics.js',
        'visualizer/action-formatter.js', 'visualizer/invoke-formatter.js',
        'visualizer/path-calculator.js', 'visualizer/node-builder.js',
        'visualizer/link-builder.js', 'visualizer/layout-manager.js',
        'optimizer/snap-calculator.js', 'optimizer/path-utils.js',
        'optimizer/csp-solver.js', 'optimizer/optimizer-core.js',
        'visualizer/focus-manager.js', 'visualizer/interaction-handler.js',
        'visualizer/renderer.js', 'collision-detector.js', 'visualizer/visualizer-core.js',
    ]) {
        vm.runInContext(fs.readFileSync(path.join(ROOT, file), 'utf8'), sandbox, { filename: file });
    }
    return sandbox;
}

// ---------------------------------------------------------------- oracle

function polyline(d) {
    const n = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
    const pts = [];
    for (let i = 0; i + 1 < n.length; i += 2) pts.push([n[i], n[i + 1]]);
    return pts;
}
const segsOf = (pts) => pts.slice(0, -1).map((p, i) => [p, pts[i + 1]]);

/**
 * Pairs of arrows running within `NEAR` px of each other for at least `RUN`.
 *
 * ⚠ Proximity, not intersection. Two lines that never touch still read as
 * one when they run closer together than a label is tall — which is what
 * "it does not overlap exactly, but it overlaps in the middle" describes,
 * and what an intersection count scores as zero.
 */
const NEAR = 24;
const RUN = 25;
// Below this the two lines are not "close", they are on top of each other:
// no reader can tell which arrow is which. Above it they are separate lines
// that happen to run alongside, which is what an orthogonal router produces
// by design.
const STACKED_GAP = 4;

function crowding(drawn) {
    const hits = [];
    for (let i = 0; i < drawn.length; i++) {
        for (let j = i + 1; j < drawn.length; j++) {
            let worst = null;
            for (const a of drawn[i].segs) {
                for (const b of drawn[j].segs) {
                    const aH = Math.abs(a[0][1] - a[1][1]) < 1;
                    const bH = Math.abs(b[0][1] - b[1][1]) < 1;
                    const aV = Math.abs(a[0][0] - a[1][0]) < 1;
                    const bV = Math.abs(b[0][0] - b[1][0]) < 1;
                    let gap = null; let run = 0;
                    if (aH && bH && Math.abs(a[0][1] - b[0][1]) <= NEAR) {
                        gap = Math.abs(a[0][1] - b[0][1]);
                        run = Math.min(Math.max(a[0][0], a[1][0]), Math.max(b[0][0], b[1][0]))
                            - Math.max(Math.min(a[0][0], a[1][0]), Math.min(b[0][0], b[1][0]));
                    } else if (aV && bV && Math.abs(a[0][0] - b[0][0]) <= NEAR) {
                        gap = Math.abs(a[0][0] - b[0][0]);
                        run = Math.min(Math.max(a[0][1], a[1][1]), Math.max(b[0][1], b[1][1]))
                            - Math.max(Math.min(a[0][1], a[1][1]), Math.min(b[0][1], b[1][1]));
                    }
                    if (gap !== null && run >= RUN && (!worst || run > worst.run)) {
                        worst = { gap, run };
                    }
                }
            }
            if (worst) hits.push({ a: drawn[i].label, b: drawn[j].label, ...worst });
        }
    }
    return hits;
}

// ------------------------------------------------------------------ run

async function measure(sandbox, elkInstance, structure, legacy, spacing) {
    const SCXMLVisualizer = vm.runInContext('SCXMLVisualizer', sandbox);
    const v = new SCXMLVisualizer('probe', structure);
    await v.initPromise;

    // ⚠ Cross the graph back to the host before ELK sees it — see the note
    // on `ELKFromHost`. The constructor's own layout hit exactly this, so
    // it is redone here rather than trusted.
    const hostGraph = JSON.parse(JSON.stringify(v.layoutManager.buildELKGraph()));

    // The spacing under test, applied here rather than by editing the tree
    // between runs. ⚠ Whatever this sweep settles on has to be WRITTEN into
    // `layout-manager.js` afterwards: an override that lives only in the
    // probe measures a configuration the product does not have.
    if (spacing) {
        Object.assign(hostGraph.layoutOptions, spacing);
    }
    v.layoutManager.applyELKLayout(await elkInstance.layout(hostGraph));

    // The OLD behaviour, reproduced exactly rather than described: the
    // previous code deleted every `elkSections` at the end of
    // `applyELKLayout`, which is what `invalidateELKRouting` now does on
    // demand. Calling it here and nowhere else is the before-picture.
    if (legacy) {
        v.layoutManager.invalidateELKRouting();
    }

    const links = v.allLinks.filter((l) => l.linkType === 'transition');
    const drawn = [];
    let nan = 0;
    let detached = 0;

    // ⚠ Added after the census reported a clean sheet for a document whose
    // drawing had two lines floating 700px from the states they connect.
    // "Undrawable" meant the path string contained NaN — a check on
    // arithmetic, not on the picture. A route of perfectly finite numbers
    // that simply does not touch its endpoints passed every column.
    const boxOf = (n) => ({
        x1: n.x - n.width / 2, y1: n.y - n.height / 2,
        x2: n.x + n.width / 2, y2: n.y + n.height / 2,
    });
    const touches = (pt, n) => {
        if (!n || !Number.isFinite(n.x)) return true;   // unplaced: not this check's business
        const b = boxOf(n);
        const slack = 4;
        return pt[0] >= b.x1 - slack && pt[0] <= b.x2 + slack
            && pt[1] >= b.y1 - slack && pt[1] <= b.y2 + slack;
    };

    // ⚠ How many edges ELK actually routes, asked of the product rather
    // than restated here. A route ELK computes and the drawing discards is
    // invisible in every other column: the fallback draws a finite, attached
    // line, so `undrawable` and `detached` both stay clean while the layout
    // ELK was asked for is thrown away.
    //
    // ⚠ Split, because one total conflates a normal case with a defect. An
    // edge ELK never routed (a self-loop, a targetless transition, an end
    // inside a collapsed container) has nothing to discard and must use the
    // fallback. An edge ELK DID route and the drawing rejected is the
    // defect, and a single "fallback" total hides it behind the first kind.
    //
    // ⚠⚠ THREE buckets, not two. An edge ELK never routed at all is the
    // third, and it is the one that can hide a whole document drawing
    // itself: `elkDropped` reads a clean zero when there was nothing to
    // drop, which looks identical to ELK having routed everything.
    let elkOffered = 0;
    let elkAbsent = 0;
    const elkDropped = [];
    for (const link of links) {
        const src = v.nodes.find((n) => n.id === (link.visualSource || link.source));
        const tgt = v.nodes.find((n) => n.id === (link.visualTarget || link.target));
        if (!src || !tgt || src.id === tgt.id) continue;   // self-loops draw their own shape
        if (!(link.elkSections && link.elkSections.length > 0)) { elkAbsent++; continue; }
        elkOffered++;
        if (!v.pathCalculator.usesELKRoute(link)) {
            elkDropped.push(`${link.source} -> ${link.target}`);
        }
    }

    for (const link of links) {
        let d;
        try { d = v.getLinkPath(link); } catch (e) { d = ''; }
        if (!d || /NaN|undefined/.test(d)) { nan++; continue; }
        const pts = polyline(d);
        if (pts.length >= 2) {
            const src = v.nodes.find((n) => n.id === (link.visualSource || link.source));
            const tgt = v.nodes.find((n) => n.id === (link.visualTarget || link.target));
            if (!touches(pts[0], src) || !touches(pts[pts.length - 1], tgt)) detached++;
        }
        drawn.push({ label: `${link.source}->${link.target}`, segs: segsOf(pts) });
    }

    // ⚠ The labels, measured where the renderer puts them and at the size
    // `label-metrics.js` reserved — not inferred from line proximity.
    //
    // The line-proximity number above uses a 24px threshold chosen because
    // "a label is about that tall", which makes it a PROXY for the thing a
    // reader actually sees colliding. A proxy is what you use when the real
    // quantity is out of reach, and this one never was: the position
    // function and the box function are both right here.
    const labelRects = [];
    for (const link of links) {
        const box = v.layoutManager.labelBoxForLink(link);
        if (!box) continue;
        let pos;
        try { pos = v.pathCalculator.getTransitionLabelPosition(link); } catch (e) { pos = null; }
        if (!pos || !Number.isFinite(pos.x) || !Number.isFinite(pos.y)) continue;
        labelRects.push({
            id: `${link.source}->${link.target}`,
            x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
            x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
        });
    }
    const rectsHit = (a, b) => a.x1 < b.x2 && a.x2 > b.x1 && a.y1 < b.y2 && a.y2 > b.y1;
    let labelLabel = 0;
    let labelLabelArea = 0;
    for (let i = 0; i < labelRects.length; i++) {
        for (let j = i + 1; j < labelRects.length; j++) {
            if (rectsHit(labelRects[i], labelRects[j])) {
                labelLabel++;
                const w = Math.min(labelRects[i].x2, labelRects[j].x2)
                    - Math.max(labelRects[i].x1, labelRects[j].x1);
                const h = Math.min(labelRects[i].y2, labelRects[j].y2)
                    - Math.max(labelRects[i].y1, labelRects[j].y1);
                labelLabelArea += w * h;
            }
        }
    }
    let labelEdge = 0;
    for (const r of labelRects) {
        for (const p of drawn) {
            if (p.label === r.id) continue;
            for (const seg of p.segs) {
                const s = {
                    x1: Math.min(seg[0][0], seg[1][0]), x2: Math.max(seg[0][0], seg[1][0]),
                    y1: Math.min(seg[0][1], seg[1][1]), y2: Math.max(seg[0][1], seg[1][1]),
                };
                if (rectsHit(s, r)) { labelEdge++; break; }
            }
        }
    }

    // ⚠ Two filters lived here and each removed the case a check exists to
    // find. `!(n.children && n.children.length)` compared LEAVES only, so a
    // document nested nine deep compared almost nothing; and dropping nodes
    // without coordinates meant a node ELK never placed left the population
    // rather than failing it. Both are kept visible now: `unplaced` is
    // reported, and overlap is asked of every positioned pair that is not an
    // ancestor of the other.
    const positioned = v.nodes.filter((n) => Number.isFinite(n.x) && Number.isFinite(n.y));
    const unplaced = v.nodes.length - positioned.length;
    // ⚠ The arrow id COLLIDES, deliberately, and only a check keeps that a
    // decision rather than a bug waiting to be "corrected".
    //
    // `getTransitionId` is `source_event_target`, and transitions differing
    // only by their guard share it — 33 of 1557 ids across 27 documents, one
    // of them naming thirteen. The engine cannot tell them apart either: it
    // reports `{source, target, event}` for what fired and nothing about the
    // guard, so narrowing the id would take highlighting from "marks all of
    // them" to "marks none".
    //
    // ⭐ What must hold is that the collision lines up with the MERGE: every
    // transition sharing an id is drawn on ONE arrow. That is what makes a
    // colliding id the right key for a drawn element rather than a defect.
    // If a future change splits a merged arrow without changing the id, this
    // is what says so.
    const getTransitionId = vm.runInContext('getTransitionId', sandbox);
    const arrowsPerId = new Map();
    for (const link of links) {
        for (const member of link.transitions || [link]) {
            const tid = getTransitionId(member);
            if (!tid) continue;
            if (!arrowsPerId.has(tid)) arrowsPerId.set(tid, new Set());
            arrowsPerId.get(tid).add(link.id);
        }
    }
    const idsOnSeveralArrows = [...arrowsPerId.values()].filter((s) => s.size > 1).length;

    const leaves = positioned.filter((n) => !(n.children && n.children.length));

    let labelNode = 0;
    for (const r of labelRects) {
        for (const n of leaves) {
            if (rectsHit(r, { x1: n.x - n.width / 2, y1: n.y - n.height / 2,
                x2: n.x + n.width / 2, y2: n.y + n.height / 2 })) labelNode++;
        }
    }
    // Every positioned pair, containers included. A container legitimately
    // encloses its own descendants, so that relation is excluded — and only
    // that one.
    const descendantsOf = (n, acc = new Set()) => {
        for (const cid of n.children || []) {
            acc.add(cid);
            const c = v.nodes.find((x) => x.id === cid);
            if (c) descendantsOf(c, acc);
        }
        return acc;
    };
    let nodeOverlap = 0;
    for (let i = 0; i < positioned.length; i++) {
        for (let j = i + 1; j < positioned.length; j++) {
            const A = positioned[i]; const B = positioned[j];
            if (descendantsOf(A).has(B.id) || descendantsOf(B).has(A.id)) continue;
            if (Math.abs(A.x - B.x) * 2 < A.width + B.width
                && Math.abs(A.y - B.y) * 2 < A.height + B.height) nodeOverlap++;
        }
    }

    // ⚠ The pair, not the count. Every crowding number here has a trivial
    // optimum — push everything apart — and a sweep that reads only the
    // left-hand column would take that trade every time, then report a
    // diagram ten times the size as an improvement. The extent is what
    // makes the trade visible.
    let minX = Infinity; let minY = Infinity; let maxX = -Infinity; let maxY = -Infinity;
    for (const n of leaves) {
        minX = Math.min(minX, n.x - n.width / 2);
        maxX = Math.max(maxX, n.x + n.width / 2);
        minY = Math.min(minY, n.y - n.height / 2);
        maxY = Math.max(maxY, n.y + n.height / 2);
    }
    const width = Number.isFinite(minX) ? maxX - minX : 0;
    const height = Number.isFinite(minY) ? maxY - minY : 0;

    return {
        states: v.nodes.length,
        transitions: (structure.transitions || []).length,
        arrows: links.length,
        undrawable: nan,
        elkOffered,
        elkAbsent,
        elkDropped,
        detached,
        unplaced,
        idsOnSeveralArrows,
        crowded: crowding(drawn),
        nodeOverlap,
        labels: labelRects.length,
        labelLabel,
        labelLabelArea: Math.round(labelLabelArea),
        labelEdge,
        labelNode,
        width: Math.round(width),
        height: Math.round(height),
        area: Math.round(width * height),
    };
}

(async () => {
    const createVisualizer = require(path.join(ROOT, 'visualizer.js'));
    const Module = await createVisualizer();
    // The vendored copy — the one the page loads, not one npm resolved.
    // A harness measuring a different build of the layout engine measures a
    // different product.
    const ELK = require(path.join(ROOT, 'vendor/elkjs/elk.bundled.js'));
    const elkInstance = new ELK();

    // ⭐ Two modes, and the difference is what the numbers are FOR.
    //
    // Default (the gate): the shipped configuration only, and the invariants
    // are asserted. `--sweep` compares configurations, which is research —
    // useful for choosing a value, wrong for a gate, because a sweep
    // includes the self-router control whose failures are the reason it was
    // replaced.
    const sweep = process.argv.includes('--sweep');
    const docs = process.argv.slice(2).filter((a) => a !== '--sweep');

    // Read every structure once, from the engine.
    const structures = [];
    let refused = 0;
    for (const rel of docs) {
        try {
            const runner = new Module.InteractiveTestRunner();
            runner.loadSCXML(fs.readFileSync(path.join(REPO, rel), 'utf8'), false);
            structures.push({ rel, structure: runner.getSCXMLStructure() });
        } catch (error) {
            console.error(`${rel}: engine refused — ${error.message || error}`);
            refused++;
        }
    }

    // ⚠ A document the engine would not load is not a document that measured
    // clean. Counted and fatal, because the quiet version of this failure is
    // the whole corpus refusing and the census reporting an immaculate zero
    // over nothing at all.
    if (refused > 0) {
        console.error(`\n${refused} document(s) could not be loaded; nothing was measured for them`);
        process.exit(1);
    }
    if (structures.length === 0) {
        console.error('no document was measured');
        process.exit(1);
    }

    // The configurations. `legacy` is the self-router, kept as the control:
    // without it the sweep only compares ELK against ELK and cannot say
    // whether keeping ELK's routing was worth anything at all.
    const s = (n, e, ee) => ({
        'elk.layered.spacing.edgeNodeBetweenLayers': String(n),
        'elk.layered.spacing.edgeEdgeBetweenLayers': String(e),
        'elk.spacing.edgeEdge': String(ee),
    });
    const configs = sweep
        ? [
            { name: 'self-router (before)', legacy: true, spacing: null },
            { name: 'elk  10/10/10', spacing: s(10, 10, 10) },
            { name: 'elk  15/10/10 (as shipped)', spacing: s(15, 10, 10) },
            { name: 'elk  20/20/15', spacing: s(20, 20, 15) },
            { name: 'elk  30/30/20 (mermaid)', spacing: s(30, 30, 20) },
            { name: 'elk  40/40/30', spacing: s(40, 40, 30) },
            { name: 'elk  60/60/40', spacing: s(60, 60, 40) },
        ]
        // `spacing: null` leaves `buildELKGraph`'s own options in place, so
        // this row measures WHAT SHIPS rather than a configuration the probe
        // supplied. An override here would be a gate judging a product that
        // does not exist.
        : [{ name: 'as shipped', spacing: null }];

    console.log(`\n${structures.length} document(s), ${configs.length} configuration(s)\n`);
    console.log('configuration'.padEnd(28)
        + 'STACKED  alongside  lbl-lbl  lbl-edge  lbl-node  no-ELK  ELK-dropped  DETACHED  node-overlap  unplaced   area');

    const violations = [];
    for (const cfg of configs) {
        let bad = 0; let ll = 0; let le = 0; let ln = 0;
        let detached = 0; let overlap = 0; let unplaced = 0; let splitIds = 0;
        let elkOffered = 0; let elkAbsent = 0; const elkDropped = [];
        // ⚠ Computed since this census was written and printed by nothing,
        // while "the lines bundle" was the complaint every round was chasing.
        //
        // ⚠⚠ Reported as two numbers, because ONE would say the wrong thing.
        // `crowding`'s NEAR is 24px, and an orthogonal router puts parallel
        // edges in adjacent channels about 10px apart ON PURPOSE — so a
        // single "pairs within 24px" total counts good routing as crowding
        // and moves the wrong way when the routing improves. Measured here:
        // turning ELK's routes back on took that total from 17 to 18 while
        // the drawing got better by every other column.
        //
        // ⭐ The discriminator is the GAP. Two lines drawn on top of each
        // other have a gap near zero; two channels have the router's
        // spacing. `stacked` counts the first kind and is the one that
        // means "unreadable".
        const stackedPairs = []; let nearby = 0;
        const areas = [];
        for (const { rel, structure } of structures) {
            try {
                const m = await measure(makeSandbox(elkInstance), elkInstance,
                    JSON.parse(JSON.stringify(structure)), cfg.legacy, cfg.spacing);
                bad += m.undrawable;
                ll += m.labelLabel;
                le += m.labelEdge;
                ln += m.labelNode;
                detached += m.detached;
                elkOffered += m.elkOffered;
                elkAbsent += m.elkAbsent;
                for (const name of m.elkDropped) elkDropped.push(`${rel}: ${name}`);
                nearby += m.crowded.length;
                for (const hit of m.crowded.filter((h) => h.gap <= STACKED_GAP)) {
                    stackedPairs.push(`${rel}: ${hit.a} over ${hit.b}`
                        + ` (${hit.gap.toFixed(1)}px apart for ${Math.round(hit.run)}px)`);
                }
                splitIds += m.idsOnSeveralArrows;
                overlap += m.nodeOverlap;
                unplaced += m.unplaced;
                areas.push(m.area);
            } catch (error) {
                bad += 1;
            }
        }
        const total = areas.reduce((x, y) => x + y, 0);
        console.log(cfg.name.padEnd(28)
            + String(stackedPairs.length).padStart(7) + String(nearby).padStart(11)
            + String(ll).padStart(9) + String(le).padStart(10) + String(ln).padStart(10)
            + String(elkAbsent).padStart(8)
            + `${elkDropped.length}/${elkOffered}`.padStart(13)
            + String(detached).padStart(10) + String(overlap).padStart(14)
            + String(unplaced).padStart(10)
            + String((total / 1e6).toFixed(2) + 'M').padStart(9)
            + (bad ? `   (${bad} undrawable)` : ''));

        // ⚠ A count sends a reader looking; a name tells them where. Every
        // wrong turn this column was added for began with a number that
        // said something was discarded and nothing that said which.
        for (const name of elkDropped) {
            console.log(`  ELK routed but the drawing dropped: ${name}`);
        }
        for (const pair of stackedPairs) {
            console.log(`  drawn on top of each other: ${pair}`);
        }

        // ⭐ The invariants, and only in the gate's mode. A sweep includes
        // the self-router control, whose failures are the reason it was
        // replaced; failing on them would make the control unusable.
        //
        // Each of these is a drawing that is WRONG, not one that is crowded:
        // an edge that never reaches its states, a label the layout placed
        // nowhere, two unrelated states sharing area, a node never placed.
        // The crowding columns are deliberately not among them — their
        // thresholds are unvalidated, and a gate enforcing an unvalidated
        // number teaches people to work around it.
        if (!sweep) {
            if (detached) violations.push(`${detached} edge(s) do not reach the states they join`);
            // ⚠ A route ELK computed and the drawing threw away. Every other
            // column stays clean when this happens — the fallback draws a
            // finite, attached line — so for a whole round the diagram was
            // routed entirely by the fallback while the census reported a
            // clean sheet. `ancestor_entry_is_not_default_entry` alone had
            // four routes, four rejections, zero uses.
            if (elkDropped.length) {
                violations.push(`${elkDropped.length} route(s) ELK computed were discarded by the`
                    + ' drawing, so the layout being asked for is not the one shown');
            }
            // ⚠ And the other half of the same defect: an edge ELK was never
            // asked to route, or whose route was never collected. Both leave
            // the fallback drawing an edge the layout knows nothing about,
            // which is where all three stacked pairs lived.
            if (elkAbsent) {
                violations.push(`${elkAbsent} edge(s) reached the renderer with no ELK route at all,`
                    + ' so the fallback drew them against a layout that never saw them');
            }
            if (overlap) violations.push(`${overlap} pair(s) of unrelated states share area`);
            if (unplaced) violations.push(`${unplaced} node(s) were never given a position`);
            if (bad) violations.push(`${bad} edge(s) could not be drawn at all`);
            if (splitIds) {
                violations.push(`${splitIds} arrow id(s) name transitions drawn on more than one`
                    + ' arrow, so highlighting one id cannot mark one thing');
            }
        }
    }

    if (violations.length) {
        console.error('\nlayout invariants violated:');
        for (const v of violations) console.error(`  - ${v}`);
        process.exit(1);
    }
})().catch((e) => { console.error(e && e.stack ? e.stack : e); process.exit(1); });
