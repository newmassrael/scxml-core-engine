// SCE-GENERATED — DO NOT EDIT
// source-hash: 982595e044dab87d2a02655ddc0eb9f9f68babd78e0aa16f5305c5e6c8b4eb30
// SCE-MAP: codec_zenoh_query_value.scxml:24 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 1 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohQueryValue: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 0

    /// Entries of `CodecZenohValueEntry`, one after another: until one says it is the last. The protocol states no depth.
    public var exts: [CodecZenohValueEntry] = []

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(exts: [CodecZenohValueEntry]) {
        self.exts = exts
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohQueryValue {
        var reader = cursor
        var frame = CodecZenohQueryValue()
        var _prev_exts_after_shm: UInt8 = 0
        var exts_items: [CodecZenohValueEntry] = []
        var exts_more = false
        while true {
            let entry = try CodecZenohValueEntry.decode(&reader, after_shm: _prev_exts_after_shm)
            exts_more = entry.Z
            exts_items.append(entry)
            _prev_exts_after_shm = (chainIdentifier(entry.extId) == 4) ? 1 : 0
            if !exts_more { break }
        }
        frame.exts = exts_items
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.exts.count < 1 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        var _prev_exts_after_shm: UInt8 = 0
        for entry in self.exts {
            var piece = BytesSink()
            try entry.encode(into: &piece, after_shm: _prev_exts_after_shm)
            frame += piece.bytes
            _prev_exts_after_shm = (chainIdentifier(entry.extId) == 4) ? 1 : 0
        }
        try sink.writeBytes(frame)
    }
}
