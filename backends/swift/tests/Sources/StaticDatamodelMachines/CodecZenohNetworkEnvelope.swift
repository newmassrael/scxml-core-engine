// SCE-GENERATED — DO NOT EDIT
// source-hash: 33abc34409c91565fc6cfdeec8aeb2e5268c909190bd30b2b38d8563435104c0
// source: codec_zenoh_network_envelope.scxml
// SCE-MAP: codec_zenoh_network_envelope.scxml:60 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecZenohNetworkEnvelope`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecZenohNetworkEnvelopeArms<A0: CodecZenohInterestSource, A1: CodecZenohResponseFinalSource, A2: CodecZenohResponseSource, A3: CodecZenohRequestSource, A4: CodecZenohPushSource, A5: CodecZenohDeclareSource, A6: CodecZenohOamSource, D: CodecZenohOamSource> {
    case codecZenohInterest(A0)
    case codecZenohResponseFinal(A1)
    case codecZenohResponse(A2)
    case codecZenohRequest(A3)
    case codecZenohPush(A4)
    case codecZenohDeclare(A5)
    case codecZenohOam(A6)
    case `default`(tag: CodecZenohNetworkEnvelopeUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecZenohInterest: return 0x19
        case .codecZenohResponseFinal: return 0x1A
        case .codecZenohResponse: return 0x1B
        case .codecZenohRequest: return 0x1C
        case .codecZenohPush: return 0x1D
        case .codecZenohDeclare: return 0x1E
        case .codecZenohOam: return 0x1F
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecZenohNetworkEnvelopeArms: Equatable where A0: Equatable, A1: Equatable, A2: Equatable, A3: Equatable, A4: Equatable, A5: Equatable, A6: Equatable, D: Equatable {}

extension CodecZenohNetworkEnvelopeArms: Sendable where A0: Sendable, A1: Sendable, A2: Sendable, A3: Sendable, A4: Sendable, A5: Sendable, A6: Sendable, D: Sendable {}

/// The arm a value of `CodecZenohNetworkEnvelope` holds, each arm the value of its own codec.
public typealias CodecZenohNetworkEnvelopeVariant = CodecZenohNetworkEnvelopeArms<CodecZenohInterest, CodecZenohResponseFinal, CodecZenohResponse, CodecZenohRequest, CodecZenohPush, CodecZenohDeclare, CodecZenohOam, CodecZenohOam>

/// The tag the catch-all arm of `CodecZenohNetworkEnvelope` holds (docs/adr/0065 §6 R3): one no arm names,
/// within the bits the tag is read from. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecZenohNetworkEnvelopeUnknownTag: Equatable, Hashable, Sendable {
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
        case 0x19...0x1F: return false
        default: return value <= 0x1F
        }
    }
}

