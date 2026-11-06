# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The SCXML Event I/O Processor's address codec, on the Python runtime
(docs/adr/0090, test 6).

``scxml_location`` writes the address a session publishes and the SCXML
processor stamps as the origin of what the session sends; a reply sent to it is
routed by ``session_id_from_scxml_location``. The two are one mapping only if a
session id survives the round trip, reserved characters included. The same
answers the C++ ``IOProcessorHelper`` is held to in
``tests/engine/IOProcessorsTest.cpp``, and the stored-origin reading of section 7.
"""

from __future__ import annotations

import pytest

from sce_runtime import (
    EventOrigin,
    origin_address_of,
    origin_processor_of,
    read_stored_origin,
    read_stored_sender,
    scxml_location,
    scxml_origin,
    session_id_from_scxml_location,
    stored_origin_text,
)
from sce_runtime.io_processors import (
    SCXML_EVENT_PROCESSOR_URI,
    build,
    scxml_origin_of,
)


@pytest.mark.parametrize(
    "session_id", ["session-1", "a b/c#d", "parent:invoke.1", "x%2Fy", "100%", "été"]
)
def test_a_session_id_with_reserved_characters_survives_the_round_trip(session_id: str) -> None:
    location = scxml_location(session_id)
    assert location.startswith("sce://scxml/"), location
    assert session_id_from_scxml_location(location) == session_id, location


def test_a_reserved_character_is_percent_encoded_once() -> None:
    # `%` is itself reserved: an id that already looks encoded is encoded again,
    # so decoding gives back the id rather than what it looked like.
    assert scxml_location("a b/c#d") == "sce://scxml/a%20b%2Fc%23d"
    assert scxml_location("x%2Fy") == "sce://scxml/x%252Fy"
    assert session_id_from_scxml_location("sce://scxml/x%252Fy") == "x%2Fy"


@pytest.mark.parametrize("text", ["mesh://motor", "motor", "sce://scxml/", ""])
def test_an_address_of_another_processor_names_no_session(text: str) -> None:
    assert session_id_from_scxml_location(text) == ""


def test_the_published_location_is_the_one_the_codec_writes() -> None:
    entries = {entry.name: entry.location for entry in build("a b")}
    assert entries["scxml"] == scxml_location("a b")
    assert entries[SCXML_EVENT_PROCESSOR_URI] == scxml_location("a b")


def test_the_senders_pair_is_the_processors_type_and_its_location() -> None:
    origin = scxml_origin("a b")
    assert origin == EventOrigin(SCXML_EVENT_PROCESSOR_URI, scxml_location("a b"))
    assert scxml_origin_of("a b") == origin
    assert scxml_origin_of("") is None, "no sender has no address to publish"
    assert scxml_origin_of(None) is None


def test_an_origin_reads_as_its_two_fields_and_none_as_two_blanks() -> None:
    origin = EventOrigin("x-sce-host", "motor")
    assert (origin_address_of(origin), origin_processor_of(origin)) == ("motor", "x-sce-host")
    assert (origin_address_of(None), origin_processor_of(None)) == ("", "")


def test_an_origin_is_constructed_whole() -> None:
    # Both fields are required: a pair with one of them cannot be spelled.
    with pytest.raises(TypeError):
        EventOrigin("x-sce-host")  # type: ignore[call-arg]


# docs/adr/0090, section 7: a format-2 origin, as a saved state stores it, is
# read back as the pair and the sender.
def test_a_format2_origin_is_read_as_the_pair_it_stood_for() -> None:
    # Empty both: none.
    assert read_stored_origin("", "") == (None, None)
    # An empty address is none whatever type was saved beside it: format 2 wrote
    # the SCXML type beside events no processor stamped, and half a pair is
    # never read back.
    assert read_stored_origin("", SCXML_EVENT_PROCESSOR_URI) == (None, None)
    assert read_stored_origin("", "x-sce-host") == (None, None)
    # An address saved with no type is the SCXML processor's, as format 2 meant.
    assert read_stored_origin("mesh://motor", "") == (
        EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://motor"),
        None,
    )
    assert read_stored_origin("kid", "") == (scxml_origin("kid"), "kid")
    # A bare id with the SCXML type: that session's pair, the id its sender.
    assert read_stored_origin("root.work.1.kid", SCXML_EVENT_PROCESSOR_URI) == (
        scxml_origin("root.work.1.kid"),
        "root.work.1.kid",
    )
    # An SCXML location, whatever the stored type: the pair, the decoded id its
    # sender.
    assert read_stored_origin("sce://scxml/a%20b", "") == (scxml_origin("a b"), "a b")
    # Anything else: the pair as stored, no sender. A host's bare address keeps
    # its type, and a Mesh address is not read as a session.
    assert read_stored_origin("motor", "x-sce-host") == (EventOrigin("x-sce-host", "motor"), None)
    assert read_stored_origin("mesh://motor", SCXML_EVENT_PROCESSOR_URI) == (
        EventOrigin(SCXML_EVENT_PROCESSOR_URI, "mesh://motor"),
        None,
    )
    # The SCXML prefix alone names no session: a queued event keeps it as the
    # pair as stored, with no sender.
    assert read_stored_origin("sce://scxml/", SCXML_EVENT_PROCESSOR_URI) == (
        EventOrigin(SCXML_EVENT_PROCESSOR_URI, "sce://scxml/"),
        None,
    )


def test_a_waiting_sends_stored_origin_is_read_as_the_session_it_names() -> None:
    # A format-1 or format-2 waiting send stores its session's id, or the
    # location one backend stored in its place, read back as the id.
    assert read_stored_sender("fixture-root") == "fixture-root"
    assert read_stored_sender("sce://scxml/a%20b") == "a b"
    assert read_stored_sender("sce://scxml/peer:1") == "peer:1"
    assert read_stored_sender("") == ""


def test_an_event_is_stored_as_format2_writes_it() -> None:
    # A session's event stores its sender's id; any other stores the address.
    assert stored_origin_text(scxml_origin("kid"), "kid") == "kid"
    assert stored_origin_text(EventOrigin("x-sce-host", "motor"), None) == "motor"
    assert stored_origin_text(None, None) == ""


def test_what_is_stored_reads_back_as_what_was_carried() -> None:
    for origin, sender in [
        (scxml_origin("root.work.1.kid"), "root.work.1.kid"),
        (EventOrigin("x-sce-host", "motor"), None),
        (EventOrigin("sce:mesh-rpc", "mesh://motor"), None),
        (None, None),
    ]:
        text = stored_origin_text(origin, sender)
        assert read_stored_origin(text, origin_processor_of(origin)) == (origin, sender), text
