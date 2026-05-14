// SCE-MAP: observer_coolant:1

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.

package observer_coolant

import "github.com/newmassrael/sce-forge-runtime/observer"

// No sce:event-domain declared on this <scxml> root: the observer falls back
// to a file-local domain. The resulting EventQueue type cannot be composed
// with observers in other files. To enable cross-file composition, add
// sce:event-domain="..." to the source SCXML. See SCE_FORGE.md Section 4.11.
type ForgeDomainTag int

const (
	ForgeDomainTagEmitWarning ForgeDomainTag = 0
	ForgeDomainTagClearWarning ForgeDomainTag = 1
	ForgeDomainTagEmergencyShutdown ForgeDomainTag = 2
)

type ObserverCoolant struct {
	warning observer.ThresholdState
	critical observer.ThresholdState
}

func (o *ObserverCoolant) Update(coolantTemp float64) *observer.EventQueue[ForgeDomainTag] {
	events := observer.NewEventQueue[ForgeDomainTag]()
	if o.warning.EnterIf(coolantTemp > 110.0) {
		events.Push(ForgeDomainTagEmitWarning)
	} else if o.warning.LeaveIf(coolantTemp < 100.0) {
		events.Push(ForgeDomainTagClearWarning)
	}
	if o.critical.EnterIf(coolantTemp > 120.0) {
		events.Push(ForgeDomainTagEmergencyShutdown)
	} else {
		o.critical.LeaveIf(coolantTemp < 105.0)
	}
	return events
}
