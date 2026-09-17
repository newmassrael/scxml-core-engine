// Does the drawing survive a drag and a collapse?
//
// Everything measured so far was ONE frame: the layout as first computed.
// `invalidateELKRouting()` is reached only from a gesture, so until this
// probe existed it had never run — new code on a path nothing exercised,
// which is the same shape as the ELK consumer branch that rotted into
// `M NaN NaN` while unreachable.
//
// What a gesture has to leave behind:
//   - ELK's routes and label positions dropped, because they described the
//     arrangement that existed before the gesture
//   - containers sized from their children again (`_elkSized` cleared), or
//     the box stops following the states inside it
//   - every edge still reaching the states it joins

const fs = require('fs');
const vm = require('vm');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const REPO = path.resolve(__dirname, '../../..');
const DOC = process.argv[2]
    || 'integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml';

let failures = 0;
const check = (ok, why) => { if (!ok) { console.error('FAIL: ' + why); failures++; } };

const fakeElement = {
    clientWidth: 1600, clientHeight: 1000,
    getBoundingClientRect: () => ({ x: 0, y: 0, width: 1600, height: 1000, top: 0, left: 0 }),
    getBBox: () => ({ x: 0, y: 0, width: 0, height: 0 }),
    appendChild() {}, removeChild() {}, setAttribute() {}, querySelector: () => null,
    querySelectorAll: () => [], addEventListener() {}, style: {},
    classList: { add() {}, remove() {}, contains: () => false },
};
// ⚠ `size()` has to be a NUMBER and `node()` an element, because the code
// interpolates them into log messages. A proxy that answers everything with
// itself throws "Cannot convert object to primitive value" the moment one
// reaches a template literal — and that happens on the collapse path, not
// the layout path, which is why it only surfaced once a gesture was
// exercised.
const d3Chain = new Proxy(function () {}, {
    get: (_t, p) => {
        if (p === 'node') return () => fakeElement;
        if (p === 'size') return () => 0;
        if (p === Symbol.toPrimitive) return () => '[d3]';
        return d3Chain;
    },
    apply: () => d3Chain,
});

