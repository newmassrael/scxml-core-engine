// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.observer_coolant

import com.sce.forge.runtime.EventDomain
import com.sce.forge.runtime.EventQueue
import com.sce.forge.runtime.ThresholdState

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting EventQueue type cannot be composed
// with other observers. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.
// SCXML event names flow into enum entries verbatim (W3C SCXML 3.12);
// `EMIT_WARNING`, `coolant.high`, etc. don't conform to Kotlin's
// UpperCamel convention but renaming would break SCE_FORGE.md §4.11
// cross-file event composition.
@Suppress("EnumEntryName")
enum class ForgeDomainTag {
    EMIT_WARNING,
    CLEAR_WARNING,
    EMERGENCY_SHUTDOWN
}

class ForgeDomain : EventDomain<ForgeDomainTag>

class ObserverCoolant {
    private val warning = ThresholdState()
    private val critical = ThresholdState()

    fun update(coolantTemp: Double): EventQueue<ForgeDomainTag> {
        val events = EventQueue<ForgeDomainTag>()
        if (warning.enterIf(coolantTemp > 110.0)) {
            events.push(ForgeDomainTag.EMIT_WARNING)
        }
        else if (warning.leaveIf(coolantTemp < 100.0)) {
            events.push(ForgeDomainTag.CLEAR_WARNING)
        }
        if (critical.enterIf(coolantTemp > 120.0)) {
            events.push(ForgeDomainTag.EMERGENCY_SHUTDOWN)
        }
        else {
            critical.leaveIf(coolantTemp < 105.0)
        }
        return events
    }
}
