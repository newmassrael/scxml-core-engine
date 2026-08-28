#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Turn the declared runtime-rewriter divergences into an SCXML document that
# asks each one as a `cond=` or an `expr=`.
#
# Why this is generated rather than committed. The population is
# `tests/ecmascript/lua_engine_divergences.json` — the set the RUNTIME rewriter
# answers differently from ECMA-262 — and a committed fixture would be a second
# copy of it, one edit away from asking 22 questions while the list holds 23.
# Generating it makes the list the only place the population lives: an entry
# added there grows a case here on the same commit, and an entry removed takes
# its case with it. That is what lets the count reach zero, which a fixture with
# its own hand-maintained case list could not.
#
# What the document is FOR is the other half of the seam
# (docs/SCE_LUA_TRANSLATION_SEAM.md): generated with `--script-engine lua` the
# C++ artifact hands its engine text the BUILD-TIME frontend lowered, so the
# rewriter these divergences belong to is bypassed and every one of them has to
# answer the language. Generated without that flag the same document hands the
# author's ECMAScript to the same engine, the rewriter runs, and the divergences
# come back. So one document measures both sides of the seam, and which side is
# under test is a codegen flag rather than a second fixture.
#
# The expectations are deliberately NOT emitted. They stay in
# `tests/ecmascript/ecma262_semantics.json`, which the C++ harness reads at run
# time: a fixture that carried the expected answers would be compared against
# itself, and the lowering under test would be judging its own homework.
#
# Refusal, not omission. A divergence entry this script cannot express is a
# hard error — a skipped case reads as a passing one, and the escape hatch
# would defeat the gate that owns it.

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from xml.sax.saxutils import escape, quoteattr

# The answer alphabet for a `cond=` case. Three values rather than a boolean,
# because "the guard was false" and "the guard raised" are different findings
# and a boolean cannot hold both: a case expecting `false` would pass on an
# expression that threw.
#
# The positive guard decides TRUE, its negation decides FALSE, and the
# unguarded fallthrough decides NEITHER. §scxml-5.9.1 makes a `cond` that
# raises evaluate to false, so an expression the engine refuses fails BOTH
# guards and lands on the fallthrough — which is the only way `NEITHER` can be
# reached and is therefore a verdict, not a default.
COND_TRUE = 1
COND_FALSE = 2
COND_NEITHER = 3

# Answer shapes the harness can compare. `empty` is absent on purpose: a
# variable assigned null/undefined is omitted from the engine's JSON encoding,
# which is indistinguishable from a case that never ran. Listing it here would
# make that case pass by not being asked.
RECORDABLE = ("bool", "number", "string")


def die(msg: str) -> None:
    print(f"generate_lowered_ecma262_fixture: {msg}", file=sys.stderr)
    raise SystemExit(1)


def load(path: pathlib.Path) -> dict:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except OSError as exc:
        die(f"cannot read {path}: {exc}")
    except json.JSONDecodeError as exc:
        die(f"{path} is not JSON: {exc}")


def join(cases: list[dict], divergences: list[dict]) -> list[tuple[int, dict]]:
    """Pair each divergence entry with the shared table's case.

    Keyed on (source, clause) because `source` alone is not unique — `a && b`
    appears twice, once as a condition and once as a value, and the clause is
    what tells them apart. A key matching zero or several cases is an error
    here rather than a guess: the divergence list would be naming a case that
    does not exist, or two.
    """
    index: dict[tuple[str, str], list[int]] = {}
    for i, case in enumerate(cases):
        index.setdefault((case["source"], case["clause"]), []).append(i)

    paired: list[tuple[int, dict]] = []
    for n, entry in enumerate(divergences):
        key = (entry.get("source"), entry.get("clause"))
        hits = index.get(key, [])
        if len(hits) != 1:
            die(
                f"divergence {n} ({key[0]!r} / {key[1]!r}) names "
                f"{len(hits)} case(s) in the shared table; it must name exactly one"
            )
        paired.append((n, cases[hits[0]]))
    return paired


