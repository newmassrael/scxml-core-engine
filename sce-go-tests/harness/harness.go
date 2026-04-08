// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package harness provides the W3C SCXML conformance test harness for Go.
//
// Ports the Rust SimpleAotTest trait and run_simple_aot_test() from
// sce-rust-tests/src/harness.rs. Each generated test calls RunTest()
// or uses AssertFinalState() to verify W3C conformance.
package harness

import (
	"encoding/json"
	"io"
	"net/http"
	"net/url"
	"strings"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scelua "github.com/newmassrael/sce-go-lua"
)

// RegisterLuaEngine registers the Lua script engine for tests requiring it.
func RegisterLuaEngine() {
	scelua.Register()
}

// AssertFinalState checks that the engine reached the expected final state.
func AssertFinalState[S comparable](t *testing.T, actual, expected S, testID string) {
	t.Helper()
	if actual != expected {
		t.Fatalf("Test %s reached wrong final state: got %v, want %v", testID, actual, expected)
	}
}

// SetupHTTPTest configures an engine for real HTTP tests against the shared
// W3C test server (standalone_http_server.js on localhost:8080/test).
//
// The callback sends a real HTTP POST, parses the JSON response, and returns
// *HttpSendResponse so the engine injects the echoed event.
func SetupHTTPTest[S comparable, E comparable](engine *sce.Engine[S, E]) {
	client := &http.Client{Timeout: 3 * time.Second}

	engine.SetHTTPSendCallback(func(req sce.HttpSendRequest) *sce.HttpSendResponse {
		// W3C SCXML C.2: Build form-encoded POST body
		form := url.Values{}
		if req.EventName != "" {
			form.Set("_scxmleventname", req.EventName)
		}
		for key, values := range req.Params {
			if key == "_scxmleventname" && req.EventName != "" {
				continue
			}
			for _, v := range values {
				form.Add(key, v)
			}
		}

		var body string
		var contentType string
		if len(form) > 0 {
			body = form.Encode()
			contentType = "application/x-www-form-urlencoded"
		} else if req.Content != "" {
			body = req.Content
			contentType = "text/plain"
		} else {
			contentType = "application/x-www-form-urlencoded"
		}

		// W3C SCXML C.2: Send real HTTP POST to shared test server
		httpReq, err := http.NewRequest("POST", req.Target, strings.NewReader(body))
		if err != nil {
			return nil
		}
		httpReq.Header.Set("Content-Type", contentType)

		resp, err := client.Do(httpReq)
		if err != nil {
			return nil
		}
		defer resp.Body.Close()

		respBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return nil
		}

		// W3C SCXML C.2: Parse JSON response for event name and data
		var result struct {
			Event string          `json:"event"`
			Data  json.RawMessage `json:"data"`
		}
		if err := json.Unmarshal(respBody, &result); err != nil {
			return nil
		}

		eventData := ""
		if result.Data != nil {
			// If data is a string, unwrap it; otherwise use raw JSON
			var strData string
			if err := json.Unmarshal(result.Data, &strData); err == nil {
				eventData = strData
			} else {
				eventData = string(result.Data)
			}
		}

		return &sce.HttpSendResponse{
			EventName: result.Event,
			EventData: eventData,
		}
	})
}

