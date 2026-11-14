// SCE-GENERATED — DO NOT EDIT
// source-hash: 7cb5c601bc5bd6648425cb4d614d7be9f82d1e41816b0f69f2fcd5bc5c43d40b
// SCE-MAP: codec_zenoh_value_zbuf.scxml:27 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 5 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohValueZbuf: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var value_len: UInt64 = 0

    /// On the wire only when `(after_shm & 0x1) == 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The bytes, as many as `value_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    public var value: [UInt8]? = nil

    /// On the wire only when `(after_shm & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var encoding: UInt64? = nil

    /// On the wire only when `(after_shm & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var slice_count: UInt64? = nil

    /// On the wire only when `(after_shm & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `slice_count` says, each a `CodecZenohValueSlice`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var slices: [CodecZenohValueSlice]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(value_len: UInt64, value: [UInt8]?, encoding: UInt64?, slice_count: UInt64?, slices: [CodecZenohValueSlice]?) {
        self.value_len = value_len
        self.value = value
        self.encoding = encoding
        self.slice_count = slice_count
        self.slices = slices
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor, after_shm: UInt8) throws(CodecError) -> CodecZenohValueZbuf {
        var reader = cursor
        var frame = CodecZenohValueZbuf()
        frame.value_len = try reader.readVle(UInt64.self)
        if (after_shm & 0x1) == 0 {
            if frame.value_len > 4294967295 {
                throw CodecError.lengthOutOfRange
            }
            frame.value = Array(try reader.readSlice(Int(clamping: frame.value_len)))
        }
        if (after_shm & 0x1) != 0 {
            frame.encoding = try reader.readVle(UInt64.self)
        }
        if (after_shm & 0x1) != 0 {
            frame.slice_count = try reader.readVle(UInt64.self)
        }
        if (after_shm & 0x1) != 0 {
            var slices_items: [CodecZenohValueSlice] = []
            for _ in 0..<Int(clamping: (frame.slice_count ?? 0)) {
                slices_items.append(try CodecZenohValueSlice.decode(&reader))
            }
            frame.slices = slices_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink, after_shm: UInt8) throws(CodecError) {
        if (after_shm & 0x1) == 0 {
            if self.value == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.value != nil {
            throw CodecError.presentIfMismatch
        }
        if (after_shm & 0x1) != 0 {
            if self.encoding == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.encoding != nil {
            throw CodecError.presentIfMismatch
        }
        if (after_shm & 0x1) != 0 {
            if self.slices == nil {
                throw CodecError.presentIfMismatch
            }
        } else if self.slices != nil {
            throw CodecError.presentIfMismatch
        }
        if let value = self.value, value.count > 4294967295 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendVle(self.value_len, to: &frame)
        if let value = self.value {
            frame += value
        }
        if let encoding = self.encoding {
            appendVle(encoding, to: &frame)
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
