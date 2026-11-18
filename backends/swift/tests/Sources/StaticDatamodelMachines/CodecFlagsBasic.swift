// SCE-GENERATED — DO NOT EDIT
// source-hash: 380120774f6558f75c02b77dfa951d04c7e39cb42d4bf17b1ff1c0ded2b8b8d1
// SCE-MAP: codec_flags_basic.scxml:7 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecFlagsBasic` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecFlagsBasic.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecFlagsBasic`, which conforms itself.
public protocol CodecFlagsBasicSource {
    var header: UInt8 { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecFlagsBasic: Equatable, Sendable, CodecFlagsBasicSource {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 7 of `header` (flag `reliable`).
    public var reliable: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// Bit 6 of `header` (flag `more`).
    public var more: Bool {
        get { (header & 0x40) != 0 }
        set { header = newValue ? (header | 0x40) : (header & ~0x40) }
    }

    /// Bit 5 of `header` (flag `drop`).
    public var drop: Bool {
        get { (header & 0x20) != 0 }
        set { header = newValue ? (header | 0x20) : (header & ~0x20) }
    }

    /// Bit 4 of `header` (flag `first`).
    public var first: Bool {
        get { (header & 0x10) != 0 }
        set { header = newValue ? (header | 0x10) : (header & ~0x10) }
    }

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8) {
        self.header = header
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecFlagsBasic {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecFlagsBasic(
            header: try cursor.peekInteger(UInt8.self, at: 0, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecFlagsBasic` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `reliable` flag of `header`, where a parent sets it.
        public var headerReliable: Bool?
        /// The `more` flag of `header`, where a parent sets it.
        public var headerMore: Bool?
        /// The `drop` flag of `header`, where a parent sets it.
        public var headerDrop: Bool?
        /// The `first` flag of `header`, where a parent sets it.
        public var headerFirst: Bool?

        public init(headerReliable: Bool? = nil, headerMore: Bool? = nil, headerDrop: Bool? = nil, headerFirst: Bool? = nil) {
            self.headerReliable = headerReliable
            self.headerMore = headerMore
            self.headerDrop = headerDrop
            self.headerFirst = headerFirst
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecFlagsBasicSource` is written,
    /// with no copy into `CodecFlagsBasic`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecFlagsBasicSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        if let set = overrides.headerReliable { header_folded = set ? (header_folded | 0x80) : (header_folded & ~0x80) }
        if let set = overrides.headerMore { header_folded = set ? (header_folded | 0x40) : (header_folded & ~0x40) }
        if let set = overrides.headerDrop { header_folded = set ? (header_folded | 0x20) : (header_folded & ~0x20) }
        if let set = overrides.headerFirst { header_folded = set ? (header_folded | 0x10) : (header_folded & ~0x10) }
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(header_folded, into: &frame, at: 0, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
