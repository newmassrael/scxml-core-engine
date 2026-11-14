// SCE-GENERATED — DO NOT EDIT
// source-hash: 1ee4c478fcd6af0bb675edd6c8e90348072aad4cd4e5fb9446a5e52706545b90
// SCE-MAP: codec_chain_has_envelope.scxml:23 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 6 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecChainHasEnvelope: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var header: UInt8 = 0

    /// The 5 bits of `header` from bit 0 (flag `kind`). A value
    /// wider than the flag is cut to it when written.
    public var kind: UInt8 {
        get { UInt8(truncatingIfNeeded: (header >> 0) & 0x1F) }
        set { header = (header & ~(0x1F << 0)) | (((UInt8(truncatingIfNeeded: newValue)) & 0x1F) << 0) }
    }

    /// Bit 7 of `header` (flag `E`).
    public var E: Bool {
        get { (header & 0x80) != 0 }
        set { header = newValue ? (header | 0x80) : (header & ~0x80) }
    }

    /// On the wire only when `(header & 0x80) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// Entries of `CodecZenohExtEntry`, one after another, at most 4: until one says it is the last. A frame
    /// with more is refused with `tooManyElements`, as is a value with more at its encode.
    public var extensions: [CodecZenohExtEntry]? = nil

    /// On the wire only when `!_has_extensions_2`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var payload_len: UInt64? = nil

    /// On the wire only when `!_has_extensions_2`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `payload_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var payload: [UInt8]? = nil

    /// On the wire only when `_has_extensions_2`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var slice_count: UInt32? = nil

    /// On the wire only when `_has_extensions_2`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `slice_count` says, each a `CodecChainHasSlice`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var slices: [CodecChainHasSlice]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(header: UInt8, extensions: [CodecZenohExtEntry]?, payload_len: UInt64?, payload: [UInt8]?, slice_count: UInt32?, slices: [CodecChainHasSlice]?) {
        self.header = header
        self.extensions = extensions
        self.payload_len = payload_len
        self.payload = payload
        self.slice_count = slice_count
        self.slices = slices
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecChainHasEnvelope {
        var reader = cursor
        var frame = CodecChainHasEnvelope()
        frame.header = try reader.readInteger(UInt8.self, .big)
        if (frame.header & 0x80) != 0 {
            var extensions_items: [CodecZenohExtEntry] = []
            var extensions_more = false
            for _ in 0..<4 {
                let entry = try CodecZenohExtEntry.decode(&reader)
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
        let _has_extensions_2 = (frame.extensions ?? []).contains(where: { _e in chainIdentifier(_e.extId) == 2 })
        if !_has_extensions_2 {
            frame.payload_len = try reader.readVle(UInt64.self)
        }
        if !_has_extensions_2 {
            if (frame.payload_len ?? 0) > 256 {
                throw CodecError.lengthOutOfRange
            }
            frame.payload = Array(try reader.readSlice(Int(clamping: (frame.payload_len ?? 0))))
        }
        if _has_extensions_2 {
            frame.slice_count = try reader.readVle(UInt32.self)
        }
        if _has_extensions_2 {
            if (frame.slice_count ?? 0) > 4 {
                throw CodecError.tooManyElements
            }
            var slices_items: [CodecChainHasSlice] = []
            for _ in 0..<Int(clamping: (frame.slice_count ?? 0)) {
                slices_items.append(try CodecChainHasSlice.decode(&reader))
            }
            frame.slices = slices_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        let _has_extensions_2 = (self.extensions ?? []).contains(where: { _e in chainIdentifier(_e.extId) == 2 })
        if !_has_extensions_2 {
            if self.payload == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.payload != nil {
            throw CodecError.presentIfMismatch
        }
        if _has_extensions_2 {
            if self.slices == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.slices != nil {
            throw CodecError.presentIfMismatch
        }
        if let value = self.extensions, value.count < 1 || value.count > 4 {
            throw CodecError.tooManyElements
        }
        if let value = self.payload, value.count > 256 {
            throw CodecError.lengthOutOfRange
        }
        if let value = self.slices, value.count > 4 {
            throw CodecError.tooManyElements
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
        if let payload_len = self.payload_len {
            appendVle(payload_len, to: &frame)
        }
        if let payload = self.payload {
            frame += payload
        }
        if let slice_count = self.slice_count {
            appendVle(slice_count, to: &frame)
        }
        if let slices = self.slices {
            for entry in slices {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
