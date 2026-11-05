# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The reply address an event carries (docs/adr/0090, section 1).

Port of the C++ ``Core::EventOrigin`` (``sce/include/core/EventOrigin.h``): the
Event I/O Processor that delivered an event and the address that processor
accepts as a ``<send>`` target. The document reads the two as
``_event.origintype`` and ``_event.origin``; they are one value because the
address means something only relative to its processor.

The processor that delivers an event writes the pair once, where it builds the
event (section 2), and nothing converts it later. An event that no processor
stamped carries ``None``, never half of a pair. A leaf module, so the SCXML
address codec in ``io_processors`` and every carrier of an event can name the
type without an import cycle.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Optional


@dataclass(frozen=True)
class EventOrigin:
    """Where a reply to an event goes: the processor and the address it accepts.

    Constructed whole: both fields are required and neither has a default, so a
    pair with one field set cannot be spelled by accident. An event with no
    origin holds ``None`` instead."""

    #: The canonical type URI of the Event I/O Processor that delivered the
    #: event (``_event.origintype``).
    processor: str
    #: A URI that processor accepts as a ``<send>`` target (``_event.origin``).
    address: str


def origin_address_of(origin: Optional[EventOrigin]) -> str:
    """``_event.origin`` of an event that carries `origin`: its address, or the
    empty string when the event has none."""
    return "" if origin is None else origin.address


def origin_processor_of(origin: Optional[EventOrigin]) -> str:
    """``_event.origintype`` of an event that carries `origin`: its processor, or
    the empty string when the event has none."""
    return "" if origin is None else origin.processor
