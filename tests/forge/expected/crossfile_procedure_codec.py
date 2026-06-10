# SCE-MAP: crossfile_procedure_codec:3

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.
#
# Event-driven state machine using ProcedureStateMachine.
# Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
# Pure decision trees (no events/sends) execute via Event.NONE transitions.
#
# External dependencies (from sce:payload expressions — must be in scope):
#   frame.encode()

from .codec_simple_frame import CodecSimpleFrame
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
    SendRequest = 0
    Decode = 1
    Done = 2
    Error = 3


class Event(IntEnum):
    NONE = 0
    ErrorExecution = 1
    Fail = 2
    Ok = 3


_FINAL_STATES = frozenset([State.Done, State.Error])


# ── Generated procedure state machine ────────────────────────────

class CrossfileProcedureCodec(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._ecu_addr: int = 0
        self._response: bytes = b""
        # Imported kinds (cross-file composition)
        self.frame: CodecSimpleFrame = CodecSimpleFrame()

    def set_ecu_addr(self, value: int) -> None:
        self._ecu_addr = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.SendRequest

    def _is_final(self, state: int) -> bool:
        return state in _FINAL_STATES

    def _final_state_name(self, state: int) -> str:
        if state == State.Done:
            return "done"
        if state == State.Error:
            return "error"
        return ""

    def _execute_entry_actions(
        self, state: int
    ) -> Tuple[int, str]:
        if state == State.SendRequest:
            if self._service_handler is not None:
                req = ProcedureServiceRequest(
                    service="Diag",
                    addr=str(self._ecu_addr),
                    payload=self.frame.encode_to_bytes(),
                )
                resp = self._service_handler(req)
                event = Event.Ok if resp.success else Event.Fail
                return (event, resp.data)
        if state == State.Done:
            self._done_data["result"] = 'success'
        if state == State.Error:
            self._done_data["result"] = 'failure'
        return (Event.NONE, "")

    def _process_transition(
        self, state: int, event: int
    ) -> Optional[Tuple[int, int, bool]]:
        if state == State.SendRequest:
            if event == Event.Ok:
                return (State.Decode, 0, True)
            if event == Event.Fail:
                return (State.Error, 1, False)
        if state == State.Decode:
            if event == Event.NONE:
                return (State.Done, 0, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> Optional[int]:
        # Returns None for normal flow; non-None signals that an
        # assign-time bytes-cap check raised an internal event that the
        # shared run_to_completion loop re-pumps through
        # _process_transition.
        if source == State.SendRequest:
            if tr_index == 0:
                _scope_tmp = self._pending_event_data.encode()
                if len(_scope_tmp) > 256:
                    return Event.ErrorExecution
                self._response = _scope_tmp
        return None


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    ecu_addr: int,
) -> ProcedureRunResult:
    sm = CrossfileProcedureCodec()
    sm.set_service_handler(handler)
    sm.set_ecu_addr(ecu_addr)
    return sm.run_to_completion()
