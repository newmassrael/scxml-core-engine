# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML C.2 + 6.2.3 ``<send namelist>`` over BasicHTTP, Python AOT path.

Two claims the IRP corpus states and cannot measure:

the namelist reaches the form
    test518 is titled "namelist values get encoded as POST parameters" and
    its whole verdict is ``<transition event="test" target="pass"/>`` — it
    passes as soon as the event comes back, whatever the message carried.

an unreadable item reports AND discards
    W3C SCXML 5.9.2 makes a namelist item a location expression and requires
    ``error.execution`` when one yields no valid location; 6.2.3 requires the
    message itself to be discarded. ``<param>``'s rule is an explicit
    per-item exception (5.7.1, "ignore the name and value") and has no
    counterpart for namelist anywhere in the specification.

This channel built the HTTP form in ``_build_http_params``, which read the
data model a second time and swallowed an unreadable namelist item with a
bare ``except: continue`` — no error, no discard, and the rest posted.

Fixture: ``integration_resources/send_namelist_over_http/send_namelist_over_http.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_send_namelist_over_http_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem send_namelist_over_http`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import send_namelist_over_http_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failNamelistNeverArrived": (
        "the BasicHTTP send never came back at all: the harness server did not "
        "answer, which is a different failure from posting the wrong form."
    ),
    "failNamelistNotPosted": (
        "`mapped` arrived without `Var1` in its data: W3C SCXML C.2 requires a "
        "namelist's variable names and values to be mapped to HTTP POST parameters."
    ),
    "failMessageNotDiscarded": (
        "`shouldNotArrive` was delivered: W3C SCXML 6.2.3 discards the message when "
        "the evaluation of a <send>'s arguments produces an error. <param>'s "
        "per-item rule (5.7.1) does not reach a namelist item."
    ),
    "failNoNamelistError": (
        "no `error.execution` preceded the timeout: W3C SCXML 5.9.2 requires it when "
        "a location expression yields no valid location, and the wire the send would "
        "have crossed does not change the answer."
    ),
}


def test_send_namelist_over_http_aot(setup_http) -> None:
    engine = _sm.create_engine()
    setup_http(engine)
    engine.initialize()

    # The two phases are settled by delayed sends (3s + 2s), so the clock has
    # to advance past both before a verdict exists.
    elapsed = 0
    while not engine.reached_final and elapsed < 15000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "send_namelist_over_http did not reach a top-level <final> within 15 s; "
        f"last leaf={engine.current_state} — the delayed `timeoutMap` / "
        "`timeoutDiscard` sends that give each phase its verdict never fired"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"send_namelist_over_http reached <final id={actual!r}>; expected 'pass'. "
        + _WHY.get(actual, "That is not a verdict state — neither claim was judged.")
    )


def test_a_static_http_target_reports_and_discards_a_broken_namelist(setup_http) -> None:
    """The same clause on the arm the document above cannot reach.

    A `<send target="http://...">` with a literal URL takes a different arm
    from `targetexpr`, and that one had no payload to render: it called
    `_build_http_params`, which read the data model itself and swallowed an
    unreadable namelist item. The `targetexpr` arm evaluated the payload
    first and aborted, so the two arms of one channel answered differently.

    A document cannot ask this question here — a literal target means a
    literal port in a committed fixture, which is the coupling
    `_ioprocessors` exists to avoid. So it is asked of the helper the arm
    calls, which is where the two answers were.
    """
    engine = _sm.create_engine()
    setup_http(engine)
    engine.initialize()
    policy = engine._policy

    # The declared name renders; W3C SCXML C.2 asks for the name AND value.
    assert policy._build_http_params(engine, [], "Var1") == {"Var1": ["2"]}

    # The undeclared one abandons the message (W3C SCXML 6.2.3) after
    # reporting it (W3C SCXML 5.9.2) — never a quiet partial form.
    try:
        policy._build_http_params(engine, [], "__sce_not_declared__")
    except Exception as exc:  # noqa: BLE001 — the sentinel type is generated
        assert type(exc).__name__ == "_ActionAbort", (
            f"a broken namelist item raised {type(exc).__name__}, not the "
            "block-abort sentinel that discards the message"
        )
    else:
        raise AssertionError(
            "a namelist item naming no location was swallowed: W3C SCXML 6.2.3 "
            "discards the message when the evaluation of a <send>'s arguments "
            "produces an error, and 5.9.2 requires error.execution — this arm "
            "posted the rest of the form and said nothing"
        )
