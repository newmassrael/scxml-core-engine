# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""SCE Accepted Subset §2.15, "Saving and restoring" (docs/adr/0045, section 8):
the saved states the shared fixtures hold are the text every backend writes and
reads, so a state saved by one backend is one another restores. Each format-2 text
is restored by the generated Python machine of its document and saved again, which
is the same text byte for byte; each format-1 text, which every backend still reads,
is restored.

The Rust, Kotlin and Go halves are ``static_datamodel.rs``,
``StaticDatamodelTest.kt`` and ``a_state_another_backend_saved_is_restored_test.go``;
the shared texts are ``sce-build/tests/fixtures/static_datamodel/saved/``.
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

# The wall-clock moment the shared fixtures were saved at, as the Rust, Kotlin and
# Go suites have it: 2023-11-14T22:13:20Z.
_SAVED_AT_MS = 1_700_000_000_000

_SAVED = _HERE.parents[4] / "sce-build" / "tests" / "fixtures" / "static_datamodel" / "saved"

# The documents a shared saved state is of, by the stem the file's name begins with.
_MACHINES = (
    "static_enum",
    "static_history",
    "static_list",
    "static_optional",
    "static_optional_record",
    "static_record",
    "static_schema_pending",
    "static_schema_revision",
    "static_send_idlocation",
    "static_timers",
)


def _machine_of(name: str) -> str:
    """The stem of the document a shared text is of: the file's own name up to the
    part that says which moment of the run it is (``static_schema_pending_first`` and
    ``static_schema_revision_second`` are moments of one document each). The longest
    stem a name begins with is the document: ``static_optional_record`` begins with
    ``static_optional`` too, and is not a moment of that one."""
    matching = [stem for stem in _MACHINES if name == stem or name.startswith(stem + "_")]
    return max(matching, key=len, default="")


def _restored_and_saved_again(stem: str, text: str) -> str:
    """The text a machine of `stem` saves after it restored `text`, at the moment it
    was restored at."""
    module = importlib.import_module(f"integration.static_datamodel.{stem}_sm")
    engine = module.restore(saved_state_from_json(text), wall_now_ms=_SAVED_AT_MS)
    return module.save_at(engine, _SAVED_AT_MS).to_json()


def _texts(directory: Path) -> dict:
    return {
        path.stem: path.read_text(encoding="utf-8").strip()
        for path in sorted(directory.glob("*.json"))
        if _machine_of(path.stem)
    }


@pytest.mark.parametrize("name", sorted(_texts(_SAVED)))
def test_a_format_two_state_another_backend_saved_is_restored_and_saved_as_it_was(name: str) -> None:
    text = _texts(_SAVED)[name]
    assert _restored_and_saved_again(_machine_of(name), text) == text


def test_every_format_two_text_of_a_document_is_held_to_its_machine() -> None:
    assert len(_texts(_SAVED)) >= 8, "the shared format-2 texts lost instances"


@pytest.mark.parametrize("name", sorted(_texts(_SAVED / "format1")))
def test_a_format_one_state_is_restored_and_takes_the_machines_own_session(name: str) -> None:
    text = _texts(_SAVED / "format1")[name]
    # What a format-1 text holds is what a restore reads; saving it again writes
    # format 2, with the machine's own session id.
    again = _restored_and_saved_again(_machine_of(name), text)
    assert again.startswith('{"format":2,'), again


def test_every_format_one_text_of_a_document_is_held_to_its_machine() -> None:
    assert len(_texts(_SAVED / "format1")) >= 10, "the shared format-1 texts lost instances"
