// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_zenoh_push.scxml:45 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohPush`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohPushArms<A0: CodecZenohPushBodySource, D: CodecZenohPushBodySource> {
    case codecZenohPushBody(A0)
    case `default`(tag: CodecZenohPushUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohPushBody: return 0x1D
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohPushArms: Equatable where A0: Equatable, D: Equatable {}

extension CodecZenohPushArms: Sendable where A0: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohPush` holds, each arm the value of its own codec.
public typealias CodecZenohPushVariant = CodecZenohPushArms<CodecZenohPushBody, CodecZenohPushBody>

/// The tag the catch-all arm of `CodecZenohPush` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohPushUnknownTag: Equatable, Hashable, Sendable {
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
        case 0x1D: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecZenohPush` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohPush.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohPush`, which conforms itself.
public protocol CodecZenohPushSource {
    /// What the body of the arm `codecZenohPushBody` is presented as: a source of `CodecZenohPushBody`, written by its encode.
    associatedtype CodecZenohPushBody_Arm: CodecZenohPushBodySource
    /// What the body of the arm `codecZenohPushBody` is presented as: a source of `CodecZenohPushBody`, written by its encode.
    associatedtype Default_Arm: CodecZenohPushBodySource
    var header: UInt8 { get }
    var body: CodecZenohPushArms<CodecZenohPushBody_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohPush: Equatable, Sendable, CodecZenohPushSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + CodecZenohPushBody.frameLength

    /// The value presents the body of the arm `codecZenohPushBody` as a value of its own codec.
    public typealias CodecZenohPushBody_Arm = CodecZenohPushBody

    /// The value presents the body of the arm `codecZenohPushBody` as a value of its own codec.
    public typealias Default_Arm = CodecZenohPushBody

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1D

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// The 3 bits of `header` from bit 5 (flag `rest`). A value
    /// wider than the flag is cut to it when written.
    public var rest: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 5) & 0x7) }
        set { header = (header & ~(0x7 << 5)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x7) << 5) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohPushVariant = .codecZenohPushBody(CodecZenohPushBody())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecZenohPushArms<CodecZenohPushBody_Arm, Default_Arm>) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohPush {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let body: CodecZenohPushVariant
        let body_tag = (header >> 0) & 0x1F
        switch body_tag {
        case 0x1D:
            body = .codecZenohPushBody(try CodecZenohPushBody.decode(&reader))
        default:
            body = .default(tag: CodecZenohPushUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohPushBody.decode(&reader))
        }
        cursor = reader
        return CodecZenohPush(header: header, body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohPushSource` is written,
    /// with no copy into `CodecZenohPush`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohPushSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var header_folded = source.header
        header_folded = (header_folded & ~0x1F) | ((UInt8(truncatingIfNeeded: source.body.tag) & 0x1F) << 0)
        var frame: [UInt8] = []
        storeInteger(header_folded, into: &frame, at: 0, .big)
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohPushBody(let arm):
            try CodecZenohPushBody.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohPushBody.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
