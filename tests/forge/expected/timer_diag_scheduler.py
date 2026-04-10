# SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
# Do not edit — regenerate from the source SCXML file.

from typing import Callable, Protocol


class Timer(Protocol):
    def start_periodic(self, interval_ms: int, callback: Callable[[], None]) -> None: ...
    def start_one_shot(self, delay_ms: int, callback: Callable[[], None]) -> None: ...
    def cancel(self) -> None: ...


class TimerDiagScheduler:
    def __init__(self) -> None:
        self.tester_present_timer: Timer | None = None
        self.response_timeout_timer: Timer | None = None
        self.retry_delay_timer: Timer | None = None

    def start_tester_present(self) -> None:
        if self.tester_present_timer is not None:
            self.tester_present_timer.start_periodic(2000, self._on_tester_present)

    def cancel_tester_present(self) -> None:
        if self.tester_present_timer is not None:
            self.tester_present_timer.cancel()

    def start_response_timeout(self) -> None:
        if self.response_timeout_timer is not None:
            self.response_timeout_timer.start_one_shot(5000, self._on_handle_timeout)

    def cancel_response_timeout(self) -> None:
        if self.response_timeout_timer is not None:
            self.response_timeout_timer.cancel()

    def start_retry_delay(self) -> None:
        if self.retry_delay_timer is not None:
            self.retry_delay_timer.start_one_shot(10000, self._on_retry_security_access)

    def cancel_retry_delay(self) -> None:
        if self.retry_delay_timer is not None:
            self.retry_delay_timer.cancel()

    def _on_tester_present(self) -> None:
        pass  # platform callback

    def _on_handle_timeout(self) -> None:
        pass  # platform callback

    def _on_retry_security_access(self) -> None:
        pass  # platform callback
