// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The browser's Interpreter agrees with the generated backends on the scenarios
// they all replay.
//
// Why: the visualizer runs a `sce-static` statechart by lowering it
// (`SceBuildWasm.lowerForInterpreter`, the codegen WASM) and handing the result
// to the C++ engine built for the browser (`InteractiveTestRunner`). Every other
// lane that replays the scenario corpus under
// `sce-build/tests/fixtures/static_datamodel/scenarios/` runs a native binary;
// none runs THIS pair — the lowering as the page calls it, on the engine as the
// page loads it — so a change that broke only the browser's path (the sibling
// fetch, the document set, the WASM build of the engine) would pass them all.
// The expected values are the corpus's, derived from the documents and not
// observed from any backend.
//
// ⚠ What it cannot replay is skipped by NAME, with the reason, and counted: a
// scenario that needs time, a dispatcher for what the machine sends, an `invoke`
// or a host this runner has none of would fail here for a reason that says
// nothing about the lowering. The replayed and skipped counts are printed and the
// replayed count has a floor, so a comparison that quietly stopped being made
// shows as a number that fell and not as a green run.
//
// Usage: node web/visualizer/measure/scenario-replay.js [scenario.json ...]
// Without arguments, every scenario under the shared fixture directory.

'use strict';

const fs = require('fs');
const path = require('path');
const { isDeepStrictEqual } = require('util');

const ROOT = path.resolve(__dirname, '..');
const REPO = path.resolve(ROOT, '..', '..');
const FIXTURES = path.join(REPO, 'sce-build', 'tests', 'fixtures', 'static_datamodel');

/**
 * The fewest scenarios a full run must replay: 31 of the 56 when the lane was
 * added (2026-10-13), the rest skipped by name below. It rises as the runner
 * learns more of the vocabulary and is lowered only by a decision that names
 * what stopped being replayed.
 */
const MIN_REPLAYED = 31;

/** The `expect` members this runner can judge. */
const JUDGED = new Set(['state', 'configuration', 'variables', 'ended']);

/** Elements that make a machine need more than this runner is given. */
const NEEDS = [
    [/<invoke\b/, 'an <invoke>: a child session needs a dispatcher'],
    [/<send\b/, 'a <send>: the runner wires no dispatcher for what a machine sends'],
    [/<sce:action\b/, 'a host action: the runner is given no host'],
];

/** Why a scenario cannot be replayed here, or null when it can. */
function whyNot(scenario, document) {
    for (const [pattern, why] of NEEDS) {
        if (pattern.test(document)) return why;
    }
    for (const step of scenario.steps) {
        if ('advance_ms' in step) return 'a step that waits: the runner has no clock to move on';
        for (const key of Object.keys(step.expect || {})) {
            if (!JUDGED.has(key)) return `an expectation of \`${key}\`, which this runner cannot observe`;
        }
    }
    return null;
}

/** A list the engine returns, whether it is an array or an embind vector. */
function listOf(value) {
    if (Array.isArray(value)) return value;
    const out = [];
    for (let i = 0; i < value.size(); i++) out.push(value.get(i));
    return out;
}

/** Run the queued events to rest: a microstep each, until none is queued. */
function settle(runner) {
    for (let i = 0; i < 1000 && runner.hasQueuedEvents(); i++) runner.stepForward();
}

/** A variable of the running machine as the JSON the script engine writes. */
function variable(runner, name) {
    const text = runner.evaluateExpression(`JSON.stringify(${name})`);
    return text === '' ? null : JSON.parse(text);
}

/**
 * Whether the variable `got`, as the script engine writes it, is the value `want` a
 * scenario states. A 64-bit integer is written in a scenario as its decimal digits,
 * because a JSON number holds no more than 53 bits, and the engine holds it as a
 * number: the two are the same value when the number's digits are the string, as
 * every other lane that replays the scenarios reads them. Records and lists are
 * compared member by member; nothing else is loosened.
 */
function sameValue(got, want) {
    if (typeof got === 'number' && typeof want === 'string' && /^-?\d+$/.test(want)) {
        return Number.isSafeInteger(got) && String(got) === want;
    }
    if (Array.isArray(want)) {
        return Array.isArray(got) && got.length === want.length
            && want.every((member, i) => sameValue(got[i], member));
    }
    if (want !== null && typeof want === 'object') {
        if (got === null || typeof got !== 'object' || Array.isArray(got)) return false;
        const names = Object.keys(want);
        return Object.keys(got).length === names.length
            && names.every((name) => name in got && sameValue(got[name], want[name]));
    }
    return isDeepStrictEqual(got, want);
}

/** The documents beside `document`, answered from the fixture directory. */
function siblingFromFixtures(name) {
    const file = path.join(FIXTURES, name);
    return fs.existsSync(file) ? fs.readFileSync(file, 'utf8') : null;
}

