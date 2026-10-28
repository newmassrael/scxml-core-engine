# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""sce_runtime — pure-Python runtime for AOT-generated SCXML state machines.

Mirrors the role of sce-go-runtime / sce-rust-runtime / sce-kotlin-runtime:
generated `*_sm.py` modules define a concrete `StatePolicy` subclass; the
generic `Engine[S, E]` here drives the W3C SCXML execution algorithm against
that policy. The bindings package (`sce`) is a different channel — it routes
through the C++ Interpreter via pybind11, not through this AOT runtime.
"""

from . import scripting
from .action_realization import ActionRealization
from .child_mesh import (
    MESH_ORIGIN_SCHEME,
    ChildMeshRouter,
    ChildMeshRouters,
    MeshCall,
    MeshEndInvoke,
    MeshRaise,
    mesh_event_metadata,
)
from .configuration import ConfigurationRejection, validate_configuration
from .engine import Engine
from .event import Event, EventMetadata, EventWithMetadata
from .host_processor import (
    HOST_INVOKE_DEADLINE_PARAM,
    MESH_PROCESSOR_TYPE,
    MESH_RPC_INVOKE_TYPE,
    RESERVED_TYPE_PREFIX,
    is_reserved_type,
    HostInvokeCancel,
    HostInvokeEnding,
    HostInvokeEvent,
    HostInvokeHandler,
    HostInvokeRequest,
    HostInvokeResponse,
    HostSendHandler,
    HostSendRequest,
    HostSendResponse,
    TreeInvokers,
    parse_host_invoke_deadline_ms,
)
from .host_invoke_request import RequestFieldType, request_field, request_field_wire
from .http import HttpSendRequest, HttpSendResponse
from .io_processors import published_origin, session_id_from_scxml_location
from .invoke import (
    ChildSession,
    Invoke,
    PendingInvoke,
    ScxmlInvoke,
    create_done_invoke_event_name,
    document_stem,
)
from .microstep import EnabledTransition, EntryTarget, HistoryTarget, StateTarget
from .root_start import RootStartRefusal
from .policy import StatePolicy
from . import saved_values, session_id
from .saved_engine import (
    enter_session,
    judge_session,
    nothing_in_transit,
    restore_history,
    save_history,
    save_session,
)
from .saved_json import JsonError, JsonNumber, parse_json, write_json
from .saved_state import (
    SAVED_FORMAT,
    SAVED_FORMAT_1,
    SAVED_FORMAT_2,
    SAVED_SCHEMA_STATUS,
    AddedField,
    AddedVariant,
    EnumHolder,
    EnumLineage,
    RecordHolder,
    RecordLineage,
    SavedAct,
    SavedChild,
    SavedEvent,
    SavedHistory,
    SavedHostInvoke,
    SavedInvokes,
    SavedInvokeStart,
    SavedRevision,
    SavedSend,
    SavedShape,
    SavedState,
    StateRefusal,
    WidenedField,
    check_saved_shape,
    refusal_within,
    saved_state_from_json,
    saved_wall_clock_ms,
    upgrade_saved_state,
)
from .scheduler import ScheduledEvent, Scheduler
from .send import (
    is_invalid_target,
    is_mesh_target,
    is_supported_send_type,
    is_unreachable_target,
    mesh_peer,
    parse_delay_ms,
)
from .scripting import (
    IScriptEngine,
    LuaScriptEngine,
    ScriptError,
    ScriptValue,
    ScriptValueKind,
)

__all__ = [
    "ActionRealization",
    "AddedField",
    "AddedVariant",
    "EnumHolder",
    "EnumLineage",
    "RecordHolder",
    "RecordLineage",
    "SavedAct",
    "SavedChild",
    "SavedEvent",
    "SavedHistory",
    "SavedHostInvoke",
    "SavedInvokeStart",
    "SavedInvokes",
    "SavedRevision",
    "SavedSend",
    "SavedShape",
    "ChildMeshRouter",
    "ChildMeshRouters",
    "ChildSession",
    "ConfigurationRejection",
    "EnabledTransition",
    "Engine",
    "EntryTarget",
    "Event",
    "EventMetadata",
    "EventWithMetadata",
    "HOST_INVOKE_DEADLINE_PARAM",
    "HistoryTarget",
    "HostSendHandler",
    "HostInvokeCancel",
    "HostInvokeEnding",
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
    "JsonError",
    "JsonNumber",
    "LuaScriptEngine",
    "MESH_ORIGIN_SCHEME",
    "MESH_PROCESSOR_TYPE",
    "MESH_RPC_INVOKE_TYPE",
    "MeshCall",
    "MeshEndInvoke",
    "MeshRaise",
    "PendingInvoke",
    "RESERVED_TYPE_PREFIX",
    "RequestFieldType",
    "RootStartRefusal",
    "SAVED_FORMAT",
    "SAVED_FORMAT_1",
    "SAVED_FORMAT_2",
    "SAVED_SCHEMA_STATUS",
    "SavedState",
    "ScheduledEvent",
    "Scheduler",
    "ScriptError",
    "ScriptValue",
    "ScriptValueKind",
    "ScxmlInvoke",
    "StatePolicy",
    "StateRefusal",
    "StateTarget",
    "TreeInvokers",
    "WidenedField",
    "check_saved_shape",
    "create_done_invoke_event_name",
    "document_stem",
    "enter_session",
    "is_invalid_target",
    "is_mesh_target",
    "is_reserved_type",
    "is_supported_send_type",
    "is_unreachable_target",
    "judge_session",
    "nothing_in_transit",
    "parse_delay_ms",
    "mesh_event_metadata",
    "mesh_peer",
    "parse_host_invoke_deadline_ms",
    "parse_json",
    "published_origin",
    "refusal_within",
    "request_field",
    "request_field_wire",
    "restore_history",
    "save_history",
    "save_session",
    "saved_state_from_json",
    "saved_values",
    "saved_wall_clock_ms",
    "session_id",
    "session_id_from_scxml_location",
    "scripting",
    "upgrade_saved_state",
    "validate_configuration",
    "write_json",
]

__version__ = "0.1.0"
