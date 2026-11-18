// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_init_syn_body.scxml:30 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements of `CodecInitSynBody` the gates `S` decide together (docs/adr/0065 §6
/// R4b, R4c): the elements present while the gates are as the one arm says. The encode derives the gates
/// from whether a value holds it, and a decode makes the arm the gates of the frame name.
public struct CodecInitSynBodyS: Equatable, Sendable {
    public var sn_res: UInt8
    public var batch_size: UInt16

    public init(sn_res: UInt8, batch_size: UInt16) {
        self.sn_res = sn_res
        self.batch_size = batch_size
    }
}

/// The elements a value of `CodecInitSynBody` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecInitSynBody.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecInitSynBody`, which conforms itself.
public protocol CodecInitSynBodySource {
    var version: UInt8 { get }
    var s: CodecInitSynBodyS? { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecInitSynBody: Equatable, Sendable, CodecInitSynBodySource {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var version: UInt8 = 0

    /// The elements the gates `S` decide together, `nil` while they name no arm.
    public var s: CodecInitSynBodyS? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(version: UInt8, s: CodecInitSynBodyS?) {
        self.version = version
        self.s = s
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, S: UInt8) throws(CodecError) -> CodecInitSynBody {
        var reader = cursor
        let version = try reader.readInteger(UInt8.self, .big)
        var sn_res: UInt8? = nil
        if (S & 0x1) != 0 {
            sn_res = try reader.readInteger(UInt8.self, .big)
        }
        var batch_size: UInt16? = nil
        if (S & 0x1) != 0 {
            batch_size = try reader.readInteger(UInt16.self, .big)
        }
        let s_group: CodecInitSynBodyS?
        if (S & 0x01) != 0 {
            s_group = CodecInitSynBodyS(sn_res: sn_res!, batch_size: batch_size!)
        } else {
            s_group = nil
        }
        cursor = reader
        return CodecInitSynBody(version: version, s: s_group)
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecInitSynBodySource` is written,
    /// with no copy into `CodecInitSynBody`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecInitSynBodySource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var sn_res_member: UInt8? = nil
        var batch_size_member: UInt16? = nil
        if let held = source.s { sn_res_member = held.sn_res; batch_size_member = held.batch_size }
        var frame: [UInt8] = []
        appendInteger(source.version, to: &frame, .big)
        if let held = sn_res_member {
            appendInteger(held, to: &frame, .big)
        }
        if let held = batch_size_member {
            appendInteger(held, to: &frame, .big)
        }
        try sink.writeBytes(frame)
    }

    /// The flag input `S` that `source` says, which the encode is not handed
    /// (docs/adr/0065 §6 R4a): 1 or 0. A parent hands the decode this bit, and derives it from
    /// here for the encode.
    public static func inputS<Source: CodecInitSynBodySource>(_ source: Source) -> UInt8 {
        (source.s != nil ? true : false) ? 1 : 0
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
