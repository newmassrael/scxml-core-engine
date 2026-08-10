// SCE-MAP: crossfile_procedure_filter:10 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.

package crossfile_procedure_filter

import (
	"example.com/sce-forge/filter_low_pass"
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateSample = 0
	stateDone = 1
)

const (
	eventNone = 0
	eventErrorExecution = 1
	eventFail = 2
	eventOk = 3
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	rawSample	float64
	smoothed	float64
	// Imported kinds (cross-file composition)
	Smoother filter_low_pass.FilterLowPass
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, rawSample float64) *policy {
	return &policy{
		rawSample: rawSample,
		Smoother: *filter_low_pass.NewFilterLowPass(),
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateSample }
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
	case stateDone:
		p.doneData["result"] = "success"
	}
	return eventNone, ""
}

func (p *policy) ProcessTransition(s int, ev int) (int, int, bool, bool) {
	switch s {
	case stateSample:
		if ev == eventNone {
			return stateDone, 0, true, true
		}
	}
	return 0, 0, false, false
}

// Returns (raised, ok). When ok is true the loop re-processes the
// source state with the raised event (a bytes-cap violation raises
// eventErrorExecution); ok=false signals normal flow.
func (p *policy) ExecuteTransitionActions(source int, trIndex int) (int, bool) {
	_ = source
	_ = trIndex
	if source == stateSample {
		if trIndex == 0 {
			p.smoothed = p.Smoother.Update(p.rawSample)
		}
	}
	return 0, false
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, rawSample float64) forge.ProcedureRunResult {
	p := newPolicy(handler, rawSample)
	return forge.RunProcedure(p)
}
