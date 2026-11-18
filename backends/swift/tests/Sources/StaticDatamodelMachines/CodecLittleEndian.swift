// SCE-GENERATED — DO NOT EDIT
// source-hash: 2e14e61a19815f19d3d0dbb6fe1326691e48aee7d4809e1595ab8383fb1a9fb7
// SCE-MAP: codec_little_endian.scxml:3 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecLittleEndian` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecLittleEndian.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecLittleEndian`, which conforms itself.
public protocol CodecLittleEndianSource {
    var sensorId: UInt8 { get }
    var value: UInt16 { get }
    var status: UInt8 { get }
}

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecLittleEndian: Equatable, Sendable, CodecLittleEndianSource {
    /// The bytes of one frame.
    public static let frameLength = 4

    /// The field at byte 0, little-endian.
    public var sensorId: UInt8 = 0

    /// The field at byte 1, little-endian.
    public var value: UInt16 = 0

    /// The field at byte 3, little-endian.
    public var status: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(sensorId: UInt8, value: UInt16, status: UInt8) {
        self.sensorId = sensorId
        self.value = value
        self.status = status
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecLittleEndian {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecLittleEndian(
            sensorId: try cursor.peekInteger(UInt8.self, at: 0, .little),
            value: try cursor.peekInteger(UInt16.self, at: 1, .little),
            status: try cursor.peekInteger(UInt8.self, at: 3, .little)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecLittleEndianSource` is written,
    /// with no copy into `CodecLittleEndian`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecLittleEndianSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.sensorId, into: &frame, at: 0, .little)
        storeInteger(source.value, into: &frame, at: 1, .little)
        storeInteger(source.status, into: &frame, at: 3, .little)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