/**
 * The text of the document a scenario is of. A scenario names its machine by a file of the
 * fixture directory, or, when the machine is a standard document, by the name the library
 * gives it (`sce:std/shared/record_list`), which is the file of that path under `stdlib`.
 */
function documentOf(scenario) {
    const standard = 'sce:std/';
    const file = scenario.machine.startsWith(standard)
        ? path.join(REPO, 'stdlib', `${scenario.machine.slice(standard.length)}.scxml`)
        : path.join(FIXTURES, `${scenario.machine}.scxml`);
    return fs.readFileSync(file, 'utf8');
}

/** The failures of one scenario, as sentences; empty when it holds. */
async function replay(Module, SceBuildWasm, scenario) {
    const document = documentOf(scenario);
    const lowered = await SceBuildWasm.lowerForInterpreter(
        document, scenario.machine, async (name) => siblingFromFixtures(name));
    const runner = new Module.InteractiveTestRunner();
    const failures = [];
    try {
        if (!runner.loadSCXML(lowered[0].text, false)) return ['the engine did not load the lowered document'];
        if (!runner.initialize()) return ['the engine did not start the lowered document'];
        scenario.steps.forEach((step, n) => {
            const at = `step ${n}${step.note ? ` (${step.note})` : ''}`;
            if ('event' in step) {
                runner.raiseEvent(step.event, 'data' in step ? JSON.stringify(step.data) : '');
                settle(runner);
            }
            const expect = step.expect || {};
            const active = listOf(runner.getActiveStates());
            if (expect.ended) {
                if (!runner.isInFinalState()) failures.push(`${at}: the machine did not end in a top-level <final>`);
                return;
            }
            if ('state' in expect && !active.includes(expect.state)) {
                failures.push(`${at}: the active states ${JSON.stringify(active)} do not hold \`${expect.state}\``);
            }
            if ('configuration' in expect) {
                const want = [...expect.configuration].sort();
                const got = [...active].sort();
                if (!isDeepStrictEqual(want, got)) {
                    failures.push(`${at}: the active states are ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
                }
            }
            for (const [name, want] of Object.entries(expect.variables || {})) {
                const got = variable(runner, name);
                if (!sameValue(got, want)) {
                    failures.push(`${at}: variable \`${name}\` is ${JSON.stringify(got)}, not ${JSON.stringify(want)}`);
                }
            }
        });
    } finally {
        if (typeof runner.delete === 'function') runner.delete();
    }
    return failures;
}

async function main(files) {
    const Module = await require(path.join(ROOT, 'visualizer.js'))();
    const rust = await import(path.join(ROOT, 'wasm', 'sce_build.js'));
    rust.initSync({ module: fs.readFileSync(path.join(ROOT, 'wasm', 'sce_build_bg.wasm')) });
    // The wrapper the page loads, on the module this harness initialised.
    const SceBuildWasm = require(path.join(ROOT, 'sce-build-wasm.js'));
    SceBuildWasm.provide(rust);

    let replayed = 0;
    let failed = 0;
    const skipped = [];
    for (const file of files) {
        const scenario = JSON.parse(fs.readFileSync(file, 'utf8'));
        const document = documentOf(scenario);
        const why = whyNot(scenario, document);
        if (why) {
            skipped.push(`${scenario.machine}: ${why}`);
            continue;
        }
        let failures;
        try {
            failures = await replay(Module, SceBuildWasm, scenario);
        } catch (e) {
            failures = [`could not be replayed: ${String(e).split('\n')[0]}`];
        }
        replayed++;
        if (failures.length > 0) {
            failed++;
            console.log(`FAIL ${scenario.machine}`);
            failures.forEach((f) => console.log(`    ${f}`));
        }
    }

    console.log(`scenario-replay: ${replayed} replayed, ${skipped.length} skipped, ${failed} failed`);
    skipped.forEach((s) => console.log(`    skipped ${s}`));
    if (failed > 0) {
        process.exit(1);
    }
    if (files.length === 0) {
        console.log('scenario-replay: no scenario was given; an empty comparison is not a clean one');
        process.exit(1);
    }
    // The floor is for the whole corpus; a run narrowed to some files is a hunt.
    if (process.argv.length <= 2 && replayed < MIN_REPLAYED) {
        console.log(`scenario-replay: only ${replayed} replayed, below the floor of ${MIN_REPLAYED}`);
        process.exit(1);
    }
}

const given = process.argv.slice(2);
const scenarios = given.length > 0
    ? given.map((f) => path.resolve(f))
    : fs.readdirSync(path.join(FIXTURES, 'scenarios'))
        .filter((f) => f.endsWith('.json'))
        .sort()
        .map((f) => path.join(FIXTURES, 'scenarios', f));
main(scenarios).catch((e) => {
    console.error(e);
    process.exit(1);
});
