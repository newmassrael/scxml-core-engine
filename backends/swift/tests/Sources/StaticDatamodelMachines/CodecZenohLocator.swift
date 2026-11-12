// SCE-GENERATED — DO NOT EDIT
// source-hash: ebedc4a8df9c203d48faa182aa817e4a2c8742ba3aab608142a81b588bd4f50d
// SCE-MAP: codec_zenoh_locator.scxml:26 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 2 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecZenohLocator: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// A variable-length integer: base-128, the least significant group first, at most as many
    /// bytes as its type needs.
    public var locator_len: UInt64 = 0

    /// The text of as many UTF-8 bytes as `locator_len` says, as many as the document's `max-size` admits: a frame that says
    /// more is refused with `lengthOutOfRange`, as is a value with more at its encode.
    /// Bytes that are not UTF-8 are refused with `invalidUtf8`.
    public var locator: String = ""

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(locator_len: UInt64, locator: String) {
        self.locator_len = locator_len
        self.locator = locator
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecZenohLocator {
        var reader = cursor
        var frame = CodecZenohLocator()
        frame.locator_len = try reader.readVle(UInt64.self)
        if frame.locator_len > 255 {
            throw CodecError.lengthOutOfRange
        }
        frame.locator = try reader.readString(Int(clamping: frame.locator_len))
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if self.locator.utf8.count > 255 {
            throw CodecError.lengthOutOfRange
        }
        var frame: [UInt8] = []
        appendVle(self.locator_len, to: &frame)
        frame += Array(self.locator.utf8)
        try sink.writeBytes(frame)
    }
}
