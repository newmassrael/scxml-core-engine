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

from .io_processors import BASIC_HTTP_EVENT_PROCESSOR_URI, SCXML_EVENT_PROCESSOR_URI


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
