// SCE-GENERATED — DO NOT EDIT
// source-hash: 4f5990782db830cf4a7157c8ef2cd2bb042598920dbc82d4054dfcd88a262807
// source: codec_zenoh_ext_unit.scxml
// SCE-MAP: codec_zenoh_ext_unit.scxml:13 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecZenohExtUnit` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecZenohExtUnit.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecZenohExtUnit`, which conforms itself.
public protocol CodecZenohExtUnitSource {
}

/// The frame this codec lays on the wire: 0 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohExtUnit: Equatable, Sendable, CodecZenohExtUnitSource {
    /// The bytes of one frame.
    public static let frameLength = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}


    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohExtUnit {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecZenohExtUnit(
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecZenohExtUnitSource` is written,
    /// with no copy into `CodecZenohExtUnit`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecZenohExtUnitSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        let frame = [UInt8](repeating: 0, count: Self.frameLength)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
