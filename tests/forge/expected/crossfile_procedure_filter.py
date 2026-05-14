# SCE-MAP: crossfile_procedure_filter:10

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.
#
# Event-driven state machine using ProcedureStateMachine.
# Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
# Pure decision trees (no events/sends) execute via Event.NONE transitions.

from .filter_low_pass import FilterLowPass
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
    Sample = 0
    Done = 1


class Event(IntEnum):
    NONE = 0
    ErrorExecution = 1
    Fail = 2
    Ok = 3


_FINAL_STATES = frozenset([State.Done])


# ── Generated procedure state machine ────────────────────────────

class CrossfileProcedureFilter(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._raw_sample: float = 0.0
        self._smoothed: float = 0.0
        # Imported kinds (cross-file composition)
        self.smoother: FilterLowPass = FilterLowPass()

    def set_raw_sample(self, value: float) -> None:
        self._raw_sample = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.Sample

    def _is_final(self, state: int) -> bool:
        return state in _FINAL_STATES

    def _final_state_name(self, state: int) -> str:
        if state == State.Done:
            return "done"
        return ""

    def _execute_entry_actions(
        self, state: int
    ) -> Tuple[int, str]:
        if state == State.Done:
            self._done_data["result"] = 'success'
        return (Event.NONE, "")

    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        if state == State.Sample:
            if event == Event.NONE:
                return (State.Done, 0, True)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> Optional[int]:
        # Returns None for normal flow; non-None signals that an
        # assign-time check (RFC `claudedocs/rfc-forge-bytes-bounded.md`
        # §3 B4 bytes cap violation) raised an internal event that the
        # shared run_to_completion loop re-pumps through
        # _process_transition.
        if source == State.Sample:
            if tr_index == 0:
                self._smoothed = self.smoother.update(self._raw_sample)
        return None


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    raw_sample: float,
) -> ProcedureRunResult:
    sm = CrossfileProcedureFilter()
    sm.set_service_handler(handler)
    sm.set_raw_sample(raw_sample)
    return sm.run_to_completion()
