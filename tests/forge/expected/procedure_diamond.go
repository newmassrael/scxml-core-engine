// SCE-MAP: procedure_diamond:2

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.

package procedure_diamond

import (
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateClassify = 0
	stateHighPath = 1
	stateMidPath = 2
	stateLowPath = 3
	stateAccept = 4
	stateReject = 5
)

const (
	eventNone = 0
	eventErrorExecution = 1
	eventFail = 2
	eventOk = 3
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	sensorValue	uint16
	mode	string
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, sensorValue uint16, mode string) *policy {
	return &policy{
		sensorValue: sensorValue,
		mode: mode,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateClassify }
func (p *policy) SetPendingEventData(d string) { p.pendingEventData = d }
func (p *policy) DoneData() map[string]string  { return p.doneData }

func (p *policy) IsFinal(s int) bool {
	switch s {
	case stateAccept:
		return true
	case stateReject:
		return true
	}
	return false
}

func (p *policy) FinalStateName(s int) string {
	switch s {
	case stateAccept:
		return "accept"
	case stateReject:
		return "reject"
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
	case stateClassify:
		if ev == eventNone {
			if p.sensorValue > 1000 {
				return stateHighPath, 0, false, true
			}
		}
		if ev == eventNone {
			if p.sensorValue > 500 {
				return stateMidPath, 1, false, true
			}
		}
		if ev == eventNone {
			return stateLowPath, 2, false, true
		}
	case stateHighPath:
		if ev == eventNone {
			if p.mode == "strict" {
				return stateReject, 0, false, true
			}
		}
		if ev == eventNone {
			return stateAccept, 1, false, true
		}
	case stateMidPath:
		if ev == eventNone {
			return stateAccept, 0, false, true
		}
	case stateLowPath:
		if ev == eventNone {
			return stateAccept, 0, false, true
		}
	}
	return 0, 0, false, false
}

// Returns (raised, ok). When ok is true the loop re-processes the
// source state with the raised event (RFC
// `claudedocs/rfc-forge-bytes-bounded.md` §3 B4 bytes cap violation
// raises eventErrorExecution); ok=false signals normal flow.
func (p *policy) ExecuteTransitionActions(source int, trIndex int) (int, bool) {
	_ = source
	_ = trIndex
	return 0, false
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, sensorValue uint16, mode string) forge.ProcedureRunResult {
	p := newPolicy(handler, sensorValue, mode)
	return forge.RunProcedure(p)
}
