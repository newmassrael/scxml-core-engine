# SCE-MAP: procedure_security_access:1 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.
#
# Event-driven state machine using ProcedureStateMachine.
# Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
# Pure decision trees (no events/sends) execute via Event.NONE transitions.
#
# External dependencies (from sce:payload expressions — must be in scope):
#   computeKey(seed)

from enum import IntEnum
from typing import Callable, Optional, Tuple

from sce_forge_runtime.procedure import (
    ProcedureRunResult,
    ProcedureServiceRequest,
    ProcedureServiceResponse,
    ProcedureStateMachine,
)


# ── <sce:helper> DI fail-fast factory ───────────────────────────
#
# Python lambdas cannot contain a raise statement, so declared helper
# closures default to a nested function that raises RuntimeError with the
# helper name and setter method name baked in. Matches the Rust / C++ / Go
# / Kotlin fail-fast defaults.

def _unset_helper_raiser(helper_name, setter_name):
    def _raiser(*_args, **_kwargs):
        raise RuntimeError(
            f"helper {helper_name!r} not set — call {setter_name}() before run_to_completion()"
        )
    return _raiser


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
    ErrorExecution = 1
    Fail = 2
    Ok = 3


_FINAL_STATES = frozenset([State.Done, State.Error])


# ── Generated procedure state machine ────────────────────────────

class ProcedureSecurityAccess(ProcedureStateMachine):

    def __init__(self) -> None:
        super().__init__()
        self._ecu_addr: int = 0
        self._seed: bytes = b""
        self._max_retries: int = 3
        self._retry_count: int = 0
        # <sce:helper> DI closures — initialise to fail-fast sentinels
        # produced by _unset_helper_raiser (module-level factory above) that
        # raise RuntimeError with the helper name + setter name baked in.
        # Matches the Rust / C++ / Go / Kotlin fail-fast rationale.
        self._compute_key: Callable[[bytes], bytes] = _unset_helper_raiser("computeKey", "set_compute_key")

    def set_compute_key(self, fn: Callable[[bytes], bytes]) -> None:
        self._compute_key = fn

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
                    addr=str(self._ecu_addr),
                )
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
                    payload=self._compute_key(self._seed),
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
                if self._retry_count < self._max_retries:
                    return (State.RequestSeed, 0, True)
            if event == Event.NONE:
                if self._retry_count >= self._max_retries:
                    return (State.Error, 1, False)
        return None

    def _execute_transition_actions(
        self, source: int, tr_index: int
    ) -> Optional[int]:
        # Returns None for normal flow; non-None signals that an
        # assign-time bytes-cap check raised an internal event that the
        # shared run_to_completion loop re-pumps through
        # _process_transition.
        if source == State.RequestSeed:
            if tr_index == 0:
                _scope_tmp = self._pending_event_data.encode()
                if len(_scope_tmp) > 64:
                    return Event.ErrorExecution
                self._seed = _scope_tmp
        if source == State.Retry:
            if tr_index == 0:
                self._retry_count = self._retry_count + 1
        return None


# ── Convenience wrapper function ─────────────────────────────────

def execute(
    handler: Callable[[ProcedureServiceRequest], ProcedureServiceResponse],
    compute_key: Callable[[bytes], bytes],
    ecu_addr: int,
) -> ProcedureRunResult:
    sm = ProcedureSecurityAccess()
    sm.set_service_handler(handler)
    sm.set_compute_key(compute_key)
    sm.set_ecu_addr(ecu_addr)
    return sm.run_to_completion()
