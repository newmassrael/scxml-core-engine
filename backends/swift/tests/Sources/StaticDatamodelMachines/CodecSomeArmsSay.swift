// SCE-GENERATED — DO NOT EDIT
// source-hash: ab118905d4df489cfce6a00bad09a4eb5c56a3b4f2f13a8198d5219e468bb5e4
// source: codec_some_arms_say.scxml
// SCE-MAP: codec_some_arms_say.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `CodecSomeArmsSay`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum CodecSomeArmsSayArms<A0: CodecSomeArmsSayPlainSource, A1: CodecEveryArmSaysNearSource> {
    case codecSomeArmsSayPlain(A0)
    case codecEveryArmSaysNear(A1)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .codecSomeArmsSayPlain: return 0x0
        case .codecEveryArmSaysNear: return 0x1
        }
    }
}

extension CodecSomeArmsSayArms: Equatable where A0: Equatable, A1: Equatable {}

extension CodecSomeArmsSayArms: Sendable where A0: Sendable, A1: Sendable {}

/// The arm a value of `CodecSomeArmsSay` holds, each arm the value of its own codec.
public typealias CodecSomeArmsSayVariant = CodecSomeArmsSayArms<CodecSomeArmsSayPlain, CodecEveryArmSaysNear>

/// The elements a value of `CodecSomeArmsSay` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecSomeArmsSay.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecSomeArmsSay`, which conforms itself.
public protocol CodecSomeArmsSaySource {
    /// What the body of the arm `codecSomeArmsSayPlain` is presented as: a source of `CodecSomeArmsSayPlain`, written by its encode.
    associatedtype CodecSomeArmsSayPlain_Arm: CodecSomeArmsSayPlainSource
    /// What the body of the arm `codecEveryArmSaysNear` is presented as: a source of `CodecEveryArmSaysNear`, written by its encode.
    associatedtype CodecEveryArmSaysNear_Arm: CodecEveryArmSaysNearSource
    var body: CodecSomeArmsSayArms<CodecSomeArmsSayPlain_Arm, CodecEveryArmSaysNear_Arm> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSomeArmsSay: Equatable, Sendable, CodecSomeArmsSaySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(CodecSomeArmsSayPlain.frameLength, CodecEveryArmSaysNear.frameLength)

    /// The value presents the body of the arm `codecSomeArmsSayPlain` as a value of its own codec.
    public typealias CodecSomeArmsSayPlain_Arm = CodecSomeArmsSayPlain

    /// The value presents the body of the arm `codecEveryArmSaysNear` as a value of its own codec.
    public typealias CodecEveryArmSaysNear_Arm = CodecEveryArmSaysNear

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: CodecSomeArmsSayVariant = .codecEveryArmSaysNear(CodecEveryArmSaysNear())

    /// The tag of the arm this value holds: the one a decode is given to read it back, which a
    /// holder that stores the bytes (a broadcast log) stores beside them. A frame carries none, so
    /// the arm that is held is what says it.
    public var tag: UInt8 {
        switch body {
        case .codecSomeArmsSayPlain: return 0x0
        case .codecEveryArmSaysNear: return 0x1
        }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: CodecSomeArmsSayArms<CodecSomeArmsSayPlain_Arm, CodecEveryArmSaysNear_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, N: UInt8, tag: UInt8) throws(CodecError) -> CodecSomeArmsSay {
        var reader = cursor
        let body: CodecSomeArmsSayVariant
        let body_tag = tag
        switch body_tag {
        case 0x0:
            body = .codecSomeArmsSayPlain(try CodecSomeArmsSayPlain.decode(&reader))
        default:
            body = .codecEveryArmSaysNear(try CodecEveryArmSaysNear.decode(&reader, N: N))
        }
        cursor = reader
        return CodecSomeArmsSay(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecSomeArmsSaySource` is written,
    /// with no copy into `CodecSomeArmsSay`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecSomeArmsSaySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var body_arm = BytesSink()
        switch source.body {
        case .codecSomeArmsSayPlain(let arm):
            try CodecSomeArmsSayPlain.encodeSource(arm, into: &body_arm)
        case .codecEveryArmSaysNear(let arm):
            try CodecEveryArmSaysNear.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// The flag input `N` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0, and `nil` where the value says nothing of it (§6 R4e). A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputN<Source: CodecSomeArmsSaySource>(_ source: Source) -> UInt8? {
        switch source.body {
        case .codecSomeArmsSayPlain: return nil
        case .codecEveryArmSaysNear(let arm): return CodecEveryArmSaysNear.inputN(arm)
        }
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
