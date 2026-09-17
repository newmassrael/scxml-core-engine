// The one place a probe gets a running visualizer without a browser.
//
// ⚠ Extracted because there were two copies and a third was about to be
// written. They had already drifted: one `d3Chain` answered `size()` with a
// number and `Symbol.toPrimitive` with a string, the other answered both
// with itself — and the difference is not cosmetic, because the collapse
// path interpolates `size()` into a log message and a proxy there throws
// "Cannot convert object to primitive value". One copy could measure a
// collapse and the other could not, which is the kind of thing two copies
// of a harness quietly acquire.
//
// Everything here is the SHIPPED code: the structure comes from the C++
// engine compiled to WASM, the graph and geometry from the real
// `SCXMLVisualizer`. Only d3 and the DOM are stubbed, because
// `visualizer-core.js` is the one file in this path that touches them.
//
// ⚠⚠ What this CANNOT measure, stated so nobody builds on it by mistake:
// anything that reads the real DOM. `focusOnTransition` looks a label up
// with `document.querySelector` and reads its attributes;
// `getContainerDimensions` reads `clientWidth`. Under these stubs they do
// not fail — they return nothing and carry on, which is indistinguishable
// from working. Zoom and focus belong in a browser, not here.

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

// ⚠ `size()` has to be a NUMBER and `node()` an element, because the code
// interpolates them into log messages. A proxy that answers everything with
// itself throws "Cannot convert object to primitive value" the moment one
// reaches a template literal — and that happens on the collapse path, not
// the layout path, which is why it only surfaced once a gesture was
// exercised.
const d3Chain = new Proxy(function () {}, {
    get: (_t, prop) => {
        if (prop === 'node') return () => fakeElement;
        if (prop === 'size') return () => 0;
        if (prop === Symbol.toPrimitive) return () => '[d3]';
        return d3Chain;
    },
    apply: () => d3Chain,
});

const SOURCES = [
    'utils.js', 'edge-direction-utils.js', 'routing-state.js', 'label-metrics.js',
    'visualizer/action-formatter.js', 'visualizer/invoke-formatter.js',
    'visualizer/path-calculator.js', 'visualizer/node-builder.js',
    'visualizer/link-builder.js', 'visualizer/layout-manager.js',
    'optimizer/snap-calculator.js', 'optimizer/path-utils.js',
    'optimizer/csp-solver.js', 'optimizer/optimizer-core.js',
    'visualizer/focus-manager.js', 'visualizer/interaction-handler.js',
    'visualizer/renderer.js', 'collision-detector.js', 'visualizer/visualizer-core.js',
];

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
        requestAnimationFrame: () => 0,
        requestIdleCallback: () => 0,
        // ⚠ The INSTANCE, built on the host, and wrapped so the graph
        // becomes a HOST object at the boundary. elkjs running on the host
        // cannot lay out an object built inside this vm context — and it
        // does not fail: `layout()` resolves with the input UNCHANGED, every
        // `x` still undefined. Code inside the context calls
        // `computeLayout()` itself, so the round trip belongs here rather
        // than at each call site.
        ELK: function ELKFromHost() {
            return { layout: (g) => elkInstance.layout(JSON.parse(JSON.stringify(g))) };
        },
    };
    sandbox.globalThis = sandbox;
    vm.createContext(sandbox);
    for (const file of SOURCES) {
        vm.runInContext(fs.readFileSync(path.join(ROOT, file), 'utf8'), sandbox, { filename: file });
    }
    return sandbox;
}

/** The WASM engine and the vendored elkjs the page itself loads. */
async function loadEngine() {
    const createVisualizer = require(path.join(ROOT, 'visualizer.js'));
    const Module = await createVisualizer();
    const ELK = require(path.join(ROOT, 'vendor/elkjs/elk.bundled.js'));
    return { Module, elk: new ELK() };
}

/** The structure `main.js` would hand the visualizer, for one document. */
function structureOf(Module, repoRelativePath) {
    const runner = new Module.InteractiveTestRunner();
    runner.loadSCXML(fs.readFileSync(path.join(REPO, repoRelativePath), 'utf8'), false);
    return runner.getSCXMLStructure();
}

/**
 * A laid-out visualizer, ready to be measured or gestured at.
 *
 * ⚠ The layout is driven here rather than left to the constructor, for the
 * reason `ELKFromHost` gives: a graph built inside the context has to cross
 * back to the host before elkjs sees it, and the constructor's own layout
 * hit exactly that.
 */
async function layoutDocument(sandbox, elkInstance, structure) {
    const SCXMLVisualizer = vm.runInContext('SCXMLVisualizer', sandbox);
    const v = new SCXMLVisualizer('probe', structure);
    await v.initPromise;
    const graph = JSON.parse(JSON.stringify(v.layoutManager.buildELKGraph()));
    v.layoutManager.applyELKLayout(await elkInstance.layout(graph));
    return v;
}

module.exports = {
    ROOT, REPO, SOURCES, fakeElement, d3Chain,
    makeSandbox, loadEngine, structureOf, layoutDocument,
};
