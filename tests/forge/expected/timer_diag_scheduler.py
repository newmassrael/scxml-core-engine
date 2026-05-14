# SCE-MAP: timer_diag_scheduler:1

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="timer")
# Shape: watching-zenoh RFC §5.D line 880-886 — single timer per
# doc with event-driven reset / state-exit cancel / fire event.
# Runtime: sce_forge_runtime::hal
# Do not edit — regenerate from the source SCXML file.

from abc import ABC, abstractmethod

from sce_forge_runtime.timer import Timer


# Period configured at compile time from `<sce:period>`. Microseconds
# (int) cover MCU microsecond ticks through minute-scale watchdogs in
# one type.
PERIOD_US: int = 2000000
PERIOD_MS: int = 2000
RESET_ON_EVENT: str = "diag.heartbeat"
CANCEL_ON_STATE_EXIT: str = "diag.idle"


class TimerDiagSchedulerHandler(ABC):
    """Handler interface for [`TimerDiagScheduler`]. The user implements the
    fire method on a state object and passes the instance to
    `TimerDiagScheduler.__init__`.
    """

    @abstractmethod
    def fire_diag_tick(self) -> None: ...


class TimerDiagScheduler:
    def __init__(
        self,
        handler: TimerDiagSchedulerHandler,
        timer: Timer,
    ) -> None:
        self._handler = handler
        self._timer = timer

    def start(self) -> None:
        """Start the periodic timer at compile-time PERIOD_MS."""
        self._timer.start_periodic(PERIOD_MS, self._handler.fire_diag_tick)

    def cancel(self) -> None:
        """Cancel the timer. Idempotent per the runtime contract."""
        self._timer.cancel()

    def on_reset_diag_heartbeat(self) -> None:
        """`<sce:reset-on event="diag.heartbeat"/>` consumer hook."""
        self.cancel()
        self.start()

    def on_cancel_diag_idle_exit(self) -> None:
        """`<sce:cancel-on state-exit="diag.idle"/>` consumer hook."""
        self.cancel()
