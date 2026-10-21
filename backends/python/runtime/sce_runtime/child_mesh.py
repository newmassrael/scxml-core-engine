# SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""The Mesh router a child session is given by its parent (SCE_MESH.md, Child
Sessions in Mesh; docs/adr/0043). The Python twin of the Rust runtime's
``ChildMeshRouter``, ``MeshCall`` and ``mesh_event_metadata``."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Callable, List, Optional, Protocol, Union

from .event import EventMetadata
from .host_processor import (
    HostInvokeEvent,
    HostInvokeResponse,
    HostSendRequest,
    HostSendResponse,
)
from .io_processors import SCXML_EVENT_PROCESSOR_URI

#: ``_event.origin`` of a Mesh-delivered event is this scheme and the sending
#: machine's name (SCE_MESH.md §mesh-10.7).
MESH_ORIGIN_SCHEME = "mesh://"


def mesh_event_metadata(data: str, source: str, send_id: str) -> EventMetadata:
    """The ``_event`` of an event that arrived over Mesh from `source`, as
    SCE_MESH.md §mesh-10.7 tabulates it: external, origin ``mesh://<source>``,
    the SCXML processor as origin type, and the send id the envelope carried.
    The one place it is built, so a child cannot tell a peer's event from the
    one its parent would have been given."""
    return EventMetadata(
        data=data,
        event_type="external",
        send_id=send_id,
        origin=MESH_ORIGIN_SCHEME + source,
        origin_type=SCXML_EVENT_PROCESSOR_URI,
    )


@dataclass
class MeshRaise:
    """Raise `name` on the external queue with the ``_event`` the endpoint
    built for it."""

    name: str
    metadata: EventMetadata


@dataclass
class MeshEndInvoke:
    """End the start `token` of the Mesh request `invoke_id` — with
    ``done.invoke``, or with ``error.invoke`` when `failed` — carrying `data` as
    ``_event.data``. `source` is the machine that answered, ``None`` for a
    deadline the requester reached itself."""

    invoke_id: str
    token: int
    failed: bool
    data: str
    source: Optional[str]


#: One thing a Mesh endpoint has for the engine it serves, in the order the
#: endpoint came to know it: what ``Engine.apply_mesh_call`` applies.
MeshCall = Union[MeshRaise, MeshEndInvoke]


class ChildMeshRouter(Protocol):
    """The Mesh router a child session is given by its parent: the child's
    endpoint attached to that one session.

    What a host installs on a machine it starts, the parent installs on the
    child it starts, before the child initializes, since an ``<onentry>`` of
    the child's initial state may already send: the child's
    ``<send type="sce:mesh">`` goes to `send` and its
    ``<invoke type="sce:mesh-rpc">`` to `invoke`, as `register_mesh_router` and
    `register_mesh_rpc_invoker` route a machine's. What arrives for the child
    waits here until the parent drives the child, which takes it with
    `take_calls` before the child's macrostep. When the session ends the
    parent says so with `detach`, after the child has stopped."""

    def send(self, request: HostSendRequest) -> List[HostSendResponse]:
        """A ``<send type="sce:mesh">`` of the child."""
        ...

    def invoke(self, event: HostInvokeEvent) -> Optional[HostInvokeResponse]:
        """An ``<invoke type="sce:mesh-rpc">`` of the child starting or being
        cancelled."""
        ...

    # §mesh-21.4: what arrives for the child waits in its router until the
    # parent drives the child, which takes it here before the child's macrostep.
    def take_calls(self) -> List[MeshCall]:
        """What arrived for the child since it was last asked, oldest first."""
        ...

    # §mesh-21.5: the router is told once that the session ended.
    def detach(self) -> None:
        """The session ended: it reached a top-level ``<final>``, or its parent
        left the invoking state and cancelled it. Called once."""
        ...

    def child_router(self, invoke_id: str, document: str) -> Optional["ChildMeshRouter"]:
        """The router of a child this child starts; ``None`` leaves that child
        with no router."""
        ...


#: The factory of the routers of a machine's children (§mesh-21.3), registered
#: with ``Engine.register_child_mesh_routers``: asked, each time an invocation starts
#: a child, with the invoke's id and the stem of the child's document (empty for
#: a child written inline). ``None`` leaves the child with no router.
ChildMeshRouters = Callable[[str, str], Optional[ChildMeshRouter]]
