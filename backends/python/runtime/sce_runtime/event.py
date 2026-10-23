# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Event types for the AOT Python runtime."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Generic, Optional, TypeVar

from .scripting.i_script_engine import ScriptValue

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
    # §scxml-6.4: the token of the host-run invocation this event completes,
    # set only by `Engine.complete_host_invoke` after it confirmed the
    # invocation was still running. A host invoke's `done.invoke.<id>` that
    # arrives without one did not come through that check, so the engine
    # refuses it. Not script-visible: `_event` has no field for it.
    host_invoke_token: Optional[int] = None
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
    # §scxml-5.10: `_event.name` is the name the event ARRIVED under. An
    # event is carried as the enumeration member of the descriptor the document
    # declares, and §scxml-3.12.1 lets `request.new` match a transition on
    # `request`, so the member alone loses the longer name. Set by a host that
    # sends by a name more specific than any the document declares, and by
    # `Engine.send_external_by_name` when it resolves a name through a prefix.
    # Empty means the member's own name IS the name, which is every event the
    # document itself raises or sends.
    name: str = ""


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


def event_data_text(data: Any) -> str:
    """W3C SCXML 5.10 + B-2-9 — the text of an event's data, which is the WIRE: a
    reader parses it under the three readings of an event's data (XML → DOM,
    JSON → value, otherwise a space-normalized string). Generated Python code marshals
    `<send>` payloads as a dict of named params or a bare scalar, which are
    written here into that wire, for the script engine's `set_current_event` and
    for a saved state, which holds an event's data as text.

    - dict / list → JSON encode (so `_event.data.foo` becomes a real table member
      at the receiving parse);
    - str → returned unchanged, which the string reading gives back whole;
    - int / float / bool → the JSON spelling, so a numeric payload arrives as a
      number rather than as its text;
    - None → `""` so the Lua side leaves `_event.data` unbound.

    What is deliberately absent is a spelling only the sender's own engine could
    read back: the receiving side used to RUN this string as Lua before parsing
    it, so a payload could execute at the far end."""
    # §scxml-B-2-8-1: the text written here is what the receiver reads under the
    # three readings, so each spelling below is one the string reading gives back
    # whole or the JSON reading gives back as the value.
    if data is None:
        return ""
    if isinstance(data, str):
        return data
    if isinstance(data, bool):
        return "true" if data else "false"
    if isinstance(data, int):
        return str(data)
    if isinstance(data, (float, dict, list)):
        # The one writer of JSON text this backend has (ARCHITECTURE.md,
        # "JSON Text", "JSON Number Text" and "JSON Object Key Order"): compact,
        # UTF-8, members in byte order, a float as ECMAScript spells it.
        # `json.dumps` did none of the last two — it spells a float with Python's
        # `repr` (`1.0`, `1e-07`) and keeps the order a dict was built in — so the
        # same payload differed byte for byte from the one another backend sent.
        return ScriptValue.of(data).to_json_literal()
    return str(data)


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
