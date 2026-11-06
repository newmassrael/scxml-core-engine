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

/// What every SCXML Event I/O Processor location begins with, before the session id it names.
let scxmlLocationPrefix = "sce://scxml/"

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

/// The origin the SCXML Event I/O Processor writes for an event the session `sender`
/// sends (docs/adr/0090, section 2).
///
/// §scxml-C-1 requires the origin of an event raised in the receiving session to match
/// the location the sending session publishes, so the pair is the processor's canonical
/// type and `scxmlProcessorLocation` of the sender, the one function that also writes
/// the session's `_ioprocessors` entry. The sender's id itself travels beside the pair,
/// as the event's `sender`, for the lookups that find a session by it. Port of the C++
/// `IOProcessorHelper::scxmlOrigin`.
public func scxmlOrigin(sender: String) -> EventOrigin {
    EventOrigin(processor: scxmlEventProcessorType, address: scxmlProcessorLocation(sessionID: sender))
}

/// `scxmlOrigin(sender:)` of a sender that may be absent: a machine that holds no
/// session id has no address to publish, so an event it sends carries no origin and
/// not the location of an empty id.
public func scxmlOrigin(of sender: String) -> EventOrigin? {
    sender.isEmpty ? nil : scxmlOrigin(sender: sender)
}

/// The `origin` text that stores an event's origin and sender in a saved state of format 1.
///
/// docs/adr/0090, section 7: a state read from a format-1 text and written back without a
/// machine stays format 1, which has no place for the pair, so its `origin` is written by the
/// rule format 2 used: the sending session's id for an event a session sent, the address
/// otherwise, and empty for none. The `origintype` beside it is the pair's processor.
/// `readStoredOrigin` reads it back.
public func storedOriginText(_ origin: EventOrigin?, sender: String?) -> String {
    if let sender, !sender.isEmpty {
        return sender
    }
    return origin?.address ?? ""
}

/// The origin and sender a stored `origin` and `origintype` of a format-1 or format-2 saved
/// state are read as.
///
/// docs/adr/0090, section 7. An empty `origin` is none whatever type was stored beside
/// it: an empty address answers nothing, and format 2 wrote the SCXML type beside it for
/// events no processor stamped. An empty `origintype` beside an address is the SCXML
/// processor's, which is what format 2 meant by it. A value without a scheme read with
/// the SCXML type is a session's id, read as that session's pair with the id as its
/// sender; an SCXML location (which one backend stored in place of the id) is read the
/// same way by the id it names. Anything else is the pair as stored, with no sender.
/// Never half a pair. Port of the C++ `IOProcessorHelper::readStoredOrigin`.
public func readStoredOrigin(_ origin: String, originType: String) -> (origin: EventOrigin?, sender: String?) {
    if origin.isEmpty {
        return (nil, nil)
    }
    let located = sessionIDFromScxmlLocation(origin)
    if !located.isEmpty {
        return (scxmlOrigin(sender: located), located)
    }
    let processor = originType.isEmpty ? scxmlEventProcessorType : originType
    let bareID = !origin.utf8.containsSubsequence("://".utf8)
    if bareID && processor.utf8.elementsEqual(scxmlEventProcessorType.utf8) {
        return (scxmlOrigin(sender: origin), origin)
    }
    return (EventOrigin(processor: processor, address: origin), nil)
}

/// The sender a stored waiting send's `origin` of a format-1 or format-2 saved state names.
///
/// docs/adr/0090, section 7: a waiting send is always a session's, so a bare id is that
/// id and an SCXML location names it. Port of the C++
/// `IOProcessorHelper::readStoredSender`.
public func readStoredSender(_ origin: String) -> String {
    let located = sessionIDFromScxmlLocation(origin)
    return located.isEmpty ? origin : located
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
