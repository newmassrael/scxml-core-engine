# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""NL→IR Item C1 Path A (EventSchema native lowering) — Python AOT
compile+run gate, the twin of the Rust ``tests/event_schema_native.rs``,
the Go ``event_schema_native`` package, the Kotlin ``EventSchemaNativeTest``,
and the C11 ``c11_integration_event_schema_native`` tests.

The committed SM (``statechart_minimal_sm.py``) is generated from
``sce-build/tests/fixtures/event_schema/statechart_minimal.scxml``
(regen: ``scripts/regen_event_schema_native_python.sh``). Because it is
imported and run here, the generated payload dataclass, the type-erased
``EventMetadata.typed_payload`` carrier round-trip, and the per-event
``raise_job_completed`` inject seam are really exercised.

The transition guard ``cond="_event.data.elapsed_ms === 0"`` lowers to a
``self._pending_job_completed_payload is not None and (…)`` native comparison
that never calls ``self._guard(...)``. Python's engine always owns a script
engine (unlike the MCU-targeting Rust / C11 / Kotlin backends), so the
no-script-engine property is pinned differently: the engine is built with a
``LuaScriptEngine`` whose ``evaluate_expression`` raises — if the machine still
reaches ``done``, the typed guard provably never routed through the script
engine.
"""
from __future__ import annotations

import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))

import statechart_bytes_sm as _bytes_sm  # noqa: E402 — path inserted above
import statechart_minimal_sm as _sm  # noqa: E402 — path inserted above
from sce_runtime.scripting import LuaScriptEngine  # noqa: E402


class _NoEvalLua(LuaScriptEngine):
    """A script engine that fails if any transition guard is evaluated through
    it. The native-lowered typed ``_event.data`` guard must be a plain Python
    comparison, so ``evaluate_expression`` must never be reached for this
    machine — a call here means the guard regressed back to ``self._guard``."""

    def evaluate_expression(self, session_id, expression):  # type: ignore[override]
        raise AssertionError(
            "native-lowered typed `_event.data` guard must not reach the "
            f"script engine; evaluate_expression called with {expression!r}"
        )


def _make_engine():
    script_engine = _NoEvalLua()
    script_engine.initialize()
    engine = _sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine


def test_typed_payload_guard_fires_natively() -> None:
    engine = _make_engine()
    assert str(engine.current_state) == "waiting", (
        f"initial state = {engine.current_state!s}, want waiting"
    )

    # Per-event typed inject — elapsed_ms == 0 satisfies the native guard.
    _sm.raise_job_completed(engine, 0)

    assert str(engine.current_state) == "done", (
        f"after raise_job_completed(0): state = {engine.current_state!s}, want "
        "done — elapsed_ms == 0 must fire the native typed `_event.data` guard"
    )


def test_typed_payload_guard_misses_on_nonzero() -> None:
    engine = _make_engine()

    # Same event, a payload the guard rejects — the machine stays put.
    _sm.raise_job_completed(engine, 5)

    assert str(engine.current_state) == "waiting", (
        f"after raise_job_completed(5): state = {engine.current_state!s}, want "
        "waiting — elapsed_ms == 5 must leave the native typed guard unfired"
    )


# RFC rfc-eventschema-bytes-guard.md §6 — the bytes-field guard
# ``cond="_event.data.raw === 'ack'"`` lowers to ``self._pending_signal_-
# received_payload.raw == b"ack"`` (a ``bytes == bytes`` comparison, NOT
# ``bytes == str`` which Python evaluates ``False`` always). Only a runtime
# transition check distinguishes the two — a match must reach ``done`` and a
# non-match must not. The ``_NoEvalLua`` engine additionally proves the guard
# never routed through the script engine.
def _make_bytes_engine():
    script_engine = _NoEvalLua()
    script_engine.initialize()
    engine = _bytes_sm.create_engine(script_engine=script_engine)
    engine.initialize()
    return engine


def test_bytes_payload_guard_fires_on_match() -> None:
    engine = _make_bytes_engine()
    assert str(engine.current_state) == "waiting"

    _bytes_sm.raise_signal_received(engine, b"ack")

    assert str(engine.current_state) == "done", (
        f"after raise_signal_received(b'ack'): state = {engine.current_state!s}, "
        "want done — raw == b'ack' must fire the native bytes guard"
    )


def test_bytes_payload_guard_misses_on_nonmatch() -> None:
    engine = _make_bytes_engine()

    _bytes_sm.raise_signal_received(engine, b"no")

    assert str(engine.current_state) == "waiting", (
        f"after raise_signal_received(b'no'): state = {engine.current_state!s}, "
        "want waiting — raw == b'no' must leave the native bytes guard unfired"
    )
