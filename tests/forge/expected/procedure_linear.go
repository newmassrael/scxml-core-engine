// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.

package procedure_linear

import (
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateStageA = 0
	stateStageB = 1
	stateStageC = 2
	stateDone = 3
)

const (
	eventNone = 0
	eventFail = 1
	eventOk = 2
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	value	int32
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, value int32) *policy {
	return &policy{
		value: value,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateStageA }
func (p *policy) SetPendingEventData(d string) { p.pendingEventData = d }
func (p *policy) DoneData() map[string]string  { return p.doneData }

func (p *policy) IsFinal(s int) bool {
	switch s {
	case stateDone:
		return true
	}
	return false
}

func (p *policy) FinalStateName(s int) string {
	switch s {
	case stateDone:
		return "done"
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
	case stateStageA:
		if ev == eventNone {
			return stateStageB, 0, false, true
		}
	case stateStageB:
		if ev == eventNone {
			return stateStageC, 0, false, true
		}
	case stateStageC:
		if ev == eventNone {
			return stateDone, 0, false, true
		}
	}
	return 0, 0, false, false
}

func (p *policy) ExecuteTransitionActions(source int, trIndex int) {
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, value int32) forge.ProcedureRunResult {
	p := newPolicy(handler, value)
	return forge.RunProcedure(p)
}