(async () => {
    const createVisualizer = require(path.join(ROOT, 'visualizer.js'));
    const Module = await createVisualizer();
    // The vendored copy — the one the page loads.
    const ELK = require(path.join(ROOT, 'vendor/elkjs/elk.bundled.js'));
    const elk = new ELK();

    const runner = new Module.InteractiveTestRunner();
    runner.loadSCXML(fs.readFileSync(path.join(REPO, DOC), 'utf8'), false);
    const structure = runner.getSCXMLStructure();

    const sandbox = {
        console: { log() {}, warn() {}, error() {}, debug() {}, info() {} },
        logger: { debug() {}, info() {}, warn() {}, error() {} },
        setTimeout: () => 0, clearTimeout() {}, Worker: undefined, d3: d3Chain,
        window: { location: { search: '' }, addEventListener() {} },
        document: {
            addEventListener() {}, querySelector: () => fakeElement, querySelectorAll: () => [],
            getElementById: () => fakeElement, createElement: () => fakeElement,
            createElementNS: () => fakeElement, body: fakeElement,
        },
        URLSearchParams: class { has() { return false; } get() { return null; } },
        performance: { now: () => Date.now() },
        requestAnimationFrame: () => 0,
        requestIdleCallback: () => 0,
        // ⚠ Wrapped, not handed over raw. Code inside the context calls
        // `computeLayout()` itself — `toggleCompoundState` now does — and a
        // graph built in this context is one elkjs on the host cannot lay
        // out, returning it UNCHANGED with no error. Every internal layout
        // would silently produce `undefined` coordinates. The round trip
        // makes the graph a host object at the boundary, once, so no caller
        // has to know.
        ELK: function () {
            return { layout: (g) => elk.layout(JSON.parse(JSON.stringify(g))) };
        },
    };
    sandbox.globalThis = sandbox;
    vm.createContext(sandbox);
    for (const f of [
        'utils.js', 'edge-direction-utils.js', 'routing-state.js', 'label-metrics.js',
        'visualizer/action-formatter.js', 'visualizer/invoke-formatter.js',
        'visualizer/path-calculator.js', 'visualizer/node-builder.js',
        'visualizer/link-builder.js', 'visualizer/layout-manager.js',
        'optimizer/snap-calculator.js', 'optimizer/path-utils.js', 'optimizer/csp-solver.js',
        'optimizer/optimizer-core.js', 'visualizer/focus-manager.js',
        'visualizer/interaction-handler.js', 'visualizer/renderer.js',
        'collision-detector.js', 'visualizer/visualizer-core.js',
    ]) {
        vm.runInContext(fs.readFileSync(path.join(ROOT, f), 'utf8'), sandbox, { filename: f });
    }

    const SCXMLVisualizer = vm.runInContext('SCXMLVisualizer', sandbox);
    const v = new SCXMLVisualizer('probe', structure);
    await v.initPromise;
    const g = JSON.parse(JSON.stringify(v.layoutManager.buildELKGraph()));
    v.layoutManager.applyELKLayout(await elk.layout(g));

    // ⚠ What is DRAWN, which is not `allLinks`.
    //
    // `getVisibleLinks` returns COPIES — `{...link, originalLink, visualSource,
    // visualTarget}` — and the collapse redirect lives only on the copy. The
    // renderer binds those copies. A probe reading `allLinks` after a
    // collapse sees endpoints inside a box nobody can see, and reports every
    // edge as detached: a measurement of a drawing that is not on screen.
    const links = () => v.getVisibleLinks(v.allLinks, v.getVisibleNodes())
        .filter((l) => l && l.linkType === 'transition');
    const withSections = () => links().filter((l) => l.elkSections).length;
    const withLabels = () => links().filter((l) => l.elkLabel).length;
    const elkSized = () => v.nodes.filter((n) => n._elkSized).length;

    // The layout must actually have produced the things a gesture drops, or
    // the assertions below pass by measuring an empty set.
    check(withSections() > 0, 'no link carries ELK routing, so dropping it proves nothing');
    check(elkSized() > 0, 'no container is ELK-sized, so clearing the mark proves nothing');
    console.log(`after layout : ${withSections()} routed, ${withLabels()} ELK-placed labels,`
        + ` ${elkSized()} ELK-sized containers`);

    const detachedCount = () => {
        let n = 0;
        for (const link of links()) {
            let d;
            try { d = v.getLinkPath(link); } catch (e) { d = ''; }
            if (!d || /NaN|undefined/.test(d)) { n++; continue; }
            const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
            if (nums.length < 4) continue;
            const pts = [[nums[0], nums[1]], [nums[nums.length - 2], nums[nums.length - 1]]];
            const ends = [
                v.nodes.find((x) => x.id === (link.visualSource || link.source)),
                v.nodes.find((x) => x.id === (link.visualTarget || link.target)),
            ];
            for (let i = 0; i < 2; i++) {
                const node = ends[i];
                if (!node || !Number.isFinite(node.x)) continue;
                const slack = 6;
                const okx = Math.abs(pts[i][0] - node.x) <= node.width / 2 + slack;
                const oky = Math.abs(pts[i][1] - node.y) <= node.height / 2 + slack;
                if (!okx || !oky) { n++; break; }
            }
        }
        return n;
    };
    check(detachedCount() === 0, 'edges were already detached before any gesture');

    // ⚠ What a reader sees, measured at every phase — not just the ELK
    // artefacts this file used to count.
    //
    // The first version asserted only that a gesture DROPPED ELK's routes
    // and label positions, and that no edge came adrift. It passed while
    // every label piled back on top of its neighbours, because dropping
    // ELK's label positions returns them to the path-midpoint fallback —
    // the exact mechanism whose collisions this round had just reduced. A
    // check that watches only what it changed is blind to what that change
    // costs somewhere else.
    const boxOf = (n) => ({
        x1: n.x - n.width / 2, y1: n.y - n.height / 2,
        x2: n.x + n.width / 2, y2: n.y + n.height / 2,
    });
    const hit = (a, b) => a.x1 < b.x2 && a.x2 > b.x1 && a.y1 < b.y2 && a.y2 > b.y1;
    const readable = () => {
        const rects = [];
        for (const link of links()) {
            const box = v.layoutManager.labelBoxForLink(link);
            if (!box) continue;
            let pos;
            try { pos = v.pathCalculator.getTransitionLabelPosition(link); } catch (e) { pos = null; }
            if (!pos || !Number.isFinite(pos.x) || !Number.isFinite(pos.y)) continue;
            rects.push({
                x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
                x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
            });
        }
        let labelLabel = 0;
        for (let i = 0; i < rects.length; i++) {
            for (let j = i + 1; j < rects.length; j++) if (hit(rects[i], rects[j])) labelLabel++;
        }
        const positioned = v.nodes.filter((n) => Number.isFinite(n.x) && Number.isFinite(n.y));
        const desc = (n, acc = new Set()) => {
            for (const c of n.children || []) {
                acc.add(c);
                const cn = v.nodes.find((x) => x.id === c);
                if (cn) desc(cn, acc);
            }
            return acc;
        };
        let nodeNode = 0;
        for (let i = 0; i < positioned.length; i++) {
            for (let j = i + 1; j < positioned.length; j++) {
                const A = positioned[i]; const B = positioned[j];
                if (desc(A).has(B.id) || desc(B).has(A.id)) continue;
                if (hit(boxOf(A), boxOf(B))) nodeNode++;
            }
        }
        return { labels: rects.length, labelLabel, nodeNode };
    };

    const atLayout = readable();
    console.log(`readable     : ${atLayout.labelLabel} label collisions,`
        + ` ${atLayout.nodeNode} state overlaps (${atLayout.labels} labels)`);

    // ---------------------------------------------------------------- drag
    const dragged = v.nodes.find((n) => n.id === 'chosen');
    check(!!dragged, 'the probe could not find the state it drags');
    dragged.isDragging = true;
    dragged.x += 220;
    dragged.y += 140;

    // The call the drag handler makes — `updateLinks(fastMode)`, delegated
    // from the visualizer, which is the entry point that reaches the
    // routing refresh.
    v.updateLinks(true);
    dragged.isDragging = false;

    // What the drag-end handler does once the gesture is over: the position
    // the reader chose becomes an input and the drawing is derived again.
    await v.layoutManager.settleAfterGesture([dragged.id]);

    // ⚠ These three used to assert the OPPOSITE — that a drag leaves no ELK
    // routing, no ELK label positions and no ELK-sized container. That was
    // right while a drag only ever discarded them. It is wrong now: the
    // gesture ends by making the reader's position an input and deriving the
    // drawing again, so the artefacts come back, freshly computed for where
    // things actually are. An assertion kept from the previous design fails
    // on the improvement.
    check(withSections() > 0, 'nothing was routed after the drag settled; the layout did not re-run');
    check(withLabels() > 0,
        'no label was placed by the layout after the drag settled — they are back at path midpoints,'
        + ' which is the arrangement this round was measured undoing');
    check(elkSized() > 0, 'no container was sized by the layout after the drag settled');
    const afterDrag = detachedCount();
    check(afterDrag === 0, `${afterDrag} edge(s) detached after a drag`);
    const dragReadable = readable();
    console.log(`after drag   : ${withSections()} routed, ${withLabels()} labels,`
        + ` ${elkSized()} sized, ${afterDrag} detached`);
    console.log(`               ${dragReadable.labelLabel} label collisions,`
        + ` ${dragReadable.nodeNode} state overlaps`);

    // ⭐ A gesture must not make the drawing worse than it was. This is the
    // property a reader actually has: they drag one state and the rest of
    // the diagram does not fall apart around it.
    //
    // ⚠ Stated against the layout's own figure rather than against zero,
    // because a drag legitimately moves a state into a tighter spot — what
    // it must not do is undo the placement of everything it did not touch.
    // ⚠ State overlap IS asserted against the pre-drag figure, because no
    // arrangement of a diagram requires two states to share area — the
    // layout can always separate them, pinned node or not.
    check(dragReadable.nodeNode <= atLayout.nodeNode,
        `a drag took state overlaps from ${atLayout.nodeNode} to ${dragReadable.nodeNode}`);

    // ⚠ Label collisions are REPORTED, not asserted against the pre-drag
    // figure, and the reason is not leniency. The reader has moved a state
    // into a place of their choosing, and the layout is now required to
    // respect it; if that place is tight, labels near it have nowhere to go.
    // Demanding "never worse than before the drag" would demand that the
    // layout undo the gesture. What IS asserted is above: the labels were
    // placed by the layout rather than dropped to path midpoints. Measured
    // on this fixture, wiring the settle took the figure from 10 to 3.
    const LABEL_COLLISION_BUDGET = 6;
    check(dragReadable.labelLabel <= LABEL_COLLISION_BUDGET,
        `${dragReadable.labelLabel} label collisions after one drag, over a budget of `
        + `${LABEL_COLLISION_BUDGET} — a ceiling on this fixture, not a validated threshold`);

    // ------------------------------------------------------------ collapse
    // Re-lay out so there is something to invalidate again.
    const g2 = JSON.parse(JSON.stringify(v.layoutManager.buildELKGraph()));
    v.layoutManager.applyELKLayout(await elk.layout(g2));
    check(withSections() > 0, 'the second layout produced no routing to invalidate');

    await v.interactionHandler.toggleCompoundState('outer');

    // ⚠ The expectation here changed with the design and the probe had to
    // change with it. Under the old handler a collapse PATCHED geometry, so
    // ELK's routes had to be dropped and never replaced — "0 routed" was the
    // invariant. The handler now re-runs the layout, so fresh routes after a
    // collapse are correct. What survives both designs is the real property:
    // every edge reaches the states it joins.
    const afterCollapse = detachedCount();
    check(afterCollapse === 0, `${afterCollapse} edge(s) detached after collapsing a compound`);
    console.log(`after collapse: ${withSections()} routed, ${afterCollapse} detached`);
    if (afterCollapse > 0) {
        for (const link of links()) {
            let d;
            try { d = v.getLinkPath(link); } catch (e) { d = ''; }
            const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
            if (nums.length < 4) continue;
            const sId = link.visualSource || link.source;
            const tId = link.visualTarget || link.target;
            const s = v.nodes.find((x) => x.id === sId);
            const t = v.nodes.find((x) => x.id === tId);
            const off = (p, n) => (!n || !Number.isFinite(n.x)) ? 'no-node'
                : `${Math.round(Math.max(0, Math.abs(p[0] - n.x) - n.width / 2))},`
                  + `${Math.round(Math.max(0, Math.abs(p[1] - n.y) - n.height / 2))}`;
            const a = off([nums[0], nums[1]], s);
            const b = off([nums[nums.length - 2], nums[nums.length - 1]], t);
            if (a !== '0,0' || b !== '0,0') {
                console.log(`    ${link.source}->${link.target}  drawn ${sId}->${tId}`
                    + `  start off ${a}, end off ${b}`
                    + `  ${link.elkSections ? '(ELK route)' : '(fallback)'}`);
            }
        }
    }

    // ⚠ Is the collapse defect MINE, or was it already there?
    //
    // After a collapse `withSections()` is 0, so nothing above is drawing
    // through ELK's routes — the paths come from the orthogonal fallback,
    // which predates every change in this round. Asserted rather than
    // reasoned: a second visualizer is built, its ELK routing dropped
    // immediately the way the old code did, and the same collapse run. If
    // it detaches there too, the defect is older than this work.
    const v2 = new SCXMLVisualizer('probe2', JSON.parse(JSON.stringify(structure)));
    await v2.initPromise;
    const g3 = JSON.parse(JSON.stringify(v2.layoutManager.buildELKGraph()));
    v2.layoutManager.applyELKLayout(await elk.layout(g3));
    v2.layoutManager.invalidateELKRouting();          // what the old code did, at layout time
    v2.nodes.forEach((n) => { delete n._elkSized; });
    v2.nodes.filter((n) => n.children && n.children.length)
        .forEach((n) => v2.layoutManager.updateCompoundBounds(n));

    const detachedIn = (vv) => {
        let n = 0;
        for (const link of vv.allLinks.filter((l) => l.linkType === 'transition')) {
            let d;
            try { d = vv.getLinkPath(link); } catch (e) { d = ''; }
            if (!d || /NaN|undefined/.test(d)) { n++; continue; }
            const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
            if (nums.length < 4) continue;
            const pts = [[nums[0], nums[1]], [nums[nums.length - 2], nums[nums.length - 1]]];
            const ends = [
                vv.nodes.find((x) => x.id === (link.visualSource || link.source)),
                vv.nodes.find((x) => x.id === (link.visualTarget || link.target)),
            ];
            for (let i = 0; i < 2; i++) {
                const node = ends[i];
                if (!node || !Number.isFinite(node.x)) continue;
                if (Math.abs(pts[i][0] - node.x) > node.width / 2 + 6
                    || Math.abs(pts[i][1] - node.y) > node.height / 2 + 6) { n++; break; }
            }
        }
        return n;
    };
    const legacyBefore = detachedIn(v2);
    await v2.interactionHandler.toggleCompoundState('outer');
    const legacyAfter = detachedIn(v2);
    console.log(`\nwithout ELK routing (the old behaviour):`);
    console.log(`  before collapse: ${legacyBefore} detached`);
    console.log(`  after  collapse: ${legacyAfter} detached`
        + (legacyAfter > 0 ? '   <- predates this round' : '   <- introduced by this round'));

    console.log(failures === 0 ? 'OK' : `${failures} failure(s)`);
    process.exit(failures === 0 ? 0 : 1);
})().catch((e) => { console.error('probe failed: ' + (e && e.stack ? e.stack : e)); process.exit(1); });
