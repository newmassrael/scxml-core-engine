// SCE Forge: Auto-generated from Extended SCXML (sce:kind="observer")
// Do not edit — regenerate from the source SCXML file.

package observer_coolant

type Event int

const (
	EventEmitWarning Event = 0
	EventClearWarning Event = 1
	EventEmergencyShutdown Event = 2
)

type ObserverCoolant struct {
	warningActive bool
	criticalActive bool
}
func (o *ObserverCoolant) Update(coolantTemp float64) []Event {
	var events []Event
	if !o.warningActive && (coolantTemp > 110.0) {
		o.warningActive = true
		events = append(events, EventEmitWarning)
	} else if o.warningActive && (coolantTemp < 100.0) {
		o.warningActive = false
		events = append(events, EventClearWarning)
	}
	if !o.criticalActive && (coolantTemp > 120.0) {
		o.criticalActive = true
		events = append(events, EventEmergencyShutdown)
	} else if o.criticalActive && (coolantTemp < 105.0) {
		o.criticalActive = false
	}
	return events
}