// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

/// One entry of the `_ioprocessors` system variable: the key it is filed under,
/// and the address external entities use to reach this session through that
/// processor.
public struct IoProcessorDescriptor: Sendable, Equatable {
    public var name: String
    public var location: String

    public init(name: String, location: String) {
        self.name = name
        self.location = location
    }
}

/// The alias the SCXML Event I/O Processor is indexed under by SCXML documents.
public let scxmlProcessorAlias = "scxml"

/// The alias the Basic HTTP Event I/O Processor is indexed under by SCXML
/// documents.
public let basicHTTPProcessorAlias = "basichttp"

private let scxmlLocationPrefix = "sce://scxml/"

/// The address that reaches this session over the SCXML Event I/O Processor.
///
/// §scxml-C-1 leaves the transport platform-specific, so the address is an
/// SCE-scheme URI naming the session. The session id is percent-encoded because it
/// is not constrained to URI-safe characters.
public func scxmlProcessorLocation(sessionID: String) -> String {
    scxmlLocationPrefix + percentEncode(sessionID)
}

/// The session id an SCXML Event I/O Processor location names, or the empty text
/// when the argument is not such a location.
///
/// The inverse of `scxmlProcessorLocation`, kept beside it so the two spellings of
/// one address cannot drift apart. §scxml-C-1 requires the location a session
/// publishes to be usable as a `<send>` target, which only holds if something can
/// read a session back out of it.
public func sessionIDFromScxmlLocation(_ uri: String) -> String {
    let bytes = Array(uri.utf8)
    let prefix = Array(scxmlLocationPrefix.utf8)
    if bytes.count <= prefix.count || !bytes.starts(with: prefix) {
        return ""
    }
    return percentDecode(Array(bytes[prefix.count...]))
}

/// The `_event.origin` a receiver should see for an event sent by
/// `originSessionID`.
///
/// §scxml-C-1 requires the origin of a delivered event to match the 'location' the
/// sending session published, which is what makes it an address the receiver can
/// answer. The engine carries the sender's bare session id internally
/// (`EventMetadata.origin`) because its session-keyed lookups (`<finalize>`
/// dispatch, cancelled-invoke filtering) match on the id. Converting where the
/// event is raised would make one value serve two consumers that need different
/// spellings. So the conversion belongs at the boundary where the value becomes
/// visible to the document, and this is that conversion: the same rule, and the
/// same shape, as the C++ `IOProcessorHelper::publishedOrigin` both engines already
/// share.
///
/// A remote invoke is the case that makes this more than a rename: its child
/// session is stamped with a URI and not an id, and wrapping a URI in
/// `scxmlProcessorLocation` would produce an address naming nothing. An argument
/// that already carries a scheme is therefore passed through, since it is already an
/// address.
public func publishedOrigin(originSessionID: String) -> String {
    if originSessionID.isEmpty {
        return ""
    }
    if originSessionID.utf8.containsSubsequence("://".utf8) {
        return originSessionID
    }
    return scxmlProcessorLocation(sessionID: originSessionID)
}

/// The `_ioprocessors` entry set for a session.
///
/// Port of the C++ `IOProcessorHelper` (`sce/include/common/IOProcessorHelper.h`).
/// Deciding the entries here and not inside each script engine is what keeps a
/// machine reading the same entry names and the same addresses whichever backend
/// runs it.
///
/// Every processor is filed twice: under the specification's entry name
/// (§scxml-C-1-1, §scxml-C-2-3) and under the short alias SCXML documents index
/// with. Both keys carry the same location, so the choice of spelling never changes
/// where an event goes.
///
/// The BasicHTTP entry appears only when `basicHTTPAccessURI` is non-empty. Support
/// for that processor is optional and per deployment, so a session with no inbound
/// endpoint advertises no address and not one nothing answers on.
public func buildIoProcessors(sessionID: String, basicHTTPAccessURI: String) -> [IoProcessorDescriptor] {
    let scxmlURI = scxmlProcessorLocation(sessionID: sessionID)
    var descriptors = [
        IoProcessorDescriptor(name: scxmlEventProcessorType, location: scxmlURI),
        IoProcessorDescriptor(name: scxmlProcessorAlias, location: scxmlURI),
    ]
    if !basicHTTPAccessURI.isEmpty {
        descriptors.append(IoProcessorDescriptor(name: basicHTTPEventProcessorType, location: basicHTTPAccessURI))
        descriptors.append(IoProcessorDescriptor(name: basicHTTPProcessorAlias, location: basicHTTPAccessURI))
    }
    return descriptors
}

/// RFC 3986 percent-encoding, leaving the unreserved set (`A-Za-z0-9-._~`) intact.
/// Walks the UTF-8 bytes, as every other backend does.
func percentEncode(_ value: String) -> String {
    let hex = Array("0123456789ABCDEF".utf8)
    var encoded: [UInt8] = []
    encoded.reserveCapacity(value.utf8.count)
    for c in value.utf8 {
        let unreserved =
            (c >= 0x41 && c <= 0x5A) || (c >= 0x61 && c <= 0x7A) || (c >= 0x30 && c <= 0x39)
            || c == 0x2D || c == 0x5F || c == 0x2E || c == 0x7E
        if unreserved {
            encoded.append(c)
            continue
        }
        encoded.append(UInt8(ascii: "%"))
        encoded.append(hex[Int(c >> 4)])
        encoded.append(hex[Int(c & 0xF)])
    }
    return String(decoding: encoded, as: UTF8.self)
}

/// Reverses `percentEncode`. A malformed escape is left verbatim and not dropped:
/// the input is an address a document supplied, and silently rewriting it would turn
/// a bad target into a different valid one.
func percentDecode(_ value: [UInt8]) -> String {
    var decoded: [UInt8] = []
    decoded.reserveCapacity(value.count)
    var i = 0
    while i < value.count {
        if value[i] == UInt8(ascii: "%") && i + 2 < value.count,
            let hi = hexNibble(value[i + 1]), let lo = hexNibble(value[i + 2])
        {
            decoded.append(hi << 4 | lo)
            i += 3
            continue
        }
        decoded.append(value[i])
        i += 1
    }
    return String(decoding: decoded, as: UTF8.self)
}

private func hexNibble(_ c: UInt8) -> UInt8? {
    switch c {
    case 0x30...0x39: return c - 0x30
    case 0x61...0x66: return c - 0x61 + 10
    case 0x41...0x46: return c - 0x41 + 10
    default: return nil
    }
}

extension Sequence where Element == UInt8 {
    /// Whether `needle` occurs in this byte sequence.
    func containsSubsequence<Needle: Collection>(_ needle: Needle) -> Bool where Needle.Element == UInt8 {
        let haystack = Array(self)
        let pattern = Array(needle)
        if pattern.isEmpty {
            return true
        }
        if haystack.count < pattern.count {
            return false
        }
        for start in 0...(haystack.count - pattern.count)
        where haystack[start..<start + pattern.count].elementsEqual(pattern) {
            return true
        }
        return false
    }
}
