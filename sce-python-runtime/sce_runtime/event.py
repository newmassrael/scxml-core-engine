# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Event types for the AOT Python runtime."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Generic, TypeVar

E = TypeVar("E")


@dataclass
class EventMetadata:
    """W3C SCXML 5.10 _event fields populated for the currently dispatching event."""

    data: Any = ""
    event_type: str = "external"
    send_id: str = ""
    origin: str = ""
    origin_type: str = ""
    invoke_id: str = ""
    # NL→IR Item C1 Path A (EventSchema MCU native lowering): the type-erased
    # typed `_event.data` payload carrier. For an event whose imported
    # EventSchema lowered a transition guard to a native comparison, the
    # generated per-event inject seam (`raise_<event>`) packs the typed payload
    # dataclass here; the generated policy's `set_current_event` override lifts
    # it into a typed `_pending_<event>_payload` field the native guard reads.
    # `None` for every untyped event, so the script-engine baseline is
    # unchanged. The Python twin of the Go `EventMetadata.TypedPayload any` /
    # Kotlin `EventMetadata.typedPayload: Any?`.
    typed_payload: Any = None


@dataclass
class Event(Generic[E]):
    """Concrete event with name + payload metadata."""

    event: E
    metadata: EventMetadata = field(default_factory=EventMetadata)


@dataclass
class EventWithMetadata(Generic[E]):
    """Alias kept for cross-backend parity with EventWithMetadata[E] in Go/Rust."""

    event: E
    metadata: EventMetadata = field(default_factory=EventMetadata)

    @property
    def name(self) -> Any:
        return self.event
