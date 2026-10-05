# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""A payload field a schema declares ``float32`` under generated Python.

Python's ``float`` is a binary64, so a lift that read the field as ``float``
would hold the double the payload carried -- ``0.1`` -- where every engine with
a single of its own holds the binary32 nearest it, ``0.10000000149011612``. The
generated lift names the width (``Single``), and ``static_record_real32`` holds
the machine to the same numbers the other engines hold; the cases here pin the
lift itself, including what it refuses.
"""
from __future__ import annotations

import importlib

import pytest

from sce_runtime.event_payload import FLOAT32_MAX, Single, TypedPayloadError, as_single, lift


def _machine():
    """The generated ``static_record_real32`` machine, started."""
    module = importlib.import_module("integration.static_datamodel.static_record_real32_sm")
    engine = module.create_engine()
    engine.initialize()
    return module, engine


def test_a_number_lands_as_the_binary32_nearest_it() -> None:
    (value,) = lift({"value": 0.1}, (("value", Single),))
    assert value == 0.10000000149011612
    assert value != 0.1


def test_a_whole_number_is_read_as_the_real_it_is() -> None:
    (value,) = lift('{"value": 3}', (("value", Single),))
    assert value == 3.0
    assert isinstance(value, float)


def test_the_largest_single_fits_and_a_number_past_it_does_not() -> None:
    (largest,) = lift({"value": FLOAT32_MAX}, (("value", Single),))
    assert largest == FLOAT32_MAX
    with pytest.raises(TypedPayloadError, match="does not fit the width"):
        lift({"value": FLOAT32_MAX * 2}, (("value", Single),))


def test_a_truth_value_and_a_text_are_not_a_number() -> None:
    for refused in (True, "0.1"):
        with pytest.raises(TypedPayloadError, match="is not a number"):
            lift({"value": refused}, (("value", Single),))


def test_a_64_bit_field_keeps_every_bit_of_the_number() -> None:
    (value,) = lift({"value": 0.1}, (("value", float),))
    assert value == 0.1


def test_the_typed_inject_seam_hands_the_machine_the_single_nearest_what_it_was_given() -> None:
    """A host that calls the typed seam with a Python ``float`` is handed the
    same field every other producer lifts: the binary32 nearest it."""
    module, engine = _machine()
    module.raise_reading32_taken(engine, 7, 0.1)
    assert engine.policy.v_last.value == 0.10000000149011612
    assert engine.policy.v_last.sensor == 7


def test_the_typed_inject_seam_writes_the_same_single_into_the_data_beside_the_carrier() -> None:
    """The script engine binds ``_event.data`` from ``data``, so the seam writes
    the single there too: the two carriers hold one number."""

    class Probe:
        def send_event(self, event, metadata) -> None:
            self.metadata = metadata

    module, _ = _machine()
    probe = Probe()
    module.raise_reading32_taken(probe, 7, 0.1)
    assert probe.metadata.data == {"sensor": 7, "value": 0.10000000149011612}
    assert probe.metadata.typed_payload.value == 0.10000000149011612


def test_the_machine_lifts_the_single_nearest_the_number_its_data_carries() -> None:
    module, engine = _machine()
    engine.send_event(
        module.StaticRecordReal32Event.READING32_TAKEN,
        module.EventMetadata(data='{"sensor": 7, "value": 0.1}'),
    )
    assert engine.policy.v_last.value == 0.10000000149011612


def test_the_inject_seams_rounding_is_the_lifts() -> None:
    assert as_single(0.1) == 0.10000000149011612
    assert as_single(FLOAT32_MAX * 2) == float("inf")
    assert as_single(-FLOAT32_MAX * 2) == float("-inf")
