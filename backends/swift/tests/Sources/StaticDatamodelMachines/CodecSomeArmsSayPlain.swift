// SCE-GENERATED — DO NOT EDIT
// source-hash: 4c856a0156c800e360d5994bedd60206ecf4c387ec76a7a558edfc1c253a400d
// source: codec_some_arms_say_plain.scxml
// SCE-MAP: codec_some_arms_say_plain.scxml:4 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The elements a value of `CodecSomeArmsSayPlain` presents, each a read-only property in the order the
/// document declares them (docs/adr/0058 §1, docs/adr/0081 §1): what
/// `CodecSomeArmsSayPlain.encodeSource(_:into:)` writes a frame from. A type of the host's own that
/// declares them is written with no copy into `CodecSomeArmsSayPlain`, which conforms itself.
public protocol CodecSomeArmsSayPlainSource {
    var id: UInt8 { get }
}

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecSomeArmsSayPlain: Equatable, Sendable, CodecSomeArmsSayPlainSource {
    /// The bytes of one frame.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var id: UInt8 = 0

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(id: UInt8) {
        self.id = id
    }

    /// Decodes the frame at the cursor and moves the cursor past it. A cursor with fewer than
    /// `frameLength` bytes left answers `needMoreBytes` and is left where it was (RFC §synth-5-B
    /// L494-519).
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecSomeArmsSayPlain {
        _ = try cursor.peekSlice(frameLength)
        let frame = CodecSomeArmsSayPlain(
            id: try cursor.peekInteger(UInt8.self, at: 0, .big)
        )
        try cursor.advance(frameLength)
        return frame
    }

    /// Writes the frame of `source` to `sink` in one write, printed from the codec's encode plan
    /// (docs/adr/0081): any type that presents the elements of `CodecSomeArmsSayPlainSource` is written,
    /// with no copy into `CodecSomeArmsSayPlain`, and every other element of the frame is derived from
    /// them. It refuses
    /// nothing. A bounded sink that cannot hold all of the frame answers
    /// `bufferOverflow` and holds none of it.
    public static func encodeSource<Source: CodecSomeArmsSayPlainSource, Sink: SceSink>(_ source: Source, into sink: inout Sink) throws(CodecError) {
        var frame = [UInt8](repeating: 0, count: Self.frameLength)
        storeInteger(source.id, into: &frame, at: 0, .big)
        try sink.writeBytes(frame)
    }

    /// Writes the frame of this value to `sink`: `encodeSource` of itself.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        try Self.encodeSource(self, into: &sink)
    }
}
