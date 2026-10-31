// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// What the HTTP fetch of the standard host features needs of the world
// (docs/SCE_HOST_FEATURES.md, "HTTP"): the request the host hands its transport, the
// response a transport answers, and the judgement of the arguments of an exchange.
// A port of `backends/go/host-features/http.go`; the transport itself is the
// embedding's.

/// The most bytes of response headers and of body a result record holds.
let httpHeadersBound = 4096
let httpBodyBound = 16384

/// The methods an exchange may use.
private let httpMethods = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]

/// One header field: its name and its value.
public struct HostHTTPHeader: Equatable, Sendable {
    public var name: String
    public var value: String

    public init(name: String, value: String) {
        self.name = name
        self.value = value
    }
}

/// What the host hands its transport: the request, its header lines already split into
/// name and value.
public struct HostHTTPRequest: Equatable, Sendable {
    public var method: String
    public var url: String
    public var headers: [HostHTTPHeader]
    public var body: String

    public init(method: String, url: String, headers: [HostHTTPHeader], body: String) {
        self.method = method
        self.url = url
        self.headers = headers
        self.body = body
    }
}

/// What a transport answers: the response as it came.
public struct HostHTTPResponse: Equatable, Sendable {
    public var status: Int
    public var headers: [HostHTTPHeader]
    public var body: [UInt8]

    public init(status: Int, headers: [HostHTTPHeader], body: [UInt8]) {
        self.status = status
        self.headers = headers
        self.body = body
    }
}

/// How one exchange ended.
public enum HostExchange: Equatable, Sendable {
    /// A response: the answer.
    case response(HostHTTPResponse)

    /// A transport that got no response (name resolution, connection, TLS, or the
    /// connection ended).
    case unreachable

    /// A transport that has not answered: the host holds the exchange until it is
    /// cancelled.
    case pending
}

/// Carries one exchange.
public protocol HostTransport: AnyObject {
    func exchange(_ request: HostHTTPRequest) -> HostExchange
}

/// An RFC 9110 token: what a header name is made of.
private func isToken(_ name: String) -> Bool {
    guard !name.isEmpty else {
        return false
    }
    let punctuation = Set("!#$%&'*+-.^_`|~".utf8)
    return name.utf8.allSatisfy { byte in
        byte < 128
            && ((0x30...0x39).contains(byte) || (0x41...0x5A).contains(byte) || (0x61...0x7A).contains(byte)
                || punctuation.contains(byte))
    }
}

/// The header lines of a request, split into name and value; nil for a line that is not
/// one. Empty lines are ignored, and the value is without the spaces and tabs around it.
///
/// Split on UTF-8 bytes: a `Character` is a grapheme, and `\r\n` is one of them.
func httpHeaderLines(_ text: String) -> [HostHTTPHeader]? {
    var headers: [HostHTTPHeader] = []
    for line in text.utf8.split(separator: 0x0A, omittingEmptySubsequences: false) {
        var bytes = Array(line)
        if bytes.last == 0x0D {
            bytes.removeLast()
        }
        if bytes.isEmpty {
            continue
        }
        guard let colon = bytes.firstIndex(of: 0x3A) else {
            return nil
        }
        let name = String(decoding: bytes[..<colon], as: UTF8.self)
        guard isToken(name) else {
            return nil
        }
        var value = bytes[(colon + 1)...]
        while let first = value.first, first == 0x20 || first == 0x09 {
            value = value.dropFirst()
        }
        while let last = value.last, last == 0x20 || last == 0x09 {
            value = value.dropLast()
        }
        headers.append(HostHTTPHeader(name: name, value: String(decoding: value, as: UTF8.self)))
    }
    return headers
}

/// An absolute http or https URL with a host, and no ASCII space or control character
/// (U+0000 to U+0020 and U+007F) in it.
private func isHTTPURL(_ url: String) -> Bool {
    let bytes = Array(url.utf8)
    let rest: ArraySlice<UInt8>
    if bytes.starts(with: Array("http://".utf8)) {
        rest = bytes.dropFirst(7)
    } else if bytes.starts(with: Array("https://".utf8)) {
        rest = bytes.dropFirst(8)
    } else {
        return false
    }
    let authority = rest.prefix { $0 != 0x2F && $0 != 0x3F && $0 != 0x23 }
    if authority.isEmpty || authority.first == 0x3A {
        return false
    }
    return !bytes.contains { $0 <= 0x20 || $0 == 0x7F }
}

/// Whether the arguments of an exchange are ones the operation accepts.
func acceptableExchange(method: String, url: String, headers: String) -> Bool {
    httpMethods.contains { $0.utf8.elementsEqual(method.utf8) }
        && isHTTPURL(url)
        && httpHeaderLines(headers) != nil
}
