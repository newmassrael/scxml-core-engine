# SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
# Do not edit — regenerate from the source SCXML file.

from enum import Enum

from sce_forge_runtime.observer import EventDomain, EventQueue, ThresholdState

# No sce:event-domain declared on this <scxml> root: the observer falls back
# to a file-local domain. The resulting EventQueue type cannot be composed
# with other observers. To enable cross-file composition, add
# sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.


class ForgeDomainTag(Enum):
    EMIT_WARNING = "EMIT_WARNING"
    CLEAR_WARNING = "CLEAR_WARNING"
    EMERGENCY_SHUTDOWN = "EMERGENCY_SHUTDOWN"


class ForgeDomain(EventDomain[ForgeDomainTag]):
    pass


class ObserverCoolant:
    def __init__(self) -> None:
        self._warning = ThresholdState()
        self._critical = ThresholdState()

    def update(self, coolant_temp: float) -> EventQueue[ForgeDomainTag]:
        events: EventQueue[ForgeDomainTag] = EventQueue()
        if self._warning.enter_if(coolant_temp > 110.0):
            events.push(ForgeDomainTag.EMIT_WARNING)
        elif self._warning.leave_if(coolant_temp < 100.0):
            events.push(ForgeDomainTag.CLEAR_WARNING)
        if self._critical.enter_if(coolant_temp > 120.0):
            events.push(ForgeDomainTag.EMERGENCY_SHUTDOWN)
        else:
            self._critical.leave_if(coolant_temp < 105.0)
        return events
