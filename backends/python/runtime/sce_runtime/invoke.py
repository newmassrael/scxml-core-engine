# SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
# SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
"""W3C SCXML 6.4 invoke runtime — child-session lifecycle, autoforward,
finalize, and done.invoke wiring.

`Invoke[E]` is the ABC every concrete invoke kind implements. `ScxmlInvoke`
covers the in-process `<invoke type="scxml">` case (child is a sibling
generated `*_sm.py` module) — the only kind γ-4a lights up. HTTP transports
(`BasicHTTPEventProcessor`) ride a different `Invoke` subclass that γ-4b
lands; mesh-rpc invokes stay permanently rejected per the C++-first mesh
policy.

The runtime keeps the state on `Engine` (`_pending_invokes`,
`_active_invokes`); generated policy code populates those lists via the
`defer_invokes_on_entry` / `execute_pending_invokes` hooks. This split
mirrors Go's `sce.PendingInvoke` / `ChildSession` + policy method
contract (`backends/go/runtime/invoke.go`) so the two backends evolve
together when W3C clarifies invoke semantics.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Generic, Iterable, List, Optional, Tuple, TypeVar

E = TypeVar("E")


@dataclass
class PendingInvoke:
    """W3C SCXML 6.4 — a `<invoke>` whose owner state has been entered but
    whose child has not yet been instantiated. The engine drains the
    pending list AFTER the current macrostep settles (defer-execute
    pattern), so onentry handlers always observe a stable configuration
    before any child starts running."""

    invoke_id: str
    owner_state_name: str


@dataclass
class ChildSession:
    """Bookkeeping for an active `<invoke>` session (W3C SCXML 6.4)."""

    invoke_id: str
    autoforward: bool = False
    finalize_data: Any = None


class Invoke(ABC, Generic[E]):
    """W3C SCXML 6.4 abstract invoke.

    Subclasses bridge the parent engine to whatever the invoke target
    actually is — another statechart (`ScxmlInvoke`), an HTTP endpoint
    (γ-4b), or any external transport. Every concrete `Invoke`:
      - starts the target on `start(parent_engine)`,
      - drains target-to-parent events through `drain_events()`,
      - forwards parent-to-target events through `forward_event(...)`,
      - reports completion via `is_done()` and `done_data()`.

    The engine never reaches into the target directly — every interaction
    flows through these five operations, which keeps the parent's
    `advance_time` / macrostep loop agnostic of the invoke kind.
    """

    @abstractmethod
    def start(self, parent_engine) -> None:
        """W3C SCXML 6.4 — instantiate and initialise the target."""

    @abstractmethod
    def tick(self) -> None:
        """W3C SCXML 6.4 — advance the target by one macrostep
        equivalent. Idempotent when the target is already done."""

    @abstractmethod
    def is_done(self) -> bool:
        """True once the target has reached its terminal configuration
        (W3C 3.7 top-level `<final>` for SCXML children)."""

    @abstractmethod
    def drain_events(self) -> Iterable[Tuple[str, Any]]:
        """W3C SCXML 6.4 — yield every `(event_name, data)` the target
        has raised to its parent (e.g. `<send target="#_parent">`)
        since the last drain. The runtime promotes each tuple onto
        the parent's external queue via `Engine.send_external_by_name`."""

    @abstractmethod
    def forward_event(self, event_name: str, data: Any) -> None:
        """W3C SCXML 6.4.1 — deliver an autoforwarded event into the
        target. The descriptor matches the wire name (`done.foo`)
        rather than the parent's `Event` enum, so the implementation
        is responsible for any local name→Event resolution."""

    @abstractmethod
    def cancel(self) -> None:
        """W3C SCXML 6.4 — terminate the target (parent state exit /
        engine shutdown). After this the engine drops the entry from
        `_active_invokes`; subsequent ticks are skipped."""

    def done_data(self) -> Any:
        """W3C SCXML 5.5 + 6.3.1 — payload captured at the target's
        terminal `<final>`. Surfaces on `done.invoke.<id>._event.data`
        in the parent. Default `None` — subclasses with no donedata
        path keep the default."""
        return None


class ScxmlInvoke(Invoke):
    """W3C SCXML 6.4 — in-process child statechart invoke. Wraps a
    pre-instantiated child `Engine` plus a parent-event queue the
    child's generated policy writes into when it executes
    `<send target="#_parent">`.

    The child's main loop is its own `advance_time(0)` macrostep settler
    — `tick()` runs that so any newly-due child schedules drain into
    the child's queues before the parent observes the child's parent-
    bound events. Forwarding and cancellation route through the
    child's `Engine` API directly; no extra metadata is invented."""

    def __init__(self, child_engine, parent_event_queue: List[Tuple[str, Any]]):
        self._child = child_engine
        self._parent_queue = parent_event_queue
        self._started = False

    def start(self, parent_engine) -> None:
        if not self._started:
            self._child.initialize()
            self._started = True

    def tick(self) -> None:
        # Drain due child schedules + run the child's macrostep so any
        # parent-bound events emitted during this iteration land in
        # `_parent_queue` before the parent's drain_events() call.
        if self._child.is_running and not self._child.reached_final:
            self._child.advance_time(0)

    def is_done(self) -> bool:
        return self._child.reached_final

    def drain_events(self) -> Iterable[Tuple[str, Any]]:
        while self._parent_queue:
            yield self._parent_queue.pop(0)

    def forward_event(self, event_name: str, data: Any) -> None:
        # The child policy's name→Event resolver lifts the string onto
        # the child's Event enum. Unknown names silently drop, matching
        # W3C 5.10.1 ("if no transition is enabled the event is lost").
        event = self._child.policy.get_event_from_name(event_name)
        if event is None:
            return
        # W3C SCXML C.1: parent→child delivery rides the SCXML Event
        # I/O Processor, so the child sees `_event.origintype` = SCXML
        # processor URI (test253). Imported lazily to avoid a cycle.
        from .engine import SCXML_EVENT_PROCESSOR_URI
        from .event import EventMetadata
        self._child.send_event(
            event,
            EventMetadata(
                event_type="external",
                data=data,
                origin_type=SCXML_EVENT_PROCESSOR_URI,
            ),
        )

    def cancel(self) -> None:
        # W3C SCXML 6.4.2 — terminate the child. Marking the engine
        # stopped is enough to keep its scheduler from delivering any
        # remaining `<send delay>` entries (advance_time gates on
        # `is_running and not reached_final`).
        self._child.stop()

    def done_data(self) -> Any:
        return getattr(self._child, "done_data", None)

    @property
    def child(self):
        """Access to the underlying child engine — only for the policy
        hooks that need to introspect child config (e.g. autoforward
        resolution). The engine itself reaches the child only through
        the five Invoke methods above."""
        return self._child


def is_platform_event(event_name: str) -> bool:
    """W3C SCXML 6.4.1 — platform events (prefix `#_`) must not be
    autoforwarded. Mirrors `sce.IsPlatformEvent` in Go."""
    return event_name.startswith("#_")


def create_done_invoke_event_name(invoke_id: str) -> str:
    """W3C SCXML 6.3.1 — `done.invoke.<id>` event name."""
    return f"done.invoke.{invoke_id}"
