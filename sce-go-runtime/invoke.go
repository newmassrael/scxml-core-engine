// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

import (
	"fmt"
	"strings"
)

// PendingInvoke represents a W3C SCXML 6.4 pending invoke structure for the
// defer/cancel/execute pattern.
//
// Ports Rust PendingInvoke from sce-rust-runtime/src/invoke.rs.
type PendingInvoke[S comparable] struct {
	// InvokeID is the W3C SCXML 6.4.1 runtime-generated invoke identifier.
	// Format: "stateid.platformid.index" (e.g., "s01.140234567890._invoke_0")
	InvokeID string

	// State is the state containing the invoke element.
	State S
}

// ChildSession represents a W3C SCXML 6.4 active child session.
//
// Ports Rust ChildSession from sce-rust-runtime/src/invoke.rs.
type ChildSession struct {
	// SessionID is the child's session ID (format: "parentSessionId.invokeId").
	SessionID string

	// InvokeID is the invoke element ID (e.g., "_invoke_0").
	InvokeID string

	// ParentSessionID is the parent's session ID.
	ParentSessionID string

	// Autoforward indicates whether to autoforward events to child (W3C SCXML 6.4.6).
	Autoforward bool

	// FinalizeScript is the <finalize> handler script content (W3C SCXML 6.5).
	FinalizeScript string
}

// DeferInvoke adds a pending invoke for deferred execution at macrostep end
// (W3C SCXML 6.4).
//
// Ports Rust defer_invoke from sce-rust-runtime/src/invoke.rs.
func DeferInvoke[S comparable](pending *[]PendingInvoke[S], invoke PendingInvoke[S]) {
	*pending = append(*pending, invoke)
}

// CancelInvokesForState removes pending invokes for an exited state
// (W3C SCXML 6.4).
//
// Ports Rust cancel_invokes_for_state from sce-rust-runtime/src/invoke.rs.
func CancelInvokesForState[S comparable](pending *[]PendingInvoke[S], state S) {
	n := 0
	for _, p := range *pending {
		if p.State != state {
			(*pending)[n] = p
			n++
		}
	}
	*pending = (*pending)[:n]
}

// ExecutePendingInvokes executes all pending invokes at macrostep end
// (W3C SCXML 6.4). Uses copy-and-clear pattern to prevent iterator
// invalidation during execution.
//
// Ports Rust execute_pending_invokes from sce-rust-runtime/src/invoke.rs.
func ExecutePendingInvokes[S comparable](pending *[]PendingInvoke[S], executor func(*PendingInvoke[S])) {
	if len(*pending) == 0 {
		return
	}

	// W3C SCXML 6.4: Copy pending list to prevent iterator invalidation
	invokesToExecute := make([]PendingInvoke[S], len(*pending))
	copy(invokesToExecute, *pending)
	*pending = (*pending)[:0]

	for i := range invokesToExecute {
		executor(&invokesToExecute[i])
	}
}

// CreateDoneInvokeEventName creates a done.invoke event name (W3C SCXML 6.3.1).
//
// Ports Rust create_done_invoke_event_name from sce-rust-runtime/src/invoke.rs.
func CreateDoneInvokeEventName(invokeID string) string {
	return fmt.Sprintf("done.invoke.%s", invokeID)
}

// IsValidInvokeID validates invoke ID format (W3C SCXML 3.12.1).
//
// Ports Rust is_valid_invoke_id from sce-rust-runtime/src/invoke.rs.
func IsValidInvokeID(invokeID string) bool {
	return invokeID != ""
}

// GetPendingCount returns the count of pending invokes (W3C SCXML 6.4).
func GetPendingCount[S comparable](pending []PendingInvoke[S]) int {
	return len(pending)
}

// IsInvokePending checks if a specific invoke is pending (W3C SCXML 6.4).
func IsInvokePending[S comparable](pending []PendingInvoke[S], invokeID string) bool {
	for _, p := range pending {
		if p.InvokeID == invokeID {
			return true
		}
	}
	return false
}

// ChildEngine is the interface that parent state machines use to interact with
// child invoke sessions (W3C SCXML 6.4). Generated code implements this for
// each concrete child state machine type.
type ChildEngine interface {
	Initialize()
	Tick()
	IsInFinalState() bool
	RaiseExternalByName(eventName, eventData string)
	SetCompletionCallback(callback func())
	GetParentEventQueue() chan ParentEvent
}

// DrainAndRaiseChildEvents drains events from a child's ParentExternalQueue
// and raises them in the parent engine with proper metadata (W3C SCXML 6.4).
// Port of Rust drain_and_raise_child_events().
func DrainAndRaiseChildEvents[S comparable, E comparable](
	childEngine ChildEngine,
	activeInvokes map[string]*ChildSession,
	invokeKey string,
	engine *Engine[S, E],
) {
	if childEngine == nil {
		return
	}
	queue := childEngine.GetParentEventQueue()
	if queue == nil {
		return
	}
	for {
		select {
		case ev := <-queue:
			event, ok := engine.policy.GetEventFromName(ev.Name)
			if !ok {
				continue
			}
			meta := NewEventWithMetadata(event)
			meta.Metadata.Data = ev.Data
			if cs, found := activeInvokes[invokeKey]; found {
				meta.Metadata.InvokeID = cs.InvokeID
				meta.Metadata.Origin = cs.SessionID
				meta.Metadata.OriginType = SCXMLEventProcessorType
			}
			engine.RaiseExternalWithMeta(meta)
		default:
			return
		}
	}
}

// RaiseDoneInvoke raises a done.invoke event in the parent engine (W3C SCXML 6.3.1).
// Port of Rust raise_done_invoke().
func RaiseDoneInvoke[S comparable, E comparable](
	invokeID string,
	engine *Engine[S, E],
) {
	eventName := "done.invoke." + invokeID
	event, ok := engine.policy.GetEventFromName(eventName)
	if !ok {
		// Try without the specific ID (generic done.invoke)
		event, ok = engine.policy.GetEventFromName("done.invoke")
		if !ok {
			return
		}
	}
	meta := NewEventWithMetadata(event)
	meta.Metadata.InvokeID = invokeID
	engine.RaiseExternalWithMeta(meta)
}

// IsPlatformEvent checks if an event name is a platform event that should NOT
// be autoforwarded (W3C SCXML 6.4.6).
func IsPlatformEvent(name string) bool {
	return strings.HasPrefix(name, "#_")
}
