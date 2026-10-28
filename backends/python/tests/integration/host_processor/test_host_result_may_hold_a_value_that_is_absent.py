# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.12 and docs/adr/0055, step 10 — a host-run
``<invoke>`` whose ``sce:result`` record holds values that may be absent.
Python compile+run gate.

The host answers through the generated typed interface, so the record it hands
in holds each field that may be absent as an ``Optional``; the engine delivers
the record as JSON, an absent value as ``null`` with its key written all the
same; and the machine reads each field where a test proves it present. What
these hold apart is the three states a field has — absent, present and zero or
empty, and present — which a sentinel could not: ``""`` is a label, ``False`` is
a flag that was answered, and ``0`` is a day.

Fixture: ``sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml``,
a ``datamodel="sce-static"`` machine generated WITH ``--host-invoker x-sce-host``
by ``scripts/regen_host_processor_python.sh``. The declaration is load-bearing:
without it codegen emits the refusal and every case would measure it.
"""
from __future__ import annotations

import sys
from dataclasses import dataclass
from pathlib import Path
from typing import List, Tuple

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

import statechart_host_invoker_optional_sm as _sm  # noqa: E402 — path inserted above

# The type the fixture was compiled for.
DECLARED_TYPE = "x-sce-host"


class _LookupHost:
    """A host implementing the generated interface; its work outlives the
    call, so each test completes the start it was handed."""

    def __init__(self) -> None:
        self.tokens: List[int] = []

    def start_lookup(self, token: int):
        self.tokens.append(token)
        return None

    def cancel_lookup(self, token: int) -> None:
        return None


@dataclass(frozen=True)
class _Answer:
    """What the machine published after a completion: ``answered`` counts the
    completions it read, and the rest is what each field of the record held."""

    answered: int = 0
    rows_seen: int = 0
    at_present: bool = False
    at_value: int = 0
    label_present: bool = False
    label_value: str = ""
    hit_known: bool = False
    hit_true: bool = False


def _read(engine) -> _Answer:
    policy = engine.policy
    return _Answer(
        answered=policy.v_answered,
        rows_seen=policy.v_rows_seen,
        at_present=policy.v_at_present,
        at_value=policy.v_at_value,
        label_present=policy.v_label_present,
        label_value=policy.v_label_value,
        hit_known=policy.v_hit_known,
        hit_true=policy.v_hit_true,
    )


def _asking() -> Tuple[object, int]:
    """A machine standing at ``asking``, with the token the host was started
    with."""
    engine = _sm.create_engine()
    host = _LookupHost()
    _sm.register_x_sce_host_invoker(engine, host)
    engine.initialize()
    engine.send_external_by_name("ask")
    # The macrostep the event starts, run to its end.
    engine.advance_time(0)
    assert len(host.tokens) == 1, f"the lookup was started {len(host.tokens)} times"
    return engine, host.tokens[0]


def _complete(result) -> _Answer:
    """Ask, complete the lookup with ``result`` through the typed interface, and
    settle the machine."""
    engine, token = _asking()
    assert _sm.complete_lookup(engine, token, result), (
        "a running invocation's typed completion was refused"
    )
    engine.advance_time(0)
    return _read(engine)


def test_an_absent_field_is_read_as_absent() -> None:
    """Every field that may be absent is: the machine reads three absent values
    and the count every answer carries."""
    got = _complete(_sm.LookupResult(rows=3, at=None, label=None, hit=None))
    assert got == _Answer(answered=1, rows_seen=3), "an all-absent answer"


def test_a_present_field_is_read_as_its_value() -> None:
    """Every field is present: the machine reads each value."""
    got = _complete(_sm.LookupResult(rows=4, at=41, label="ab", hit=True))
    assert got == _Answer(
        answered=1,
        rows_seen=4,
        at_present=True,
        at_value=41,
        label_present=True,
        label_value="ab",
        hit_known=True,
        hit_true=True,
    ), "an all-present answer"


def test_a_zero_an_empty_text_and_a_false_flag_are_values_and_not_absences() -> None:
    """A zero, an empty text and a false flag are values, not absences: each is
    present, and is read as itself."""
    got = _complete(_sm.LookupResult(rows=1, at=0, label="", hit=False))
    assert got == _Answer(
        answered=1, rows_seen=1, at_present=True, label_present=True, hit_known=True
    ), "zero, empty and false are present"


def test_each_field_is_absent_or_present_on_its_own() -> None:
    """Each field is independent: a day with no label and a flag."""
    got = _complete(_sm.LookupResult(rows=2, at=7, label=None, hit=True))
    assert got == _Answer(
        answered=1, rows_seen=2, at_present=True, at_value=7, hit_known=True, hit_true=True
    ), "a mixed answer"


def test_the_record_is_written_with_null_for_an_absent_value() -> None:
    """The record's JSON writes the key of every field, ``null`` where the value
    is absent, in the order the schema declares them. The day is absent in both,
    so that what is held is the key, the ``null`` and the other values' own
    spelling, not how a 64-bit integer is written."""
    absent = _sm.LookupResult(rows=3, at=None, label=None, hit=None)
    assert absent.wire() == '{"rows":3,"at":null,"label":null,"hit":null}'
    mixed = _sm.LookupResult(rows=3, at=None, label="ab", hit=False)
    assert mixed.wire() == '{"rows":3,"at":null,"label":"ab","hit":false}'


def test_a_completion_that_leaves_a_key_out_is_refused() -> None:
    """A key left out is not an absent value: every field of the first revision
    of a schema is carried by every delivery, and ``null`` is the one spelling
    of absence, so a completion that lacks ``at`` is refused as any typed
    payload the data does not fit is, and the machine reads no answer."""
    engine, token = _asking()
    assert engine.complete_host_invoke(
        DECLARED_TYPE, "lookup", token, '{"rows":1,"label":null,"hit":null}'
    ), "a running invocation's completion was refused"
    engine.advance_time(0)
    assert _read(engine).answered == 0, "a record without `at` was read as an answer"
