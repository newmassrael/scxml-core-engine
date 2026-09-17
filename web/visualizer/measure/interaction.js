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

// ⚠ The harness is `harness.js` now. This file used to carry its own copy
// and the two drifted — the `d3Chain` here answered `size()` with a number
// and the other answered with itself, which throws as soon as the collapse
// path puts it in a log message. One copy could measure a collapse and the
// other could not.
const { makeSandbox } = require('./harness');

(async () => {
    const createVisualizer = require(path.join(ROOT, 'visualizer.js'));
    const Module = await createVisualizer();
    // The vendored copy — the one the page loads.
    const ELK = require(path.join(ROOT, 'vendor/elkjs/elk.bundled.js'));
    const elk = new ELK();

    const runner = new Module.InteractiveTestRunner();
    runner.loadSCXML(fs.readFileSync(path.join(REPO, DOC), 'utf8'), false);
    const structure = runner.getSCXMLStructure();

    const sandbox = makeSandbox(elk);

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
        // ⚠ The STAGE first, because the drawing runs it before it writes a
        // single label — `Renderer.updateLabels`. Reading
        // `getTransitionLabelPosition` without it measures the per-label
        // seed, which is a position the product never draws.
        //
        // ⚠⚠⚠ This does NOT run the placement stage. It reads what the
        // product left behind, and that distinction is the whole reason
        // this check is worth anything.
        //
        // The first version called `placeTransitionLabels` itself and read
        // the result. It reported zero collisions at every phase while a
        // browser showed ten overlapping pairs after the same gesture —
        // because the defect was never in the stage, it was in WHEN the
        // product ran it. A probe that performs the step it is checking can
        // only ever confirm that the step works in isolation.
        //
        // ⚠ Positions survive `getVisibleLinks` returning fresh `{...link}`
        // copies because the stage syncs them back to `originalLink`, the
        // way `routing` always has. Without that sync this would read
        // nothing and look like a failure of the stage rather than of the
        // plumbing.
        const visible = links();
        const rects = [];
        for (const link of visible) {
            const box = v.layoutManager.labelBoxForLink(link);
            if (!box) continue;
            let pos;
            try { pos = v.pathCalculator.getTransitionLabelPosition(link); } catch (e) { pos = null; }
            if (!pos || !Number.isFinite(pos.x) || !Number.isFinite(pos.y)) continue;
            rects.push({
                id: `${link.source}->${link.target}`,
                x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
                x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
            });
        }
        // ⚠ The pairs, not just how many. A count sends a reader looking;
        // a name tells them where, and every wrong turn in this area began
        // with a number that said something collided and nothing that said
        // which.
        let labelLabel = 0;
        const collisions = [];
        for (let i = 0; i < rects.length; i++) {
            for (let j = i + 1; j < rects.length; j++) {
                if (!hit(rects[i], rects[j])) continue;
                labelLabel++;
                collisions.push(`${rects[i].id} X ${rects[j].id}`
                    + ` (at ${Math.round(rects[i].x1)},${Math.round(rects[i].y1)}`
                    + ` and ${Math.round(rects[j].x1)},${Math.round(rects[j].y1)})`);
            }
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
        // ⚠ How far each label's BOX is from the line it names.
        //
        // The property this catches: a label placed correctly against a path
        // that has since been re-derived. In a browser that put labels up to
        // 412px from their own arrow while every collision count read zero —
        // the placement was right when it was made and the line moved under
        // it afterwards, which no count of overlaps can see.
        //
        // ⚠⚠ It cannot catch the ORDERING that caused it: the drawing
        // re-derives its paths when a 50ms timer clears `isDragging`, and
        // nothing here runs timers or the real drag handlers. What it locks
        // in is the RESULT, so a future change that leaves labels adrift in
        // the headless path fails here instead of in somebody's browser.
        let worstOffLine = 0;
        let worstOffLineEdge = null;
        for (const link of visible) {
            const box = v.layoutManager.labelBoxForLink(link);
            const pos = v.pathCalculator.getTransitionLabelPosition(link);
            if (!box || !pos) continue;
            let d;
            try { d = v.getLinkPath(link); } catch (e) { continue; }
            if (!d || /NaN|undefined/.test(d)) continue;
            const nums = (d.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
            const r = {
                x1: pos.x - box.width / 2, y1: pos.y - box.height / 2,
                x2: pos.x + box.width / 2, y2: pos.y + box.height / 2,
            };
            // ⚠ To the SEGMENTS, not to the vertices.
            //
            // Measuring to vertices reported a label 334px from its line
            // while the label sat 30px from the segment it was anchored to —
            // the nearest corner of that long vertical run happened to be
            // 354px away. Three explanations were chased for that number
            // before the path was printed and the arithmetic done by hand.
            // It is the fourth time this round a measurement, not the
            // drawing, was the thing that was wrong.
            const pts = [];
            for (let i = 0; i + 1 < nums.length; i += 2) {
                pts.push({ x: nums[i], y: nums[i + 1] });
            }
            const distToRect = (p) => {
                const dx = Math.max(r.x1 - p.x, 0, p.x - r.x2);
                const dy = Math.max(r.y1 - p.y, 0, p.y - r.y2);
                return Math.hypot(dx, dy);
            };
            let best = Infinity;
            for (let i = 1; i < pts.length; i++) {
                const a = pts[i - 1]; const b = pts[i];
                const len = Math.hypot(b.x - a.x, b.y - a.y);
                const steps = Math.max(1, Math.ceil(len / 4));
                for (let s = 0; s <= steps; s++) {
                    best = Math.min(best, distToRect({
                        x: a.x + (b.x - a.x) * s / steps,
                        y: a.y + (b.y - a.y) * s / steps,
                    }));
                }
            }
            if (Number.isFinite(best) && best > worstOffLine) {
                worstOffLine = best;
                worstOffLineEdge = `${link.source}->${link.target}`;
            }
        }

        const placements = visible
            .filter((l) => l.labelPlacement && l.labelPlacement.reason !== 'clear')
            .map((l) => `${l.source}->${l.target}: ${l.labelPlacement.reason},`
                + ` ${l.labelPlacement.spots} spot(s) on its line,`
                + ` ${l.labelPlacement.tried} tried, residual ${l.labelPlacement.residual}`);
        return {
            labels: rects.length, labelLabel, nodeNode, collisions, placements,
            worstOffLine: Math.round(worstOffLine), worstOffLineEdge,
        };
    };

    const atLayout = readable();
    console.log(`readable     : ${atLayout.labelLabel} label collisions,`
        + ` ${atLayout.nodeNode} state overlaps (${atLayout.labels} labels)`);

    // ---------------------------------------------------------------- drag
    // ⚠ `lobby`, not the hub. On this fixture `chosen` sits on all eight
    // edges, so dragging it invalidates everything and the narrowing this
    // probe exists to check cannot be seen — a fixture that makes the
    // property untestable, which is the shape of a vacuous pass. `lobby`
    // carries two edges and leaves six bystanders to keep their labels.
    const dragged = v.nodes.find((n) => n.id === 'lobby');
    check(!!dragged, 'the probe could not find the state it drags');
    const before = new Map(v.nodes
        .filter((n) => Number.isFinite(n.x))
        .map((n) => [n.id, [n.x, n.y]]));

    // The path each arrow is drawn along, before anything moves. A drag is
    // allowed to re-route what it touches; every other line on the canvas
    // should be exactly where the reader last saw it.
    const pathsBefore = new Map();
    for (const link of links()) {
        try { pathsBefore.set(link.id, v.getLinkPath(link)); } catch (e) { /* counted elsewhere */ }
    }

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
    // ⚠ Nothing but the dragged state may move. A reader who drops one box
    // and watches the rest of the diagram rearrange a second later has lost
    // the thread of what they are looking at, and that is worse than
    // crowding — crowding at least holds still.
    let moved = 0;
    let worst = 0;
    for (const n of v.nodes) {
        const was = before.get(n.id);
        if (!was || !Number.isFinite(n.x) || n.id === dragged.id) continue;
        const d = Math.hypot(n.x - was[0], n.y - was[1]);
        if (d > 1) { moved++; worst = Math.max(worst, d); }
    }
    check(moved === 0,
        `${moved} state(s) the reader did not touch moved, by up to ${Math.round(worst)}px`);

    // ⭐ And the same for the lines. During a gesture the drawing must change
    // only where the gesture reaches — an arrow between two states the reader
    // never touched has no reason to take a different route, and one that
    // does is the canvas moving on its own, which is the complaint this whole
    // sequence of rounds came from.
    let reroutedBystanders = 0;
    for (const link of links()) {
        if ([link.source, link.target, link.visualSource, link.visualTarget].includes(dragged.id)) {
            continue;
        }
        const was = pathsBefore.get(link.id);
        if (was === undefined) continue;
        let now;
        try { now = v.getLinkPath(link); } catch (e) { now = undefined; }
        if (now !== was) {
            reroutedBystanders++;
            if (reroutedBystanders <= 3) {
                console.error(`FAIL: ${link.source}->${link.target} re-routed by a drag of ${dragged.id}`);
            }
        }
    }
    check(reroutedBystanders === 0,
        `${reroutedBystanders} arrow(s) not touching the dragged state changed route`);

    console.log(`untouched    : ${moved} state(s) moved,`
        + ` ${reroutedBystanders} arrow(s) re-routed`);

    // ⭐ What the drag is allowed to invalidate: the edges that TOUCH what
    // moved, and nothing else. This has been asserted three different ways
    // across one session, each matching a design that then changed —
    // "everything is dropped", then "everything comes back", now this. The
    // property that survived all three is further down: no edge comes
    // adrift, and nothing the reader did not touch moves.
    const incident = (l) => [l.source, l.target, l.visualSource, l.visualTarget].includes(dragged.id);
    const bystanders = links().filter((l) => !incident(l));
    const bystandersKeepingLabels = bystanders.filter((l) => l.elkLabel).length;

    if (bystanders.length === 0) {
        // ⚠ Not a pass. On this fixture the dragged state is a hub — all
        // eight edges touch `chosen` — so there is no bystander to keep
        // anything, and the narrowing this asserts cannot be seen here.
        // Said out loud rather than scored, because a check that measures an
        // empty set reports success for the wrong reason.
        console.log('               (every edge touches the dragged state; narrowing not measurable here)');
    } else {
        check(bystandersKeepingLabels === bystanders.length,
            `${bystanders.length - bystandersKeepingLabels} of ${bystanders.length} edge(s) not touching`
            + ' the dragged state lost the label position the layout gave them');
    }
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
    // ⚠ Neither figure is asserted against the pre-drag one, and it is worth
    // being exact about why, because the temptation to assert it is strong.
    //
    // The reader has moved a state to a place of their choosing. If that
    // place is tight, the states and labels near it are crowded — by the
    // gesture, not by a defect. Demanding "never worse than before" would
    // demand that the drawing undo the gesture, which is the behaviour this
    // file's own check above now forbids.
    //
    // So they are reported. What is asserted is everything that does NOT
    // depend on where the reader chose to put things: no edge adrift,
    // nothing untouched moving, bystander labels kept.
    console.log(`               (was ${atLayout.labelLabel} label collisions,`
        + ` ${atLayout.nodeNode} state overlaps before the drag)`);

    // ------------------------------------------------- dragging the hub
    // ⚠ The case the `lobby` drag above CANNOT reach, and the one a reader
    // actually hit. `lobby` is chosen deliberately: it leaves bystanders, so
    // the narrowing property is measurable. But that same choice means only
    // two labels are ever re-placed, and two labels on two different lines
    // do not collide — so "0 label collisions after a drag" was true and
    // meant nothing.
    //
    // `chosen` sits on all eight edges, five of them `check` transitions
    // leaving the same side and running down one shared trunk. Dragging it
    // invalidates every label at once, which is exactly when a per-label
    // rule puts five labels in one spot. Measured in a browser before the
    // placement stage existed: two overlapping pairs, both among those five.
    const hub = v.nodes.find((n) => n.id === 'chosen');
    check(!!hub, 'the probe could not find the hub state it drags');
    const labelsOnHub = links().filter((l) =>
        [l.source, l.target, l.visualSource, l.visualTarget].includes(hub.id)
        && v.layoutManager.labelBoxForLink(l)).length;
    check(labelsOnHub >= 5,
        `only ${labelsOnHub} labelled edge(s) touch the hub, so crowding cannot be measured here`);

    hub.isDragging = true;
    hub.x += 60;
    hub.y += 90;
    v.updateLinks(true);
    hub.isDragging = false;
    v.updateLinks(false);

    const hubDetached = detachedCount();
    check(hubDetached === 0, `${hubDetached} edge(s) detached after dragging the hub`);

    // ⚠⚠⚠ Does asking for the same line twice give the same line?
    //
    // Everything downstream assumes it does: the placement stage samples a
    // path to choose where a label goes, and the drawing asks for that path
    // again to draw it. If the two answers differ, a label is placed against
    // a line nobody draws — and that is indistinguishable, from the outside,
    // from the stage choosing badly. It cost this round three wrong
    // explanations before the question was asked directly.
    const unstable = [];
    for (const link of links()) {
        let a; let b;
        try { a = v.getLinkPath(link); b = v.getLinkPath(link); } catch (e) { continue; }
        if (a !== b) unstable.push(`${link.source}->${link.target}`);
    }
    check(unstable.length === 0,
        `${unstable.length} arrow(s) return a different path when asked twice`
        + ` without anything moving: ${unstable.slice(0, 4).join(', ')}`);

    // ⚠⚠ Did the STAGE place these, or is this reading the per-label seed?
    // Without this the check below passes either way: the seed is ELK's
    // position, which is uncollided on load, so a probe that never reaches
    // the stage reports a clean sheet for the wrong reason — which is
    // exactly what happened, for a whole round, while a browser showed ten
    // overlapping pairs after the same gesture.
    const labelled = links().filter((l) => v.layoutManager.labelBoxForLink(l));
    const staged = labelled.filter((l) => l.labelAnchor && l.labelPlacement).length;
    check(staged === labelled.length,
        `${labelled.length - staged} of ${labelled.length} label(s) carry no placement from the`
        + ' stage after a settled gesture, so what follows is reading the per-label seed');
    const hubReadable = readable();
    console.log(`after hub drag: ${hubReadable.labelLabel} label collisions,`
        + ` ${hubReadable.nodeNode} state overlaps (${labelsOnHub} labels on the hub)`);
    for (const pair of hubReadable.collisions) {
        console.log(`                ${pair}`);
    }
    // ⚠ And what the stage says it did, which is the difference between
    // "it could not find a clear spot" and "it never looked".
    for (const note of hubReadable.placements) {
        console.log(`                ${note}`);
    }

    // ⭐ Asserted, not merely reported — unlike the figures above. Those are
    // about where the READER chose to put a state, which may legitimately be
    // tight. This one is about labels the reader never positioned at all: a
    // gesture hands every one of them back to the placement stage, and a
    // stage that returns them overlapping has failed at the only job it has.
    check(hubReadable.labelLabel === 0,
        `${hubReadable.labelLabel} label pair(s) overlap after the hub was dragged,`
        + ' so the placement stage did not separate labels it re-placed');

    // ⭐ And attached. A label that names a transition has to be ON it; a
    // stage free to put labels anywhere can reach zero collisions by moving
    // them all into empty space, which is not an improvement and is what a
    // browser showed when the placement outlived the path it was made for.
    //
    // ⚠ The bound is the label's own box plus a margin, because the stage
    // deliberately places labels BESIDE their line rather than on it — a
    // label sitting on a shared trunk covers the other arrows using it.
    const OFF_LINE_LIMIT = 40;
    console.log(`               labels sit at most ${hubReadable.worstOffLine}px from their own line`
        + ` (${hubReadable.worstOffLineEdge})`);
    // Only when it is close to the limit: the detail is for diagnosing a
    // label that has come adrift, and printing an anchor every run buries
    // the line that matters.
    if (hubReadable.worstOffLine > 20) {
        // ⚠ The anchor and the line it is an anchor ON. Printed because
        // three explanations for this number were wrong in a row, each of
        // them plausible and none of them checked against the actual data.
        const worst = links().find((l) => `${l.source}->${l.target}` === hubReadable.worstOffLineEdge);
        if (worst) {
            let d = '';
            try { d = v.getLinkPath(worst); } catch (e) { d = '(threw)'; }
            console.log(`                anchor ${JSON.stringify(worst.labelAnchor)}`);
            console.log(`                drawn at ${JSON.stringify(v.pathCalculator.getTransitionLabelPosition(worst))}`);
            console.log(`                path ${d.slice(0, 160)}`);
        }
    }
    check(hubReadable.worstOffLine <= OFF_LINE_LIMIT,
        `a label sits ${hubReadable.worstOffLine}px from the line it names (limit ${OFF_LINE_LIMIT}),`
        + ' so it was placed against geometry that is no longer drawn');

    // ------------------------------------------------------------ collapse
    // Re-lay out so there is something to invalidate again.
    const g2 = JSON.parse(JSON.stringify(v.layoutManager.buildELKGraph()));
    v.layoutManager.applyELKLayout(await elk.layout(g2));
    check(withSections() > 0, 'the second layout produced no routing to invalidate');

    // ⚠ `drive`, not `outer`. Both are compounds, but `drive` is the one
    // whose collapse exposed the stale-size defect: it sits inside a
    // `<parallel>` and holds another compound, so keeping its expanded size
    // puts a box larger than its own parent on the canvas. `outer` is small
    // enough that the same bug changed no number this file counts.
    await v.interactionHandler.toggleCompoundState('drive');

    // ⚠ The expectation here changed with the design and the probe had to
    // change with it. Under the old handler a collapse PATCHED geometry, so
    // ELK's routes had to be dropped and never replaced — "0 routed" was the
    // invariant. The handler now re-runs the layout, so fresh routes after a
    // collapse are correct. What survives both designs is the real property:
    // every edge reaches the states it joins.
    const afterCollapse = detachedCount();
    check(afterCollapse === 0, `${afterCollapse} edge(s) detached after collapsing a compound`);

    // ⚠ A collapsed container must be the SIZE of a collapsed container.
    //
    // This probe collapsed `outer` and passed while the defect was live,
    // because `outer` is small enough that keeping its expanded size broke
    // nothing the other columns count. Collapsing `drive` left it 580x835
    // inside a parent of 460x407 — a white rectangle over the diagram — and
    // every number here still read clean. Checking the size directly is what
    // the collision counts could not do for it.
    for (const node of v.nodes) {
        if (!node.collapsed || !Number.isFinite(node.x)) continue;
        const expected = {
            width: v.nodeBuilder.getNodeWidth(node),
            height: v.nodeBuilder.getNodeHeight(node),
        };
        check(Math.abs(node.width - expected.width) < 1 && Math.abs(node.height - expected.height) < 1,
            `collapsed ${node.id} is ${Math.round(node.width)}x${Math.round(node.height)},`
            + ` not the ${expected.width}x${expected.height} a collapsed box is`);
        const parent = v.nodes.find((p) => (p.children || []).includes(node.id));
        if (parent && Number.isFinite(parent.width)) {
            check(node.width <= parent.width && node.height <= parent.height,
                `collapsed ${node.id} (${Math.round(node.width)}x${Math.round(node.height)})`
                + ` is larger than the ${parent.id} that holds it`
                + ` (${Math.round(parent.width)}x${Math.round(parent.height)})`);
        }
    }
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
