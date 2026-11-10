// SCE-GENERATED — DO NOT EDIT
// source-hash: f3a316aacf7403325399aec899293549220a4c98b78ad03825aff15e55acb6a9
// SCE-MAP: codec_repeat_present_if_basic.scxml:37 :: _forge_body

// SCE Forge: Auto-generated from Extended SCXML (sce:kind="codec")
// Runtime: none
// Do not edit — regenerate from the source SCXML file.

import SceForgeRuntime

/// The frame this codec lays on the wire: 3 field(s) of integers, each at its
/// byte offset and in its byte order. A value, so a host that reads one holds a copy.
public struct CodecRepeatPresentIfBasic: Equatable, Sendable {
    /// The fewest bytes a frame can have: its frame is read field by field, and a variable-length
    /// integer or a field of bytes may take more.
    public static let frameLength = 1

    /// The field at byte 0, big-endian.
    public var carrier: UInt8 = 0

    /// Bit 0 of `carrier` (flag `has_list`).
    public var hasList: Bool {
        get { (carrier & 0x1) != 0 }
        set { carrier = newValue ? (carrier | 0x1) : (carrier & ~0x1) }
    }

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// The field at byte 1, big-endian.
    public var num_elems: UInt8? = nil

    /// On the wire only when `(carrier & 0x1) != 0`; `nil` when it is not. An encode
    /// writes it when it is not `nil`.
    /// As many entries as `num_elems` says, each a `CodecRepeatElem`, as many as the document's
    /// `max-count` admits: a frame that says more is refused with `tooManyElements`, as is a value
    /// with more at its encode.
    public var elems: [CodecRepeatElem]? = nil

    /// A frame of zeros, but for the bits a flag's `value` sets: what a holder that has read no
    /// frame yet holds, as a validator that composes this codec does until it is given one.
    public init() {}

    public init(carrier: UInt8, num_elems: UInt8?, elems: [CodecRepeatElem]?) {
        self.carrier = carrier
        self.num_elems = num_elems
        self.elems = elems
    }

    /// Decodes the frame at the cursor and moves the cursor past it. The frame is read field by
    /// field through a copy of the cursor, which replaces it only when the whole frame was read: a
    /// frame that is short, or says more than it holds, answers `needMoreBytes`, and a variable-
    /// length integer wider than its type `vleWidthOverflow`, and either leaves the cursor where
    /// it was. A count the wire states is clamped and then added to, so a huge one wraps negative
    /// and is refused like any other count past the end, and is no trap.
    public static func decode(_ cursor: inout SceCursor) throws(CodecError) -> CodecRepeatPresentIfBasic {
        var reader = cursor
        var frame = CodecRepeatPresentIfBasic()
        frame.carrier = try reader.readInteger(UInt8.self, .big)
        if (frame.carrier & 0x1) != 0 {
            frame.num_elems = try reader.readInteger(UInt8.self, .big)
        }
        if (frame.carrier & 0x1) != 0 {
            if (frame.num_elems ?? 0) > 32 {
                throw CodecError.tooManyElements
            }
            var elems_items: [CodecRepeatElem] = []
            for _ in 0..<Int(clamping: (frame.num_elems ?? 0)) {
                elems_items.append(try CodecRepeatElem.decode(&reader))
            }
            frame.elems = elems_items
        }
        cursor = reader
        return frame
    }

    /// Writes the frame to `sink` in one write: a bounded sink that cannot hold all of it answers
    /// `bufferOverflow` and holds none of it.
    public func encode<Sink: SceSink>(into sink: inout Sink) throws(CodecError) {
        if let value = self.elems, value.count > 32 {
            throw CodecError.tooManyElements
        }
        var frame: [UInt8] = []
        appendInteger(self.carrier, to: &frame, .big)
        if let num_elems = self.num_elems {
            appendInteger(num_elems, to: &frame, .big)
        }
        if let elems = self.elems {
            for entry in elems {
                var piece = BytesSink()
                try entry.encode(into: &piece)
                frame += piece.bytes
            }
        }
        try sink.writeBytes(frame)
    }
}
