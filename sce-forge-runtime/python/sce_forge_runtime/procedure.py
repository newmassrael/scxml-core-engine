# SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
# SCE Forge: Procedure types and execution engine for Level 2 procedures.
#
# Generated code extends ProcedureStateMachine and implements the abstract
# methods. The event loop lives here; generated code provides only the policy.

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Callable, Dict, Optional, Tuple


# ── Service types ────────────────────────────────────────────────

@dataclass
class ProcedureServiceRequest:
    """Request sent to a service handler during procedure execution.

    Each field maps 1:1 to a ``<send>`` attribute in the SCXML source::

        <send sce:service="Diag" sce:subfunc="0x02"
              sce:addr="ecuAddr" sce:payload="frame.encode()"/>

    ``service`` is always present; the other three are ``Optional`` so
    absent attributes are distinguishable from empty values. ``payload``
    is typed as raw ``bytes`` because its semantic role is a wire-format
    data blob originating from codec ``encode()`` calls. ``subfunc`` and
    ``addr`` remain textual since the user may reference datamodel
    variables of any SCE type.
    """
    service: str = ""
    subfunc: Optional[str] = None
    addr: Optional[str] = None
    payload: Optional[bytes] = None


@dataclass
class ProcedureServiceResponse:
    """Response received from a service handler."""
    success: bool = False
    data: str = ""


@dataclass
class ProcedureRunResult:
    """Result of running a procedure to completion."""
    completed: bool = False
    final_state: str = ""
    done_data: Dict[str, str] = field(default_factory=dict)


# Safety limit for the event loop — prevents infinite loops from misconfigured procedures.
_MAX_ITERATIONS = 1000


# ── Procedure state machine base class ───────────────────────────

class ProcedureStateMachine(ABC):
    """Abstract base class for Level 2 (event-driven) procedure state machines.

    Generated code extends this class and implements the abstract methods
    that define the state machine's behavior. This class provides:
      - Service handler management
      - Event-driven execution loop (run_to_completion)
      - Done data and pending event data storage
    """

    def __init__(self) -> None:
        self._service_handler: Optional[
            Callable[[ProcedureServiceRequest], ProcedureServiceResponse]
        ] = None
        self._done_data: Dict[str, str] = {}
        self._pending_event_data: str = ""

    def set_service_handler(
        self,
        handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    ) -> None:
        """Set the service handler for <send sce:service> actions."""
        self._service_handler = handler

    def run_to_completion(self) -> ProcedureRunResult:
        """Run the procedure to completion (blocking).

        Drives the state machine from the initial state through service sends
        until a <final> state is reached or no transition is possible.
        """
        none_event = self._none_event()
        current = self._initial_state()
        event = none_event

        event, event_data = self._execute_entry_actions(current)
        if event_data:
            self._pending_event_data = event_data

        for _ in range(_MAX_ITERATIONS):
            if self._is_final(current):
                break
            transition = self._process_transition(current, event)
            if transition is None:
                break
            next_state, tr_index, has_assigns = transition
            if has_assigns:
                self._execute_transition_actions(current, tr_index)
            current = next_state
            event = none_event
            event, event_data = self._execute_entry_actions(current)
            if event_data:
                self._pending_event_data = event_data

        completed = self._is_final(current)
        return ProcedureRunResult(
            completed=completed,
            final_state=self._final_state_name(current) if completed else "",
            done_data=dict(self._done_data) if completed else {},
        )

    # ── Abstract policy methods (implemented by generated code) ──

    @abstractmethod
    def _none_event(self) -> int:
        """The event value representing 'no event'."""
        ...

    @abstractmethod
    def _initial_state(self) -> int:
        """Initial state of the procedure."""
        ...

    @abstractmethod
    def _is_final(self, state: int) -> bool:
        """Whether the given state is a <final> state."""
        ...

    @abstractmethod
    def _final_state_name(self, state: int) -> str:
        """SCXML id of the final state (e.g., 'done', 'error')."""
        ...

    @abstractmethod
    def _execute_entry_actions(self, state: int) -> Tuple[int, str]:
        """Execute entry actions for a state; returns (event, eventData)."""
        ...

    @abstractmethod
    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        """Process a transition; returns (nextState, trIndex, hasAssigns) or None."""
        ...

    @abstractmethod
    def _execute_transition_actions(self, source: int, tr_index: int) -> None:
        """Execute <assign> actions for a transition."""
        ...
