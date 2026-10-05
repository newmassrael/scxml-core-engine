// SCE-MAP: crossfile_observer_condition:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package crossfile_observer_condition

import (
	sce_condition_threshold "example.com/sce-forge/condition_threshold"
	"github.com/newmassrael/sce-forge-runtime/observer"
)

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting EventQueue type cannot be composed
// with observers in other files. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.
type ForgeDomainTag int

const (
	ForgeDomainTagRaiseAlarm ForgeDomainTag = 0
	ForgeDomainTagClearAlarm ForgeDomainTag = 1
)

type CrossfileObserverCondition struct {
	alarm observer.ThresholdState
}

func (sceSelf *CrossfileObserverCondition) Update(coolantTemp float64, oilTemp float64) *observer.EventQueue[ForgeDomainTag] {
	sceEvents := observer.NewEventQueue[ForgeDomainTag]()
	if sceSelf.alarm.EnterIf(sce_condition_threshold.ConditionThreshold(coolantTemp, oilTemp, 110.0)) {
		sceEvents.Push(ForgeDomainTagRaiseAlarm)
	} else if sceSelf.alarm.LeaveIf(sce_condition_threshold.ConditionThreshold(coolantTemp, oilTemp, 120.0) == false) {
		sceEvents.Push(ForgeDomainTagClearAlarm)
	}
	return sceEvents
}
