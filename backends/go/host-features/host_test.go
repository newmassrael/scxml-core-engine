// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"strconv"
	"strings"
	"testing"
)

type unreachable struct{}

func (unreachable) Exchange(HTTPRequest) (*HTTPResponse, error) { return nil, http.ErrAbortHandler }

func decode(t *testing.T, outcome Outcome) map[string]any {
	t.Helper()
	if outcome.Kind != Done {
		t.Fatalf("the host did not complete: %+v", outcome)
	}
	var fields map[string]any
	if err := json.Unmarshal([]byte(outcome.Data), &fields); err != nil {
		t.Fatalf("a result is JSON: %v", err)
	}
	return fields
}

// startNumber numbers the starts start makes, so no two of them are one
// invocation asked twice.
var startNumber uint64

func start(t *testing.T, h *ReferenceHost, operation string, params Params) Outcome {
	t.Helper()
	startNumber++
	outcome, served := h.Start(operation, Invocation{SessionID: "session", ID: "id", Start: startNumber}, params)
	if !served {
		t.Fatalf("%s is a standard type and was not served", operation)
	}
	return outcome
}

// startAs starts operation as invocation, which a test names to ask it twice.
func startAs(t *testing.T, h *ReferenceHost, operation string, invocation Invocation, params Params) Outcome {
	t.Helper()
	outcome, served := h.Start(operation, invocation, params)
	if !served {
		t.Fatalf("%s is a standard type and was not served", operation)
	}
	return outcome
}

// docs/adr/0107, D4: a put-if started again under the identity it had is
// answered with its first result and writes nothing, and a start of another
// number, of another session or of another `<invoke>` is an operation of its
// own.
func TestAPutIfRepeatedUnderItsIdentityIsAnsweredWithItsFirstResult(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	first := Invocation{SessionID: "s1", ID: "putif", Start: 1}
	wrote := startAs(t, h, "sce-host-kv-put-if", first, Params{"key": "k", "value": "a", "expected_version": "0"})
	if fields := decode(t, wrote); fields["written"] != true || fields["version"] != "1" {
		t.Fatalf("the first put-if answered %v", fields)
	}
	// The repeat finds the key at version 1, which would answer written false; it
	// is answered as the first start was, and the store is as it was.
	again := startAs(t, h, "sce-host-kv-put-if", first, Params{"key": "k", "value": "a", "expected_version": "0"})
	if again != wrote {
		t.Fatalf("the repeat answered %+v, the first %+v", again, wrote)
	}
	if entries := h.StoreEntries(); len(entries) != 1 || entries[0] != [2]string{"k", "a"} {
		t.Fatalf("the store holds %v", entries)
	}
	// Each of these is a new identity, judged against the store as it is.
	for _, other := range []Invocation{
		{SessionID: "s1", ID: "putif", Start: 2},
		{SessionID: "s2", ID: "putif", Start: 1},
		{SessionID: "s1", ID: "other", Start: 1},
	} {
		lost := decode(t, startAs(t, h, "sce-host-kv-put-if", other,
			Params{"key": "k", "value": "b", "expected_version": "0"}))
		if lost["written"] != false || lost["version"] != "1" {
			t.Errorf("%+v answered %v", other, lost)
		}
	}
	// The host was told every start, the repeat among them, with its identity.
	starts := h.TakeStarts()
	if len(starts) != 5 || starts[1].SessionID != "s1" || starts[1].ID != "putif" || starts[1].Number != 1 {
		t.Fatalf("the starts are %+v", starts)
	}
}

// A refusal is no result: a put-if refused and asked again under the same
// identity is judged again, and a failure is a result the repeat answers.
func TestAPutIfKeepsItsFailureAndNotItsRefusal(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	refused := Invocation{SessionID: "s1", ID: "putif", Start: 1}
	for i := 0; i < 2; i++ {
		if got := startAs(t, h, "sce-host-kv-put-if", refused,
			Params{"key": "", "value": "a", "expected_version": "0"}); got.Kind != Refuse {
			t.Fatalf("an empty key was %+v", got)
		}
	}
	h.SetQuota(0)
	failed := Invocation{SessionID: "s1", ID: "putif", Start: 2}
	first := startAs(t, h, "sce-host-kv-put-if", failed, Params{"key": "k", "value": "a", "expected_version": "0"})
	if fields := decode(t, first); fields["error"] != "quota_exceeded" {
		t.Fatalf("over the quota answered %v", fields)
	}
	h.SetQuota(8)
	if again := startAs(t, h, "sce-host-kv-put-if", failed,
		Params{"key": "k", "value": "a", "expected_version": "0"}); again != first {
		t.Fatalf("the repeat answered %+v, the failure was %+v", again, first)
	}
	if entries := h.StoreEntries(); len(entries) != 0 {
		t.Fatalf("the store holds %v", entries)
	}
}

