// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
// SCE Forge: Procedure types and execution engine for Level 2 procedures.
//
// Generated code implements ProcedurePolicy; this package provides the
// shared types and the event-driven execution loop.

package forge

// ── Service types ───────────────────────────────────────────────

// ProcedureServiceRequest represents a service call during procedure execution.
//
// Fields map 1:1 to the four <send> attributes in the SCXML source:
//
//	<send sce:service="Diag" sce:subfunc="0x02"
//	      sce:addr="ecuAddr" sce:payload="frame.encode()"/>
//
// Service is always present. The other three use pointer types so absent
// attributes are distinguishable from empty strings. Payload is typed as
// raw bytes — it carries wire-format data from codec encode() calls. The
// stringy fields (Subfunc, Addr) remain textual to accommodate datamodel
// variables of any SCE type.
type ProcedureServiceRequest struct {
	Service string
	Subfunc *string
	Addr    *string
	Payload []byte
}

// ProcedureServiceResponse represents the result of a service call.
type ProcedureServiceResponse struct {
	Success bool
	Data    string
}

// ProcedureRunResult holds the outcome of running a procedure to completion.
type ProcedureRunResult struct {
	Completed  bool
	FinalState string
	DoneData   map[string]string
}

// ServiceHandler is the callback type for service dispatch.
type ServiceHandler func(ProcedureServiceRequest) ProcedureServiceResponse

// ── Procedure policy interface ──────────────────────────────────

// ProcedurePolicy is the interface that generated Level 2 procedure code implements.
// Each method corresponds to a section of the generated state machine.
type ProcedurePolicy interface {
	// NoneEvent returns the event value representing "no event".
	NoneEvent() int
	// InitialState returns the initial state of the procedure.
	InitialState() int
	// IsFinal returns whether the given state is a <final> state.
	IsFinal(state int) bool
	// FinalStateName returns the SCXML id of the final state.
	FinalStateName(state int) string
	// SetPendingEventData stores _event.data for access in guards/actions.
	SetPendingEventData(data string)
	// DoneData returns the collected <donedata> from final states.
	DoneData() map[string]string
	// ExecuteEntryActions executes entry actions for a state; returns (event, eventData).
	ExecuteEntryActions(state int) (event int, data string)
	// ProcessTransition processes a transition; returns (nextState, trIndex, hasAssigns, ok).
	ProcessTransition(state int, event int) (nextState int, trIndex int, hasAssigns bool, ok bool)
	// ExecuteTransitionActions executes <assign> actions for a transition.
	ExecuteTransitionActions(source int, trIndex int)
}

// ── Execution engine ────────────────────────────────────────────

// maxIterations is the safety limit for the event loop — prevents infinite loops.
const maxIterations = 1000

// RunProcedure drives a procedure state machine to completion.
// It executes the event loop from initial state through service sends
// until a <final> state is reached or no transition is possible.
func RunProcedure(policy ProcedurePolicy) ProcedureRunResult {
	current := policy.InitialState()
	event := policy.NoneEvent()

	entryEvent, entryData := policy.ExecuteEntryActions(current)
	event = entryEvent
	if entryData != "" {
		policy.SetPendingEventData(entryData)
	}

	for i := 0; i < maxIterations; i++ {
		if policy.IsFinal(current) {
			break
		}
		nextState, trIndex, hasAssigns, ok := policy.ProcessTransition(current, event)
		if !ok {
			break
		}
		if hasAssigns {
			policy.ExecuteTransitionActions(current, trIndex)
		}
		current = nextState
		event = policy.NoneEvent()
		entryEvent, entryData = policy.ExecuteEntryActions(current)
		event = entryEvent
		if entryData != "" {
			policy.SetPendingEventData(entryData)
		}
	}

	completed := policy.IsFinal(current)
	finalState := ""
	if completed {
		finalState = policy.FinalStateName(current)
	}
	doneData := map[string]string{}
	if completed {
		for k, v := range policy.DoneData() {
			doneData[k] = v
		}
	}
	return ProcedureRunResult{Completed: completed, FinalState: finalState, DoneData: doneData}
}
