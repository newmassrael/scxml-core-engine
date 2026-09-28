# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

"""The run-time judgements a ``<send>`` makes about the values its own
arguments evaluated to.

Port of the C++ ``SendHelper`` (``sce/include/common/SendHelper.h``), whose
Rust and Go ports are ``helpers::send`` and ``runtime/send.go``. A generated
send reads these rather than spelling the rules inline, so a ``typeexpr`` or a
``targetexpr`` is judged by the same rule on every backend.
"""

from __future__ import annotations

from typing import Optional

from .io_processors import BASIC_HTTP_EVENT_PROCESSOR_URI, SCXML_EVENT_PROCESSOR_URI

MAX_DELAY_MS = (1 << 63) - 1
"""The largest delay any engine can hold, in milliseconds: the positive range
of a signed 64-bit count, because Kotlin's ``Long`` is signed and every engine
answers the same text the same way."""

_ASCII_WHITESPACE = " \t\n\r\f\v"
_DIGITS = frozenset("0123456789")


def is_supported_send_type(send_type: str) -> bool:
    """Whether this platform delivers through ``send_type``: the SCXML Event
    I/O Processor, named or defaulted, and the Basic HTTP one."""
    # §scxml-6.2 — a type outside this set is the same error as a type that
    # could not be evaluated: error.execution, and the message is discarded.
    return send_type in ("", SCXML_EVENT_PROCESSOR_URI, BASIC_HTTP_EVENT_PROCESSOR_URI)


def is_invalid_target(target: str) -> bool:
    """Whether ``target`` is one this processor cannot address (W3C test194):
    a target that opens with ``!``."""
    # §scxml-6.2 — a target the processor cannot address is refused with
    # error.execution before anything is delivered.
    return target.startswith("!")


def mesh_peer(target: str) -> Optional[str]:
    """The peer a ``<send target>`` names, when it names one: ``#`` followed
    by at least one character, where ``#_`` stays reserved for the targets
    §scxml-6.2.4 defines (``#_internal``, ``#_parent``, ...).

    The Python copy of C++ ``SendHelper::isMeshTarget``; every copy reads
    ``tests/mesh/mesh_target_cases.json``, so a target one of them routes over
    Mesh is one they all do."""
    if not target.startswith("#"):
        return None
    peer = target[1:]
    return peer if peer and not peer.startswith("_") else None


def is_mesh_target(target: str) -> bool:
    """Whether a ``<send target>`` names a Mesh peer (see `mesh_peer`)."""
    return mesh_peer(target) is not None


def parse_delay_ms(text: str) -> Optional[int]:
    """A ``<send>`` delay in milliseconds, or ``None`` when the text is not a
    time — a bare number included — so the caller raises the argument error
    rather than choosing a wait.

    The grammar is ARCHITECTURE.md's "Durations (Single Source of Truth)":
    surrounding ASCII whitespace aside, a non-negative number (digits with an
    optional fraction of at least one digit, or a leading ``.`` and digits; no
    sign, no exponent) followed directly by ``ms`` or ``s``, either case. The
    milliseconds are computed in exact decimal and truncated, never through a
    float. ``tests/durations/css2_time.json`` holds the cases every engine is
    measured against."""
    # §scxml-6.2 — 'delay' and the value of 'delayexpr' must be a valid CSS2
    # time; anything else is not one.
    s = text.strip(_ASCII_WHITESPACE)
    lowered = s.lower()
    if lowered.endswith("ms"):
        number, scale = s[:-2], 1
    elif lowered.endswith("s"):
        number, scale = s[:-1], 1000
    else:
        return None
    whole, point, fraction = number.partition(".")
    if not set(whole) <= _DIGITS or (point and (not fraction or not set(fraction) <= _DIGITS)):
        return None
    if not whole and not point:
        return None
    ms = int(whole or "0") * scale
    # Only the fraction digits that name whole milliseconds count; the rest
    # truncate.
    place = scale // 10
    for digit in fraction:
        if place == 0:
            break
        ms += int(digit) * place
        place //= 10
    return ms if ms <= MAX_DELAY_MS else None
