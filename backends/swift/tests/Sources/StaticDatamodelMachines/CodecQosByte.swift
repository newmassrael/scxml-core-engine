// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_qos_byte.scxml:15 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecQosByte` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecQosByte.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecQosByte`, which conforms itself.
public protocol CodecQosByteSource {
    var qos: UInt8 { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecQosByte: Equatable, Sendable, CodecQosByteSource {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var qos: UInt8 = 0

    /// The 3 bits of `qos` from bit 0 (flag `priority`). A value
    /// wider than the flag is cut to it when written.
    public var priority: UInt8 {
        get { UInt8(truncatingIfNeeded: (qos >> 0) & 0x7) }
        set { qos = (qos & ~(0x7 << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x7) << 0) }
    }

    /// Bit 3 of `qos` (flag `reliable`).
    public var reliable: Bool {
        get { (qos & 0x8) != 0 }
        set { qos = newValue ? (qos | 0x8) : (qos & ~0x8) }
    }

    /// The 2 bits of `qos` from bit 4 (flag `congestion`). A value
    /// wider than the flag is cut to it when written.
    public var congestion: UInt8 {
        get { UInt8(truncatingIfNeeded: (qos >> 4) & 0x3) }
        set { qos = (qos & ~(0x3 << 4)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x3) << 4) }
    }

    /// Bit 6 of `qos` (flag `express`).
    public var express: Bool {
        get { (qos & 0x40) != 0 }
        set { qos = newValue ? (qos | 0x40) : (qos & ~0x40) }
    }

    /// Bit 7 of `qos` (flag `reserved`).
    public var reserved: Bool {
        get { (qos & 0x80) != 0 }
        set { qos = newValue ? (qos | 0x80) : (qos & ~0x80) }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(qos: UInt8) {
        self.qos = qos
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecQosByte {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecQosByte(
            qos: try cursor.peekInteger(UInt8.self, at: 0, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecQosByte` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `reliable` flag of `qos`, where a parent sets it.
        public var qosReliable: Bool?
        /// The `express` flag of `qos`, where a parent sets it.
        public var qosExpress: Bool?
        /// The `reserved` flag of `qos`, where a parent sets it.
        public var qosReserved: Bool?

        public init(qosReliable: Bool? = nil, qosExpress: Bool? = nil, qosReserved: Bool? = nil) {
            self.qosReliable = qosReliable
            self.qosExpress = qosExpress
            self.qosReserved = qosReserved
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecQosByteSource` is written,
    /// with no copy into `CodecQosByte`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecQosByteSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var qos_folded = source.qos
        if let set = overrides.qosReliable { qos_folded = set ? (qos_folded | 0x8) : (qos_folded & ~0x8) }
        if let set = overrides.qosExpress { qos_folded = set ? (qos_folded | 0x40) : (qos_folded & ~0x40) }
        if let set = overrides.qosReserved { qos_folded = set ? (qos_folded | 0x80) : (qos_folded & ~0x80) }
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(qos_folded, into: &frame, at: 0, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
