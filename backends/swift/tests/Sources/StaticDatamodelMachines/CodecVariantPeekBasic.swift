// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_variant_peek_basic.scxml:29 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecVariantPeekBasic`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecVariantPeekBasicArms<A0: CodecPeekArmASource, A1: CodecPeekArmBSource> {
    case codecPeekArmA(A0)
    case codecPeekArmB(A1)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecPeekArmA: return 0x0
        case .codecPeekArmB: return 0x1
        }
    }
}

extension CodecVariantPeekBasicArms: Equatable where A0: Equatable, A1: Equatable {}

extension CodecVariantPeekBasicArms: Sendable where A0: Sendable, A1: Sendable {}

/// The arm a value of `CodecVariantPeekBasic` holds, each arm the value of its own codec.
public typealias CodecVariantPeekBasicVariant = CodecVariantPeekBasicArms<CodecPeekArmA, CodecPeekArmB>

/// The elements a value of `CodecVariantPeekBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecVariantPeekBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecVariantPeekBasic`, which conforms itself.
public protocol CodecVariantPeekBasicSource {
    /// What the body of the arm `codecPeekArmA` is presented as: a source of `CodecPeekArmA`, written by its encode.
    associatedtype CodecPeekArmA_Arm: CodecPeekArmASource
    /// What the body of the arm `codecPeekArmB` is presented as: a source of `CodecPeekArmB`, written by its encode.
    associatedtype CodecPeekArmB_Arm: CodecPeekArmBSource
    var body: CodecVariantPeekBasicArms<CodecPeekArmA_Arm, CodecPeekArmB_Arm> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecVariantPeekBasic: Equatable, Sendable, CodecVariantPeekBasicSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecPeekArmA.frameLength, CodecPeekArmB.frameLength)

    /// The value presents the body of the arm `codecPeekArmA` as a value of its own codec.
    public typealias CodecPeekArmA_Arm = CodecPeekArmA

    /// The value presents the body of the arm `codecPeekArmB` as a value of its own codec.
    public typealias CodecPeekArmB_Arm = CodecPeekArmB

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecVariantPeekBasicVariant = .codecPeekArmA(CodecPeekArmA())

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecVariantPeekBasicArms<CodecPeekArmA_Arm, CodecPeekArmB_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecVariantPeekBasic {
        var reader = cursor
        let body: CodecVariantPeekBasicVariant
        let body_tag = try (reader.peekInteger(UInt8.self, at: 0, .big) >> 0) & 0x1
        switch body_tag {
        case 0x0:
            body = .codecPeekArmA(try CodecPeekArmA.decode(&reader))
        default:
            body = .codecPeekArmB(try CodecPeekArmB.decode(&reader))
        }
        cursor = reader
        return CodecVariantPeekBasic(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecVariantPeekBasicSource` is written,
    /// with no copy into `CodecVariantPeekBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecVariantPeekBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var body_arm = BytesSink()
        switch source.body {
        case .codecPeekArmA(let arm):
            try CodecPeekArmA.encodeSource(arm, into: &body_arm)
        case .codecPeekArmB(let arm):
            try CodecPeekArmB.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
