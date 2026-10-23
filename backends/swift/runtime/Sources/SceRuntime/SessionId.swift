// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The id of a root session, the value of the system variable `_sessionid`
// (docs/adr/0045, "Questions the owner answered" 2).
//
// `_sessionid` is bound when a session is loaded and kept until it ends, and it
// is an XML Nmtoken: another session addresses this one as `#_scxml_<id>`, and
// every child's id is its parent's and the id the child was started under,
// `<parent>.<invokeid>`. So a root's id is never empty. The host may give it when
// it creates the machine (`sessionIDRefusalOf` tells it when what it gives is not
// an Nmtoken, and the id is not replaced), and where it gives none the runtime
// draws one (`drawSessionID`), a UUID version 4 in lowercase hexadecimal with
// hyphens, the one spelling every backend draws, so no reader can tell which
// backend drew it. A restore binds the saved id instead of either, since the
// session carries on.
//
// Cross-language siblings: `com.sce.runtime.SessionId` (Kotlin),
// `sce_rust_runtime::helpers::session_id` (Rust) and `SessionIDRefusalOf` (Go).

/// Why `id` is not an id a session may have, or the empty text when it is one:
/// an XML 1.0 Nmtoken, one name character or more (production `Nmtoken`, fifth
/// edition).
///
/// A Swift `String` holds well-formed Unicode and repairs a stray byte when it is
/// made, so the Go and Rust refusal of a byte that is no UTF-8 has nothing to act
/// on here; it is the caller's decoding that decides it.
public func sessionIDRefusalOf(_ id: String) -> String {
    if id.isEmpty {
        return "the session id is empty, and a session id is one name character or more (an XML Nmtoken)"
    }
    for scalar in id.unicodeScalars where !isNameChar(scalar.value) {
        let hex = String(scalar.value, radix: 16, uppercase: true)
        let padded = String(repeating: "0", count: max(0, 4 - hex.count)) + hex
        return "the session id holds U+\(padded), which is not a name character: a session id is an "
            + "XML Nmtoken (letters, digits, '.', '-', '_', ':')"
    }
    return ""
}

/// Whether `id` is an XML 1.0 Nmtoken.
public func isNmtoken(_ id: String) -> Bool {
    sessionIDRefusalOf(id).isEmpty
}

/// A session id drawn from the platform's randomness, a UUID version 4 (RFC 9562,
/// section 5.4) in lowercase hexadecimal in groups of 8, 4, 4, 4 and 12, for a
/// session whose host gave none.
public func drawSessionID() -> String {
    var generator = SystemRandomNumberGenerator()
    var bytes = (0..<16).map { _ in UInt8.random(in: 0...255, using: &generator) }
    bytes[6] = bytes[6] & 0x0F | 0x40
    bytes[8] = bytes[8] & 0x3F | 0x80
    let hex = bytes.map { byte -> String in
        let digits = String(byte, radix: 16)
        return digits.count == 1 ? "0" + digits : digits
    }
    return [hex[0..<4], hex[4..<6], hex[6..<8], hex[8..<10], hex[10..<16]]
        .map { $0.joined() }
        .joined(separator: "-")
}

/// XML 1.0 `NameStartChar`.
private func isNameStartChar(_ c: UInt32) -> Bool {
    c == 0x3A || (0x41...0x5A).contains(c) || c == 0x5F || (0x61...0x7A).contains(c)
        || (0xC0...0xD6).contains(c) || (0xD8...0xF6).contains(c) || (0xF8...0x2FF).contains(c)
        || (0x370...0x37D).contains(c) || (0x37F...0x1FFF).contains(c) || (0x200C...0x200D).contains(c)
        || (0x2070...0x218F).contains(c) || (0x2C00...0x2FEF).contains(c) || (0x3001...0xD7FF).contains(c)
        || (0xF900...0xFDCF).contains(c) || (0xFDF0...0xFFFD).contains(c) || (0x10000...0xEFFFF).contains(c)
}

/// XML 1.0 `NameChar`.
private func isNameChar(_ c: UInt32) -> Bool {
    isNameStartChar(c) || c == 0x2D || c == 0x2E || (0x30...0x39).contains(c) || c == 0xB7
        || (0x300...0x36F).contains(c) || (0x203F...0x2040).contains(c)
}
