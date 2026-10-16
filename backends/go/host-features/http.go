// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

package hostfeatures

import (
	"bytes"
	"errors"
	"io"
	"net/http"
	"sort"
	"strings"
	"unicode"
	"unicode/utf8"
)

// The most bytes of response headers and of body a result record holds.
const (
	headersBound = 4096
	bodyBound    = 16384
)

var methods = []string{"GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"}

// HTTPHeader is one header field: its name and its value.
type HTTPHeader struct{ Name, Value string }

// HTTPRequest is what the host hands its transport: the request, its header lines
// already split into name and value.
type HTTPRequest struct {
	Method  string
	URL     string
	Headers []HTTPHeader
	Body    string
}

// HTTPResponse is what a transport answers: the response as it came.
type HTTPResponse struct {
	Status  int
	Headers []HTTPHeader
	Body    []byte
}

// Transport carries one exchange. A response is the answer; an error is a
// transport that got no response (name resolution, connection, TLS, or the
// connection ended); a nil response and a nil error is a transport that has not
// answered, and the host holds the exchange until it is cancelled.
type Transport interface {
	Exchange(request HTTPRequest) (*HTTPResponse, error)
}

// isToken is an RFC 9110 token: what a header name is made of.
func isToken(name string) bool {
	if name == "" {
		return false
	}
	for _, r := range name {
		ok := r < 128 && (unicode.IsLetter(r) || unicode.IsDigit(r) || strings.ContainsRune("!#$%&'*+-.^_`|~", r))
		if !ok {
			return false
		}
	}
	return true
}

// headerLines are the header lines of a request, split into name and value; false
// for a line that is not one. Empty lines are ignored, and the value is without
// the spaces and tabs around it.
func headerLines(text string) ([]HTTPHeader, bool) {
	headers := []HTTPHeader{}
	for _, line := range strings.Split(text, "\n") {
		line = strings.TrimSuffix(line, "\r")
		if line == "" {
			continue
		}
		name, value, found := strings.Cut(line, ":")
		if !found || !isToken(name) {
			return nil, false
		}
		headers = append(headers, HTTPHeader{Name: name, Value: strings.Trim(value, " \t")})
	}
	return headers, true
}

// isHTTPURL is an absolute http or https URL with a host, and no ASCII space or
// control character (U+0000 to U+0020 and U+007F) in it.
func isHTTPURL(url string) bool {
	rest, ok := strings.CutPrefix(url, "http://")
	if !ok {
		rest, ok = strings.CutPrefix(url, "https://")
	}
	if !ok {
		return false
	}
	authority := rest
	if end := strings.IndexAny(rest, "/?#"); end >= 0 {
		authority = rest[:end]
	}
	if authority == "" || strings.HasPrefix(authority, ":") {
		return false
	}
	for _, r := range url {
		if r <= ' ' || r == 0x7f {
			return false
		}
	}
	return true
}

// acceptable is whether the arguments of an exchange are ones the operation accepts.
func acceptable(method, url, headers string) bool {
	listed := false
	for _, m := range methods {
		listed = listed || m == method
	}
	_, lines := headerLines(headers)
	return listed && isHTTPURL(url) && lines
}

func (h *ReferenceHost) httpFetch(id, method, url, headers, body string) Outcome {
	lines, _ := headerLines(headers)
	response, err := h.transport.Exchange(HTTPRequest{Method: method, URL: url, Headers: lines, Body: body})
	if err != nil {
		return failure("sce-host-http-fetch", "network")
	}
	if response == nil {
		h.held = append(h.held, id)
		return Outcome{Kind: Hold}
	}
	fields := make([]string, 0, len(response.Headers))
	for _, header := range response.Headers {
		fields = append(fields, strings.ToLower(header.Name)+": "+strings.TrimSpace(header.Value))
	}
	joined := strings.Join(fields, "\n")
	// A response to HEAD has no body, whatever the transport carried.
	payload := response.Body
	if method == "HEAD" {
		payload = nil
	}
	if len(joined) > headersBound || len(payload) > bodyBound {
		return failure("sce-host-http-fetch", "response_too_large")
	}
	if !utf8.Valid(payload) {
		return failure("sce-host-http-fetch", "invalid_body")
	}
	return record(map[string]any{"status": response.Status, "headers": joined, "body": string(payload)})
}

// NetTransport carries an exchange over the standard library's HTTP client. It
// follows no redirect and keeps no cookie. The client keeps a response's fields in
// a map, so the order they were received in is lost: they are given sorted by
// name, which is the order a transport that cannot give the received one gives.
type NetTransport struct{ Client *http.Client }

// NewNetTransport is a transport over a client that follows no redirect.
func NewNetTransport() *NetTransport {
	return &NetTransport{Client: &http.Client{
		CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse },
	}}
}

// Exchange sends the request and reads the whole response. Any failure to get a
// response, a body that cannot be read to its end included, is an error.
func (t *NetTransport) Exchange(request HTTPRequest) (*HTTPResponse, error) {
	var body io.Reader
	if request.Body != "" {
		body = strings.NewReader(request.Body)
	}
	outgoing, err := http.NewRequest(request.Method, request.URL, body)
	if err != nil {
		return nil, err
	}
	for _, header := range request.Headers {
		outgoing.Header.Add(header.Name, header.Value)
	}
	response, err := t.Client.Do(outgoing)
	if err != nil {
		return nil, err
	}
	defer response.Body.Close()
	// Read one byte past the bound, which is enough to see it passed.
	payload, err := io.ReadAll(io.LimitReader(response.Body, bodyBound+1))
	if err != nil {
		return nil, errors.New("the body could not be read")
	}
	names := make([]string, 0, len(response.Header))
	for name := range response.Header {
		names = append(names, name)
	}
	sort.Strings(names)
	var headers []HTTPHeader
	for _, name := range names {
		for _, value := range response.Header[name] {
			headers = append(headers, HTTPHeader{Name: name, Value: value})
		}
	}
	return &HTTPResponse{Status: response.StatusCode, Headers: headers, Body: bytes.Clone(payload)}, nil
}
