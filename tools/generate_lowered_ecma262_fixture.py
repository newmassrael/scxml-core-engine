#!/usr/bin/env python3
# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#
# Turn the shared ECMA-262 table into an SCXML document that asks every case as
# a `cond=` or an `expr=`.
#
# WHAT THE POPULATION IS, and why it changed. This script used to expand
# `tests/ecmascript/lua_engine_divergences.json` — the set the RUNTIME rewriter
# answers differently — into one state per entry. That made the fixture a
# projection of the very list the harness then checked, with two consequences
# neither of which was visible from a green run:
#
#   1. The other 75 cases of the shared table were never asked through a
#      build-time-lowered artifact at all. Build-time lowering could answer any
#      of them wrongly and no gate would see it, because the population was
#      built from the OTHER path's failures. A path's divergences cannot be
#      enumerated by a list derived from a different path.
#   2. Deleting an entry deleted its case. The list could only be checked
#      against questions it had itself chosen, so an entry removed by mistake
#      removed the question that would have caught the mistake.
#
# The population is now the shared table, in full, and the divergence list is
# read only by the HARNESS — as the expectation about which cases lowering gets
# wrong. That is the shape this file's own refusal already prescribed when the
# list empties: *"Retire the gate or repoint it at the shared table in full."*
#
# What the document is FOR (docs/SCE_LUA_TRANSLATION_SEAM.md): generated with
# `--script-engine lua` the C++ artifact hands its engine text the BUILD-TIME
# frontend lowered, so the runtime rewriter is bypassed. Generated without that
# flag the same document hands the author's ECMAScript to the same engine and
# the rewriter runs. So one document measures both sides of the seam, and which
# side is under test is a codegen flag rather than a second fixture.
#
# The expectations are deliberately NOT emitted. They stay in
# `tests/ecmascript/ecma262_semantics.json`, which the C++ harness reads at run
# time: a fixture that carried the expected answers would be compared against
# itself, and the lowering under test would be judging its own homework.
#
# Refusal, not omission. A case this script cannot express is a hard error — a
# skipped case reads as a passing one, and the escape hatch would defeat the
# gate that owns it.

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from xml.sax.saxutils import escape, quoteattr

# The answer alphabet for a `cond=` case: the guard held, or it did not.
#
# ⚠ There used to be a THIRD value, reached by asking the guard AND `!(source)`
# and calling the unguarded fallthrough "neither held — the engine refused the
# expression". That claim is false, and the gate found it by measuring:
# `cond="a"` with `var a = 0` under the runtime rewriter answers false
# CORRECTLY, and `cond="!(a)"` answers false too, because the rewriter hands
# Lua `not a` and Lua counts 0 as true. Both guards false, fallthrough taken,
# and the harness reported an engine refusal that had not happened — on three
# cases at once (§7.1.2 ToBoolean of 0, '' and NaN).
#
# The defect is asking a DERIVED expression. `!a` is a divergence of its own
# (§12.5.9) and the shared table already asks it as a case; folding it into
# every other condition case's protocol made one entry's divergence show up as
# three other entries' refusals.
#
# So the fixture asks only what the author wrote, and "could the engine
# evaluate this at all" is answered by the probe below — which exists for
# exactly that question, reaches the same entry point a `cond=` reaches, and
# carries its own control in the harness.
COND_HELD = 1
COND_NOT_HELD = 2

# Shared with `LoweredEcma262Test.cpp` and `ecma262_scoreboard_contract.rs`,
# which carry the same number for the same reason: a table that shrank to
# nothing would score every engine perfectly, and a fixture expanded from it
# would pass by asking nothing.
MIN_CASES = 55

# Answer shapes the harness can compare.
#
# `empty` is here now, and admitting it took the recording protocol below
# rather than a tolerance. A variable holding null/undefined is OMITTED from
# the engine's JSON encoding, so "the answer is undefined" and "this case was
# never asked" arrive as the same absence — which is why this shape used to be
# refused. The sentinel-first ordering in `emit_value_case` separates them, so
# the shape is now expressible rather than excluded.
RECORDABLE = ("bool", "number", "string", "empty")


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


# Where the refusal probe parks its result, and the value it holds when the
# engine could not evaluate the expression.
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

