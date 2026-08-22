# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Which reading of §scxml-B-2-8-1 a payload actually got.

The clause gives ``_event.data`` three readings and no fourth: content the
processor can interpret as XML becomes a DOM, content it can interpret as a
value becomes that value, and "otherwise, the Processor MUST treat the content
as a space-normalized string literal". Every engine here walks that ladder, and
until now every engine dropped which rung it landed on.

Dropping it is what makes a lost payload silent. Measured 2026-08-22 on three
independent Lua implementations (mlua, go-lua and Lua 5.4), a host that hands
over ``{["milestone"]="refined"}`` — Lua's own table syntax — gets the third
rung, and a document that then reads ``_event.data.milestone`` assigns nothing.
In the worked supervision loop that emptied ``start_prompt`` as well, so the
restarted session was primed with an empty string and the run converged anyway.
Nothing failed; the information stopped existing.

``UNDECODABLE`` is the one a host acts on, and it is not the engine guessing
from a leading brace: the script engine reports it because it ATTEMPTED a
structured read and that read failed, which is a fact only the ladder holds.

⚠ A module of its own rather than a member of the script-engine interface. The
type started beside ``SetCurrentEventArgs`` in the C++ port and had to move: the
AOT engine counts these readings and includes no ``scripting/`` header, because
the policy owns the engine handle. A reading is a fact about an EVENT's
payload, not about the interface that happens to produce it, and the Python port
mirrors that placement so the seven backends stay recognisable to one reader.

Cross-language siblings: ``SCE::PayloadReading`` (C++),
``sce_rust_runtime::PayloadReading`` and ``sce.PayloadReading`` (Go).
"""

from enum import Enum


class PayloadReading(Enum):
    """The rung §scxml-B-2-8-1 gave a delivered payload."""

    #: The event carried no payload, so no rung applies.
    ABSENT = "absent"
    #: Rung one: read as an XML document, bound as a DOM.
    DOM = "dom"
    #: Rung two: read as a value, bound as that value.
    STRUCTURED = "structured"
    #: Rung three, and nothing suggested the content was structured. A
    #: ``<content>`` element holding prose lands here, and that is correct —
    #: W3C test 562 pins it.
    TEXT = "text"
    #: Rung three, taken AFTER a structured read was attempted and failed. The
    #: payload announced itself as structure and the datamodel could not read
    #: it, so ``_event.data`` holds the raw characters and every
    #: ``_event.data.<field>`` the document reads is empty.
    UNDECODABLE = "undecodable"


def payload_reading_of_text(payload: str) -> PayloadReading:
    """Which third-rung reading a payload that fell through to text deserves.

    The clause treats prose and a malformed object identically — both are
    "otherwise" — and a host does not. This is the one place that rule is
    written, so the ladder's implementations mirror a definition instead of
    each deciding for itself what "looks structured" means.

    The test is the opening character, and deliberately only ``{`` and ``[``. A
    number, a bare word or a quoted string is what an author writes in a
    ``<content>`` element, and W3C test 562 requires those to arrive as text
    without complaint; an object or an array is what a host CONSTRUCTS, and
    nobody constructs one by accident. Widening this to "anything not obviously
    prose" would report the ladder working as a defect, which is the failure
    that gets a diagnostic ignored.
    """
    stripped = payload.lstrip(" \t\n\r\f\v")
    if stripped.startswith("{") or stripped.startswith("["):
        return PayloadReading.UNDECODABLE
    return PayloadReading.TEXT
