// SCE-MAP: crossfile_observer_condition:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package com.sce.generated.crossfile_observer_condition

import com.sce.generated.condition_threshold.*
import com.sce.forge.runtime.EventDomain
import com.sce.forge.runtime.EventQueue
import com.sce.forge.runtime.ThresholdState

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting EventQueue type cannot be composed
// with other observers. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.
enum class ForgeDomainTag {
    RAISE_ALARM,
    CLEAR_ALARM
}

class ForgeDomain : EventDomain<ForgeDomainTag>

class CrossfileObserverCondition {
    private val alarm = ThresholdState()

    fun update(coolantTemp: Double, oilTemp: Double): EventQueue<ForgeDomainTag> {
        val events = EventQueue<ForgeDomainTag>()
        if (alarm.enterIf(conditionThreshold(coolantTemp, oilTemp, 110.0))) {
            events.push(ForgeDomainTag.RAISE_ALARM)
        }
        else if (alarm.leaveIf(conditionThreshold(coolantTemp, oilTemp, 120.0) == false)) {
            events.push(ForgeDomainTag.CLEAR_ALARM)
        }
        return events
    }
}
