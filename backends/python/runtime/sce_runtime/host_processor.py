# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""Host-supplied Event I/O Processors — the payload types a host registers
a handler for.

W3C SCXML 6.2.5 makes a `<send>` `type` an extensible identifier, so the
set of Event I/O Processors is open by design. SCE implements two of them;
anything else was refused with `error.execution` and no platform could
widen the set — a consumer could name a processor and be refused, but not
name one and be served. A host declares the types it serves at build time
(so codegen emits a dispatch instead of a refusal) and registers a handler
for each at run time.

Mirrors `backends/rust/runtime/src/host_processor.rs`,
`sce/include/core/HostProcessor.h`, `backends/go/runtime/host_processor.go`
and `backends/c/runtime/include/sce/host_processor.h` field for field.
Keeping the shape identical is what lets one host be described once and
ported, and it is the same reason `HttpSendRequest` beside it matches its
siblings.

Shaped after `http.py`, which is the same idea fixed to one type: a request
carrying what `<send>` said, and a reply the engine turns back into an
event. The difference is the key — this one is looked up by the `type`
string.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Callable, Dict, List, Optional


@dataclass
class HostSendRequest:
    """What a `<send>` addressed to a host-served processor said.

    Every field is what the document wrote, not an interpretation of it: a
    handler that wants to reject a malformed request needs to see the same
    thing the author typed.

    `processor_type` is present even though the handler was looked up by
    it, because one handler may be registered for several types and would
    otherwise have to be told which it is by a closure per registration.
    `target` is passed through uninterpreted — the specification leaves a
    target's meaning to the processor that serves it. `params` is a
    multi-map because `<param>` may repeat with the same name and every
    value must be delivered."""

    processor_type: str = ""
    event_name: str = ""
    target: str = ""
    content: str = ""
    params: Dict[str, List[str]] = field(default_factory=dict)
    send_id: str = ""


@dataclass
class HostSendResponse:
    """One event a host-served act produced.

    The engine raises each on the external queue, which is where a reply
    from outside the machine belongs (W3C SCXML C.1). A name the generated
    machine does not declare is dropped, matching what the engine does with
    any such event."""

    event_name: str = ""
    event_data: str = ""


#: What a host registers for one declared processor type.
#:
#: It answers with the events the act produced, IN ORDER. A list rather
#: than a single reply, for one reason: an act can produce two observations
#: that the document must see in a particular order, and every other way of
#: expressing that costs portability or hides state.
#:
#: `examples/ai_loop/` is the case. Its `priming` state leaves on
#: `prompt.sent` — "the session has been told what it is here for" — and
#: only then is the machine somewhere a turn result means anything;
#: reporting the turn first leaves the run sitting in `priming` forever. So
#: prompting a fresh session produces exactly two events with exactly one
#: correct order.
#:
#: An empty list is "performed, nothing to report", which is the common
#: case for a fire-and-forget act and for real work that will answer later
#: through the host's own loop. It is NOT an error, and the engine does not
#: treat it as one.
#:
#: A handler that raises is a host defect and is not caught by the engine —
#: it cannot invent a W3C-meaningful outcome for one, and swallowing it
#: would produce exactly the silence this whole surface exists to remove.
HostSendHandler = Callable[[HostSendRequest], List[HostSendResponse]]


# ── §scxml-6.4.1: `<invoke>` the HOST runs ───────────────────────────────
#
# The clause leaves the invokable set to the platform in the same words 6.2.5
# uses for `<send>`, so a host may implement its own `type` here too — but an
# invoke is not a send. It has a LIFETIME: it starts when the state is entered,
# it is cancelled if the state exits, and the document may be waiting on
# `done.invoke.<id>`. That is why the handler receives an event rather than a
# bare request, and why this is a second registry rather than a second use of
# the first: a host that can deliver an event is not thereby able to run a
# process it must also be able to stop.


@dataclass
class HostInvokeRequest:
    """An ``<invoke>`` the host runs, at the point the state was entered."""

    #: The `type` this ``<invoke>`` named.
    processor_type: str = ""
    #: The invoke's id (§scxml-6.4.1), auto-derived when the author
    #: declared none. This is the name the DOCUMENT waits on: a completion
    #: is ``done.invoke.<invoke_id>``, so a host that finishes
    #: asynchronously must keep it.
    invoke_id: str = ""
    #: ``<invoke src="...">``, empty when the document named none. SCE does
    #: not interpret it — what a src means is the invoked processor's
    #: business.
    src: str = ""
    #: ``<param>`` values keyed by name; a repeated name keeps every value
    #: in document order.
    params: Dict[str, List[str]] = field(default_factory=dict)
    #: Inline ``<content>``, empty when the document carried none.
    content: str = ""


@dataclass
class HostInvokeCancel:
    """An ``<invoke>`` the host was running, at the point its state exited."""

    #: The `type` the ``<invoke>`` named.
    processor_type: str = ""
    #: The invocation being cancelled — the same id its start carried.
    invoke_id: str = ""


@dataclass
class HostInvokeEvent:
    """One turn of a host-run invoke's lifecycle.

    Exactly one of ``start`` and ``cancel`` is set. Both arms go to ONE
    registered handler rather than to two separately registered callbacks,
    because a host that can start an invocation and cannot stop it is not a
    working invoker — and two registrations make that state reachable. One
    handler means the pair is registered together or not at all."""

    #: §scxml-6.4: the state was entered and the macrostep has settled.
    #: Begin the invoked process.
    start: Optional[HostInvokeRequest] = None
    #: §scxml-6.4: the state exited. Stop it.
    #:
    #: Delivered only for an invocation that actually started: a state that
    #: exits before the macrostep ends never runs its invoke, and cancelling
    #: something that never began would have the host tearing down state it
    #: never built.
    cancel: Optional[HostInvokeCancel] = None


@dataclass
class HostInvokeResponse:
    """A host invoker's answer to a start.

    Read only for a start; an answer to a cancel is ignored, because there
    is nothing left for it to mean."""

    #: Payload for an immediate ``done.invoke.<invoke_id>``, for an
    #: invocation that completed before returning. ``None`` is the ordinary
    #: case: the work outlives the call and the host raises the completion
    #: itself when it finishes. SCE does not synthesise a completion the
    #: host did not report — an invoked process that never terminates never
    #: fires ``done.invoke``, which is what §scxml-6.4 says.
    done_data: Optional[str] = None


#: A registered invoke-lifecycle handler.
HostInvokeHandler = Callable[[HostInvokeEvent], Optional[HostInvokeResponse]]
