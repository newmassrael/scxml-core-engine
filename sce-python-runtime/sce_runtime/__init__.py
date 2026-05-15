# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""sce_runtime — pure-Python runtime for AOT-generated SCXML state machines.

Mirrors the role of sce-go-runtime / sce-rust-runtime / sce-kotlin-runtime:
generated `*_sm.py` modules define a concrete `StatePolicy` subclass; the
generic `Engine[S, E]` here drives the W3C SCXML execution algorithm against
that policy. The bindings package (`sce`) is a different channel — it routes
through the C++ Interpreter via pybind11, not through this AOT runtime.
"""

from .engine import Engine
from .event import Event, EventMetadata, EventWithMetadata
from .http import HttpSendRequest, HttpSendResponse
from .invoke import (
    ChildSession,
    Invoke,
    PendingInvoke,
    ScxmlInvoke,
    create_done_invoke_event_name,
    is_platform_event,
)
from .policy import StatePolicy, TransitionResult
from .scheduler import ScheduledEvent, Scheduler

__all__ = [
    "ChildSession",
    "Engine",
    "Event",
    "EventMetadata",
    "EventWithMetadata",
    "HttpSendRequest",
    "HttpSendResponse",
    "Invoke",
    "PendingInvoke",
    "ScheduledEvent",
    "Scheduler",
    "ScxmlInvoke",
    "StatePolicy",
    "TransitionResult",
    "create_done_invoke_event_name",
    "is_platform_event",
]

__version__ = "0.1.0"
