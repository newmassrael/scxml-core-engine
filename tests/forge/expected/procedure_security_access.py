# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure", Level 2)
# Do not edit — regenerate from the source SCXML file.
#
# Level 2 procedure: event-driven state machine using ProcedureStateMachine.
# Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
#
# External dependencies (from sce:payload expressions — must be in scope):
#   computeKey(seed)

from enum import IntEnum
from typing import Callable, Optional, Tuple

from sce_forge import (
    ProcedureRunResult,
    ProcedureServiceRequest,
    ProcedureServiceResponse,
    ProcedureStateMachine,
)


# ── State and Event enums ────────────────────────────────────────

class State(IntEnum):
    SendTesterPresent = 0
    RequestSeed = 1
    SendKey = 2
    Retry = 3
    Done = 4
    Error = 5


class Event(IntEnum):
    NONE = 0
    Fail = 1
    Ok = 2


_FINAL_STATES = frozenset([State.Done, State.Error])


# ── Generated procedure state machine ────────────────────────────

class ProcedureSecurityAccess(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._ecu_addr: int = 0
        self._seed: bytes = b""
        self._max_retries: int = 3
        self._retry_count: int = 0

    def set_ecu_addr(self, value: int) -> None:
        self._ecu_addr = value

    def _none_event(self) -> int:
        return Event.NONE

    def _initial_state(self) -> int:
        return State.SendTesterPresent

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
        if state == State.SendTesterPresent:
            if self._service_handler is not None:
                req = ProcedureServiceRequest(
                    service="TesterPresent",
                )
                req.params["addr"] = str(self__ecu_addr)
                resp = self._service_handler(req)
                event = Event.Ok if resp.success else Event.Fail
                return (event, resp.data)
        if state == State.RequestSeed:
            if self._service_handler is not None:
                req = ProcedureServiceRequest(
                    service="SecurityAccess",
                    subfunc="0x01",
                )
                resp = self._service_handler(req)
                event = Event.Ok if resp.success else Event.Fail
                return (event, resp.data)
        if state == State.SendKey:
            if self._service_handler is not None:
                req = ProcedureServiceRequest(
                    service="SecurityAccess",
                    subfunc="0x02",
                )
                req.params["payload"] = str(compute_key(self__seed))
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
        if state == State.SendTesterPresent:
            if event == Event.Ok:
                return (State.RequestSeed, 0, False)
            if event == Event.Fail:
                return (State.Error, 1, False)
        if state == State.RequestSeed:
            if event == Event.Ok:
                return (State.SendKey, 0, True)
            if event == Event.Fail:
                return (State.Retry, 1, False)
        if state == State.SendKey:
            if event == Event.Ok:
                return (State.Done, 0, False)
            if event == Event.Fail:
                return (State.Retry, 1, False)
        if state == State.Retry:
            if event == Event.NONE:
                if self__retry_count < self__max_retries:
                    return (State.RequestSeed, 0, True)
            if event == Event.NONE:
                if self__retry_count >= self__max_retries:
                    return (State.Error, 1, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> None:
        if source == State.RequestSeed:
            if tr_index == 0:
                self._seed = self__pending_event_data.encode()
        if source == State.Retry:
            if tr_index == 0:
                self._retry_count = self__retry_count + 1


# ── Convenience wrapper function ─────────────────────────────────

def execute_procedure_security_access(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    ecu_addr: int,
) -> ProcedureRunResult:
    sm = ProcedureSecurityAccess()
    sm.set_service_handler(handler)
    sm.set_ecu_addr(ecu_addr)
    return sm.run_to_completion()
