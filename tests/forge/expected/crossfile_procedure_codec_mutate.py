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
    Init = 0
    Send = 1
    Done = 2
    Error = 3


class Event(IntEnum):
    NONE = 0
    Fail = 1
    Ok = 2


_FINAL_STATES = frozenset([State.Done, State.Error])


# ── Generated procedure state machine ────────────────────────────

class CrossfileProcedureCodecMutate(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._msg_id: int = 0
        # Imported kinds (cross-file composition)
        self.frame: CodecSimpleFrame = CodecSimpleFrame()

    def set_msg_id(self, value: int) -> None:
        self._msg_id = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.Init

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
        if state == State.Send:
            if self._service_handler is not None:
                req = ProcedureServiceRequest(
                    service="transport",
                    payload=self.frame.encode(),
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
        if state == State.Init:
            if event == Event.NONE:
                return (State.Send, 0, True)
        if state == State.Send:
            if event == Event.Ok:
                return (State.Done, 0, False)
            if event == Event.Fail:
                return (State.Error, 1, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> None:
        if source == State.Init:
            if tr_index == 0:
                self.frame.msg_id = self._msg_id


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    msg_id: int,
) -> ProcedureRunResult:
    sm = CrossfileProcedureCodecMutate()
    sm.set_service_handler(handler)
    sm.set_msg_id(msg_id)
    return sm.run_to_completion()
