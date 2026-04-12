// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.

package procedure_startup_check

import (
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateCheckVoltage = 0
	stateCheckTemp = 1
	stateSuccess = 2
	stateFailVoltage = 3
	stateFailOvertemp = 4
)

const (
	eventNone = 0
	eventFail = 1
	eventOk = 2
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	voltage	float32
	temperature	float32
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, voltage float32, temperature float32) *policy {
	return &policy{
		voltage: voltage,
		temperature: temperature,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateCheckVoltage }
func (p *policy) SetPendingEventData(d string) { p.pendingEventData = d }
func (p *policy) DoneData() map[string]string  { return p.doneData }

func (p *policy) IsFinal(s int) bool {
	switch s {
	case stateSuccess:
		return true
	case stateFailVoltage:
		return true
	case stateFailOvertemp:
		return true
	}
	return false
}

func (p *policy) FinalStateName(s int) string {
	switch s {
	case stateSuccess:
		return "success"
	case stateFailVoltage:
		return "fail_voltage"
	case stateFailOvertemp:
		return "fail_overtemp"
	}
	return ""
}

func (p *policy) ExecuteEntryActions(s int) (int, string) {
	switch s {
	}
	return eventNone, ""
}

func (p *policy) ProcessTransition(s int, ev int) (int, int, bool, bool) {
	switch s {
	case stateCheckVoltage:
		if ev == eventNone {
			if p.voltage >= 11.5 && p.voltage <= 14.5 {
				return stateCheckTemp, 0, false, true
			}
		}
		if ev == eventNone {
			return stateFailVoltage, 1, false, true
		}
	case stateCheckTemp:
		if ev == eventNone {
			if p.temperature < 80.0 {
				return stateSuccess, 0, false, true
			}
		}
		if ev == eventNone {
			return stateFailOvertemp, 1, false, true
		}
	}
	return 0, 0, false, false
}

func (p *policy) ExecuteTransitionActions(source int, trIndex int) {
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, voltage float32, temperature float32) forge.ProcedureRunResult {
	p := newPolicy(handler, voltage, temperature)
	return forge.RunProcedure(p)
}
