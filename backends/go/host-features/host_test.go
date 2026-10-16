// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

func start(t *testing.T, h *ReferenceHost, operation string, params Params) Outcome {
	t.Helper()
	outcome, served := h.Start(operation, "id", params)
	if !served {
		t.Fatalf("%s is a standard type and was not served", operation)
	}
	return outcome
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
	if _, served := h.Start("x-app-host", "id", Params{}); served {
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
