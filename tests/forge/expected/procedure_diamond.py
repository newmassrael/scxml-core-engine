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
    Classify = 0
    HighPath = 1
    MidPath = 2
    LowPath = 3
    Accept = 4
    Reject = 5


class Event(IntEnum):
    NONE = 0
    Fail = 1
    Ok = 2


_FINAL_STATES = frozenset([State.Accept, State.Reject])


# ── Generated procedure state machine ────────────────────────────

class ProcedureDiamond(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._sensor_value: int = 0
        self._mode: str = ""

    def set_sensor_value(self, value: int) -> None:
        self._sensor_value = value

    def set_mode(self, value: str) -> None:
        self._mode = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.Classify

    def _is_final(self, state: int) -> bool:
        return state in _FINAL_STATES

    def _final_state_name(self, state: int) -> str:
        if state == State.Accept:
            return "accept"
        if state == State.Reject:
            return "reject"
        return ""

    def _execute_entry_actions(
        self, state: int
    ) -> Tuple[int, str]:
        return (Event.NONE, "")

    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        if state == State.Classify:
            if event == Event.NONE:
                if self._sensor_value > 1000:
                    return (State.HighPath, 0, False)
            if event == Event.NONE:
                if self._sensor_value > 500:
                    return (State.MidPath, 1, False)
            if event == Event.NONE:
                return (State.LowPath, 2, False)
        if state == State.HighPath:
            if event == Event.NONE:
                if self._mode == 'strict':
                    return (State.Reject, 0, False)
            if event == Event.NONE:
                return (State.Accept, 1, False)
        if state == State.MidPath:
            if event == Event.NONE:
                return (State.Accept, 0, False)
        if state == State.LowPath:
            if event == Event.NONE:
                return (State.Accept, 0, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> None:
        pass


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    sensor_value: int,
    mode: str,
) -> ProcedureRunResult:
    sm = ProcedureDiamond()
    sm.set_service_handler(handler)
    sm.set_sensor_value(sensor_value)
    sm.set_mode(mode)
    return sm.run_to_completion()
