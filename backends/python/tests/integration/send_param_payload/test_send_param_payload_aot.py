# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.2 ``<send>`` ``<param>`` payload delivery on the Python AOT path.

Two paths were fixed at the template layer with no runtime witness, because
no committed fixture had a machine of the required shape. The suites could
only show that nothing regressed; that same absence is why the defects
survived as long as they did.

engine-less child -> parent
    Param emission used to be gated on the *machine* needing a script
    engine rather than on the send needing one, so a ``datamodel="null"``
    child shipped its ``<send>`` with the params dropped.

``#_internal``
    The internal raise took no event data, so params were built and then
    discarded.

The two reach distinct final states, so a failure names the path.

Fixture: ``integration_resources/send_param_payload/send_param_payload.scxml``.

Regeneration (after fixture or template edit):
  ``scripts/regen_send_param_payload_python.sh`` (local)
  ``sce-codegen generate-integration -l python --stem send_param_payload`` (CI)
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import send_param_payload_sm as _sm  # noqa: E402 — path inserted above

_WHY = {
    "failChildPayload": (
        "`fromChild` arrived without `_event.data.value`: a datamodel=\"null\" child "
        "needs no script engine, but its <send> still has to carry the params it "
        "declares. The gate is whether this send folds to literals, not whether the "
        "machine needs an engine."
    ),
    "failInternalPayload": (
        "`loopback` arrived without `_event.data.carried`: a <send target=\"#_internal\"> "
        "must raise its params as event data, not build them and drop them at the "
        "internal-raise boundary."
    ),
    "failNumberType": (
        "`typed` arrived with `_event.data.n` unequal to 7: expr=\"7\" is the Number 7, "
        "and a backend that stringifies on the way through delivers \"7\", which `===` "
        "finds unequal."
    ),
    "failStringType": (
        "`typed` arrived with `_event.data.s` unequal to 'kept': a param that has to be "
        "EVALUATED reaches the runtime serialiser, whose string arm must emit the value "
        "rather than an engine spelling of it."
    ),
    "failDuplicateParams": (
        "`typed` did not carry both values of the repeated name `d` with their types: "
        "W3C SCXML 6.2 lets a name repeat and every value must be delivered."
    ),
    "failNoParamError": (
        "`withBadParam` arrived with no `error.execution` before it: W3C SCXML 5.7.1 "
        "puts that error on the internal queue while the <send> is being evaluated, so "
        "it is dequeued first."
    ),
    "failBrokenParamDelivered": (
        "`_event.data.broken` arrived as the empty string: 5.7.1 says ignore the name "
        "AND the value, so a receiver must find no field at all rather than a "
        "placeholder under the name."
    ),
    "failSiblingParamLost": (
        "`_event.data.kept` did not survive alongside the failed param: one <param> "
        "that will not evaluate costs its own pair and nothing else."
    ),
}


def test_send_param_payload_aot() -> None:
    engine = _sm.create_engine()
    engine.initialize()

    elapsed = 0
    while not engine.reached_final and elapsed < 100:
        engine.advance_time(10)
        elapsed += 10

    assert engine.reached_final, (
        "send_param_payload did not reach a top-level <final> within 100 ms; "
        f"last leaf={engine.current_state} — the parent never saw `fromChild`, never "
        "saw its own `loopback`, or discarded a whole <send> because one <param> "
        "would not evaluate (W3C SCXML 5.7.1 drops the pair, not the message)"
    )
    actual = str(engine.current_state)
    assert actual == "pass", (
        f"send_param_payload reached <final id={actual!r}>; expected 'pass'. "
        + _WHY.get(actual, "That is not a verdict state — neither send was judged.")
    )
