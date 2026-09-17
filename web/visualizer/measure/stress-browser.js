// Zoom and focus, which only a browser can answer.
//
// ⚠ This is NOT a gate and cannot become one under the headless harness.
// `focusOnTransition` finds a label with `document.querySelector` and reads
// its attributes; `getContainerDimensions` reads `clientWidth`; the pan is a
// d3 transition on a real `<svg>`. Under `harness.js`'s stubs every one of
// those returns something harmless and carries on, so a headless version of
// this file would pass without measuring anything — the exact failure this
// round spent a day on elsewhere.
//
// HOW TO RUN IT
//   1. Serve the repo:  python3 -m http.server 8777
//   2. Open web/visualizer/visualizer.html#url=<a document>
//   3. Paste this file into the browser console, or inject it with any
//      automation that can evaluate a script in the page.
//   It reports to the console and resolves to a summary object.
//
// ⚠⚠ It drives the controls a READER has. Clicking a label that is off
// screen is not one of them — that was the first version of this file, and
// "focus did not move the view" was trivially true because the click could
// never have happened. Focus is exercised through `.transition-list-item`
// in the side panel, which is visible at any zoom.

(async function stressBrowser({ zoomSteps = 6, settle = 900 } = {}) {
    const out = { checked: 0, failures: [] };
    const fail = (why) => { out.failures.push(why); console.error('FAIL: ' + why); };

    const svg = document.querySelector('#state-diagram-single svg');
    const zoomed = document.querySelector('#state-diagram-single g[transform]');
    if (!svg || !zoomed) {
        console.error('no diagram on this page');
        return out;
    }

    const wait = (ms) => new Promise((r) => setTimeout(r, ms));
    const frame = () => svg.getBoundingClientRect();
    const transform = () => zoomed.getAttribute('transform') || '';
    // ⚠ From the MATRIX, not by parsing the attribute. The first version
    // read `scale(...)` out of the transform string and reported 1.00 at
    // every zoom level, because the attribute is `translate(...)` alone
    // until a scale is applied — so the whole zoom section passed while
    // measuring nothing.
    const scaleOf = () => {
        const m = zoomed.getCTM();
        return m ? Math.hypot(m.a, m.b) : NaN;
    };
    const inView = (el) => {
        const r = frame();
        const b = el.getBoundingClientRect();
        return b.width > 0 && b.right > r.left && b.left < r.right
            && b.bottom > r.top && b.top < r.bottom;
    };
    const wheel = (deltaY) => {
        const r = frame();
        svg.dispatchEvent(new WheelEvent('wheel', {
            bubbles: true, cancelable: true, deltaY,
            clientX: r.left + r.width / 2, clientY: r.top + r.height / 2,
        }));
    };

    // ---------------------------------------------------------------- zoom
    //
    // What must hold at any zoom: the transform stays a finite number, and a
    // label stays beside the line it names. The second is the interesting
    // one — label geometry is stored as a place ALONG the path, so zooming
    // must not move a label relative to its arrow, only relative to the
    // screen.
    const labelToLineGap = () => {
        const paths = new Map();
        for (const p of document.querySelectorAll('#state-diagram-single path')) {
            const d = p.__data__;
            if (d && d.linkType === 'transition') paths.set(d.id, p);
        }
        let worst = 0;
        let worstId = null;
        for (const f of document.querySelectorAll('#state-diagram-single foreignObject')) {
            const d = f.__data__;
            const p = d && paths.get(d.id);
            if (!p) continue;
            // ⚠ From the `d` string, not `getPointAtLength`. The two
            // disagreed on this page — 1726 against 1819 for the same path —
            // and the API's answer sent a measurement 400px wrong.
            const nums = (p.getAttribute('d').match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
            const pts = [];
            for (let i = 0; i + 1 < nums.length; i += 2) pts.push({ x: nums[i], y: nums[i + 1] });
            if (pts.length < 2) continue;
            const b = f.getBoundingClientRect();
            const toUser = (cx, cy) => {
                const q = svg.createSVGPoint();
                q.x = cx; q.y = cy;
                return q.matrixTransform(p.getScreenCTM().inverse());
            };
            const tl = toUser(b.left, b.top);
            const br = toUser(b.right, b.bottom);
            const r = {
                x1: Math.min(tl.x, br.x), y1: Math.min(tl.y, br.y),
                x2: Math.max(tl.x, br.x), y2: Math.max(tl.y, br.y),
            };
            let best = Infinity;
            for (let i = 1; i < pts.length; i++) {
                const a = pts[i - 1]; const c = pts[i];
                const len = Math.hypot(c.x - a.x, c.y - a.y);
                const steps = Math.max(1, Math.ceil(len / 6));
                for (let s = 0; s <= steps; s++) {
                    const q = { x: a.x + (c.x - a.x) * s / steps, y: a.y + (c.y - a.y) * s / steps };
                    const dx = Math.max(r.x1 - q.x, 0, q.x - r.x2);
                    const dy = Math.max(r.y1 - q.y, 0, q.y - r.y2);
                    best = Math.min(best, Math.hypot(dx, dy));
                }
            }
            if (Number.isFinite(best) && best > worst) { worst = best; worstId = d.source + '->' + d.target; }
        }
        return { worst: Math.round(worst), worstId };
    };

    const GAP_LIMIT = 48;
    const atStart = labelToLineGap();
    console.log(`zoom ${scaleOf().toFixed(2)}: labels within ${atStart.worst}px of their line`);

    for (const [name, delta, times] of [['in', -240, zoomSteps], ['out', 240, zoomSteps * 2]]) {
        for (let i = 0; i < times; i++) wheel(delta);
        await wait(settle);
        out.checked++;
        const t = transform();
        if (/NaN|undefined/.test(t)) fail(`zoom ${name}: the view transform became "${t}"`);
        const s = scaleOf();
        if (!Number.isFinite(s) || s <= 0) fail(`zoom ${name}: scale is ${s}`);
        const gap = labelToLineGap();
        console.log(`zoom ${name} -> ${s.toFixed(2)}: labels within ${gap.worst}px of their line`);
        if (gap.worst > GAP_LIMIT) {
            fail(`zoom ${name}: ${gap.worstId} sits ${gap.worst}px from its line (limit ${GAP_LIMIT})`);
        }
    }

    // --------------------------------------------------------------- focus
    //
    // ⭐ The property: choosing a transition in the side panel brings it
    // into view. Measured on 2026-09-18 at scale 3.08 with all eight labels
    // off screen, six rows in a row: each one HIGHLIGHTED its arrow — so the
    // click reached the handler, which is what separates this from a dead
    // listener — and not one of them moved the view. The transition a reader
    // asked to see stayed off screen.
    const rows = [...document.querySelectorAll('.transition-list-item')];
    if (!rows.length) {
        console.log('focus: no transition rows in the side panel, nothing to check');
        return out;
    }

    // Zoom in until something IS off screen, or the check is vacuous.
    for (let i = 0; i < zoomSteps; i++) wheel(-240);
    await wait(settle);
    const labels = [...document.querySelectorAll('.transition-label')];
    const offScreen = labels.filter((l) => !inView(l));
    console.log(`focus: at scale ${scaleOf().toFixed(2)}, ${offScreen.length} of ${labels.length} label(s) are off screen`);
    if (!offScreen.length) {
        console.log('focus: nothing is off screen, so focusing cannot be shown to bring anything in');
        return out;
    }

    for (const row of rows.slice(0, 4)) {
        // ⚠ The id is on the ROW. The first version looked for a child
        // carrying it, found none, and reported every target as "unknown" —
        // which skipped the assertion and printed OK.
        const id = row.getAttribute('data-transition-id');
        const name = (row.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 40);
        const target = id && document.querySelector(`.transition-label[data-transition-id~="${id}"]`);
        out.checked++;
        if (!target) {
            // ⭐ A FAILURE, not a shrug. "I could not find what I was
            // supposed to look at" and "what I looked at was fine" have to
            // read differently, or a check that stops working reports
            // success forever.
            fail(`focus "${name}": no label carries id "${id}", so the check could not be made`);
            continue;
        }
        const wasVisible = inView(target);
        const before = transform();
        row.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, view: window }));
        await wait(settle + 400);      // the pan is a 750ms d3 transition
        const moved = transform() !== before;
        const visible = inView(target);
        console.log(`focus "${name}": was ${wasVisible ? 'in view' : 'off screen'},`
            + ` view ${moved ? 'moved' : 'did not move'}, now ${visible ? 'in view' : 'OFF SCREEN'}`);
        if (!wasVisible && !visible) {
            fail(`focus "${name}": chosen while off screen and still off screen afterwards`);
        }
    }

    console.log(out.failures.length ? `${out.failures.length} failure(s)` : 'OK');
    return out;
})();
