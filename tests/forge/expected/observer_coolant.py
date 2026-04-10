# SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
# Do not edit — regenerate from the source SCXML file.

from enum import Enum, auto


class Event(Enum):
    EMIT_WARNING = auto()
    CLEAR_WARNING = auto()
    EMERGENCY_SHUTDOWN = auto()


class ObserverCoolant:
    def __init__(self) -> None:
        self._warning_active: bool = False
        self._critical_active: bool = False

    def update(self, coolant_temp: float) -> list[Event]:
        events: list[Event] = []
        if not self._warning_active and (coolant_temp > 110.0):
            self._warning_active = True
            events.append(Event.EMIT_WARNING)
        elif self._warning_active and (coolant_temp < 100.0):
            self._warning_active = False
            events.append(Event.CLEAR_WARNING)
        if not self._critical_active and (coolant_temp > 120.0):
            self._critical_active = True
            events.append(Event.EMERGENCY_SHUTDOWN)
        elif self._critical_active and (coolant_temp < 105.0):
            self._critical_active = False
        return events