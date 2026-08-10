// SCE-MAP: procedure_security_access:1 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure")
// Runtime: sce_forge_runtime
// Do not edit — regenerate from the source SCXML file.
//
// Event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
// Pure decision trees (no events/sends) execute via eventNone transitions.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   computeKey(seed)

package procedure_security_access

import (
	"fmt"
	"github.com/newmassrael/sce-forge-runtime/forge"
)

// ── State and Event constants ───────────────────────────────────

const (
	stateSendTesterPresent = 0
	stateRequestSeed = 1
	stateSendKey = 2
	stateRetry = 3
	stateDone = 4
	stateError = 5
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
	seed	[]byte
	maxRetries	int32
	retryCount	int32
	// <sce:helper> DI closures
	computeKey func([]byte) []byte
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, computeKey func([]byte) []byte, ecuAddr uint32) *policy {
	if computeKey == nil {
		computeKey = func(_arg0 []byte) []byte { panic("helper 'computeKey' passed nil to Execute — pass a non-nil func([]byte) []byte argument") }
	}
	return &policy{
		ecuAddr: ecuAddr,
		maxRetries: 3,
		retryCount: 0,
		computeKey: computeKey,
		serviceHandler: handler,
		doneData:       make(map[string]string),
	}
}

func (p *policy) NoneEvent() int             { return eventNone }
func (p *policy) InitialState() int          { return stateSendTesterPresent }
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
	case stateSendTesterPresent:
		if p.serviceHandler != nil {
			addrVal := fmt.Sprint(p.ecuAddr)
			req := forge.ProcedureServiceRequest{
				Service: "TesterPresent",
				Addr: &addrVal,
			}
			resp := p.serviceHandler(req)
			if resp.Success {
				return eventOk, resp.Data
			}
			return eventFail, resp.Data
		}
	case stateRequestSeed:
		if p.serviceHandler != nil {
			subfuncVal := "0x01"
			req := forge.ProcedureServiceRequest{
				Service: "SecurityAccess",
				Subfunc: &subfuncVal,
			}
			resp := p.serviceHandler(req)
			if resp.Success {
				return eventOk, resp.Data
			}
			return eventFail, resp.Data
		}
	case stateSendKey:
		if p.serviceHandler != nil {
			subfuncVal := "0x02"
			req := forge.ProcedureServiceRequest{
				Service: "SecurityAccess",
				Subfunc: &subfuncVal,
				Payload: p.computeKey(p.seed),
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
	case stateSendTesterPresent:
		if ev == eventOk {
			return stateRequestSeed, 0, false, true
		}
		if ev == eventFail {
			return stateError, 1, false, true
		}
	case stateRequestSeed:
		if ev == eventOk {
			return stateSendKey, 0, true, true
		}
		if ev == eventFail {
			return stateRetry, 1, false, true
		}
	case stateSendKey:
		if ev == eventOk {
			return stateDone, 0, false, true
		}
		if ev == eventFail {
			return stateRetry, 1, false, true
		}
	case stateRetry:
		if ev == eventNone {
			if p.retryCount < p.maxRetries {
				return stateRequestSeed, 0, true, true
			}
		}
		if ev == eventNone {
			if p.retryCount >= p.maxRetries {
				return stateError, 1, false, true
			}
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
	if source == stateRequestSeed {
		if trIndex == 0 {
			{
				scopeTmp := []byte(p.pendingEventData)
				if len(scopeTmp) > 64 {
					return eventErrorExecution, true
				}
				p.seed = scopeTmp
			}
		}
	}
	if source == stateRetry {
		if trIndex == 0 {
			p.retryCount = p.retryCount + 1
		}
	}
	return 0, false
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, computeKey func([]byte) []byte, ecuAddr uint32) forge.ProcedureRunResult {
	p := newPolicy(handler, computeKey, ecuAddr)
	return forge.RunProcedure(p)
}