/// The elements a value of `CodecZenohNetworkEnvelope` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohNetworkEnvelope.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohNetworkEnvelope`, which conforms itself.
public protocol CodecZenohNetworkEnvelopeSource {
    /// What the body of the arm `codecZenohInterest` is presented as: a source of `CodecZenohInterest`, written by its encode.
    associatedtype CodecZenohInterest_Arm: CodecZenohInterestSource
    /// What the body of the arm `codecZenohResponseFinal` is presented as: a source of `CodecZenohResponseFinal`, written by its encode.
    associatedtype CodecZenohResponseFinal_Arm: CodecZenohResponseFinalSource
    /// What the body of the arm `codecZenohResponse` is presented as: a source of `CodecZenohResponse`, written by its encode.
    associatedtype CodecZenohResponse_Arm: CodecZenohResponseSource
    /// What the body of the arm `codecZenohRequest` is presented as: a source of `CodecZenohRequest`, written by its encode.
    associatedtype CodecZenohRequest_Arm: CodecZenohRequestSource
    /// What the body of the arm `codecZenohPush` is presented as: a source of `CodecZenohPush`, written by its encode.
    associatedtype CodecZenohPush_Arm: CodecZenohPushSource
    /// What the body of the arm `codecZenohDeclare` is presented as: a source of `CodecZenohDeclare`, written by its encode.
    associatedtype CodecZenohDeclare_Arm: CodecZenohDeclareSource
    /// What the body of the arm `codecZenohOam` is presented as: a source of `CodecZenohOam`, written by its encode.
    associatedtype CodecZenohOam_Arm: CodecZenohOamSource
    /// What the body of the arm `codecZenohOam` is presented as: a source of `CodecZenohOam`, written by its encode.
    associatedtype Default_Arm: CodecZenohOamSource
    var body: CodecZenohNetworkEnvelopeArms<CodecZenohInterest_Arm, CodecZenohResponseFinal_Arm, CodecZenohResponse_Arm, CodecZenohRequest_Arm, CodecZenohPush_Arm, CodecZenohDeclare_Arm, CodecZenohOam_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohNetworkEnvelope: Equatable, Sendable, CodecZenohNetworkEnvelopeSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecZenohInterest.frameLength, CodecZenohResponseFinal.frameLength, CodecZenohResponse.frameLength, CodecZenohRequest.frameLength, CodecZenohPush.frameLength, CodecZenohDeclare.frameLength, CodecZenohOam.frameLength)

    /// The value presents the body of the arm `codecZenohInterest` as a value of its own codec.
    public typealias CodecZenohInterest_Arm = CodecZenohInterest

    /// The value presents the body of the arm `codecZenohResponseFinal` as a value of its own codec.
    public typealias CodecZenohResponseFinal_Arm = CodecZenohResponseFinal

    /// The value presents the body of the arm `codecZenohResponse` as a value of its own codec.
    public typealias CodecZenohResponse_Arm = CodecZenohResponse

    /// The value presents the body of the arm `codecZenohRequest` as a value of its own codec.
    public typealias CodecZenohRequest_Arm = CodecZenohRequest

    /// The value presents the body of the arm `codecZenohPush` as a value of its own codec.
    public typealias CodecZenohPush_Arm = CodecZenohPush

    /// The value presents the body of the arm `codecZenohDeclare` as a value of its own codec.
    public typealias CodecZenohDeclare_Arm = CodecZenohDeclare

    /// The value presents the body of the arm `codecZenohOam` as a value of its own codec.
    public typealias CodecZenohOam_Arm = CodecZenohOam

    /// The value presents the body of the arm `codecZenohOam` as a value of its own codec.
    public typealias Default_Arm = CodecZenohOam

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecZenohNetworkEnvelopeVariant = .codecZenohOam(CodecZenohOam())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecZenohNetworkEnvelopeArms<CodecZenohInterest_Arm, CodecZenohResponseFinal_Arm, CodecZenohResponse_Arm, CodecZenohRequest_Arm, CodecZenohPush_Arm, CodecZenohDeclare_Arm, CodecZenohOam_Arm, Default_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohNetworkEnvelope {
        var reader = cursor
        let body: CodecZenohNetworkEnvelopeVariant
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1F
        switch body_tag {
        case 0x19:
            body = .codecZenohInterest(try CodecZenohInterest.decode(&reader))
        case 0x1A:
            body = .codecZenohResponseFinal(try CodecZenohResponseFinal.decode(&reader))
        case 0x1B:
            body = .codecZenohResponse(try CodecZenohResponse.decode(&reader))
        case 0x1C:
            body = .codecZenohRequest(try CodecZenohRequest.decode(&reader))
        case 0x1D:
            body = .codecZenohPush(try CodecZenohPush.decode(&reader))
        case 0x1E:
            body = .codecZenohDeclare(try CodecZenohDeclare.decode(&reader))
        case 0x1F:
            body = .codecZenohOam(try CodecZenohOam.decode(&reader))
        default:
            body = .default(tag: CodecZenohNetworkEnvelopeUnknownTag(admitted: UInt8(truncatingIfNeeded: body_tag)), body: try CodecZenohOam.decode(&reader))
        }
        cursor = reader
        return CodecZenohNetworkEnvelope(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohNetworkEnvelopeSource` is written,
    /// with no copy into `CodecZenohNetworkEnvelope`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohNetworkEnvelopeSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var body_arm = BytesSink()
        switch source.body {
        case .codecZenohInterest(let arm):
            try CodecZenohInterest.encodeSource(arm, into: &body_arm)
        case .codecZenohResponseFinal(let arm):
            try CodecZenohResponseFinal.encodeSource(arm, into: &body_arm)
        case .codecZenohResponse(let arm):
            try CodecZenohResponse.encodeSource(arm, into: &body_arm)
        case .codecZenohRequest(let arm):
            try CodecZenohRequest.encodeSource(arm, into: &body_arm)
        case .codecZenohPush(let arm):
            try CodecZenohPush.encodeSource(arm, into: &body_arm)
        case .codecZenohDeclare(let arm):
            try CodecZenohDeclare.encodeSource(arm, into: &body_arm)
        case .codecZenohOam(let arm):
            try CodecZenohOam.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecZenohOam.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
