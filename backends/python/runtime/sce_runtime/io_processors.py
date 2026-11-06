# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""``_ioprocessors`` entry set for the Event I/O Processors.

Port of the C++ ``IOProcessorHelper`` (``sce/include/common/IOProcessorHelper.h``).
Deciding the entries here rather than inside each script engine is what keeps a
machine reading the same entry names and the same addresses whichever backend
runs it — before this existed the Lua engine published ``#<uri>`` as every
processor's location, which names nothing an external component can post to.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional, Tuple
from urllib.parse import quote, unquote

from .event_origin import EventOrigin, origin_address_of

SCXML_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
BASIC_HTTP_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

#: Alias the SCXML Event I/O Processor is indexed under by SCXML documents.
SCXML_ALIAS = "scxml"

#: Alias the Basic HTTP Event I/O Processor is indexed under by SCXML documents.
BASIC_HTTP_ALIAS = "basichttp"

#: What every location the SCXML Event I/O Processor publishes begins with,
#: before the session id it names. Alone, it names no session.
SCXML_LOCATION_PREFIX = "sce://scxml/"


@dataclass(frozen=True)
class IoProcessorDescriptor:
    """One ``_ioprocessors`` entry.

    ``name`` is the key the entry is filed under; ``location`` is the address
    external entities use to reach this session through that processor.
    """

    name: str
    location: str


def scxml_location(session_id: str) -> str:
    """Address that reaches this session over the SCXML Event I/O Processor.

    The specification leaves the transport platform-specific, so the address
    is an SCE-scheme URI naming the session. The session id is percent-encoded
    because it is not constrained to URI-safe characters.
    """
    # §scxml-C-1 — this builds the location the SCXML Event I/O Processor
    # publishes for a session, which is the address `<send>` may target.
    return SCXML_LOCATION_PREFIX + quote(session_id, safe="-_.~")


def session_id_from_scxml_location(uri: str) -> str:
    """Session id an SCXML Event I/O Processor location names, or ``""``.

    The inverse of :func:`scxml_location`, kept beside it so the two spellings
    of one address cannot drift apart. A published location is only usable as
    a ``<send>`` target if something can read a session back out of it.
    """
    # §scxml-C-1 — reads a session id back out of a published location; the
    # routing requirement itself is realised by the location builder above.
    prefix = SCXML_LOCATION_PREFIX
    if not uri.startswith(prefix) or len(uri) <= len(prefix):
        return ""
    return unquote(uri[len(prefix):])


def scxml_origin(sender_session_id: str) -> EventOrigin:
    """The origin the SCXML Event I/O Processor writes for an event a session
    sends (docs/adr/0090, section 2).

    The origin of an event raised in the receiving session must match the
    location the sending session publishes, so the pair is the processor's
    canonical type and :func:`scxml_location` of the sender, the one function
    that also writes the session's ``_ioprocessors`` entry. The sender's id
    itself travels beside the pair, as the event's ``sender``, for the lookups
    that find a session by it. Port of the C++ ``IOProcessorHelper::scxmlOrigin``.
    """
    return EventOrigin(SCXML_EVENT_PROCESSOR_URI, scxml_location(sender_session_id))


def scxml_origin_of(sender_session_id: Optional[str]) -> Optional[EventOrigin]:
    """:func:`scxml_origin` of a sender that may be absent: a machine that holds
    no session id has no address to publish, so an event it sends carries no
    origin rather than the location of an empty id."""
    if not sender_session_id:
        return None
    return scxml_origin(sender_session_id)


def stored_origin_text(origin: Optional[EventOrigin], sender: Optional[str]) -> str:
    """The ``origin`` text that stores an event's origin and sender in a text
    that has no place for the pair.

    docs/adr/0090, section 7: the sending session's id for an event a session
    sent, the address otherwise, and empty for none. A format-3 text holds the
    pair itself; this is the rule a state read from a format-1 text is written
    back by, without a machine, as format 1, whose ``origintype`` beside it is
    the pair's processor."""
    return sender if sender else origin_address_of(origin)


