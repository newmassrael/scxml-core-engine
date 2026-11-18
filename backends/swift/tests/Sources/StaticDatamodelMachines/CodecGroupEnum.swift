// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_group_enum.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecGroupEnum` the gates `flags.has_session` decide together (docs/adr/0065 §6
/// R4b, R4c): the elements present while the gates are as the one arm says. The encode derives the gates
/// from whether a value holds it, and a decode makes the arm the gates of the frame name.
public struct CodecGroupEnumFlagsHasSession: Equatable, Sendable {
    public var state: EnumSessionState
    public var attempts: UInt8

    public init(state: EnumSessionState, attempts: UInt8) {
        self.state = state
        self.attempts = attempts
    }
}

/// The elements a value of `CodecGroupEnum` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecGroupEnum.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecGroupEnum`, which conforms itself.
public protocol CodecGroupEnumSource {
    var flags: UInt8 { get }
    var flags_has_session: CodecGroupEnumFlagsHasSession? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecGroupEnum: Equatable, Sendable, CodecGroupEnumSource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var flags: UInt8 = 0

    /// Bit 0 of `flags` (flag `has_session`).
    public var hasSession: Bool {
        get { (flags & 0x1) != 0 }
        set { flags = newValue ? (flags | 0x1) : (flags & ~0x1) }
    }

    /// The elements the gates `flags.has_session` decide together, `nil` while they name no arm.
    public var flags_has_session: CodecGroupEnumFlagsHasSession? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(flags: UInt8, flags_has_session: CodecGroupEnumFlagsHasSession?) {
        self.flags = flags
        self.flags_has_session = flags_has_session
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecGroupEnum {
        var reader = cursor
        let flags = try reader.readInteger(UInt8.self, .big)
        var state: EnumSessionState? = nil
        if (flags & 0x1) != 0 {
            state = try declaredEnumValue(EnumSessionState.fromUnderlying(reader.readInteger(UInt8.self, .big)))
        }
        var attempts: UInt8? = nil
        if (flags & 0x1) != 0 {
            attempts = try reader.readInteger(UInt8.self, .big)
        }
        let flags_has_session_group: CodecGroupEnumFlagsHasSession?
        if ((flags >> 0) & 1) != 0 {
            flags_has_session_group = CodecGroupEnumFlagsHasSession(state: state!, attempts: attempts!)
        } else {
            flags_has_session_group = nil
        }
        cursor = reader
        return CodecGroupEnum(flags: flags, flags_has_session: flags_has_session_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecGroupEnumSource` is written,
    /// with no copy into `CodecGroupEnum`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecGroupEnumSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var state_member: EnumSessionState? = nil
        var attempts_member: UInt8? = nil
        if let held = source.flags_has_session { state_member = held.state; attempts_member = held.attempts }
        var flags_folded = source.flags
        flags_folded = (flags_folded & ~0x1) | ((state_member != nil) ? 0x1 : 0)
        var frame: [UInt8] = []
        appendInteger(flags_folded, to: &frame, .big)
        if let held = state_member {
            appendInteger(EnumSessionState.toUnderlying(held), to: &frame, .big)
        }
        if let held = attempts_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
