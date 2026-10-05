# SCE-MAP: crossfile_observer_condition:4 :: _forge_body

# SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
# Runtime: sce_forge_runtime
# Do not edit — regenerate from the source SCXML file.

from enum import Enum

from . import condition_threshold as sce_condition_threshold
from sce_forge_runtime.observer import EventDomain, EventQueue, ThresholdState

# No sce:event-domain declared on this <scxml> root: the observer falls back
# to a file-local domain. The resulting EventQueue type cannot be composed
# with other observers. To enable cross-file composition, add
# sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.


class ForgeDomainTag(Enum):
    RAISE_ALARM = "RAISE_ALARM"
    CLEAR_ALARM = "CLEAR_ALARM"


class ForgeDomain(EventDomain[ForgeDomainTag]):
    pass


class CrossfileObserverCondition:
    def __init__(self) -> None:
        self._alarm = ThresholdState()

    def update(self, coolant_temp: float, oil_temp: float) -> EventQueue[ForgeDomainTag]:
        _events: EventQueue[ForgeDomainTag] = EventQueue()
        if self._alarm.enter_if(sce_condition_threshold.condition_threshold(coolant_temp, oil_temp, 110.0)):
            _events.push(ForgeDomainTag.RAISE_ALARM)
        elif self._alarm.leave_if(sce_condition_threshold.condition_threshold(coolant_temp, oil_temp, 120.0) == False):
            _events.push(ForgeDomainTag.CLEAR_ALARM)
        return _events
