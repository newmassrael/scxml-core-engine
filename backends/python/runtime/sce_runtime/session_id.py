# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The id of a root session, the value of the system variable ``_sessionid``
(docs/adr/0045, "Questions the owner answered" 2).

``_sessionid`` is bound when a session is loaded and kept until it ends, and it
is an XML Nmtoken: another session addresses this one as ``#_scxml_<id>``, and
every child's id is its parent's and the id the child was started under,
``<parent>.<invokeid>``. So a root's id is never empty. The host may give it when
it creates the machine (:func:`refusal_of` tells it when what it gives is not an
Nmtoken, rather than having it replaced), and where it gives none the runtime
draws one (:func:`draw`), a UUID version 4 in lowercase hexadecimal with hyphens,
the one spelling every backend draws, so no reader can tell which backend drew
it. A restore binds the saved id instead of either, since the session carries on.

Cross-language siblings: ``backends/go/runtime/session_id.go``,
``com.sce.runtime.SessionId`` (Kotlin) and ``sce_rust_runtime::helpers::session_id``
(Rust).
"""

from __future__ import annotations

import uuid
from typing import Optional

__all__ = ["draw", "is_nmtoken", "refusal_of"]


def refusal_of(session_id: str) -> Optional[str]:
    """Why ``session_id`` is not an id a session may have, or ``None`` when it is
    one: an XML 1.0 Nmtoken, one name character or more (production ``Nmtoken``,
    fifth edition)."""
    if not session_id:
        return "the session id is empty, and a session id is one name character or more (an XML Nmtoken)"
    for character in session_id:
        point = ord(character)
        if not _is_name_char(point):
            return (
                f"the session id holds U+{point:04X}, which is not a name character: a session id is "
                "an XML Nmtoken (letters, digits, '.', '-', '_', ':')"
            )
    return None


def is_nmtoken(session_id: str) -> bool:
    """Whether ``session_id`` is an XML 1.0 Nmtoken."""
    return refusal_of(session_id) is None


def draw() -> str:
    """A session id drawn from the platform's randomness, a UUID version 4
    (RFC 9562, section 5.4) in lowercase hexadecimal in groups of 8, 4, 4, 4 and
    12, for a session whose host gave none."""
    return str(uuid.uuid4())


def _is_name_start_char(c: int) -> bool:
    """XML 1.0 ``NameStartChar``."""
    return (
        c == 0x3A
        or 0x41 <= c <= 0x5A
        or c == 0x5F
        or 0x61 <= c <= 0x7A
        or 0xC0 <= c <= 0xD6
        or 0xD8 <= c <= 0xF6
        or 0xF8 <= c <= 0x2FF
        or 0x370 <= c <= 0x37D
        or 0x37F <= c <= 0x1FFF
        or 0x200C <= c <= 0x200D
        or 0x2070 <= c <= 0x218F
        or 0x2C00 <= c <= 0x2FEF
        or 0x3001 <= c <= 0xD7FF
        or 0xF900 <= c <= 0xFDCF
        or 0xFDF0 <= c <= 0xFFFD
        or 0x10000 <= c <= 0xEFFFF
    )


def _is_name_char(c: int) -> bool:
    """XML 1.0 ``NameChar``."""
    return (
        _is_name_start_char(c)
        or c == 0x2D
        or c == 0x2E
        or 0x30 <= c <= 0x39
        or c == 0xB7
        or 0x300 <= c <= 0x36F
        or 0x203F <= c <= 0x2040
    )
