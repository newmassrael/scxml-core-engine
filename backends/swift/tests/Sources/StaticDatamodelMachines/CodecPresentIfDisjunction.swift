// SCE-GENERATED — DO NOT EDIT
// source-hash: 56c949dec5be5c35095606ccb42fcfd2e6698634361d004009f29ae095f792aa
// source: codec_present_if_disjunction.scxml
// SCE-MAP: codec_present_if_disjunction.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecPresentIfDisjunction` the gates `flags.wants_a || flags.wants_b` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
public enum CodecPresentIfDisjunctionFlagsWantsAOrWantsB: Equatable, Sendable {
    /// While flags.wants_a set, flags.wants_b clear.
    case wantsA(seq: UInt16)
    /// While flags.wants_a clear, flags.wants_b set.
    case wantsB(seq: UInt16)
    /// While flags.wants_a set, flags.wants_b set.
    case wantsAWantsB(seq: UInt16)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .wantsA: return 0
        case .wantsB: return 1
        case .wantsAWantsB: return 2
        }
    }
}

/// The elements a value of `CodecPresentIfDisjunction` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfDisjunction.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfDisjunction`, which conforms itself.
public protocol CodecPresentIfDisjunctionSource {
    var flags: UInt8 { get }
    var flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB? { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfDisjunction: Equatable, Sendable, CodecPresentIfDisjunctionSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `wants_a`).
    public var wantsA: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// Bit 1 of `flags` (flag `wants_b`).
    public var wantsB: Bool {
        get { (flags & 0x2) != 0 }
        set { flags = newValue ? (flags | 0x2) : (flags & ~0x2) }
    }

    /// The elements the gates `flags.wants_a || flags.wants_b` decide together, `nil` while they name no arm.
    public var flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, flags_wants_a_or_wants_b: CodecPresentIfDisjunctionFlagsWantsAOrWantsB?) {
        self.flags = flags
        self.flags_wants_a_or_wants_b = flags_wants_a_or_wants_b
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfDisjunction {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var seq: UInt16? = nil
        if (flags & 0x1) != 0 || (flags & 0x2) != 0 {
            seq = try reader.readInteger(UInt16.self, .big)
        }
        let flags_wants_a_or_wants_b_group: CodecPresentIfDisjunctionFlagsWantsAOrWantsB?
        if ((flags >> 0) & 1) != 0 && ((flags >> 1) & 1) == 0 {
            flags_wants_a_or_wants_b_group = .wantsA(seq: seq!)
        } else if ((flags >> 0) & 1) == 0 && ((flags >> 1) & 1) != 0 {
            flags_wants_a_or_wants_b_group = .wantsB(seq: seq!)
        } else if ((flags >> 0) & 1) != 0 && ((flags >> 1) & 1) != 0 {
            flags_wants_a_or_wants_b_group = .wantsAWantsB(seq: seq!)
        } else {
            flags_wants_a_or_wants_b_group = nil
        }
        cursor = reader
        return CodecPresentIfDisjunction(flags: flags, flags_wants_a_or_wants_b: flags_wants_a_or_wants_b_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfDisjunctionSource` is written,
    /// with no copy into `CodecPresentIfDisjunction`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfDisjunctionSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var seq_member: UInt16? = nil
        switch source.flags_wants_a_or_wants_b {
        case .wantsA(let seq)?: seq_member = seq
        case .wantsB(let seq)?: seq_member = seq
        case .wantsAWantsB(let seq)?: seq_member = seq
        case nil: break
        }
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((source.flags_wants_a_or_wants_b.map { [0, 2].contains($0.arm) } ?? false) ? 0x1 : 0)
        flags_folded = (flags_folded & ~0x2) | ((source.flags_wants_a_or_wants_b.map { [1, 2].contains($0.arm) } ?? false) ? 0x2 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if let held = seq_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
