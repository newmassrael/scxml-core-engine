// SCE Forge: Auto-generated from Extended SCXML (sce:kind="procedure", Level 2)
// Do not edit — regenerate from the source SCXML file.
//
// Level 2 procedure: event-driven state machine using forge.ProcedurePolicy.
// Supports <onentry>/<send>, event-driven <transition>, <assign>, <donedata>.
//
// External dependencies (from sce:payload expressions — must be in scope):
//   computeKey(seed)

package procedure_security_access

import (
	"fmt"
	"github.com/newmassrael/sce-go-runtime/forge"
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
	eventFail = 1
	eventOk = 2
)

// ── Generated procedure policy ──────────────────────────────────

type policy struct {
	ecuAddr	uint32
	seed	[]byte
	maxRetries	int32
	retryCount	int32
	serviceHandler   forge.ServiceHandler
	doneData         map[string]string
	pendingEventData string
}

func newPolicy(handler forge.ServiceHandler, ecuAddr uint32) *policy {
	return &policy{
		ecuAddr: ecuAddr,
		maxRetries: 3,
		retryCount: 0,
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
			req := forge.ProcedureServiceRequest{
				Service: "TesterPresent",
				Params:  make(map[string]string),
			}
			req.Params["addr"] = fmt.Sprint(p.ecuAddr)
			resp := p.serviceHandler(req)
			if resp.Success {
				return eventOk, resp.Data
			}
			return eventFail, resp.Data
		}
	case stateRequestSeed:
		if p.serviceHandler != nil {
			req := forge.ProcedureServiceRequest{
				Service: "SecurityAccess",
				Subfunc: "0x01",
				Params:  make(map[string]string),
			}
			resp := p.serviceHandler(req)
			if resp.Success {
				return eventOk, resp.Data
			}
			return eventFail, resp.Data
		}
	case stateSendKey:
		if p.serviceHandler != nil {
			req := forge.ProcedureServiceRequest{
				Service: "SecurityAccess",
				Subfunc: "0x02",
				Params:  make(map[string]string),
			}
			req.Params["payload"] = fmt.Sprint(computeKey(p.seed))
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

func (p *policy) ExecuteTransitionActions(source int, trIndex int) {
	if source == stateRequestSeed {
		if trIndex == 0 {
			p.seed = []byte(p.pendingEventData)
		}
	}
	if source == stateRetry {
		if trIndex == 0 {
			p.retryCount = p.retryCount + 1
		}
	}
}

// ── Convenience wrapper function ────────────────────────────────

// Execute runs the procedure to completion.
func Execute(handler forge.ServiceHandler, ecuAddr uint32) forge.ProcedureRunResult {
	p := newPolicy(handler, ecuAddr)
	return forge.RunProcedure(p)
}
