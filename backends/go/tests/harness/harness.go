// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package harness provides the W3C SCXML conformance test harness for Go.
//
// Ports the Rust SimpleAotTest trait and run_simple_aot_test() from
// backends/rust/tests/src/harness.rs. Each generated test calls RunTest()
// or uses AssertFinalState() to verify W3C conformance.
package harness

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	"strconv"
	"strings"
	"testing"
	"time"

	sce "github.com/newmassrael/sce-go-runtime"
	scelua "github.com/newmassrael/sce-go-lua"
)

// NewLuaEngine returns a fresh Lua script engine instance for tests.
//
// Engine DI Parity RFC (Path B+): each test owns its engine, replacing the
// pre-cleanup `RegisterLuaEngine` + `sce.GetScriptEngine` singleton pair.
func NewLuaEngine() sce.IScriptEngine {
	return scelua.NewLuaEngine()
}

// AssertFinalState checks that the engine reached the expected final state.
func AssertFinalState[S comparable](t *testing.T, actual, expected S, testID string) {
	t.Helper()
	if actual != expected {
		t.Fatalf("Test %s reached wrong final state: got %v, want %v", testID, actual, expected)
	}
}

// BasicHTTPAccessURI reports where the harness's inbound BasicHTTP listener
// (standalone_http_server.js) answers, and therefore the address the generated
// tests declare as the machine's published _ioprocessors location (W3C SCXML
// C.2.3). Bind address and published address are one fact — a document that
// posts somewhere the listener never claimed would fail for a reason unrelated
// to what it tests.
//
// The default is PINNED to tests/w3c/basic_http_test_endpoint.h, the header
// that owns the endpoint for every channel. It is spelled here rather than read
// from there because sce-build embeds THIS FILE and writes it into the
// standalone suites it emits, where no repository sits above it — a harness
// that reached back for the header would compile in-tree and fail wherever it
// ships. `scripts/gates/http-endpoint-ssot.sh` reads both values and refuses a
// tree where they disagree, so the forced copy cannot rot.
//
// SCE_W3C_HTTP_PORT overrides it, which is how a second checkout runs the
// BasicHTTP suites while the first holds the port; the gates, CMake and CI all
// export it from the one owner.
const (
	defaultEndpointPort = 8080
	defaultEndpointPath = "/test"
)

func BasicHTTPAccessURI() string {
	return fmt.Sprintf("http://localhost:%d%s", endpointPort(), defaultEndpointPath)
}

func endpointPort() int {
	if raw := os.Getenv("SCE_W3C_HTTP_PORT"); raw != "" {
		port, err := strconv.Atoi(raw)
		if err != nil || port < 1 || port > 65535 {
			panic(fmt.Sprintf("harness: SCE_W3C_HTTP_PORT=%q is not a TCP port", raw))
		}
		return port
	}
	return defaultEndpointPort
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

