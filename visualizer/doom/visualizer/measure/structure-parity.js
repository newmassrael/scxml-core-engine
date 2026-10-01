// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The structure the GUI draws, from the Rust model, agrees with the one the
// C++ engine builds — on every field the GUI joins to the running engine.
//
// Why: the GUI takes its drawing input from `sce_build::gui_structure` (the
// Rust model every other review surface reads) while execution stays with
// the C++ interpreter. The live highlight joins the drawn structure to that
// runtime by state id and by (source, sourceIndex), so a disagreement there
// lights the wrong state or the wrong arrow. This compares the two producers
// document by document.
//
// ⚠ A difference passes ONLY where a DECLARED predicate below recognises
// that exact shape — never by deleting a field from both sides. Each one is
// a place the two readers legitimately differ, and says which is right.
//
// Usage: node web/visualizer/measure/structure-parity.js <document>...

'use strict';

const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..');
const REPO = path.resolve(ROOT, '..', '..');

/** The two additions the C++ builder never had; not compared. */
const RUST_ONLY = new Set(['unresolved', 'assumed']);

/**
 * Differences the two readers are allowed, each with its reason. A predicate
 * gets the dotted path, the C++ value and the Rust value.
 */
const DECLARED = [
    {
        why: 'inline <invoke><content>: the same child document; C++ re-serialises it, the model keeps the author\'s text',
        applies: (p, c, r) => /\.invokeContent$/.test(p) && typeof c === 'string' && typeof r === 'string',
    },
    {
        why: '<scxml> with no initial: C++ reports "", the model the §3.2 default (the first child state)',
        applies: (p, c, r) => p === '.initial' && c === '' && typeof r === 'string' && r !== '',
    },
    {
        why: 'a language-prefixed <script>: C++ keeps the written wrapper, the model the lowered body it runs',
        applies: (p, c, r) => /\.content$/.test(p) && typeof c === 'string' && typeof r === 'string'
            && /^<(cpp|kt|kotlin|rust)>/.test(c.trim()),
    },
    {
        why: '<log label>: the C++ parser drops it; the model keeps what the author wrote',
        applies: (p, c, r) => /\.label$/.test(p) && c === undefined && typeof r === 'string',
    },
    {
        why: 'a transition cond the C++ builder omits (W3C 519, 534); the model keeps it',
        applies: (p, c, r) => /^\.transitions\[\d+\]\.cond$/.test(p) && c === undefined && typeof r === 'string',
    },
    {
        why: 'an <invoke type> or typeexpr spelling: C++ repeats it, the model lowers the invoke and keeps its kind, not the spelling',
        applies: (p, c) => /\.invokeType(Expr)?$/.test(p) && typeof c === 'string',
    },
    {
        why: '<finalize>: C++ repeats its XML, the model holds the lowered script it runs',
        applies: (p, c, r) => /\.invokeFinalize$/.test(p) && typeof c === 'string' && typeof r === 'string',
    },
    {
        why: '<content> text or markup: the same content, whitespace and serialisation differ (W3C 561, 562)',
        applies: (p, c, r) => /\.content$/.test(p) && typeof c === 'string' && typeof r === 'string'
            && c.replace(/\s+/g, '') === r.replace(/\s+/g, ''),
    },
    {
        why: '<assign> with inline content (W3C 530): C++ quotes the child into expr, the model keeps the child as content',
        applies: (p, c, r) => /\.expr$/.test(p) && typeof c === 'string' && c.startsWith('"<') && r === '',
    },
    {
        why: 'a multi-token initial: C++ keeps the first token, the model all of them (the GUI splits on whitespace)',
        applies: (p, c, r) => /\.initial$/.test(p) && typeof c === 'string' && typeof r === 'string'
            && c !== '' && r.split(/\s+/)[0] === c && r.includes(' '),
    },
    {
        why: 'an <if> nested in a branch: the C++ builder drops it; the model keeps the whole block',
        applies: (p, c, r) => /\.branches\[\d+\]\.actions$/.test(p) && Array.isArray(c) && Array.isArray(r)
            && r.length > c.length && r.some((a) => a && a.actionType === 'if'),
    },
];

function differences(c, r, at = '', out = []) {
    if (JSON.stringify(c) === JSON.stringify(r)) return out;
    if (Array.isArray(c) && Array.isArray(r) && c.length === r.length) {
        c.forEach((_, i) => differences(c[i], r[i], `${at}[${i}]`, out));
        return out;
    }
    if (c && r && typeof c === 'object' && typeof r === 'object'
            && !Array.isArray(c) && !Array.isArray(r)) {
        const keys = new Set([...Object.keys(c), ...Object.keys(r)]);
        for (const k of [...keys].sort()) {
            if (RUST_ONLY.has(k)) continue;
            differences(c[k], r[k], `${at}.${k}`, out);
        }
        return out;
    }
    out.push({ path: at, cpp: c, rust: r });
    return out;
}

async function main(documents) {
    const Module = await require(path.join(ROOT, 'visualizer.js'))();
    const rust = await import(path.join(ROOT, 'wasm', 'sce_build.js'));
    rust.initSync({ module: fs.readFileSync(path.join(ROOT, 'wasm', 'sce_build_bg.wasm')) });

    let failed = 0;
    const declared = new Map();
    for (const doc of documents) {
        const text = fs.readFileSync(path.join(REPO, doc), 'utf8');
        // A document either reader cannot load is a failure by name, not a
        // skip: the list is pinned, and a fixture that stopped loading would
        // otherwise shrink the comparison in silence.
        let cpp;
        let rs;
        try {
            const runner = new Module.InteractiveTestRunner();
            runner.loadSCXML(text, false);
            cpp = runner.getSCXMLStructure();
            rs = JSON.parse(rust.gui_structure(text, path.basename(doc)));
        } catch (e) {
            failed++;
            console.log(`CANNOT COMPARE ${doc}: ${String(e).split('\n')[0]}`);
            continue;
        }
        const undeclared = [];
        for (const d of differences(cpp, rs)) {
            const rule = DECLARED.find((x) => x.applies(d.path, d.cpp, d.rust));
            if (rule) declared.set(rule.why, (declared.get(rule.why) || 0) + 1);
            else undeclared.push(d);
        }
        if (undeclared.length) {
            failed++;
            console.log(`DIFFERS ${doc}`);
            for (const d of undeclared.slice(0, 5)) {
                console.log(`  ${d.path}: C++ ${JSON.stringify(d.cpp)} / Rust ${JSON.stringify(d.rust)}`);
            }
        }
    }
    for (const [why, n] of declared) console.log(`declared (${n}): ${why}`);
    console.log(`structure-parity: ${documents.length - failed} of ${documents.length} agree`);
    return failed === 0;
}

main(process.argv.slice(2)).then(
    (ok) => process.exit(ok ? 0 : 1),
    (e) => { console.error(e); process.exit(2); },
);
