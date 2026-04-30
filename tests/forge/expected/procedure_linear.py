# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Runtime: none
# Do not edit — regenerate from the source SCXML file.
#
# Event-driven state machine using ProcedureStateMachine.
# Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
# Pure decision trees (no events/sends) execute via Event.NONE transitions.

from enum import IntEnum
from typing import Callable, Optional, Tuple

from sce_forge_runtime.procedure import (
    ProcedureRunResult,
    ProcedureServiceRequest,
    ProcedureServiceResponse,
    ProcedureStateMachine,
)


# ── State and Event enums ────────────────────────────────────────

class State(IntEnum):
    StageA = 0
    StageB = 1
    StageC = 2
    Done = 3


class Event(IntEnum):
    NONE = 0
    ErrorExecution = 1
    Fail = 2
    Ok = 3


_FINAL_STATES = frozenset([State.Done])


# ── Generated procedure state machine ────────────────────────────

class ProcedureLinear(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._value: int = 0

    def set_value(self, value: int) -> None:
        self._value = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.StageA

    def _is_final(self, state: int) -> bool:
        return state in _FINAL_STATES

    def _final_state_name(self, state: int) -> str:
        if state == State.Done:
            return "done"
        return ""

    def _execute_entry_actions(
        self, state: int
    ) -> Tuple[int, str]:
        return (Event.NONE, "")

    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        if state == State.StageA:
            if event == Event.NONE:
                return (State.StageB, 0, False)
        if state == State.StageB:
            if event == Event.NONE:
                return (State.StageC, 0, False)
        if state == State.StageC:
            if event == Event.NONE:
                return (State.Done, 0, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> Optional[int]:
        # Returns None for normal flow; non-None signals that an
        # assign-time check (RFC `claudedocs/rfc-forge-bytes-bounded.md`
        # §3 B4 bytes cap violation) raised an internal event that the
        # shared run_to_completion loop re-pumps through
        # _process_transition.
        return None


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    value: int,
) -> ProcedureRunResult:
    sm = ProcedureLinear()
    sm.set_service_handler(handler)
    sm.set_value(value)
    return sm.run_to_completion()
