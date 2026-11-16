// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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

	scelua "github.com/newmassrael/sce-go-lua"
	sce "github.com/newmassrael/sce-go-runtime"
)

// NewLuaEngine returns a fresh Lua script engine instance for tests.
//
// Engine DI Parity RFC (Path B+): each test owns its engine, replacing the
// pre-cleanup `RegisterLuaEngine` + `sce.GetScriptEngine` singleton pair.
func NewLuaEngine() sce.IScriptEngine {
	return scelua.NewLuaEngine()
}

// AssertEndedIn checks that the run ended at the expected top-level <final>,
// given what Engine.TerminalState reported — the ending itself, not a reading
// of the configuration, which Appendix D's exitInterpreter leaves empty.
func AssertEndedIn[S comparable](t *testing.T, ended S, hasEnded bool, expected S, testID string) {
	t.Helper()
	if !hasEnded {
		t.Fatalf("Test %s did not end at a top-level final state, want %v", testID, expected)
	}
	if ended != expected {
		t.Fatalf("Test %s reached wrong final state: got %v, want %v", testID, ended, expected)
	}
}

// BasicHTTPAccessURI reports where the harness's inbound BasicHTTP listener
// (standalone_http_server.js) answers, and therefore the address the generated
// tests declare as the machine's published _ioprocessors location (W3C SCXML
// C.2.3). Bind address and published address are one fact — a document that
// posts somewhere the listener never claimed would fail for a reason unrelated
// to what it tests.
//
// The path is PINNED to tests/w3c/basic_http_test_endpoint.h, the header that
// owns the endpoint for every channel. It is spelled here rather than read from
// there because sce-build embeds THIS FILE and writes it into the standalone
// suites it emits, where no repository sits above it — a harness that reached
// back for the header would compile in-tree and fail wherever it ships.
// `scripts/gates/http-endpoint-ssot.sh` reads both values and refuses a tree
// where they disagree, so the forced copy cannot rot.
//
// The port has no default: it is the one the listener of this run bound
// (endpointPort).
const defaultEndpointPath = "/test"

func BasicHTTPAccessURI() string {
	return fmt.Sprintf("http://localhost:%d%s", endpointPort(), defaultEndpointPath)
}

// endpointPort is the port the out-of-process fixture listener bound: the one
// the file SCE_W3C_HTTP_PORT_FILE names holds, or the one SCE_W3C_HTTP_PORT
// names, as basic_http_test_endpoint.h reads it. Whoever starts the listener (a
// gate, the verify action) starts it on port 0 and hands on the port its
// `[READY] port=<n>` line names; a port assumed here would be one another
// program on the host may hold, which made every run need that one port free.
func endpointPort() int {
	if file := os.Getenv("SCE_W3C_HTTP_PORT_FILE"); file != "" {
		text, err := os.ReadFile(file)
		if err != nil {
			panic(fmt.Sprintf("harness: SCE_W3C_HTTP_PORT_FILE names %s, which cannot be read (%v): "+
				"the out-of-process W3C BasicHTTP listener did not write the port it bound there", file, err))
		}
		return parseEndpointPort(strings.TrimSuffix(string(text), "\n"), file)
	}
	if raw := os.Getenv("SCE_W3C_HTTP_PORT"); raw != "" {
		return parseEndpointPort(raw, "SCE_W3C_HTTP_PORT")
	}
	panic("harness: neither SCE_W3C_HTTP_PORT_FILE nor SCE_W3C_HTTP_PORT says where the W3C BasicHTTP " +
		"fixture listener answers. Start it with `node tests/w3c/standalone_http_server.js 0` and set " +
		"SCE_W3C_HTTP_PORT to the port its `[READY] port=<n>` line names. It has no default port: one " +
		"assumed is one another program on this host may hold.")
}

// parseEndpointPort reads raw as a TCP port, or panics naming source, where it came from.
func parseEndpointPort(raw, source string) int {
	port, err := strconv.Atoi(raw)
	if err != nil || port < 1 || port > 65535 {
		panic(fmt.Sprintf("harness: %s holds %q, which is not a TCP port", source, raw))
	}
	return port
}

// SetupHTTPTest configures an engine for real HTTP tests against the shared
// W3C test server (standalone_http_server.js on the port endpointPort reads).
//
// The callback sends a real HTTP POST, parses the JSON response, and returns
// *HttpSendResponse so the engine injects the echoed event. A POST that does
// not reach the server, or that the server refuses with a status outside 2xx,
// is answered as an error, which the engine reports as error.communication
// (W3C SCXML 6.2.4): a failed POST used to be answered nil, a delivery with no
// reply, and the document then waited for an answer that was never coming.
func SetupHTTPTest[S comparable, E comparable](engine *sce.Engine[S, E]) {
	client := &http.Client{Timeout: 3 * time.Second}

	engine.SetHTTPSendCallback(func(req sce.HttpSendRequest) (*sce.HttpSendResponse, error) {
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
			return nil, err
		}
		httpReq.Header.Set("Content-Type", contentType)

		resp, err := client.Do(httpReq)
		if err != nil {
			return nil, err
		}
		defer resp.Body.Close()
		if resp.StatusCode < 200 || resp.StatusCode > 299 {
			return nil, fmt.Errorf("the server answered HTTP %d", resp.StatusCode)
		}

		// Delivered from here on: a reply that cannot be read names no event.
		respBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return nil, nil
		}

		// W3C SCXML C.2: Parse JSON response for event name and data
		var result struct {
			Event string          `json:"event"`
			Data  json.RawMessage `json:"data"`
		}
		if err := json.Unmarshal(respBody, &result); err != nil {
			return nil, nil
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
		}, nil
	})
}
