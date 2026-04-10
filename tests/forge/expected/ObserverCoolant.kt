// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Do not edit — regenerate from the source SCXML file.

enum class Event {
    EMIT_WARNING,
    CLEAR_WARNING,
    EMERGENCY_SHUTDOWN
}

class ObserverCoolant {
    private var warningActive = false
    private var criticalActive = false

    fun update(coolantTemp: Double): List<Event> {
        val events = mutableListOf<Event>()
        if (!warningActive && (coolantTemp > 110.0)) {
            warningActive = true
            events.add(Event.EMIT_WARNING)
        } else if (warningActive && (coolantTemp < 100.0)) {
            warningActive = false
            events.add(Event.CLEAR_WARNING)
        }
        if (!criticalActive && (coolantTemp > 120.0)) {
            criticalActive = true
            events.add(Event.EMERGENCY_SHUTDOWN)
        } else if (criticalActive && (coolantTemp < 105.0)) {
            criticalActive = false
        }
        return events
    }
}