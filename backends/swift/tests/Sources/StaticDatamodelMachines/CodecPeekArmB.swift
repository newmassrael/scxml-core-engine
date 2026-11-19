// SCE-GENERATED — DO NOT EDIT
// source-hash: 16458c099e857874f517f500cf8a3ad4da9e51b54a796105c9991e7b6f033f3b
// source: codec_peek_arm_b.scxml
// SCE-MAP: codec_peek_arm_b.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecPeekArmB` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecPeekArmB.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecPeekArmB`, which conforms itself.
public protocol CodecPeekArmBSource {
    var header: UInt8 { get }
    var payload: UInt16 { get }
}

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecPeekArmB: Equatable, Sendable, CodecPeekArmBSource {
    /// The bytes of one frame.
    public static let frameLength = 3

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0x1

    /// Bit 0 of `header` (flag `kind`).
    public var kind: Bool {
        get { (header & 0x1) != 0 }
        set { header = newValue ? (header | 0x1) : (header & ~0x1) }
    }

    /// The field at byte 1, big-endian.
    public var payload: UInt16 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, payload: UInt16) {
        self.header = header
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecPeekArmB {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecPeekArmB(
            header: try cursor.peekInteger(UInt8.self, at: 0, .big),
            payload: try cursor.peekInteger(UInt16.self, at: 1, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// One-bit flags a parent may set in place of the ones a value presents, such as the flag a
    /// chain whose entries are `CodecPeekArmB` ends them on, which the chain derives from where
    /// each entry stands. A flag left `nil` is the value's own.
    public struct Overrides: Equatable, Sendable {
        /// The `kind` flag of `header`, where a parent sets it.
        public var headerKind: Bool?

        public init(headerKind: Bool? = nil) {
            self.headerKind = headerKind
        }
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecPeekArmBSource` is written,
    /// with no copy into `CodecPeekArmB`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecPeekArmBSource, Sink: SceSink>(_ source: Source, into sink: inout Sink, overrides: Overrides = Overrides()) throws(CodecError) {
        var header_folded = source.header
        if let set = overrides.headerKind { header_folded = set ? (header_folded | 0x1) : (header_folded & ~0x1) }
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(header_folded, into: &frame, at: 0, .big)
        storeInteger(source.payload, into: &frame, at: 1, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
