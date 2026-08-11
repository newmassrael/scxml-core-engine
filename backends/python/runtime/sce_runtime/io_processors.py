# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
from typing import List
from urllib.parse import quote, unquote

SCXML_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor"
BASIC_HTTP_EVENT_PROCESSOR_URI = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor"

#: Alias the SCXML Event I/O Processor is indexed under by SCXML documents.
SCXML_ALIAS = "scxml"

#: Alias the Basic HTTP Event I/O Processor is indexed under by SCXML documents.
BASIC_HTTP_ALIAS = "basichttp"


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
    return "sce://scxml/" + quote(session_id, safe="-_.~")


def session_id_from_scxml_location(uri: str) -> str:
    """Session id an SCXML Event I/O Processor location names, or ``""``.

    The inverse of :func:`scxml_location`, kept beside it so the two spellings
    of one address cannot drift apart. A published location is only usable as
    a ``<send>`` target if something can read a session back out of it.
    """
    # §scxml-C-1 — reads a session id back out of a published location; the
    # routing requirement itself is realised by the location builder above.
    prefix = "sce://scxml/"
    if not uri.startswith(prefix) or len(uri) <= len(prefix):
        return ""
    return unquote(uri[len(prefix):])


def published_origin(origin_session_id: str) -> str:
    """The ``_event.origin`` a receiver should see for an event sent by
    ``origin_session_id``.

    The origin of a delivered event must match the 'location' the sending
    session published, which is what makes it an address the
    receiver can answer. The engine carries the sender's BARE session id
    internally — ``EventMetadata.origin`` — because its session-keyed lookups
    (``<finalize>`` dispatch, cancelled-invoke filtering) match on the id.
    Converting where the event is raised would make one value serve two
    consumers that need different spellings. So the conversion belongs at the
    boundary where the value becomes visible to the document, and this is that
    conversion — the same rule, and the same shape, as the C++
    ``IOProcessorHelper::publishedOrigin`` both engines already share.

    A remote invoke is the case that makes this more than a rename: its child
    session is stamped with a URI rather than an id, and wrapping a URI in
    :func:`scxml_location` would produce an address naming nothing. An argument
    that already carries a scheme is therefore passed through — it is already
    an address.
    """
    # §scxml-C-1 — decides the origin a receiver sees, and it is the location
    # the sending session published, not the bare id the engine routes on.
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
