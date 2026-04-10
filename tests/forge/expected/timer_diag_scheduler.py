# SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
# Do not edit — regenerate from the source SCXML file.

from abc import ABC, abstractmethod

from sce_forge_runtime.timer import Timer


class TimerDiagSchedulerHandler(ABC):
    """Handler interface for [`TimerDiagScheduler`]. The user implements these
    methods on a state object and passes the instance to `TimerDiagScheduler.__init__`.
    Missing methods raise `TypeError` at handler instantiation time, so there
    is no silent fallback.
    """

    @abstractmethod
    def fire_tester_present(self) -> None: ...
    @abstractmethod
    def fire_handle_timeout(self) -> None: ...
    @abstractmethod
    def fire_retry_security_access(self) -> None: ...


class TimerDiagScheduler:
    def __init__(
        self,
        handler: TimerDiagSchedulerHandler,
        tester_present_timer: Timer,
        response_timeout_timer: Timer,
        retry_delay_timer: Timer,
    ) -> None:
        self._handler = handler
        self._tester_present_timer = tester_present_timer
        self._response_timeout_timer = response_timeout_timer
        self._retry_delay_timer = retry_delay_timer

    def start_tester_present(self) -> None:
        self._tester_present_timer.start_periodic(2000, self._handler.fire_tester_present)

    def cancel_tester_present(self) -> None:
        self._tester_present_timer.cancel()

    def start_response_timeout(self) -> None:
        self._response_timeout_timer.start_one_shot(5000, self._handler.fire_handle_timeout)

    def cancel_response_timeout(self) -> None:
        self._response_timeout_timer.cancel()

    def start_retry_delay(self) -> None:
        self._retry_delay_timer.start_one_shot(10000, self._handler.fire_retry_security_access)

    def cancel_retry_delay(self) -> None:
        self._retry_delay_timer.cancel()
