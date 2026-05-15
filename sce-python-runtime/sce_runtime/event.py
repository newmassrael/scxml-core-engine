# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Event types for the AOT Python runtime."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Generic, Optional, TypeVar

E = TypeVar("E")


@dataclass
class EventMetadata:
    """W3C SCXML 5.10 _event fields populated for the currently dispatching event."""

    data: str = ""
    event_type: str = "external"
    send_id: str = ""
    origin: str = ""
    origin_type: str = ""
    invoke_id: str = ""


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
