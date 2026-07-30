# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""``_ioprocessors`` entry set (§scxml-C-1-1, §scxml-C-2-3).

Port of the C++ ``IOProcessorHelper`` (``sce/include/common/IOProcessorHelper.h``).
Deciding the entries here rather than inside each script engine is what keeps a
machine reading the same entry names and the same addresses whichever backend
runs it — before this existed the Lua engine published ``#<uri>`` as every
processor's location, which names nothing an external component can post to.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import List
from urllib.parse import quote

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

    §scxml-C-1 leaves the transport platform-specific, so the address is an
    SCE-scheme URI naming the session. The session id is percent-encoded
    because it is not constrained to URI-safe characters.
    """
    return "sce://scxml/" + quote(session_id, safe="-_.~")


def build(session_id: str, basic_http_access_uri: str = "") -> List[IoProcessorDescriptor]:
    """Entry set for a session.

    Every processor is filed twice: under the specification's entry name and
    under the short alias SCXML documents index with. Both keys carry the same
    location, so the choice of spelling never changes where an event goes.

    §scxml-C-2-3's entry appears only when ``basic_http_access_uri`` is
    non-empty. Support for that processor is optional and per-deployment, so a
    session with no inbound endpoint advertises no address rather than one
    nothing answers on.
    """
    scxml_uri = scxml_location(session_id)
    descriptors = [
        IoProcessorDescriptor(SCXML_EVENT_PROCESSOR_URI, scxml_uri),
        IoProcessorDescriptor(SCXML_ALIAS, scxml_uri),
    ]
    if basic_http_access_uri:
        descriptors.append(
            IoProcessorDescriptor(BASIC_HTTP_EVENT_PROCESSOR_URI, basic_http_access_uri)
        )
        descriptors.append(IoProcessorDescriptor(BASIC_HTTP_ALIAS, basic_http_access_uri))
    return descriptors
