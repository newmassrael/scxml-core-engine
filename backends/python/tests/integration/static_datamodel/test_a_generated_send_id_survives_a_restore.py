# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring": the id a machine generates for a
``<send idlocation>`` is held by the document in a variable, and by the send that waits
under it, so a saved state carries both and the count the next id comes from. A machine
restored from it cancels the send by the id the document holds, and numbers the ids it
generates next after the count: an id the saved state already holds is never generated
again, which would let one ``<cancel>`` remove two sends.

``static_send_idlocation.scxml``: ``arm`` sends two delayed events, ``first.due`` and
``second.due``, each under an id written to ``first`` and ``second``, and
``cancel.first`` and ``cancel.second`` remove the send whose id the variable holds.
Everything runs on the engine's virtual clock, so no case sleeps. The shared fixture
``saved/static_send_idlocation_armed.json`` is the format-2 text every backend writes
(docs/adr/0045), its root given the id the fixtures name it by, and ``saved/format1/``
holds the format-1 text of the same run, which still restores. The Rust, Kotlin and Go
halves are ``a_generated_send_id_survives_a_restore.rs``,
``AGeneratedSendIdSurvivesARestoreTest.kt`` and
``a_generated_send_id_survives_a_restore_test.go``.
"""
from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE.parents[1]))
sys.path.insert(0, str(_HERE.parents[2] / "runtime"))
sys.path.insert(0, str(_HERE.parents[2] / "forge-runtime"))

from sce_runtime import saved_state_from_json  # noqa: E402 -- path inserted above

_SAVED_AT_MS = 1_700_000_000_000
_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"
_SEND_ID = importlib.import_module("integration.static_datamodel.static_send_idlocation_sm")


def _shared_armed(format1: bool = False) -> str:
    directory = _SAVED / "format1" if format1 else _SAVED
    return (directory / "static_send_idlocation_armed.json").read_text(encoding="utf-8").strip()


def _send(engine, name: str) -> None:
    engine.send_event(engine.policy.resolve_event_by_name(name))
    engine.advance_time(0)


def _armed():
    """A machine that has run `arm` once, on a clock the test owns."""
    engine = _SEND_ID.create_engine(session_id="fixture-root")
    engine.initialize()
    _send(engine, "arm")
    return engine


def _restored_from(text: str):
    return _SEND_ID.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS)


def test_a_machine_that_armed_two_sends_saves_the_ids_it_generated() -> None:
    saved = _SEND_ID.save_at(_armed(), _SAVED_AT_MS)
    assert saved.auto_send_seq == 2 and len(saved.pending) == 2, "two ids were generated and two sends wait"
    text = saved.to_json()
    for send_id in ("_auto_send_1", "_auto_send_2"):
        assert f'"sendid":"{send_id}"' in text, f"a waiting send carries {send_id}: {text}"
    # The text every backend writes for the same run, byte for byte: the variables hold
    # the ids the waiting sends are known by.
    assert text == _shared_armed()


@pytest.mark.parametrize("format1", [False, True])
def test_a_restored_send_is_cancelled_by_the_id_the_document_holds(format1: bool) -> None:
    # From the format-2 text, and from the format-1 text of the same run.
    engine = _restored_from(_shared_armed(format1))
    _send(engine, "cancel.first")
    engine.advance_time(250)
    assert engine.policy.v_first_fired == 0, "its send was removed"
    assert engine.policy.v_second_fired == 1, "the other came due"


def test_a_restored_machine_numbers_its_next_ids_after_the_saved_count() -> None:
    engine = _restored_from(_shared_armed())
    # The ids the saved state holds are 1 and 2: `arm` again writes 3 and 4 to the
    # variables, and a `<cancel>` by `first` removes the send of id 3 and not the one of
    # id 1, which is still waiting.
    _send(engine, "arm")
    saved = _SEND_ID.save_at(engine, _SAVED_AT_MS)
    assert saved.auto_send_seq == 4, "four ids were generated"
    assert '"first":"_auto_send_3","second":"_auto_send_4"' in saved.to_json(), saved.to_json()

    _send(engine, "cancel.first")
    engine.advance_time(250)
    assert engine.policy.v_first_fired == 1, "the send of id 1 came due, the one of id 3 was removed"
    assert engine.policy.v_second_fired == 2, "ids 2 and 4 came due"
