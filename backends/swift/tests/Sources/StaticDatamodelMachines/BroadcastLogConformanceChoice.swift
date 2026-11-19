// SCE-GENERATED — DO NOT EDIT
// source-hash: 6d95af6fbc211ea1c5b1e249d16b701ebad41657a9f4b213349f1c668ce4ccbb
// source: broadcast_log_conformance_choice.scxml
// source: broadcast_log_conformance_choice_large.scxml
// source: broadcast_log_conformance_choice_small.scxml
// source: broadcast_log_conformance_chosen.scxml
// source: broadcast_log_conformance_flagged.scxml
// source: broadcast_log_conformance_flagged_event.scxml
// SCE-MAP: broadcast_log_conformance_choice.scxml:11 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The arm of `BroadcastLogConformanceChoice`'s variant a value holds, each arm's body a source of the arm's
/// codec (docs/adr/0058 §1): a host's type is written with no copy into the arm's own value. The
/// tag is no member: the encode derives it from the arm, and the catch-all holds the tag no arm
/// names.
public enum BroadcastLogConformanceChoiceArms<A0: BroadcastLogConformanceChoiceLargeSource, A1: BroadcastLogConformanceChoiceSmallSource> {
    case broadcastLogConformanceChoiceLarge(A0)
    case broadcastLogConformanceChoiceSmall(A1)

    /// The tag of the arm this is: the one the document gives a named arm, and the one the
    /// catch-all holds. It is the tag the encode writes, or the one a caller hands the decode of a
    /// variant whose frame carries none.
    public var tag: UInt8 {
        switch self {
        case .broadcastLogConformanceChoiceLarge: return 0x0
        case .broadcastLogConformanceChoiceSmall: return 0x1
        }
    }
}

extension BroadcastLogConformanceChoiceArms: Equatable where A0: Equatable, A1: Equatable {}

extension BroadcastLogConformanceChoiceArms: Sendable where A0: Sendable, A1: Sendable {}

/// The arm a value of `BroadcastLogConformanceChoice` holds, each arm the value of its own codec.
public typealias BroadcastLogConformanceChoiceVariant = BroadcastLogConformanceChoiceArms<BroadcastLogConformanceChoiceLarge, BroadcastLogConformanceChoiceSmall>

/// The elements a value of `BroadcastLogConformanceChoice` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `BroadcastLogConformanceChoice.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `BroadcastLogConformanceChoice`, which conforms itself.
public protocol BroadcastLogConformanceChoiceSource {
    /// What the body of the arm `broadcastLogConformanceChoiceLarge` is presented as: a source of `BroadcastLogConformanceChoiceLarge`, written by its encode.
    associatedtype BroadcastLogConformanceChoiceLarge_Arm: BroadcastLogConformanceChoiceLargeSource
    /// What the body of the arm `broadcastLogConformanceChoiceSmall` is presented as: a source of `BroadcastLogConformanceChoiceSmall`, written by its encode.
    associatedtype BroadcastLogConformanceChoiceSmall_Arm: BroadcastLogConformanceChoiceSmallSource
    var body: BroadcastLogConformanceChoiceArms<BroadcastLogConformanceChoiceLarge_Arm, BroadcastLogConformanceChoiceSmall_Arm> { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct BroadcastLogConformanceChoice: Equatable, Sendable, BroadcastLogConformanceChoiceSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = Swift.min(BroadcastLogConformanceChoiceLarge.frameLength, BroadcastLogConformanceChoiceSmall.frameLength)

    /// The value presents the body of the arm `broadcastLogConformanceChoiceLarge` as a value of its own codec.
    public typealias BroadcastLogConformanceChoiceLarge_Arm = BroadcastLogConformanceChoiceLarge

    /// The value presents the body of the arm `broadcastLogConformanceChoiceSmall` as a value of its own codec.
    public typealias BroadcastLogConformanceChoiceSmall_Arm = BroadcastLogConformanceChoiceSmall

    /// The arm the tag selects, whose struct holds the rest of the frame. The tag is the field it was
    /// read from; an encode writes that field as it stands, and the arm after it, so the two are
    /// kept in step by whoever holds the value.
    public var body: BroadcastLogConformanceChoiceVariant = .broadcastLogConformanceChoiceLarge(BroadcastLogConformanceChoiceLarge())

    /// The tag of the arm this value holds: the one a decode is given to read it back, which a
    /// holder that stores the bytes (a broadcast log) stores beside them. A frame carries none, so
    /// the arm that is held is what says it.
    public var tag: UInt8 {
        switch body {
        case .broadcastLogConformanceChoiceLarge: return 0x0
        case .broadcastLogConformanceChoiceSmall: return 0x1
        }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(body: BroadcastLogConformanceChoiceArms<BroadcastLogConformanceChoiceLarge_Arm, BroadcastLogConformanceChoiceSmall_Arm>) {
        self.body = body
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, tag: UInt8) throws(CodecError) -> BroadcastLogConformanceChoice {
        var reader = cursor
        let body: BroadcastLogConformanceChoiceVariant
        let body_tag = tag
        switch body_tag {
        case 0x0:
            body = .broadcastLogConformanceChoiceLarge(try BroadcastLogConformanceChoiceLarge.decode(&reader))
        default:
            body = .broadcastLogConformanceChoiceSmall(try BroadcastLogConformanceChoiceSmall.decode(&reader))
        }
        cursor = reader
        return BroadcastLogConformanceChoice(body: body)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `BroadcastLogConformanceChoiceSource` is written,
    /// with no copy into `BroadcastLogConformanceChoice`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: BroadcastLogConformanceChoiceSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame: [UInt8] = []
        var body_arm = BytesSink()
        switch source.body {
        case .broadcastLogConformanceChoiceLarge(let arm):
            try BroadcastLogConformanceChoiceLarge.encodeSource(arm, into: &body_arm)
        case .broadcastLogConformanceChoiceSmall(let arm):
            try BroadcastLogConformanceChoiceSmall.encodeSource(arm, into: &body_arm)
        }
        frame += body_arm.bytes
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