# Where the probe below parks its result before it is recorded, and the value it
# holds when the engine refused the expression.
#
# A BARE name on purpose. §scxml-5.4 assignment takes two different routes:
# a bare location is `evaluateExpression` then `setVariable`, and a dotted one is
# one `executeScript` of `<loc> = (<expr>);`. Those are different engine entry
# points with different semantics — `LuaEngine::evaluateExpressionInternal` runs
# the undeclared-variable ReferenceError check and `executeScriptInternal` does
# not — and a `cond=` guard goes through the FIRST. Measured 2026-08-29: a probe
# written to `answers.vN` directly reported `typeof missingVariable !==
# 'undefined'` as evaluable, because it was asking the entry point that does not
# raise. So the probe assigns to a bare name and only then records it.
PROBE_VAR = "probe"
PROBE_UNEVALUATED = "<unevaluated>"


def probe_var(n: int) -> str:
    """Where a `cond=` case records whether the engine could evaluate it AT ALL.

    §scxml-5.9.1 makes a `cond` that raises evaluate to false, so a guard
    verdict of "false" covers two different findings: the engine answered
    `false`, and the engine refused the expression. For an entry whose expected
    answer IS false those are indistinguishable — and one of the declared
    divergences turns out to sit exactly there.

    So each condition case first resets a bare variable to a sentinel and then
    assigns its own expression into it through the same entry point a guard
    uses. §scxml-5.4 leaves the location unchanged when the expression fails, so
    the sentinel surviving IS the refusal. The recorded value is deliberately
    NOT compared against the expectation: the shared table's answer for these
    entries is the answer a `cond=` gives, not the value form's (`a && b` is
    `false` as a condition and `0` as a value).
    """
    return f"answers.v{n}"


def answer_var(n: int) -> str:
    """The datamodel path a case records into.

    One `<data>` object holds every answer, read back through the generated
    `answers()` accessor — the engine's own `JSON.stringify`, which is the only
    read surface that carries a value's TYPE. The per-variable scalar accessors
    are typed from the authored literal, so a table holding booleans, numbers
    and strings would need three of them and a case whose answer changed type
    would read as absent rather than as wrong.
    """
    return f"answers.d{n}"


def repr_js(text: str) -> str:
    """A single-quoted ECMAScript string literal for `text`.

    Only ever called on this file's own sentinel, so escaping is a guard against
    the sentinel being changed to something with a quote in it rather than a
    general-purpose literal writer.
    """
    escaped = text.replace("\\", "\\\\").replace("'", "\\'")
    return f"'{escaped}'"


def setup_script(case: dict) -> list[str]:
    """The case's ECMAScript preamble, as the `<script>` a document would use.

    Emitted per case rather than once at the top: the table's setups reuse `a`,
    `b`, `v` and `n`, so a case's declarations have to be the last thing to run
    before its own expression. A case with no setup contributes nothing — an
    empty `<onentry>` is a §scxml-3.8 element with no executable content, and
    emitting one would put a shape in the fixture that no case asked for.
    """
    setup = case.get("setup", "")
    if not setup.strip():
        return []
    return [f"      <script>{escape(setup)}</script>"]


def emit_onentry(body: list[str], out: list[str]) -> None:
    if not body:
        return
    out.append("    <onentry>")
    out.extend(body)
    out.append("    </onentry>")


def emit_condition_case(n: int, case: dict, target: str, out: list[str]) -> None:
    source = case["source"]
    var = answer_var(n)
    # The probe runs BEFORE the guards, which is only sound while no condition
    # case has a side effect — asserted in `build()` against the shared table's
    # own `side-effecting` group rather than assumed here.
    probe = [
        f'      <assign location="{PROBE_VAR}" expr={quoteattr(repr_js(PROBE_UNEVALUATED))}/>',
        f'      <assign location="{PROBE_VAR}" expr={quoteattr(source)}/>',
        f'      <assign location="{probe_var(n)}" expr="{PROBE_VAR}"/>',
    ]
    out.append(f'  <state id="d{n}">')
    emit_onentry(setup_script(case) + probe, out)
    for cond, verdict in (
        (source, COND_TRUE),
        ("!(" + source + ")", COND_FALSE),
        (None, COND_NEITHER),
    ):
        guard = f" cond={quoteattr(cond)}" if cond is not None else ""
        out.append(
            f"    <transition{guard} target={quoteattr(target)}>"
            f'<assign location="{var}" expr="{verdict}"/></transition>'
        )
    out.append("  </state>")


