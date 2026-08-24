# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""sce_runtime — pure-Python runtime for AOT-generated SCXML state machines.

Mirrors the role of sce-go-runtime / sce-rust-runtime / sce-kotlin-runtime:
generated `*_sm.py` modules define a concrete `StatePolicy` subclass; the
generic `Engine[S, E]` here drives the W3C SCXML execution algorithm against
that policy. The bindings package (`sce`) is a different channel — it routes
through the C++ Interpreter via pybind11, not through this AOT runtime.
"""

from . import scripting
from .configuration import ConfigurationRejection, validate_configuration
from .engine import Engine
from .event import Event, EventMetadata, EventWithMetadata
from .host_processor import (
    HostInvokeCancel,
    HostInvokeEvent,
    HostInvokeHandler,
    HostInvokeRequest,
    HostInvokeResponse,
    HostSendHandler,
    HostSendRequest,
    HostSendResponse,
)
from .http import HttpSendRequest, HttpSendResponse
from .io_processors import published_origin, session_id_from_scxml_location
from .invoke import (
    ChildSession,
    Invoke,
    PendingInvoke,
    ScxmlInvoke,
    create_done_invoke_event_name,
)
from .policy import StatePolicy, TransitionResult
from .scheduler import ScheduledEvent, Scheduler
from .scripting import (
    IScriptEngine,
    LuaScriptEngine,
    ScriptError,
    ScriptValue,
    ScriptValueKind,
)

__all__ = [
    "ChildSession",
    "ConfigurationRejection",
    "Engine",
    "Event",
    "EventMetadata",
    "EventWithMetadata",
    "HostSendHandler",
    "HostInvokeCancel",
    "HostInvokeEvent",
    "HostInvokeHandler",
    "HostInvokeRequest",
    "HostInvokeResponse",
    "HostSendRequest",
    "HostSendResponse",
    "HttpSendRequest",
    "HttpSendResponse",
    "IScriptEngine",
    "Invoke",
    "LuaScriptEngine",
    "PendingInvoke",
    "ScheduledEvent",
    "Scheduler",
    "ScriptError",
    "ScriptValue",
    "ScriptValueKind",
    "ScxmlInvoke",
    "StatePolicy",
    "TransitionResult",
    "create_done_invoke_event_name",
    "published_origin",
    "session_id_from_scxml_location",
    "scripting",
    "validate_configuration",
]

__version__ = "0.1.0"
