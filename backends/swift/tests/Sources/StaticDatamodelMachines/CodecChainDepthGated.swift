// SCE-GENERATED — DO NOT EDIT
// source-hash: adac8824ce3bf12969cf8a9f4a1db094aa59b5b7a2d7b37b7a611f3c7eb217dc
// SCE-MAP: codec_chain_depth_gated.scxml:17 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 4 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainDepthGated: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 2

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// Bit 7 of `header` (flag `Z`).
    public var Z: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecChainDepthEntry`, one after another, at most 8: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var extensions: [CodecChainDepthEntry]? = nil

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var payload_len: UInt64 = 0

    /// The bytes, as many as `payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var payload: [UInt8] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, extensions: [CodecChainDepthEntry]?, payload_len: UInt64, payload: [UInt8]) {
        self.header = header
        self.extensions = extensions
        self.payload_len = payload_len
        self.payload = payload
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainDepthGated {
        var reader = cursor
        var frame = CodecChainDepthGated()
        frame.header = try reader.readInteger(UInt8.self, .big)
        if (frame.header & 0x80) != 0 {
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
        }
        frame.payload_len = try reader.readVle(UInt64.self)
        if frame.payload_len > 16 {
            throw CodecError.lengthOutOfRange
        }
        frame.payload = Array(try reader.readSlice(Int(clamping: frame.payload_len)))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.extensions, value.count < 1 || value.count > 8 {
            throw CodecError.tooManyElements
        }
        if self.payload.count > 16 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendInteger(self.header, to: &frame, .big)
        if let extensions = self.extensions {
            for entry in extensions {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        appendVle(self.payload_len, to: &frame)
        frame += self.payload
        try sink.writeBytes(frame)
    }
}
