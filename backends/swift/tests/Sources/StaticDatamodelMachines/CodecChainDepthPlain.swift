// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_chain_depth_plain.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainDepthPlain: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header_flags: UInt8 = 0

    /// Entries of `CodecChainDepthEntry`, one after another, at most 8: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var extensions: [CodecChainDepthEntry] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header_flags: UInt8, extensions: [CodecChainDepthEntry]) {
        self.header_flags = header_flags
        self.extensions = extensions
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainDepthPlain {
        var reader = cursor
        var frame = CodecChainDepthPlain()
        frame.header_flags = try reader.readInteger(UInt8.self, .big)
        var extensions_items: [CodecChainDepthEntry] = []
        var extensions_more = false
        for _ in 0..<8 {
            let entry = try CodecChainDepthEntry.decode(&reader)
            extensions_more = entry.Z
            extensions_items.append(entry)
            if !extensions_more { break }
        }
        if extensions_more && reader.remaining == 0 {
            throw CodecError.needMoreBytes
        }
        if extensions_more {
            throw CodecError.tooManyElements
        }
        frame.extensions = extensions_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.extensions.count < 1 || self.extensions.count > 8 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.header_flags, to: &frame, .big)
        for entry in self.extensions {
            var piece = BytesSink()
            try entry.encode(into: &piece)
            frame += piece.bytes
        }
        try sink.writeBytes(frame)
    }
}
