// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   frame.encode()

package crossfile_procedure_codec_mutate

import (
	"example.com/sce-forge/codec_simple_frame"
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateInit = 0
	stateSend = 1
	stateDone = 2
	stateError = 3
)

const (
	eventNone = 0
	eventFail = 1
	eventOk = 2
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	msgId	uint8
	// Imported kinds (cross-file composition)
	Frame codec_simple_frame.CodecSimpleFrame
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, msgId uint8) *policy {
	return &policy{
		msgId: msgId,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateInit }
func (p *policy) SetPendingEventData(d string) { p.pendingEventData = d }
func (p *policy) DoneData() map[string]string  { return p.doneData }

func (p *policy) IsFinal(s int) bool {
	switch s {
	case stateDone:
		return true
	case stateError:
		return true
	}
	return false
}

func (p *policy) FinalStateName(s int) string {
	switch s {
	case stateDone:
		return "done"
	case stateError:
		return "error"
	}
	return ""
}

func (p *policy) ExecuteEntryActions(s int) (int, string) {
	switch s {
	case stateSend:
		if p.serviceHandler != nil {
			req := forge.ProcedureServiceRequest{
				Service: "transport",
				Payload: p.Frame.Encode(),
			}
			resp := p.serviceHandler(req)
			if resp.Success {
				return eventOk, resp.Data
			}
			return eventFail, resp.Data
		}
	case stateDone:
		p.doneData["result"] = "success"
	case stateError:
		p.doneData["result"] = "failure"
	}
	return eventNone, ""
}

func (p *policy) ProcessTransition(s int, ev int) (int, int, bool, bool) {
	switch s {
	case stateInit:
		if ev == eventNone {
			return stateSend, 0, true, true
		}
	case stateSend:
		if ev == eventOk {
			return stateDone, 0, false, true
		}
		if ev == eventFail {
			return stateError, 1, false, true
		}
	}
	return 0, 0, false, false
}

func (p *policy) ExecuteTransitionActions(source int, trIndex int) {
	if source == stateInit {
		if trIndex == 0 {
			p.Frame.MsgId = p.msgId
		}
	}
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, msgId uint8) forge.ProcedureRunResult {
	p := newPolicy(handler, msgId)
	return forge.RunProcedure(p)
}
