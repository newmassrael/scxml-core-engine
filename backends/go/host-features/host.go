// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// Package hostfeatures is the reference host of the standard host features
// (docs/SCE_HOST_FEATURES.md): clock, key-value store, notifications and HTTP, as
// the operations a generated machine's host-run invokes ask for.
//
// A ReferenceHost is the whole of the four features' semantics as one object. It
// is told the start of an operation (its type and the request's text parameters)
// and answers with an Outcome: a completion, a refusal, or nothing yet. It knows
// no engine, so the same object serves a generated machine through Serve, a test,
// or another embedding. Time comes from a ClockSource and the network from a
// Transport, both the embedding's to supply.
package hostfeatures

import "encoding/json"

// Types is every operation type of the standard host features, in the order the
// specification lists them.
var Types = []string{
	"sce-host-clock-now",
	"sce-host-clock-zone",
	"sce-host-kv-get",
	"sce-host-kv-put",
	"sce-host-kv-delete",
	"sce-host-kv-next-key",
	"sce-host-notify-permission",
	"sce-host-notify-request-permission",
	"sce-host-notify-post",
	"sce-host-notify-cancel",
	"sce-host-http-fetch",
}

// Params is the request of an operation as the engine hands it to a host: each
// <param> by name, as text. A 64-bit integer is its decimal digits.
type Params map[string]string

// Kind says how a host answered a start.
type Kind int

const (
	// Done is a completion: Data is the result record, as the JSON a completion
	// carries. A failure is such a record whose error is a code of the
	// operation's closed set (see failure).
	Done Kind = iota
	// Refuse is a host that would not start the operation; Data is the reason, as
	// the JSON error.execution carries.
	Refuse
	// Hold is a host that has started the operation and answers later, or never.
	Hold
)

// Outcome is what a host answers a start with.
type Outcome struct {
	Kind Kind
	Data string
}

// invalidArgument is the refusal the specification gives an argument the
// operation does not accept.
const invalidArgument = `{"reason":"invalid_argument"}`

// Start is a start the host was given: the operation, the invocation's id and the
// request's parameters.
type Start struct {
	Operation string
	ID        string
	Params    Params
}

// ReferenceHost is the standard host features' semantics, over a clock and a
// transport.
type ReferenceHost struct {
	clock     ClockSource
	transport Transport
	store     store
	notify    notifications
	// faults are failures to inject: the operation type to the code it fails with.
	faults map[string]string
	// held are the invocations started and not yet answered, by id.
	held []string
	// cancelled are the invocations the host was told to stop, in the order told.
	cancelled []string
	// monotonicFloor is the highest monotonic reading answered, which no answer
	// goes below.
	monotonicFloor int64
	monotonicSet   bool
	starts         []Start
}

// NewReferenceHost is a host reading clock and carrying HTTP over transport, with
// an empty store, nothing posted, and the notification permission undetermined.
func NewReferenceHost(clock ClockSource, transport Transport) *ReferenceHost {
	return &ReferenceHost{
		clock:     clock,
		transport: transport,
		notify:    notifications{permission: "undetermined", userDecision: "none"},
		faults:    map[string]string{},
	}
}

// SetFaults injects failures: each operation type named fails with the code it
// maps to, once the request has been accepted. An empty map ends them.
func (h *ReferenceHost) SetFaults(faults map[string]string) { h.faults = faults }

// SetQuota bounds the store to keys keys; a write of a new key past it fails
// quota_exceeded.
func (h *ReferenceHost) SetQuota(keys int) { h.store.quota, h.store.hasQuota = keys, true }

// SetPermission is the notification permission as the platform says it: granted,
// denied or undetermined.
func (h *ReferenceHost) SetPermission(state string) { h.notify.permission = state }

// SetUserDecision is what the user chooses when asked: granted, denied, or none
// for a host with no user to ask.
func (h *ReferenceHost) SetUserDecision(decision string) { h.notify.userDecision = decision }

// StoreEntries are the store's keys and values, in key order.
func (h *ReferenceHost) StoreEntries() [][2]string { return h.store.entries() }

// Waiting are the ids of the waiting notifications, in the order posted.
func (h *ReferenceHost) Waiting() []string { return h.notify.ids(h.notify.waiting) }

// Shown are the ids of the shown notifications, in the order posted.
func (h *ReferenceHost) Shown() []string { return h.notify.ids(h.notify.shown) }

