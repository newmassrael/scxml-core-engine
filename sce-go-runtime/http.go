// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package sce

// HttpSendRequest is the W3C SCXML C.2 HTTP send request payload passed to the
// on_http_send callback.
//
// Ports Rust HttpSendRequest from sce-rust-runtime/src/http.rs. Matches C++
// SCE::Static::HttpSendRequest 1:1.
type HttpSendRequest struct {
	// Target is the absolute HTTP URL (e.g., "http://localhost:8080/test").
	Target string

	// EventName is the event name to encode in the HTTP POST payload.
	EventName string

	// Content is the raw content body (W3C SCXML C.2: sent as multipart form data).
	Content string

	// Params are form parameters (W3C SCXML 6.2 <param> elements).
	// Multiple values per key are allowed.
	Params map[string][]string

	// SendID is the send ID for correlation and cancellation (W3C SCXML 6.2.5).
	SendID string
}
