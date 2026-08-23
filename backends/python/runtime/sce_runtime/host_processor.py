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
from typing import Callable, Dict, List


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