// The system clock's monotonic reading never goes below one already answered, and
// its UTC reading is the wall clock.
func TestTheSystemClockAnswersAMonotonicReadingThatDoesNotDecrease(t *testing.T) {
	h := NewReferenceHost(NewSystemClock(), unreachable{})
	last := int64(-1)
	for i := 0; i < 50; i++ {
		fields := decode(t, start(t, h, "sce-host-clock-now", Params{}))
		mono, err := strconv.ParseInt(fields["monotonic_ms"].(string), 10, 64)
		if err != nil {
			t.Fatal(err)
		}
		if mono < last {
			t.Fatalf("the monotonic reading went from %d to %d", last, mono)
		}
		last = mono
	}
	zone := decode(t, start(t, h, "sce-host-clock-zone", Params{}))
	if zone["error"] != "" {
		t.Fatalf("the system zone failed: %v", zone)
	}
}

// A type that is not a standard host feature is not served.
func TestAnOperationOutsideTheStandardSetIsNotServed(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	if _, served := h.Start("x-app-host", Invocation{SessionID: "session", ID: "id", Start: 1}, Params{}); served {
		t.Fatal("a type the specification does not define was served")
	}
}

// Keys are ordered by their UTF-8 bytes.
func TestKeysAreOrderedByTheirUtf8Bytes(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	for _, key := range []string{"\U0001F600", "～"} {
		start(t, h, "sce-host-kv-put", Params{"key": key, "value": "v"})
	}
	fields := decode(t, start(t, h, "sce-host-kv-next-key", Params{"after": "", "prefix": ""}))
	if fields["key"] != "～" {
		t.Fatalf("the first key was %q", fields["key"])
	}
}

// A key that holds U+0000 is refused, as an empty one is.
func TestAKeyWithANulIsRefused(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	if got := start(t, h, "sce-host-kv-get", Params{"key": "a\x00b"}); got.Kind != Refuse {
		t.Fatalf("a key with U+0000 was %+v", got)
	}
}

// A put-if is refused for a key with U+0000 as a get is, and writes nothing.
func TestAPutIfWithANulInItsKeyIsRefused(t *testing.T) {
	h := NewReferenceHost(NewScriptedClock(), unreachable{})
	params := Params{"key": "a\x00b", "value": "v", "expected_version": "0"}
	if got := start(t, h, "sce-host-kv-put-if", params); got.Kind != Refuse {
		t.Fatalf("a put-if of a key with U+0000 was %+v", got)
	}
	if entries := h.StoreEntries(); len(entries) != 0 {
		t.Fatalf("a refused put-if wrote %v", entries)
	}
}

// The transport over the standard library's client: a local server answers, the
// host carries the request's headers and body to it, follows no redirect, and
// gives the response's fields in a fixed order.
func TestTheNetTransportCarriesAnExchangeToALocalServer(t *testing.T) {
	var seen *http.Request
	var seenBody strings.Builder
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		seen = r
		buf := make([]byte, 64)
		for {
			n, err := r.Body.Read(buf)
			seenBody.Write(buf[:n])
			if err != nil {
				break
			}
		}
		switch r.URL.Path {
		case "/moved":
			w.Header().Set("Location", "/elsewhere")
			w.WriteHeader(http.StatusMovedPermanently)
		case "/missing":
			http.NotFound(w, r)
		default:
			w.Header().Set("X-Reply", "yes")
			w.WriteHeader(http.StatusCreated)
			_, _ = w.Write([]byte("made"))
		}
	}))
	defer server.Close()
	h := NewReferenceHost(NewScriptedClock(), NewNetTransport())

	fields := decode(t, start(t, h, "sce-host-http-fetch", Params{
		"method": "POST", "url": server.URL + "/items", "headers": "X-Trace: 7\nContent-Type: text/plain", "body": "payload",
	}))
	if fields["error"] != "" || fields["status"] != float64(201) || fields["body"] != "made" {
		t.Fatalf("the exchange was %v", fields)
	}
	if !strings.Contains(fields["headers"].(string), "x-reply: yes") {
		t.Fatalf("the response's fields were %q", fields["headers"])
	}
	if seen.Header.Get("X-Trace") != "7" || seenBody.String() != "payload" || seen.Method != "POST" {
		t.Fatalf("the server saw %v %q", seen.Header, seenBody.String())
	}

	moved := decode(t, start(t, h, "sce-host-http-fetch", Params{"method": "GET", "url": server.URL + "/moved", "headers": "", "body": ""}))
	if moved["status"] != float64(301) || !strings.Contains(moved["headers"].(string), "location: /elsewhere") {
		t.Fatalf("a redirect was followed or lost: %v", moved)
	}
	missing := decode(t, start(t, h, "sce-host-http-fetch", Params{"method": "GET", "url": server.URL + "/missing", "headers": "", "body": ""}))
	if missing["error"] != "" || missing["status"] != float64(404) {
		t.Fatalf("an error status was a failure: %v", missing)
	}

	server.Close()
	gone := decode(t, start(t, h, "sce-host-http-fetch", Params{"method": "GET", "url": server.URL + "/items", "headers": "", "body": ""}))
	if gone["error"] != "network" {
		t.Fatalf("a server that is not there was %v", gone)
	}
}