# What a case's answer slot holds before the case's own expression is asked.
#
# Same string as the probe's, and for the same reason — it is the mark of an
# expression the engine would not evaluate. It has to be written BEFORE the
# case's setup and expression so that the three outcomes are three different
# readings; see `emit_value_case`.
ANSWER_UNEVALUATED = PROBE_UNEVALUATED

# The probe's discriminating power, asked as two DECLARED effects rather than
# read off whatever the shared table happens to contain.
#
# The harness refuses to read a condition case's verdict unless the probe says
# the expression evaluated, so a probe stuck on one answer would decide the
# whole suite by itself. That was guarded by counting the table's own
# refusals and requiring at least one — and on 2026-08-29 the count reached
# ZERO, because the engine's lowering seam had grown to answer every guard in
# the table. A control whose zero is forbidden is not a control: it made the
# finish line unreachable and turned a repair into a red.
#
# So the two outcomes are now produced on purpose. Neither depends on a case:
#
#   * `answers.missing` is nil in an object nothing has written that key into,
#     so reading a member OF it raises — a TypeError in ECMA-262 and an
#     "index a nil value" in Lua, on either artifact and under either
#     lowering. The probe assignment therefore fails and the sentinel
#     survives, which is exactly what a refusal looks like.
#   * `1` is a literal. Nothing can refuse it.
#
# Both go through the BARE-name probe, so both ask the same entry point a
# `cond=` guard asks (§scxml-5.4 / `evaluateExpression`) — a control taken
# through the other entry point would not be about this probe at all.
CONTROL_REFUSED_EXPR = "answers.missing.deep"
CONTROL_EVALUABLE_EXPR = "1"
CONTROL_REFUSED_VAR = "answers.ctlRefused"
CONTROL_EVALUABLE_VAR = "answers.ctlEvaluable"


def answer_var(n: int) -> str:
    """The datamodel path a case records its answer into.

    One `<data>` object holds every answer, read back through the generated
    `answers()` accessor — the engine's own `JSON.stringify`, which is the only
    read surface that carries a value's TYPE. The per-variable scalar accessors
    are typed from the authored literal, so a table holding booleans, numbers
    and strings would need three of them and a case whose answer changed type
    would read as absent rather than as wrong.
    """
    return f"answers.d{n}"


def probe_answer_var(n: int) -> str:
    """Where a `cond=` case records whether the engine could evaluate it AT ALL.

    §scxml-5.9.1 makes a `cond` that raises evaluate to false, so a guard
    verdict of "false" covers two different findings: the engine answered
    `false`, and the engine refused the expression. For an entry whose expected
    answer IS false those are indistinguishable — and one of the declared
    divergences turns out to sit exactly there.
    """
    return f"answers.v{n}"


def reached_var(n: int) -> str:
    """The mark that this case's state was actually entered.

    Written as the FIRST action of the case's own state, before anything that
    can fail. Without it, "the expression answered undefined", "the engine
    refused the expression" and "this state was never reached" are one absence —
    and the third is what a stale fixture, a machine that stopped early, or a
    generator and harness that disagree about the population all look like. An
    absence that can mean "we did not ask" is not an answer, so the harness
    reads this before it reads anything else.
    """
    return f"answers.r{n}"


