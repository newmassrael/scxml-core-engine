// SCE-GENERATED — DO NOT EDIT
// source-hash: 0f1e387d53ff6e6796730f8ba3508cf71d462ed4d4dd44e67ac39baaaac0fdb5
// source: codec_present_if_length_ref.scxml
// SCE-MAP: codec_present_if_length_ref.scxml:16 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecPresentIfLengthRef` the gates `flags.has_payload` decide together (docs/adr/0065 §6
/// R4b, R4c): the arm a value holds, each arm the elements present under one assignment of
/// the gates. The encode derives the gates
/// from which arm it is, and a decode makes the arm the gates of the frame name.
public enum CodecPresentIfLengthRefFlagsHasPayload: Equatable, Sendable {
    /// While flags.has_payload set.
    case hasPayload(payload: BoundedBytes<CodecPresentIfLengthRef.PayloadBound>)
    /// While flags.has_payload clear.
    case notHasPayload(payload_size: UInt8)

    /// The place of the arm among the arms, in the order the document's gates make them: what the
    /// encode derives each gate from.
    public var arm: Int {
        switch self {
        case .hasPayload: return 0
        case .notHasPayload: return 1
        }
    }
}

/// The elements a value of `CodecPresentIfLengthRef` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPresentIfLengthRef.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPresentIfLengthRef`, which conforms itself.
public protocol CodecPresentIfLengthRefSource {
    var flags: UInt8 { get }
    var flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPresentIfLengthRef: Equatable, Sendable, CodecPresentIfLengthRefSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// How many bytes `payload` holds on the wire, as the document bounds it (docs/adr/0065 §6
    /// R1): its type is held to it where a value of it is made.
    public enum PayloadBound: SceByteBound {
        public static let least = 0
        public static let most: Int? = 32
    }

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_payload`).
    public var hasPayload: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// The elements the gates `flags.has_payload` decide together.
    public var flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload = .notHasPayload(payload_size: 0)

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, flags_has_payload: CodecPresentIfLengthRefFlagsHasPayload) {
        self.flags = flags
        self.flags_has_payload = flags_has_payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPresentIfLengthRef {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        let payload_size = try reader.readInteger(UInt8.self, .big)
        var payload: BoundedBytes<CodecPresentIfLengthRef.PayloadBound>? = nil
        if (flags & 0x1) != 0 {
            if payload_size > 32 {
                throw CodecError.lengthOutOfRange
            }
            payload = try BoundedBytes<CodecPresentIfLengthRef.PayloadBound>(decoding: Array(try reader.readSlice(Int(clamping: payload_size))))
        }
        let flags_has_payload_group: CodecPresentIfLengthRefFlagsHasPayload
        if ((flags >> 0) & 1) != 0 {
            flags_has_payload_group = .hasPayload(payload: payload!)
        } else {
            flags_has_payload_group = .notHasPayload(payload_size: payload_size)
        }
        cursor = reader
        return CodecPresentIfLengthRef(flags: flags, flags_has_payload: flags_has_payload_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPresentIfLengthRefSource` is written,
    /// with no copy into `CodecPresentIfLengthRef`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPresentIfLengthRefSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var payload_member: BoundedBytes<CodecPresentIfLengthRef.PayloadBound>? = nil
        var payload_size_member: UInt8? = nil
        switch source.flags_has_payload {
        case .hasPayload(let payload): payload_member = payload
        case .notHasPayload(let payload_size): payload_size_member = payload_size
        }
        let payload_size_derived: UInt8 = payload_member != nil ? UInt8((payload_member?.byteCount ?? 0)) : (payload_size_member ?? 0)
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((payload_member != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        appendInteger(payload_size_derived, to: &frame, .big)
        if let held = payload_member {
            held.append(to: &frame)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
