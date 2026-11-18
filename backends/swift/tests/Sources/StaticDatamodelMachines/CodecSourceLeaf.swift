// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_source_leaf.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecSourceLeaf`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecSourceLeafArms<A0: CodecVariantSessionOpenSource, A1: CodecVariantSessionCloseSource> {
    case codecVariantSessionOpen(A0)
    case codecVariantSessionClose(A1)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecVariantSessionOpen: return 0x0
        case .codecVariantSessionClose: return 0x1
        }
    }
}

extension CodecSourceLeafArms: Equatable where A0: Equatable, A1: Equatable {}

extension CodecSourceLeafArms: Sendable where A0: Sendable, A1: Sendable {}

/// The arm a value of `CodecSourceLeaf` holds, each arm the value of its own codec.
public typealias CodecSourceLeafVariant = CodecSourceLeafArms<CodecVariantSessionOpen, CodecVariantSessionClose>

/// The elements a value of `CodecSourceLeaf` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecSourceLeaf.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecSourceLeaf`, which conforms itself.
public protocol CodecSourceLeafSource {
    /// What the body of the arm `codecVariantSessionOpen` is presented as: a source of `CodecVariantSessionOpen`, written by its encode.
    associatedtype CodecVariantSessionOpen_Arm: CodecVariantSessionOpenSource
    /// What the body of the arm `codecVariantSessionClose` is presented as: a source of `CodecVariantSessionClose`, written by its encode.
    associatedtype CodecVariantSessionClose_Arm: CodecVariantSessionCloseSource
    var body: CodecSourceLeafArms<CodecVariantSessionOpen_Arm, CodecVariantSessionClose_Arm> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSourceLeaf: Equatable, Sendable, CodecSourceLeafSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecVariantSessionOpen.frameLength, CodecVariantSessionClose.frameLength)

    /// The value presents the body of the arm `codecVariantSessionOpen` as a value of its own codec.
    public typealias CodecVariantSessionOpen_Arm = CodecVariantSessionOpen

    /// The value presents the body of the arm `codecVariantSessionClose` as a value of its own codec.
    public typealias CodecVariantSessionClose_Arm = CodecVariantSessionClose

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecSourceLeafVariant = .codecVariantSessionClose(CodecVariantSessionClose())

    /// The tag of the arm this value holds: the one a decode is given to read it back, which a
    /// holder that stores the bytes (a broadcast log) stores beside them. A frame carries none, so
    /// the arm that is held is what says it.
    public var tag: UInt8 {
        switch body {
        case .codecVariantSessionOpen: return 0x0
        case .codecVariantSessionClose: return 0x1
        }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecSourceLeafArms<CodecVariantSessionOpen_Arm, CodecVariantSessionClose_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, tag: UInt8) throws(CodecError) -> CodecSourceLeaf {
        var reader = cursor
        let body: CodecSourceLeafVariant
        let body_tag = tag
        switch body_tag {
        case 0x0:
            body = .codecVariantSessionOpen(try CodecVariantSessionOpen.decode(&reader))
        default:
            body = .codecVariantSessionClose(try CodecVariantSessionClose.decode(&reader))
        }
        cursor = reader
        return CodecSourceLeaf(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecSourceLeafSource` is written,
    /// with no copy into `CodecSourceLeaf`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecSourceLeafSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var body_arm = BytesSink()
        switch source.body {
        case .codecVariantSessionOpen(let arm):
            try CodecVariantSessionOpen.encodeSource(arm, into: &body_arm)
        case .codecVariantSessionClose(let arm):
            try CodecVariantSessionClose.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
