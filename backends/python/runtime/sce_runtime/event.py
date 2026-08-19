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


def is_error_event(event_name: str) -> bool:
    """W3C SCXML 3.12.2 — whether `event_name` names an error the processor
    itself raised, as opposed to an event the document asked for.

    The clause reserves the whole `error.` prefix for them: it defines
    `error.execution` and `error.communication`, lets a platform add a suffix
    to either, and reserves `error.platform` with or without a suffix on top of
    that. The prefix is therefore the test — an enumeration would be wrong the
    first time the set is extended, which the same paragraph says may happen.

    Used by the engine's internal-queue drain to tell an error nobody answered
    from an author's own unmatched `<raise>`. The two are indistinguishable in
    the queue and are not the same event to a host: the author wrote one and can
    read its fate in the document, while the other was written by the engine to
    report that the document did not do what it said.
    """
    # §scxml-3.12.2: the processor "MUST signal any errors that occur by
    # raising SCXML events whose names begin with 'error.'". Cited as a `#`
    # comment rather than in the docstring above because the ledger's
    # comment-only reader does not see docstrings.
    return event_name.startswith("error.")
