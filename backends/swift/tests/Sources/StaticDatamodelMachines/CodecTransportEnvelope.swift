// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_transport_envelope.scxml:68 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecTransportEnvelope`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecTransportEnvelopeArms<A0: CodecZenohInitBodySource, A1: CodecZenohOpenBodySource, A2: CodecZenohCloseSource, A3: CodecZenohKeepAliveSource, A4: CodecZenohFrameSource, A5: CodecZenohFragmentSource, A6: CodecZenohJoinSource, D: CodecZenohCloseSource> {
    case codecZenohInitBody(A0)
    case codecZenohOpenBody(A1)
    case codecZenohClose(A2)
    case codecZenohKeepAlive(A3)
    case codecZenohFrame(A4)
    case codecZenohFragment(A5)
    case codecZenohJoin(A6)
    case `default`(tag: CodecTransportEnvelopeUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohInitBody: return 0x1
        case .codecZenohOpenBody: return 0x2
        case .codecZenohClose: return 0x3
        case .codecZenohKeepAlive: return 0x4
        case .codecZenohFrame: return 0x5
        case .codecZenohFragment: return 0x6
        case .codecZenohJoin: return 0x7
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecTransportEnvelopeArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, A3: Equatable, A4: Equatable, A5: Equatable, A6: Equatable, D: Equatable {}

extension CodecTransportEnvelopeArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, A3: Sendable, A4: Sendable, A5: Sendable, A6: Sendable, D: Sendable {}

/// The arm a value of `CodecTransportEnvelope` holds, each arm the value of its own codec.
public typealias CodecTransportEnvelopeVariant = CodecTransportEnvelopeArms<CodecZenohInitBody, CodecZenohOpenBody, CodecZenohClose, CodecZenohKeepAlive, CodecZenohFrame, CodecZenohFragment, CodecZenohJoin, CodecZenohClose>

/// The tag the catch-all arm of `CodecTransportEnvelope` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecTransportEnvelopeUnknownTag: Equatable, Hashable, Sendable {
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
        case 0x1...0x7: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecTransportEnvelope` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecTransportEnvelope.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecTransportEnvelope`, which conforms itself.
public protocol CodecTransportEnvelopeSource {
    /// What the body of the arm `codecZenohInitBody` is presented as: a source of `CodecZenohInitBody`, written by its encode.
    associatedtype CodecZenohInitBody_Arm: CodecZenohInitBodySource
    /// What the body of the arm `codecZenohOpenBody` is presented as: a source of `CodecZenohOpenBody`, written by its encode.
    associatedtype CodecZenohOpenBody_Arm: CodecZenohOpenBodySource
    /// What the body of the arm `codecZenohClose` is presented as: a source of `CodecZenohClose`, written by its encode.
    associatedtype CodecZenohClose_Arm: CodecZenohCloseSource
    /// What the body of the arm `codecZenohKeepAlive` is presented as: a source of `CodecZenohKeepAlive`, written by its encode.
    associatedtype CodecZenohKeepAlive_Arm: CodecZenohKeepAliveSource
    /// What the body of the arm `codecZenohFrame` is presented as: a source of `CodecZenohFrame`, written by its encode.
    associatedtype CodecZenohFrame_Arm: CodecZenohFrameSource
    /// What the body of the arm `codecZenohFragment` is presented as: a source of `CodecZenohFragment`, written by its encode.
    associatedtype CodecZenohFragment_Arm: CodecZenohFragmentSource
    /// What the body of the arm `codecZenohJoin` is presented as: a source of `CodecZenohJoin`, written by its encode.
    associatedtype CodecZenohJoin_Arm: CodecZenohJoinSource
    /// What the body of the arm `codecZenohClose` is presented as: a source of `CodecZenohClose`, written by its encode.
    associatedtype Default_Arm: CodecZenohCloseSource
    var header: UInt8 { get }
    var body: CodecTransportEnvelopeArms<CodecZenohInitBody_Arm, CodecZenohOpenBody_Arm, CodecZenohClose_Arm, CodecZenohKeepAlive_Arm, CodecZenohFrame_Arm, CodecZenohFragment_Arm, CodecZenohJoin_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecTransportEnvelope: Equatable, Sendable, CodecTransportEnvelopeSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecZenohInitBody.frameLength, CodecZenohOpenBody.frameLength, CodecZenohClose.frameLength, CodecZenohKeepAlive.frameLength, CodecZenohFrame.frameLength, CodecZenohFragment.frameLength, CodecZenohJoin.frameLength)

    /// The value presents the body of the arm `codecZenohInitBody` as a value of its own codec.
    public typealias CodecZenohInitBody_Arm = CodecZenohInitBody

    /// The value presents the body of the arm `codecZenohOpenBody` as a value of its own codec.
    public typealias CodecZenohOpenBody_Arm = CodecZenohOpenBody

    /// The value presents the body of the arm `codecZenohClose` as a value of its own codec.
    public typealias CodecZenohClose_Arm = CodecZenohClose

    /// The value presents the body of the arm `codecZenohKeepAlive` as a value of its own codec.
    public typealias CodecZenohKeepAlive_Arm = CodecZenohKeepAlive

    /// The value presents the body of the arm `codecZenohFrame` as a value of its own codec.
    public typealias CodecZenohFrame_Arm = CodecZenohFrame

    /// The value presents the body of the arm `codecZenohFragment` as a value of its own codec.
    public typealias CodecZenohFragment_Arm = CodecZenohFragment

    /// The value presents the body of the arm `codecZenohJoin` as a value of its own codec.
    public typealias CodecZenohJoin_Arm = CodecZenohJoin

    /// The value presents the body of the arm `codecZenohClose` as a value of its own codec.
    public typealias Default_Arm = CodecZenohClose

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 5 bits of `header` from bit 0 (flag `mid`). A value
    /// wider than the flag is cut to it when written.
    public var mid: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 5 of `header` (flag `A`).
    public var A: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 6 of `header` (flag `S`).
    public var S: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecTransportEnvelopeVariant = .codecZenohClose(CodecZenohClose())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, body: CodecTransportEnvelopeArms<CodecZenohInitBody_Arm, CodecZenohOpenBody_Arm, CodecZenohClose_Arm, CodecZenohKeepAlive_Arm, CodecZenohFrame_Arm, CodecZenohFragment_Arm, CodecZenohJoin_Arm, Default_Arm>) {
        self.header = header
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecTransportEnvelope {
        var reader = cursor
        let header = try reader.readInteger(UInt8.self, .big)
        let body: CodecTransportEnvelopeVariant
        let body_tag = (header >> 0) & 0x1F
        switch body_tag {
        case 0x1:
            body = .codecZenohInitBody(try CodecZenohInitBody.decode(&reader, S: UInt8(truncatingIfNeeded: (header >> 6) & 0x1), A: UInt8(truncatingIfNeeded: (header >> 5) & 0x1)))
        case 0x2:
            body = .codecZenohOpenBody(try CodecZenohOpenBody.decode(&reader, A: UInt8(truncatingIfNeeded: (header >> 5) & 0x1)))
        case 0x3:
            body = .codecZenohClose(try CodecZenohClose.decode(&reader))
        case 0x4:
            body = .codecZenohKeepAlive(try CodecZenohKeepAlive.decode(&reader))
        case 0x5:
            body = .codecZenohFrame(try CodecZenohFrame.decode(&reader))
        case 0x6:
            body = .codecZenohFragment(try CodecZenohFragment.decode(&reader))
        case 0x7:
            body = .codecZenohJoin(try CodecZenohJoin.decode(&reader, S: UInt8(truncatingIfNeeded: (header >> 6) & 0x1)))
        default:
            body = .default(tag: CodecTransportEnvelopeUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohClose.decode(&reader))
        }
        cursor = reader
        return CodecTransportEnvelope(header: header, body: body)
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecTransportEnvelope` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `Z` flag of `header`, where a parent sets it.
        public var headerZ: Bool?

        public init(headerZ: Bool? = nil) {
            self.headerZ = headerZ
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecTransportEnvelopeSource` is written,
    /// with no copy into `CodecTransportEnvelope`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecTransportEnvelopeSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_A_derived_said2: Int? = nil
        if case .codecZenohInitBody(let arm) = source.body { header_A_derived_said2 = Int(CodecZenohInitBody.inputA(arm)) }
        if case .codecZenohOpenBody(let arm) = source.body { header_A_derived_said2 = Int(CodecZenohOpenBody.inputA(arm)) }
        var header_folded = source.header
        if header_A_derived_said2 != nil { header_folded = (header_folded & ~0x20) | ((UInt8(truncatingIfNeeded: (header_A_derived_said2 ?? 0)) & 0x1) << 5) }
        var header_S_derived_said2: Int? = nil
        if case .codecZenohInitBody(let arm) = source.body { header_S_derived_said2 = Int(CodecZenohInitBody.inputS(arm)) }
        if case .codecZenohJoin(let arm) = source.body { header_S_derived_said2 = Int(CodecZenohJoin.inputS(arm)) }
        if header_S_derived_said2 != nil { header_folded = (header_folded & ~0x40) | ((UInt8(truncatingIfNeeded: (header_S_derived_said2 ?? 0)) & 0x1) << 6) }
        header_folded = (header_folded & ~0x1F) | ((UInt8(truncatingIfNeeded: source.body.tag) & 0x1F) << 0)
        if let set = overrides.headerZ { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        var frame: [UInt8] = []
        storeInteger(header_folded, into: &frame, at: 0, .big)
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohInitBody(let arm):
            try CodecZenohInitBody.encodeSource(arm, into: &body_arm)
        case .codecZenohOpenBody(let arm):
            try CodecZenohOpenBody.encodeSource(arm, into: &body_arm)
        case .codecZenohClose(let arm):
            try CodecZenohClose.encodeSource(arm, into: &body_arm)
        case .codecZenohKeepAlive(let arm):
            try CodecZenohKeepAlive.encodeSource(arm, into: &body_arm)
        case .codecZenohFrame(let arm):
            try CodecZenohFrame.encodeSource(arm, into: &body_arm)
        case .codecZenohFragment(let arm):
            try CodecZenohFragment.encodeSource(arm, into: &body_arm)
        case .codecZenohJoin(let arm):
            try CodecZenohJoin.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohClose.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
