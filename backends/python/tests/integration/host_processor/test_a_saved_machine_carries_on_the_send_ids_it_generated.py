# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring": a machine hands the document an
id for a ``<send idlocation>``, the document keeps it, and may name it in a ``<cancel>``
long after. Two ids a machine generates must never be the same, and a restore must not
start counting again: the document that was saved may still hold ``_auto_send_1``, and a
send waiting in the saved state carries it. A saved state says how many ids the machine
has generated (``sendseq``), and a restore carries on from it.

Driven on the engine's virtual clock, no case sleeps. The shared instance
``saved/statechart_static_host_invoke_running.json`` has generated no id, so each case
that needs a count writes one into it. The Rust, Kotlin and Go halves are
``a_saved_machine_carries_on_the_send_ids_it_generated.rs``,
``ASavedMachineCarriesOnTheSendIdsItGeneratedTest.kt`` and
``a_saved_machine_carries_on_the_send_ids_it_generated_test.go``.
"""
from __future__ import annotations

import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
# The checked operations a lowered expression calls are the forge runtime's.
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import StateRefusal, saved_state_from_json  # noqa: E402 -- path inserted above

import statechart_static_host_invoke_sm as _sm  # noqa: E402 -- path inserted above

_SAVED_AT_MS = 1_700_000_000_000
_SHARED = (
    _HERE.parents[4]
    / "sce-build"
    / "tests"
    / "fixtures"
    / "host_processor"
    / "saved"
    / "statechart_static_host_invoke_running.json"
)


def _shared() -> str:
    return _SHARED.read_text(encoding="utf-8").strip()


def _shared_with_count(count: str) -> str:
    """The shared instance with `sendseq` written as `count`."""
    return _shared().replace('"sendseq":"0"', f'"sendseq":"{count}"', 1)


def _started():
    engine = _sm.create_engine()
    engine.initialize()
    return engine


def _restored(text: str):
    return _sm.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS)


def test_a_machine_counts_the_ids_it_generates_from_one() -> None:
    engine = _started()
    assert [engine._next_auto_sendid() for _ in range(3)] == ["_auto_send_1", "_auto_send_2", "_auto_send_3"]


def test_two_machines_do_not_share_a_count() -> None:
    first, second = _started(), _started()
    assert (first._next_auto_sendid(), first._next_auto_sendid()) == ("_auto_send_1", "_auto_send_2")
    assert second._next_auto_sendid() == "_auto_send_1", "an id does not depend on what else ran beside the machine"


def test_the_count_is_saved_with_the_machine() -> None:
    engine = _started()
    assert _sm.save_at(engine, _SAVED_AT_MS).auto_send_seq == 0, "no id generated yet"
    engine._next_auto_sendid()
    engine._next_auto_sendid()
    saved = _sm.save_at(engine, _SAVED_AT_MS)
    assert saved.auto_send_seq == 2 and '"sendseq":"2"' in saved.to_json()


def test_a_restore_carries_on_from_the_count_it_was_saved_with() -> None:
    engine = _restored(_shared_with_count("5"))
    assert engine._next_auto_sendid() == "_auto_send_6", "the id a saved machine already handed out is not handed out again"
    assert engine._next_auto_sendid() == "_auto_send_7"


def test_a_machine_restored_and_saved_again_before_it_is_driven_keeps_the_count() -> None:
    engine = _restored(_shared_with_count("5"))
    saved = _sm.save_at(engine, _SAVED_AT_MS)
    assert saved.to_json() == _shared_with_count("5"), "repeating the round trip never loses the count"


@pytest.mark.parametrize(
    "what, old, new, expected",
    [
        ("no sendseq", '"sendseq":"0",', "", "has no 'sendseq'"),
        ("a count that is a number", '"sendseq":"0"', '"sendseq":0', "'sendseq' is not a text"),
        ("a count that is not digits", '"sendseq":"0"', '"sendseq":"-1"', "'sendseq' (-1) is not a whole number"),
        (
            "a count past a signed 64-bit count",
            '"sendseq":"0"',
            '"sendseq":"9223372036854775808"',
            "'sendseq' (9223372036854775808) is not a whole number",
        ),
    ],
)
def test_the_send_count_is_required_and_must_be_a_count(what: str, old: str, new: str, expected: str) -> None:
    with pytest.raises(StateRefusal) as refused:
        saved_state_from_json(_shared().replace(old, new, 1))
    assert expected in str(refused.value), what
