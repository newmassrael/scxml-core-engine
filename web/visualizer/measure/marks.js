// Do the marks a document carries reach the elements the page draws?
//
// Two chains, each from the Rust model to the class a drawn element gets:
//
//   the author's marks   `sce:unresolved` / `sce:assumed` on a state or a
//                        transition → `gui_structure()` → the node and link
//                        builders → `authorMarkClassFor`.
//   a requirement        `sce:req` on a transition → `annotation_overlay()`
//                        → the link builder's lookup by (source, position).
//
// ⚠ The second chain is here for a defect it already had. The link builder
// counted each source's transitions as it walked the structure — but the
// structure carries one object per TARGET, so a transition naming two
// targets was counted twice and every later transition of that source was
// looked up one position too far, receiving another transition's
// requirements or none. The position is `sourceIndex` now, and the document
// below puts a two-target transition in front of a claimed one so that the
// old count would hand the claim to nothing.
//
// Everything is the shipped code, loaded through `harness.js`; the
// documents are written here because each exists to hold one shape, and
// the assertions name what that shape is.

const vm = require('vm');
const path = require('path');
const { makeSandbox, loadEngine, ROOT } = require('./harness');
const { AnnotationOverlay } = require(path.join(ROOT, 'annotation-overlay.js'));

const MARKED = `<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="idle" datamodel="null" name="marks_probe">
  <state id="idle" sce:unresolved="IDLE_TIMEOUT" sce:unresolved-reason="No timeout given.">
    <transition event="go" target="running" sce:assumed="GO_FROM_IDLE" sce:assumed-reason="Read off the drawing."/>
    <transition event="halt" target="done"/>
    <transition event="quit" target="done" sce:assumed="QUIT_ENDS_IT" sce:assumed-reason="Guessed."/>
  </state>
  <state id="running">
    <transition event="stop" target="done" sce:unresolved="STOP_WHILE_RUNNING" sce:unresolved-reason="Open."/>
  </state>
  <final id="done"/>
</scxml>`;

const TWO_TARGETS = `<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" xmlns:sce="http://sce.dev/ext"
       version="1.0" initial="p" datamodel="null" name="two_targets_probe">
  <parallel id="p">
    <state id="left">
      <state id="l0">
        <transition event="both" target="l1 r1"/>
        <transition event="one" target="l2" sce:req="REQ-ONE"/>
      </state>
      <state id="l1"/>
      <state id="l2"/>
    </state>
    <state id="right">
      <state id="r0"/>
      <state id="r1"/>
    </state>
  </parallel>
</scxml>`;

let failures = 0;
const check = (ok, why) => { if (!ok) { console.error('FAIL: ' + why); failures++; } };

async function build(rust, elk, text, name, withOverlay) {
    const structure = JSON.parse(rust.gui_structure(text, name));
    if (withOverlay) {
        structure.annotationOverlay = new AnnotationOverlay(
            JSON.parse(rust.annotation_overlay(text, name)));
    }
    const sandbox = makeSandbox(elk);
    const SCXMLVisualizer = vm.runInContext('SCXMLVisualizer', sandbox);
    const v = new SCXMLVisualizer('probe', structure);
    await v.initPromise;
    return { v, classFor: vm.runInContext('authorMarkClassFor', sandbox) };
}

const arrow = (v, source, event) => v.allLinks.find(
    (l) => l.linkType === 'transition' && l.source === source
        && (l.transitions || [l]).some((t) => t.event === event));

(async () => {
    const { rust, elk } = await loadEngine();

    // The author's marks.
    {
        const { v, classFor } = await build(rust, elk, MARKED, 'marks_probe', false);
        const idle = v.nodes.find((n) => n.id === 'idle');
        check(idle && classFor(idle) === ' sce-unresolved',
            `state idle carries sce:unresolved, drawn with '${idle && classFor(idle)}'`);
        check(idle && idle.unresolved && idle.unresolved.join() === 'IDLE_TIMEOUT',
            'state idle does not name its marker IDLE_TIMEOUT');

        const running = v.nodes.find((n) => n.id === 'running');
        check(running && classFor(running) === '',
            `state running carries no mark, drawn with '${running && classFor(running)}'`);

        const go = arrow(v, 'idle', 'go');
        check(go && classFor(go) === ' sce-assumed',
            `transition idle→running carries sce:assumed, drawn with '${go && classFor(go)}'`);

        const stop = arrow(v, 'running', 'stop');
        check(stop && classFor(stop) === ' sce-unresolved',
            `transition running→done carries sce:unresolved, drawn with '${stop && classFor(stop)}'`);

        // `halt` and `quit` share idle→done and are drawn as ONE arrow; only
        // `quit` is marked, and the arrow must not lose that to the merge.
        const merged = arrow(v, 'idle', 'halt');
        check(merged && (merged.transitions || []).length === 2,
            'idle→done was expected to be one arrow for two transitions');
        check(merged && classFor(merged) === ' sce-assumed'
                && (merged.assumed || []).join() === 'QUIT_ENDS_IT',
            `the merged idle→done arrow dropped its second transition's mark (drawn with '${merged && classFor(merged)}')`);
    }

    // A requirement behind a two-target transition.
    {
        const { v } = await build(rust, elk, TWO_TARGETS, 'two_targets_probe', true);
        const one = arrow(v, 'l0', 'one');
        check(one && (one.requirements || []).join() === 'REQ-ONE',
            `l0's second transition claims REQ-ONE, the arrow carries [${one && one.requirements}]`);
        for (const link of v.allLinks.filter((l) => l.linkType === 'transition'
            && l.source === 'l0' && (l.transitions || [l]).some((t) => t.event === 'both'))) {
            check((link.requirements || []).length === 0,
                `l0's two-target transition claims nothing, its arrow to ${link.target} carries [${link.requirements}]`);
        }
    }

    if (failures > 0) {
        console.error(`\n${failures} mark(s) did not reach the drawing`);
        process.exit(1);
    }
    console.log('marks: every author mark and requirement reached the element drawn for it');
})().catch((e) => { console.error(e); process.exit(1); });