def repr_js(text: str) -> str:
    """A single-quoted ECMAScript string literal for `text`.

    Only ever called on this file's own sentinels, so escaping is a guard
    against a sentinel being changed to something with a quote in it rather
    than a general-purpose literal writer.
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
    """Two states: one that probes, one that asks the guard.

    The probe is not an extra: with a two-way guard the fallthrough alone
    cannot tell "the expression is false" from "the engine would not evaluate
    it", and §scxml-5.9.1 makes both arrive as a guard that did not hold. The
    harness therefore refuses to read a condition case's verdict at all unless
    the probe says the expression evaluated — which is what keeps a lowered
    artifact emitting unparseable Lua from passing every case whose expected
    answer is false.

    The probe used to share the case's state, which forced this generator to
    REFUSE any condition in the shared table's `side-effecting` group: the
    probe evaluates the expression once, so the guards after it would see a
    datamodel the probe had already changed. Measured on `++v == 2` with
    `var v = 1`: probing first leaves `v` at 2, the positive guard then makes it
    3 and answers false, and the negation makes it 4 and answers true — so the
    fixture would have recorded FALSE for a case whose answer is true, without
    anything being wrong with the engine.

    Splitting the probe into its own state and re-running the setup in the
    second gives the guards the datamodel the case describes. That retires the
    refusal by construction rather than by exempting a group — an exemption
    list is the shape `docs/SCE_LUA_TRANSLATION_SEAM.md` names as "a hole one
    line wide", and here it would have silently excused the three cases in the
    table that exercise evaluation order.
    """
    source = case["source"]
    out.append(f'  <state id="p{n}">')
    emit_onentry(
        setup_script(case)
        + [
            f'      <assign location="{PROBE_VAR}" expr={quoteattr(repr_js(PROBE_UNEVALUATED))}/>',
            f'      <assign location="{PROBE_VAR}" expr={quoteattr(source)}/>',
        ],
        out,
    )
    out.append(f'    <transition target="d{n}"/>')
    out.append("  </state>")

    # `probe` is read into the answer object HERE rather than in the state that
    # wrote it. §scxml-4.9 stops a block at the element that raised, so a probe
    # assignment the engine refused would take the recording line down with it
    # and the refusal would arrive as an absence indistinguishable from the
    # state never running. Read in the NEXT block, the sentinel always makes it
    # out, and reading a bare name cannot itself fail.
    out.append(f'  <state id="d{n}">')
    emit_onentry(
        [
            f'      <assign location="{reached_var(n)}" expr="1"/>',
            f'      <assign location="{probe_answer_var(n)}" expr="{PROBE_VAR}"/>',
        ]
        + setup_script(case),
        out,
    )
    for cond, verdict in ((source, COND_HELD), (None, COND_NOT_HELD)):
        guard = f" cond={quoteattr(cond)}" if cond is not None else ""
        out.append(
            f"    <transition{guard} target={quoteattr(target)}>"
            f'<assign location="{answer_var(n)}" expr="{verdict}"/></transition>'
        )
    out.append("  </state>")


def emit_value_case(n: int, case: dict, target: str, out: list[str]) -> None:
    """One state, and the ORDER of its four actions is the whole protocol.

    An answer slot can end up in four different states and they are four
    different findings, so each has to be reachable only one way:

      1. `answers.rN` absent                — the state was never entered.
      2. `answers.dN` holds the sentinel    — the setup or the expression was
                                              refused; §scxml-4.9 stopped the
                                              block, or §scxml-5.4 left the
                                              location alone.
      3. `answers.dN` absent, `rN` present  — the expression evaluated, to
                                              null/undefined. This is the
                                              `empty` answer, and it is the
                                              reading that used to be
                                              impossible: an engine's JSON
                                              encoding omits such a key, so
                                              without the sentinel written
                                              first it was the same absence as
                                              case 1.
      4. `answers.dN` holds a value         — that value.

    The sentinel therefore goes down BEFORE the setup, not between the setup
    and the expression: a setup that raises must land on reading 2 rather than
    on reading 3, where it would be reported as the answer `undefined`.
    """
    source = case["source"]
    var = answer_var(n)
    out.append(f'  <state id="d{n}">')
    emit_onentry(
        [
            f'      <assign location="{reached_var(n)}" expr="1"/>',
            f'      <assign location="{var}" expr={quoteattr(repr_js(ANSWER_UNEVALUATED))}/>',
        ]
        + setup_script(case)
        + [f'      <assign location="{var}" expr={quoteattr(source)}/>'],
        out,
    )
    out.append(f"    <transition target={quoteattr(target)}/>")
    out.append("  </state>")


def emit_probe_controls(target: str, out: list[str]) -> None:
    """The two states that make the probe prove it distinguishes.

    Each control is the same two-state shape a condition case uses, and for the
    same reason: §scxml-4.9 stops a block at the element that raised, so a
    probe read in the block that wrote it would go down with the refusal it
    exists to report. Reading it in the NEXT state is what lets the sentinel
    get out.

    They are emitted BEFORE the cases, and the run STARTS in the first of them,
    so what the probe is shown to do is decided ahead of the population rather
    than by it. That ordering is the whole repair: the control this replaced
    swept the case list, so the day the engine stopped refusing anything in the
    table the control had no refusal left to find.
    """
    for name, expr, slot in (
        ("ctlRefused", CONTROL_REFUSED_EXPR, CONTROL_REFUSED_VAR),
        ("ctlEvaluable", CONTROL_EVALUABLE_EXPR, CONTROL_EVALUABLE_VAR),
    ):
        out.append(f'  <state id="{name}Probe">')
        emit_onentry(
            [
                f'      <assign location="{PROBE_VAR}" expr={quoteattr(repr_js(PROBE_UNEVALUATED))}/>',
                f'      <assign location="{PROBE_VAR}" expr={quoteattr(expr)}/>',
            ],
            out,
        )
        out.append(f'    <transition target="{name}Read"/>')
        out.append("  </state>")

        out.append(f'  <state id="{name}Read">')
        emit_onentry([f'      <assign location="{slot}" expr="{PROBE_VAR}"/>'], out)
        nxt = "ctlEvaluableProbe" if name == "ctlRefused" else target
        out.append(f"    <transition target={quoteattr(nxt)}/>")
        out.append("  </state>")


def first_state(cases: list[dict], n: int) -> str:
    """The state a case is entered at — its probe state when it has one."""
    return f"p{n}" if cases[n].get("form") == "condition" else f"d{n}"


def build(cases: list[dict], cases_rel: str) -> str:
    out: list[str] = []
    out.append('<?xml version="1.0" encoding="UTF-8"?>')
    out.append("<!--")
    out.append("  GENERATED by tools/generate_lowered_ecma262_fixture.py — do not edit.")
    out.append("")
    out.append(f"  One case per entry of {cases_rel}, asked in the form that")
    out.append("  file says it takes. State `dN` asks case N and records into")
    out.append("  `answers.dN`; a condition case is preceded by `pN`, which asks the")
    out.append("  same expression once to find out whether the engine can evaluate it")
    out.append("  at all. `answers.rN` marks that the case was reached.")
    out.append("")
    out.append("  The run OPENS with two controls for that probe, `answers.ctlRefused`")
    out.append("  and `answers.ctlEvaluable`, which make it show both outcomes on")
    out.append("  purpose. Counting the table's own refusals used to do that job, and")
    out.append("  the count reached zero the day the engine stopped refusing anything")
    out.append("  in the table — a control whose zero is forbidden forbids the finish")
    out.append("  line, so the outcomes are declared here instead.")
    out.append("")
    out.append("  The expected answers are NOT here: the harness reads them from the")
    out.append("  same table, so the lowering under test cannot mark its own work.")
    out.append("  Nor is the divergence list here — the harness reads that too, as the")
    out.append("  expectation about WHICH cases lowering gets wrong. A fixture shaped")
    out.append("  by that list could only ever ask questions the list had chosen.")
    out.append("-->")
    first = first_state(cases, 0) if cases else "done"
    out.append(
        '<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" '
        'datamodel="ecmascript" initial="ctlRefusedProbe">'
    )
    out.append("  <datamodel>")
    out.append('    <data id="answers" expr="{}"/>')
    out.append(f'    <data id="{PROBE_VAR}" expr="\'\'"/>')
    out.append("  </datamodel>")

    emit_probe_controls(first, out)

    for n, case in enumerate(cases):
        target = first_state(cases, n + 1) if n + 1 < len(cases) else "done"
        form = case.get("form")
        expect = case.get("expect", {})
        shape = next(iter(expect), None)
        if shape not in RECORDABLE:
            die(
                f"case {n} ({case['source']!r}) expects a {shape!r} answer, which "
                f"this fixture has no reading for. Recordable shapes: "
                f"{', '.join(RECORDABLE)}."
            )
        if form == "condition":
            emit_condition_case(n, case, target, out)
        elif form == "value":
            emit_value_case(n, case, target, out)
        else:
            die(
                f"case {n} ({case['source']!r}) has form {form!r}; "
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
    ap.add_argument("-o", "--output", required=True, type=pathlib.Path,
                    help="SCXML document to write")
    args = ap.parse_args()

    cases = load(args.cases).get("cases")
    if not isinstance(cases, list) or not cases:
        die(f"{args.cases} carries no `cases` array")

    # The same floor the suites that read this table carry, for the same
    # reason: a table that shrank to nothing would score every engine
    # perfectly, and a fixture expanded from it would pass by asking nothing.
    if len(cases) < MIN_CASES:
        die(
            f"{args.cases} holds {len(cases)} case(s); the floor is {MIN_CASES}. "
            f"A fixture this short is a smaller suite reported under the same name."
        )

    doc = build(cases, cases_rel=args.cases.name)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(doc, encoding="utf-8")
    print(f"wrote {args.output} — {len(cases)} case(s)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
