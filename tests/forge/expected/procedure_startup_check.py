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
    CheckVoltage = 0
    CheckTemp = 1
    Success = 2
    FailVoltage = 3
    FailOvertemp = 4


class Event(IntEnum):
    NONE = 0
    ErrorExecution = 1
    Fail = 2
    Ok = 3


_FINAL_STATES = frozenset([State.Success, State.FailVoltage, State.FailOvertemp])


# ── Generated procedure state machine ────────────────────────────

class ProcedureStartupCheck(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._voltage: float = 0.0
        self._temperature: float = 0.0

    def set_voltage(self, value: float) -> None:
        self._voltage = value

    def set_temperature(self, value: float) -> None:
        self._temperature = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.CheckVoltage

    def _is_final(self, state: int) -> bool:
        return state in _FINAL_STATES

    def _final_state_name(self, state: int) -> str:
        if state == State.Success:
            return "success"
        if state == State.FailVoltage:
            return "fail_voltage"
        if state == State.FailOvertemp:
            return "fail_overtemp"
        return ""

    def _execute_entry_actions(
        self, state: int
    ) -> Tuple[int, str]:
        return (Event.NONE, "")

    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        if state == State.CheckVoltage:
            if event == Event.NONE:
                if self._voltage >= 11.5 and self._voltage <= 14.5:
                    return (State.CheckTemp, 0, False)
            if event == Event.NONE:
                return (State.FailVoltage, 1, False)
        if state == State.CheckTemp:
            if event == Event.NONE:
                if self._temperature < 80.0:
                    return (State.Success, 0, False)
            if event == Event.NONE:
                return (State.FailOvertemp, 1, False)
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
    voltage: float,
    temperature: float,
) -> ProcedureRunResult:
    sm = ProcedureStartupCheck()
    sm.set_service_handler(handler)
    sm.set_voltage(voltage)
    sm.set_temperature(temperature)
    return sm.run_to_completion()
