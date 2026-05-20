// SCE-MAP: crossfile_procedure_codec:3

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   frame.encode()

package crossfile_procedure_codec

import (
	"example.com/sce-forge/codec_simple_frame"
	"fmt"
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateSendRequest = 0
	stateDecode = 1
	stateDone = 2
	stateError = 3
)

const (
	eventNone = 0
	eventErrorExecution = 1
	eventFail = 2
	eventOk = 3
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	ecuAddr	uint32
	response	[]byte
	// Imported kinds (cross-file composition)
	Frame codec_simple_frame.CodecSimpleFrame
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, ecuAddr uint32) *policy {
	return &policy{
		ecuAddr: ecuAddr,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateSendRequest }
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
	case stateSendRequest:
		if p.serviceHandler != nil {
			addrVal := fmt.Sprint(p.ecuAddr)
			req := forge.ProcedureServiceRequest{
				Service: "Diag",
				Addr: &addrVal,
				Payload: p.Frame.EncodeToBytes(),
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
	case stateSendRequest:
		if ev == eventOk {
			return stateDecode, 0, true, true
		}
		if ev == eventFail {
			return stateError, 1, false, true
		}
	case stateDecode:
		if ev == eventNone {
			return stateDone, 0, false, true
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
	if source == stateSendRequest {
		if trIndex == 0 {
			{
				scopeTmp := []byte(p.pendingEventData)
				if len(scopeTmp) > 256 {
					return eventErrorExecution, true
				}
				p.response = scopeTmp
			}
		}
	}
	return 0, false
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, ecuAddr uint32) forge.ProcedureRunResult {
	p := newPolicy(handler, ecuAddr)
	return forge.RunProcedure(p)
}