// Cancelled are the invocations the host was told to stop, in the order told.
func (h *ReferenceHost) Cancelled() []string { return append([]string{}, h.cancelled...) }

// TakeStarts are the starts the host was given since this was last called, oldest
// first.
func (h *ReferenceHost) TakeStarts() []Start {
	taken := h.starts
	h.starts = nil
	return taken
}

// Start starts the operation of invocation id with params.
//
// The second result is false for a type that is not one of the standard host
// features: the host serves none of it. Otherwise the arguments are judged first
// (a refusal starts nothing), then a failure the embedding injected, then the
// operation itself.
func (h *ReferenceHost) Start(operation, id string, params Params) (Outcome, bool) {
	known := false
	for _, t := range Types {
		known = known || t == operation
	}
	if !known {
		return Outcome{}, false
	}
	copied := Params{}
	for name, value := range params {
		copied[name] = value
	}
	h.starts = append(h.starts, Start{Operation: operation, ID: id, Params: copied})
	refused := false
	switch operation {
	case "sce-host-kv-get", "sce-host-kv-delete", "sce-host-kv-put":
		refused = !validKey(params["key"])
	case "sce-host-notify-post", "sce-host-notify-cancel":
		refused = params["id"] == ""
	case "sce-host-http-fetch":
		refused = !acceptable(params["method"], params["url"], params["headers"])
	}
	if refused {
		return Outcome{Kind: Refuse, Data: invalidArgument}, true
	}
	if code, ok := h.faults[operation]; ok {
		return failure(operation, code), true
	}
	switch operation {
	case "sce-host-clock-now":
		return h.clockNow(), true
	case "sce-host-clock-zone":
		return h.clockZone(), true
	case "sce-host-kv-get":
		return h.kvGet(params["key"]), true
	case "sce-host-kv-put":
		return h.kvPut(params["key"], params["value"]), true
	case "sce-host-kv-delete":
		return h.kvDelete(params["key"]), true
	case "sce-host-kv-next-key":
		return h.kvNextKey(params["after"], params["prefix"]), true
	case "sce-host-notify-permission":
		return h.notifyPermission(), true
	case "sce-host-notify-request-permission":
		return h.notifyRequestPermission(), true
	case "sce-host-notify-post":
		return h.notifyPost(params["id"], params["title"], params["body"], params["at_ms"]), true
	case "sce-host-notify-cancel":
		return h.notifyCancel(params["id"]), true
	default:
		return h.httpFetch(id, params["method"], params["url"], params["headers"], params["body"]), true
	}
}

// Cancel tells the host to stop invocation id, because its state exited or its
// deadline passed. What it held for it is dropped.
func (h *ReferenceHost) Cancel(id string) {
	kept := h.held[:0]
	for _, held := range h.held {
		if held != id {
			kept = append(kept, held)
		}
	}
	h.held = kept
	h.cancelled = append(h.cancelled, id)
}

// record is a result record as the JSON a completion carries, its error empty. A
// 64-bit integer is written as the text of its digits, as a payload's reader
// takes it.
func record(fields map[string]any) Outcome {
	fields["error"] = ""
	data, err := json.Marshal(fields)
	if err != nil {
		panic(err)
	}
	return Outcome{Kind: Done, Data: string(data)}
}

// failure is the failure code of operation: its result record with the fields at
// their zero and error the code, so a machine reads the failure as a typed field.
func failure(operation, code string) Outcome {
	zero := map[string]map[string]any{
		"sce-host-clock-now":                 {"utc_ms": "0", "monotonic_ms": "0"},
		"sce-host-clock-zone":                {"zone_id": "", "utc_offset_s": 0},
		"sce-host-kv-get":                    {"found": false, "value": ""},
		"sce-host-kv-put":                    {"replaced": false},
		"sce-host-kv-delete":                 {"existed": false},
		"sce-host-notify-cancel":             {"existed": false},
		"sce-host-kv-next-key":               {"found": false, "key": ""},
		"sce-host-notify-permission":         {"state": ""},
		"sce-host-notify-request-permission": {"state": ""},
		"sce-host-notify-post":               {"status": ""},
		"sce-host-http-fetch":                {"status": 0, "headers": "", "body": ""},
	}[operation]
	fields := map[string]any{}
	for name, value := range zero {
		fields[name] = value
	}
	fields["error"] = code
	data, err := json.Marshal(fields)
	if err != nil {
		panic(err)
	}
	return Outcome{Kind: Done, Data: string(data)}
}
