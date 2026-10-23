// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// The §scxml-C-2 HTTP send request handed to the HTTP send callback.
///
/// Ports Rust `HttpSendRequest` from `backends/rust/runtime/src/http.rs`. Matches
/// C++ `SCE::Static::HttpSendRequest`.
public struct HttpSendRequest: Sendable {
    /// The absolute HTTP URL, such as `http://localhost:8080/test`.
    public var target: String

    /// The event name to encode in the HTTP POST payload.
    public var eventName: String

    /// The raw content body (§scxml-C-2: sent as multipart form data).
    public var content: String

    /// The form parameters (§scxml-6.2 `<param>` elements). Several values per key
    /// are allowed.
    public var params: [String: [String]]

    /// The send id, for correlation and cancellation (§scxml-6.2.5).
    public var sendID: String

    public init(
        target: String,
        eventName: String,
        content: String,
        params: [String: [String]],
        sendID: String
    ) {
        self.target = target
        self.eventName = eventName
        self.content = content
        self.params = params
        self.sendID = sendID
    }
}

/// The §scxml-C-2 HTTP send response returned by the HTTP send callback. When
/// returned, the engine injects the response event into the external queue,
/// enabling real HTTP round trips against the shared W3C HTTP test server.
public struct HttpSendResponse: Sendable {
    /// The event name extracted from the HTTP response, such as `event1` and
    /// `HTTP.POST`.
    public var eventName: String

    /// The event data from the response (§scxml-5.10.1: `_event.data`).
    public var eventData: String

    public init(eventName: String, eventData: String) {
        self.eventName = eventName
        self.eventData = eventData
    }
}
