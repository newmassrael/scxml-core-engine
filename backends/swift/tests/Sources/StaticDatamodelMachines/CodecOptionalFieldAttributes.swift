// SCE-GENERATED — DO NOT EDIT
// source-hash: 914e4efa5463dce80fb95f88994414983871951061ec2a757682380cbc5289ea
// source: codec_optional_field_attributes.scxml
// SCE-MAP: codec_optional_field_attributes.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecOptionalFieldAttributes` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecOptionalFieldAttributes.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecOptionalFieldAttributes`, which conforms itself.
public protocol CodecOptionalFieldAttributesSource {
    var status: UInt8 { get }
    var reading: UInt16 { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecOptionalFieldAttributes: Equatable, Sendable, CodecOptionalFieldAttributesSource {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, little-endian.
    public var status: UInt8 = 0

    /// Bit 2 of `status` (flag `ready`).
    public var ready: Bool {
        get { (status & 0x4) != 0 }
        set { status = newValue ? (status | 0x4) : (status & ~0x4) }
    }

    /// Bit 3 of `status` (flag `fault`).
    public var fault: Bool {
        get { (status & 0x8) != 0 }
        set { status = newValue ? (status | 0x8) : (status & ~0x8) }
    }

    /// The 12 bits from bit 4 of the 2 bytes from byte 1, read as one
    /// big-endian word. A value wider than that is cut to it when written.
    public var reading: UInt16 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(status: UInt8, reading: UInt16) {
        self.status = status
        self.reading = reading
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecOptionalFieldAttributes {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecOptionalFieldAttributes(
            status: try cursor.peekInteger(UInt8.self, at: 0, .little),
            reading: try (cursor.peekInteger(UInt16.self, at: 1, .big) >> 4) & 0xFFF
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecOptionalFieldAttributes` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `ready` flag of `status`, where a parent sets it.
        public var statusReady: Bool?
        /// The `fault` flag of `status`, where a parent sets it.
        public var statusFault: Bool?

        public init(statusReady: Bool? = nil, statusFault: Bool? = nil) {
            self.statusReady = statusReady
            self.statusFault = statusFault
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecOptionalFieldAttributesSource` is written,
    /// with no copy into `CodecOptionalFieldAttributes`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecOptionalFieldAttributesSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var status_folded = source.status
        if let set = overrides.statusReady { status_folded = set ? (status_folded | 0x4) : (status_folded & ~0x4) }
        if let set = overrides.statusFault { status_folded = set ? (status_folded | 0x8) : (status_folded & ~0x8) }
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(status_folded, into: &frame, at: 0, .little)
        frame[1] |= UInt8(truncatingIfNeeded: (source.reading >> 4) & 0xFF) << 0
        frame[2] |= UInt8(truncatingIfNeeded: source.reading & 0xF) << 4
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
