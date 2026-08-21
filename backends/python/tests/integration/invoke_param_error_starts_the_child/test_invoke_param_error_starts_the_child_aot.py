# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 5.7.1 under 6.4, on the Python AOT path.

A ``<param>`` of an ``<invoke>`` whose expression will not evaluate is the one
place two clauses meet. §scxml-6.4.2 terminates the element when "the
evaluation of its arguments produces an error"; §scxml-5.7.1 says a failing
``<param>`` costs ``error.execution`` and "MUST ignore the name and value",
then delegates only the SUCCESSFUL name and value to the context — naming
``<donedata>``, ``<send>`` and ``<invoke>`` in that sentence.

5.7.1 governs: it has already said what a failed ``<param>`` costs, in this
context by name, and reading 6.4.2 over it would leave "ignore the name and
value" with no session for the name to be absent from. W3C test343 settles the
same clause from the ``<donedata>`` side; no IRP document asks it of
``<invoke>``.

Fixture: ``integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_invoke_param_error_starts_the_child_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem invoke_param_error_starts_the_child`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import invoke_param_error_starts_the_child_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failNoParamError": (
        "`childUp` arrived with no `error.execution` before it: W3C SCXML 5.7.1 puts "
        "that error on the internal queue while the <invoke> is being evaluated, so "
        "it is dequeued before the child's first word."
    ),
    "failInvokeNotStarted": (
        "the child never started: this channel read W3C SCXML 6.4.2's \"terminate the "
        "processing of the element\" over 5.7.1's per-item rule. One <param> that will "
        "not evaluate costs its own pair, not the session."
    ),
    "failGoodParamLost": (
        "the child's `kept` did not arrive as 'here': W3C SCXML 6.4.3 seeds the child's "
        "matching <data> from the param's value, and one sibling that failed does not "
        "cost the others."
    ),
    "failBrokenParamSeeded": (
        "the child found the empty string under `broken`: 5.7.1 says ignore the name AND "
        "the value, so the child must find its own declaration untouched rather than a "
        "placeholder the author never wrote."
    ),
}


def test_invoke_param_error_starts_the_child_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    # The `timeout` that judges a never-started child is a delayed <send>, so
    # the clock has to advance past it before a verdict exists.
    elapsed = 0
    while not engine.reached_final and elapsed < 10000:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "invoke_param_error_starts_the_child did not reach a top-level <final> "
        f"within 10 s; last leaf={engine.current_state}"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"invoke_param_error_starts_the_child reached <final id={actual!r}>; "
        "expected 'pass'. " + _WHY.get(actual, "That is not a verdict state.")
    )