def emit_value_case(n: int, case: dict, target: str, out: list[str]) -> None:
    source = case["source"]
    var = answer_var(n)
    out.append(f'  <state id="d{n}">')
    emit_onentry(
        setup_script(case) + [f'      <assign location="{var}" expr={quoteattr(source)}/>'],
        out,
    )
    out.append(f"    <transition target={quoteattr(target)}/>")
    out.append("  </state>")


def build(paired: list[tuple[int, dict]], divergences_rel: str, cases_rel: str) -> str:
    out: list[str] = []
    out.append('<?xml version="1.0" encoding="UTF-8"?>')
    out.append("<!--")
    out.append("  GENERATED by tools/generate_lowered_ecma262_fixture.py — do not edit.")
    out.append("")
    out.append(f"  One state per entry of {divergences_rel}, asking the")
    out.append(f"  expression that entry names in the form {cases_rel} says it")
    out.append("  takes. State `dN` asks divergence N and records into `answers.dN`.")
    out.append("")
    out.append("  The expected answers are NOT here: the harness reads them from the")
    out.append("  shared table, so the lowering under test cannot mark its own work.")
    out.append("-->")
    first = f"d{paired[0][0]}" if paired else "done"
    out.append(
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
        f'datamodel="ecmascript" initial="{first}">'
    )
    out.append("  <datamodel>")
    out.append('    <data id="answers" expr="{}"/>')
    out.append(f'    <data id="{PROBE_VAR}" expr="\'\'"/>')
    out.append("  </datamodel>")

    for pos, (n, case) in enumerate(paired):
        target = f"d{paired[pos + 1][0]}" if pos + 1 < len(paired) else "done"
        form = case.get("form")
        expect = case.get("expect", {})
        shape = next(iter(expect), None)
        if shape not in RECORDABLE:
            die(
                f"divergence {n} ({case['source']!r}) expects a {shape!r} answer, "
                f"which no datamodel variable can hold distinguishably from an "
                f"absent one. Recordable shapes: {', '.join(RECORDABLE)}."
            )
        if form == "condition":
            # The refusal probe runs before this case's guards, so a condition
            # whose evaluation changes the datamodel would be asked twice and
            # the guards would see the second state. Refused rather than
            # tolerated: the shared table labels such cases itself, so the
            # constraint is checked against its own vocabulary instead of a
            # reviewer noticing.
            if case.get("group") == "side-effecting":
                die(
                    f"divergence {n} ({case['source']!r}) is a condition in the "
                    f"shared table's `side-effecting` group. The refusal probe "
                    f"evaluates the expression once before the guards, so a "
                    f"side effect would make the guards see a different "
                    f"datamodel. Give the probe its own state before extending "
                    f"the fixture to this shape."
                )
            emit_condition_case(n, case, target, out)
        elif form == "value":
            emit_value_case(n, case, target, out)
        else:
            die(
                f"divergence {n} ({case['source']!r}) has form {form!r}; "
                f"this generator expresses 'condition' and 'value'"
            )

    out.append('  <final id="done"/>')
    out.append("</scxml>")
    out.append("")
    return "\n".join(out)


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--cases", required=True, type=pathlib.Path,
                    help="tests/ecmascript/ecma262_semantics.json")
    ap.add_argument("--divergences", required=True, type=pathlib.Path,
                    help="tests/ecmascript/lua_engine_divergences.json")
    ap.add_argument("-o", "--output", required=True, type=pathlib.Path,
                    help="SCXML document to write")
    args = ap.parse_args()

    cases = load(args.cases).get("cases")
    divergences = load(args.divergences).get("divergences")
    if not isinstance(cases, list) or not cases:
        die(f"{args.cases} carries no `cases` array")
    if not isinstance(divergences, list):
        die(f"{args.divergences} carries no `divergences` array")

    # An empty list is the axis's own end state — every divergence closed — and
    # a fixture with no case would then be a suite that passes by asking
    # nothing. Say so here rather than emit it: whoever empties the list should
    # decide what this gate becomes, and be told by a failure rather than by a
    # green run.
    if not divergences:
        die(
            f"{args.divergences} is empty. Every declared divergence is closed, "
            f"so this fixture would ask nothing and pass. Retire the gate or "
            f"repoint it at the shared table in full."
        )

    paired = join(cases, divergences)
    doc = build(paired,
                divergences_rel=args.divergences.name,
                cases_rel=args.cases.name)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(doc, encoding="utf-8")
    print(f"wrote {args.output} — {len(paired)} case(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