def read_stored_origin(
    origin: str, origin_type: str
) -> Tuple[Optional[EventOrigin], Optional[str]]:
    """The origin and sender a stored ``origin`` and ``origintype`` are read as.

    docs/adr/0090, section 7. An empty ``origin`` is none whatever type was
    stored beside it: an empty address answers nothing, and format 2 wrote the
    SCXML type beside it for events no processor stamped. An empty
    ``origintype`` beside an address is the SCXML processor's, which is what
    format 2 meant by it. A value without a scheme read with the SCXML type is
    a session's id, read as that session's pair with the id as its sender; an
    SCXML location (which one backend stored in place of the id) is read the
    same way by the id it names. Anything else is the pair as stored, with no
    sender. Never half a pair. Port of the C++
    ``IOProcessorHelper::readStoredOrigin``."""
    if not origin:
        return None, None
    located = session_id_from_scxml_location(origin)
    if located:
        return scxml_origin(located), located
    processor = origin_type or SCXML_EVENT_PROCESSOR_URI
    if "://" not in origin and processor == SCXML_EVENT_PROCESSOR_URI:
        return scxml_origin(origin), origin
    return EventOrigin(processor, origin), None


def read_stored_sender(origin: str) -> str:
    """The sending session's id a waiting send's stored format-1 or format-2
    ``origin`` names, or ``""`` for an empty one.

    docs/adr/0090, section 7: such a text stores the sending session's id, or
    its location as one backend stored it, which is read back as the id it
    names. Port of the C++ ``IOProcessorHelper::readStoredSender``."""
    located = session_id_from_scxml_location(origin)
    return located if located else origin


def published_origin(origin_session_id: str) -> str:
    """The ``_event.origin`` an engine that carries the sender's bare session id
    publishes for it.

    This runtime no longer calls it: an event carries the pair the delivering
    processor wrote (:func:`scxml_origin`), so nothing converts at the read. It
    is kept, with the C++ ``IOProcessorHelper::publishedOrigin`` and the other
    runtimes' ports, until docs/adr/0090 removes the rule from every runtime at
    once (its step 7). It leaves an argument that carries a scheme as it is and
    wraps anything else as an SCXML location.
    """
    # §scxml-C-1 — the location the sending session published, from the bare
    # id an engine that routes on it carried.
    if not origin_session_id:
        return ""
    if "://" in origin_session_id:
        return origin_session_id
    return scxml_location(origin_session_id)


def build(session_id: str, basic_http_access_uri: str = "") -> List[IoProcessorDescriptor]:
    """Entry set for a session.

    Every processor is filed twice: under the specification's entry name and
    under the short alias SCXML documents index with. Both keys carry the same
    location, so the choice of spelling never changes where an event goes.

    The Basic HTTP entry appears only when ``basic_http_access_uri`` is
    non-empty. Support for that processor is optional and per-deployment, so a
    session with no inbound endpoint advertises no address rather than one
    nothing answers on.
    """
    scxml_uri = scxml_location(session_id)
    # §scxml-C-1-1 — the SCXML Event I/O Processor's entry, filed under the
    # specification's name and under the alias documents index with.
    descriptors = [
        IoProcessorDescriptor(SCXML_EVENT_PROCESSOR_URI, scxml_uri),
        IoProcessorDescriptor(SCXML_ALIAS, scxml_uri),
    ]
    # §scxml-C-2-3 — the Basic HTTP entry, present only for a deployment that
    # actually serves an inbound endpoint.
    if basic_http_access_uri:
        descriptors.append(
            IoProcessorDescriptor(BASIC_HTTP_EVENT_PROCESSOR_URI, basic_http_access_uri)
        )
        descriptors.append(IoProcessorDescriptor(BASIC_HTTP_ALIAS, basic_http_access_uri))
    return descriptors
