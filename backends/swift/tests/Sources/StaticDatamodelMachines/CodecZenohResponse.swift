// SCE-GENERATED — DO NOT EDIT
// source-hash: eb0d58f29a9e7f796442667316a0932ca5aec3c4d623b93aa53a57ba6820d0ae
// source: codec_zenoh_response.scxml
// SCE-MAP: codec_zenoh_response.scxml:76 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohResponse`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohResponseArms<A0: CodecZenohReplySource, A1: CodecZenohErrSource, D: CodecZenohReplySource> {
    case codecZenohReply(A0)
    case codecZenohErr(A1)
    case `default`(tag: CodecZenohResponseUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohReply: return 0x4
        case .codecZenohErr: return 0x5
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohResponseArms: Equatable where A0: Equatable, A1: Equatable, D: Equatable {}

extension CodecZenohResponseArms: Sendable where A0: Sendable, A1: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohResponse` holds, each arm the value of its own codec.
public typealias CodecZenohResponseVariant = CodecZenohResponseArms<CodecZenohReply, CodecZenohErr, CodecZenohReply>

/// The tag the catch-all arm of `CodecZenohResponse` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohResponseUnknownTag: Equatable, Hashable, Sendable {
    /// The tag.
    public let value: UInt8

    /// The tag `value`, or `nil` where an arm names it or its bits do not hold it.
    public init?(_ value: UInt8) {
        guard Self.admits(value) else { return nil }
        self.value = value
    }

    /// A tag the decode read where no arm names one, or the least one the catch-all admits.
    fileprivate init(admitted value: UInt8) {
        self.value = value
    }

    /// Whether the catch-all may hold `value`.
    public static func admits(_ value: UInt8) -> Bool {
        switch value {
        case 0x4...0x5: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecZenohResponse` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohResponse.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohResponse`, which conforms itself.
public protocol CodecZenohResponseSource {
    /// What each entry of `extensions` is presented as: a source of `CodecZenohExtEntry`, written by its encode.
    associatedtype Extensions_Entry: CodecZenohExtEntrySource
    /// What the body of the arm `codecZenohReply` is presented as: a source of `CodecZenohReply`, written by its encode.
    associatedtype CodecZenohReply_Arm: CodecZenohReplySource
    /// What the body of the arm `codecZenohErr` is presented as: a source of `CodecZenohErr`, written by its encode.
    associatedtype CodecZenohErr_Arm: CodecZenohErrSource
    /// What the body of the arm `codecZenohReply` is presented as: a source of `CodecZenohReply`, written by its encode.
    associatedtype Default_Arm: CodecZenohReplySource
    var header: UInt8 { get }
    var request_id: UInt64 { get }
    var key_id: UInt32 { get }
    var suffix: BoundedText<CodecZenohResponse.SuffixBound>? { get }
    var extensions: BoundedList<Extensions_Entry, CodecZenohResponse.ExtensionsBound>? { get }
    var body: CodecZenohResponseArms<CodecZenohReply_Arm, CodecZenohErr_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 7 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohResponse: Equatable, Sendable, CodecZenohResponseSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 3 + Swift.min(CodecZenohReply.frameLength, CodecZenohErr.frameLength)

    /// How many bytes `suffix` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum SuffixBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 65535
    }

    /// How many entries `extensions` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R2, R7): its type is held to it where a value of it is made.
    public enum ExtensionsBound: SceCountBound {
        public static let least = 1
        public static let most: Int? = nil
    }

    /// The value presents each entry of `extensions` as a value of its own codec.
    public typealias Extensions_Entry = CodecZenohExtEntry

    /// The value presents the body of the arm `codecZenohReply` as a value of its own codec.
    public typealias CodecZenohReply_Arm = CodecZenohReply

    /// The value presents the body of the arm `codecZenohErr` as a value of its own codec.
    public typealias CodecZenohErr_Arm = CodecZenohErr

    /// The value presents the body of the arm `codecZenohReply` as a value of its own codec.
    public typealias Default_Arm = CodecZenohReply

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1B

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `N`).
    public var N: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `M`).
    public var M: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var request_id: UInt64 = 0

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var key_id: UInt32 = 0

    /// On the wire only when `(header & 0x20) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The text of as many UTF-8 bytes as `suffix_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, and a value with more is no value of its type; the encode derives what it says.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var suffix: BoundedText<CodecZenohResponse.SuffixBound>? = nil

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var extensions: BoundedList<CodecZenohExtEntry, CodecZenohResponse.ExtensionsBound>? = nil

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohResponseVariant = .codecZenohReply(CodecZenohReply())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, request_id: UInt64, key_id: UInt32, suffix: BoundedText<CodecZenohResponse.SuffixBound>?, extensions: BoundedList<Extensions_Entry, CodecZenohResponse.ExtensionsBound>?, body: CodecZenohResponseArms<CodecZenohReply_Arm, CodecZenohErr_Arm, Default_Arm>) {
        self.header = header
        self.request_id = request_id
        self.key_id = key_id
        self.suffix = suffix
        self.extensions = extensions
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohResponse {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let request_id = try reader.readVle(UInt64.self)
        let key_id = try reader.readVle(UInt32.self)
        var suffix_len: UInt64? = nil
        if (header & 0x20) != 0 {
            suffix_len = try reader.readVle(UInt64.self)
        }
        var suffix: BoundedText<CodecZenohResponse.SuffixBound>? = nil
        if (header & 0x20) != 0 {
            if (suffix_len ?? 0) > 65535 {
                throw CodecError.lengthOutOfRange
            }
            suffix = try BoundedText<CodecZenohResponse.SuffixBound>(decoding: try reader.readString(Int(clamping: (suffix_len ?? 0))))
        }
        var extensions: BoundedList<CodecZenohExtEntry, CodecZenohResponse.ExtensionsBound>? = nil
        if (header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            while true {
                let entry = try CodecZenohExtEntry.decode(&reader)
                extensions_more = entry.Z
                extensions_items.append(entry)
                if !extensions_more { break }
            }
            extensions = try BoundedList<CodecZenohExtEntry, CodecZenohResponse.ExtensionsBound>(decoding: extensions_items)
        }
        let body: CodecZenohResponseVariant
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1F
        switch body_tag {
        case 0x4:
            body = .codecZenohReply(try CodecZenohReply.decode(&reader))
        case 0x5:
            body = .codecZenohErr(try CodecZenohErr.decode(&reader))
        default:
            body = .default(tag: CodecZenohResponseUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohReply.decode(&reader))
        }
        cursor = reader
        return CodecZenohResponse(header: header, request_id: request_id, key_id: key_id, suffix: suffix, extensions: extensions, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecZenohResponse` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `M` flag of `header`, where a parent sets it.
        public var headerM: Bool?

        public init(headerM: Bool? = nil) {
            self.headerM = headerM
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohResponseSource` is written,
    /// with no copy into `CodecZenohResponse`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohResponseSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        let suffix_len_derived: UInt64 = source.suffix != nil ? UInt64((source.suffix?.byteCount ?? 0)) : 0
        var header_folded = source.header
        header_folded = (header_folded & ~0x20) | ((source.suffix != nil) ? 0x20 : 0)
        header_folded = (header_folded & ~0x80) | ((source.extensions != nil) ? 0x80 : 0)
        if let set = overrides.headerM { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        var frame: [UInt8] = []
        appendInteger(header_folded, to: &frame, .big)
        appendVle(source.request_id, to: &frame)
        appendVle(source.key_id, to: &frame)
        if ((header_folded >> 5) & 1) != 0 {
            appendVle(suffix_len_derived, to: &frame)
        }
        if let held = source.suffix {
            held.append(to: &frame)
        }
        if let held = source.extensions {
            var extensions_entries = BytesSink()
            var extensions_left = held.count
            for entry in held {
                extensions_left -= 1
                try CodecZenohExtEntry.encodeSource(entry, into: &extensions_entries, overrides: CodecZenohExtEntry.Overrides(headerZ: extensions_left != 0))
            }
            frame += extensions_entries.bytes
        }
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohReply(let arm):
            try CodecZenohReply.encodeSource(arm, into: &body_arm)
        case .codecZenohErr(let arm):
            try CodecZenohErr.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohReply.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
