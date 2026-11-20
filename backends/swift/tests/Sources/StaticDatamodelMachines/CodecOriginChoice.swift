// SCE-GENERATED — DO NOT EDIT
// source-hash: fab7fba48eabd12b0a7fb9e00dc8fecbbe0e6706ecdadb44981933a20ca61fcd
// source: codec_origin_choice.scxml
// SCE-MAP: codec_origin_choice.scxml:9 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecOriginChoice`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecOriginChoiceArms<A0: CodecOriginLeafSource, A1: CodecOriginScalarSource, D: CodecOriginScalarSource> {
    case codecOriginLeaf(A0)
    case codecOriginScalar(A1)
    case `default`(tag: CodecOriginChoiceUnknownTag, body: D)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecOriginLeaf: return 0x1
        case .codecOriginScalar: return 0x2
        case .default(let tag, _): return tag.value
        }
    }
}

extension CodecOriginChoiceArms: Equatable where A0: Equatable, A1: Equatable, D: Equatable {}

extension CodecOriginChoiceArms: Sendable where A0: Sendable, A1: Sendable, D: Sendable {}

/// The arm a value of `CodecOriginChoice` holds, each arm the value of its own codec.
public typealias CodecOriginChoiceVariant = CodecOriginChoiceArms<CodecOriginLeaf, CodecOriginScalar, CodecOriginScalar>

/// The tag the catch-all arm of `CodecOriginChoice` holds (docs/adr/0065 §6 R3): one no arm names. The failable initializer is the one door, so a
/// catch-all written is read back as the catch-all, and its encode refuses nothing.
public struct CodecOriginChoiceUnknownTag: Equatable, Hashable, Sendable {
    /// The tag.
    public let value: UInt8

    /// The tag `value`, or `nil` where an arm names it.
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
        case 0x1...0x2: return false
        default: return true
        }
    }
}

/// The elements a value of `CodecOriginChoice` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecOriginChoice.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecOriginChoice`, which conforms itself.
public protocol CodecOriginChoiceSource {
    /// What the body of the arm `codecOriginLeaf` is presented as: a source of `CodecOriginLeaf`, written by its encode.
    associatedtype CodecOriginLeaf_Arm: CodecOriginLeafSource
    /// What the body of the arm `codecOriginScalar` is presented as: a source of `CodecOriginScalar`, written by its encode.
    associatedtype CodecOriginScalar_Arm: CodecOriginScalarSource
    /// What the body of the arm `codecOriginScalar` is presented as: a source of `CodecOriginScalar`, written by its encode.
    associatedtype Default_Arm: CodecOriginScalarSource
    var body: CodecOriginChoiceArms<CodecOriginLeaf_Arm, CodecOriginScalar_Arm, Default_Arm> { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOriginChoice: Equatable, Sendable, CodecOriginChoiceSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1 + Swift.min(CodecOriginLeaf.frameLength, CodecOriginScalar.frameLength)

    /// The value presents the body of the arm `codecOriginLeaf` as a value of its own codec.
    public typealias CodecOriginLeaf_Arm = CodecOriginLeaf

    /// The value presents the body of the arm `codecOriginScalar` as a value of its own codec.
    public typealias CodecOriginScalar_Arm = CodecOriginScalar

    /// The value presents the body of the arm `codecOriginScalar` as a value of its own codec.
    public typealias Default_Arm = CodecOriginScalar

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecOriginChoiceVariant = .codecOriginScalar(CodecOriginScalar())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecOriginChoiceArms<CodecOriginLeaf_Arm, CodecOriginScalar_Arm, Default_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOriginChoice {
        var reader = cursor
        let tag = try reader.readInteger(UInt8.self, .big)
        let body: CodecOriginChoiceVariant
        let body_tag = tag
        switch body_tag {
        case 0x1:
            body = .codecOriginLeaf(try CodecOriginLeaf.decode(&reader))
        case 0x2:
            body = .codecOriginScalar(try CodecOriginScalar.decode(&reader))
        default:
            body = .default(tag: CodecOriginChoiceUnknownTag(admitted: body_tag), body: try CodecOriginScalar.decode(&reader))
        }
        cursor = reader
        return CodecOriginChoice(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecOriginChoiceSource` is written,
    /// with no copy into `CodecOriginChoice`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecOriginChoiceSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let tag_derived: UInt8 = UInt8(truncatingIfNeeded: source.body.tag)
        var frame = [UInt8](repeating: 0, count: 1)
        storeInteger(tag_derived, into: &frame, at: 0, .big)
        var body_arm = BytesSink()
        switch source.body {
        case .codecOriginLeaf(let arm):
            try CodecOriginLeaf.encodeSource(arm, into: &body_arm)
        case .codecOriginScalar(let arm):
            try CodecOriginScalar.encodeSource(arm, into: &body_arm)
        case .default(_, let arm):
            try CodecOriginScalar.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
